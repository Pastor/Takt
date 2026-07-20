use crate::context::Context;
use crate::eval::value::Value;
use crate::predicate::create_predicate;
use crate::unit::statement::compile_block_body;
use crate::unit::{Execution, Predicate, Unit, UnitKind};
use grammar::diagnostics::{Diagnostic, Location};
use grammar::semantic::extend::Extend;
use grammar::semantic::type_node::TypeNode;
use grammar::semantic::{
    ConditionNode, ExpressionNode, ModelNode, StateNode, StateNodeKind, VariableNode,
};
use std::cell::RefCell;
use std::collections::HashMap;
use std::rc::Rc;

type Executions = HashMap<String, Vec<Execution>>;

// ── ModelNodeContext ──────────────────────────────────────────────────────────

/// Контекст с иерархической структурой, зеркалирующей цепочку ModelNode.upper.
///
/// Хранит прямую ссылку на ModelNode (Rc) — переменные не копируются.
/// При запросе переменной:
/// 1. Проверяет локальный кэш (Value уже вычислен ранее).
/// 2. Запрашивает `model.variables` напрямую, вычисляет Value из ExpressionNode.
/// 3. Копирует результат в кэш (ленивая инициализация).
/// 4. Если в текущей модели не найдено — поднимается к `parent` (ModelNode.upper).
///
/// Для параллельных моделей `parent` является общим (`Rc`) — изменения одной
/// подмодели сразу видны остальным через общий родительский контекст.
struct ModelNodeContext {
    model: Rc<RefCell<ModelNode>>,
    cache: RefCell<HashMap<String, Value>>,
    parent: Option<Rc<RefCell<dyn Context>>>,
}

impl ModelNodeContext {
    fn new(model: Rc<RefCell<ModelNode>>) -> Self {
        let parent = model
            .borrow()
            .upper
            .as_ref()
            .and_then(|w| w.upgrade())
            .map(|parent_rc| {
                Rc::new(RefCell::new(ModelNodeContext::new(parent_rc))) as Rc<RefCell<dyn Context>>
            });
        Self {
            model,
            cache: RefCell::new(HashMap::new()),
            parent,
        }
    }

    fn new_with_parent(
        model: Rc<RefCell<ModelNode>>,
        parent: Option<Rc<RefCell<dyn Context>>>,
    ) -> Self {
        Self {
            model,
            cache: RefCell::new(HashMap::new()),
            parent,
        }
    }
}

impl Context for ModelNodeContext {
    fn get_value(&self, name: &str) -> Option<Value> {
        if let Some(v) = self.cache.borrow().get(name) {
            return Some(v.clone());
        }
        let value = {
            let borrowed = self.model.borrow();
            borrowed.variables.get(name).and_then(|var| {
                match eval_expr(var_expr(var)) {
                    Some(v) => Some(coerce_initial(v, var, &borrowed)),
                    // Переменная без инициализатора → нулевое значение по типу
                    // (как default-init в C). Прежде так делалась только структура
                    // (фича 0034), а скаляр (`var q: u8;`) оставался
                    // незарегистрированным → SIM-009 (гэп 0034-04). Фича 0086
                    // распространяет политику на все типы: `default_field`
                    // покрывает bool/rational/fixed/array/struct/целое единообразно.
                    None => var_type(var).map(|ty| default_field(ty, &borrowed)),
                }
            })
        };
        if let Some(value) = value {
            self.cache
                .borrow_mut()
                .insert(name.to_string(), value.clone());
            return Some(value);
        }
        self.parent
            .as_ref()
            .and_then(|p| p.borrow().get_value(name))
    }

    fn set_value(&mut self, name: &str, value: Value) {
        if self.model.borrow().variables.contains_key(name) {
            self.cache.borrow_mut().insert(name.to_string(), value);
        } else if let Some(parent) = &self.parent {
            parent.borrow_mut().set_value(name, value);
        } else {
            self.cache.borrow_mut().insert(name.to_string(), value);
        }
    }

    /// Определение структуры по имени (фича 0034): `search_struct` учитывает
    /// родительские модели по слабым ссылкам `upper`.
    fn find_struct(&self, name: &str) -> Option<grammar::semantic::StructDefinitionNode> {
        self.model.borrow().search_struct(name)
    }

    /// Перечисляет значения состояния модели для снимка (фича 0032).
    ///
    /// Идёт по именам `model.variables`, вычисляя значение через собственный
    /// `get_value` (что попутно материализует ленивый кэш). **Константы
    /// исключаются** — их значение задано исходником, восстанавливать из файла
    /// опасно (исходник мог измениться). Родитель накладывается **первым**, затем
    /// перекрывается значениями текущей модели — та же приоритетность, что у
    /// `get_value` (локальное имя выигрывает у родительского).
    fn dump(&self) -> HashMap<String, Value> {
        let mut out: HashMap<String, Value> = self
            .parent
            .as_ref()
            .map(|p| p.borrow().dump())
            .unwrap_or_default();
        let names: Vec<String> = {
            let borrowed = self.model.borrow();
            borrowed
                .variables
                .iter()
                .filter(|(_, var)| !matches!(var, VariableNode::Const { .. }))
                .map(|(name, _)| name.clone())
                .collect()
        };
        for name in names {
            if let Some(value) = self.get_value(&name) {
                out.insert(name, value);
            }
        }
        out
    }
}

fn var_expr(var: &VariableNode) -> &ExpressionNode {
    match var {
        VariableNode::Simple { expr, .. }
        | VariableNode::Port { expr, .. }
        | VariableNode::Const { expr, .. } => expr,
        VariableNode::Unresolved => &ExpressionNode::None,
    }
}

/// Объявленный тип переменной (или `None` для `Unresolved`).
fn var_type(var: &VariableNode) -> Option<&TypeNode> {
    match var {
        VariableNode::Simple { ty, .. }
        | VariableNode::Port { ty, .. }
        | VariableNode::Const { ty, .. } => Some(ty),
        VariableNode::Unresolved => None,
    }
}

/// Приводит начальное значение к типу `q(m, n)`, чтобы переменная хранилась как
/// [`Value::Fixed`] (фича 0061). Иначе арифметика над ней пошла бы целочисленным
/// путём (представление без сдвига у `*`/`/`) → молча неверный результат.
///
/// Для **прочих** типов начальное значение оставляем как есть: историческое
/// поведение симулятора (усечение init по типу не выполнялось, а его добавление —
/// смена поведения вне объёма этой фичи). Литерал `q` уже понижен грамматикой в
/// представление, поэтому `coerce_to_type(Number, Fixed)` трактует его как сырой
/// repr — двойного масштабирования нет.
fn coerce_initial(value: Value, var: &VariableNode, model: &ModelNode) -> Value {
    match var_type(var) {
        Some(ty @ TypeNode::Fixed { .. }) => {
            crate::eval::coerce_to_type(value.clone(), ty).unwrap_or(value)
        }
        // Структура (фича 0034): инициализатор `{…}` (пришёл как `Array`)
        // приводится к `Value::Struct` по определению из модели. При неудаче
        // (арность/тип) значение остаётся `Array` — ошибка тогда всплывёт при
        // ДОСТУПЕ к полю (`read_member` → `FieldOfNonStruct`), а не молча: та же
        // консервативность, что и у `Fixed` выше (get_value ленив и Result не
        // возвращает — задача сузить его контракт вынесена, см. 0034-04 п.4).
        Some(ty @ TypeNode::Struct(_)) => {
            crate::eval::coerce_to_type_with(value.clone(), ty, &ModelStructs(model))
                .unwrap_or(value)
        }
        _ => value,
    }
}

/// Вычисляет простое константное выражение в Value.
fn eval_expr(expr: &ExpressionNode) -> Option<Value> {
    match expr {
        ExpressionNode::Number(n) => Some(Value::Number(*n)),
        ExpressionNode::Bool(b) => Some(Value::Boolean(*b)),
        ExpressionNode::Rational(s, neg) => {
            let v: f64 = s.parse().ok()?;
            Some(Value::Real(if *neg { -v } else { v }))
        }
        ExpressionNode::Negate(inner) => match eval_expr(inner)? {
            Value::Number(n) => Some(Value::Number(-n)),
            Value::Real(f) => Some(Value::Real(-f)),
            _ => None,
        },
        ExpressionNode::Parenthesis(inner) => eval_expr(inner),
        // Инициализатор `{…}` структуры (фича 0034) и массивный литерал `[…]` —
        // оба дают список значений; тип цели (структура/массив) различит
        // `coerce_initial` по объявленному типу переменной.
        ExpressionNode::Array(items) | ExpressionNode::Initializer(items) => {
            let values: Option<Vec<Value>> = items.iter().map(eval_expr).collect();
            Some(Value::Array(values?))
        }
        // Адресные порты (bit = 0x600:0) инициализируются нулём по умолчанию
        ExpressionNode::Address(_, _) => Some(Value::Number(0)),
        _ => None,
    }
}

/// Значение поля по умолчанию (нулевое) по его типу — для структуры без
/// инициализатора (фича 0034). Совпадает с default-init полей структуры в C.
fn default_field(ty: &TypeNode, model: &ModelNode) -> Value {
    match ty {
        TypeNode::Bool => Value::Boolean(false),
        TypeNode::Rational => Value::Real(0.0),
        TypeNode::Fixed { m, n } => Value::Fixed {
            repr: 0,
            m: *m,
            n: *n,
        },
        TypeNode::Array(size, elem) => {
            Value::Array((0..*size).map(|_| default_field(elem, model)).collect())
        }
        TypeNode::Struct(name) => model
            .search_struct(name)
            .map(|def| Value::Struct {
                name: name.clone(),
                fields: def
                    .fields
                    .iter()
                    .map(|(f, t)| (f.clone(), default_field(t, model)))
                    .collect(),
            })
            .unwrap_or(Value::Number(0)),
        // Integer/Enum/Bit/Address/прочее — целочисленный ноль.
        _ => Value::Number(0),
    }
}

/// Реестр структур над семантической моделью (фича 0034): поиск учитывает
/// родительские модели через [`ModelNode::search_struct`].
struct ModelStructs<'a>(&'a ModelNode);

impl crate::eval::StructRegistry for ModelStructs<'_> {
    fn find_struct(&self, name: &str) -> Option<grammar::semantic::StructDefinitionNode> {
        self.0.search_struct(name)
    }
}

// ── Точка входа ───────────────────────────────────────────────────────────────

/// Строит дерево [`Unit`] из семантической модели.
pub fn build(model: Rc<RefCell<ModelNode>>) -> Result<Unit, Diagnostic> {
    build_impl(model, None)
}

fn build_impl(
    model: Rc<RefCell<ModelNode>>,
    shared_parent: Option<Rc<RefCell<dyn Context>>>,
) -> Result<Unit, Diagnostic> {
    let has_states = model.borrow().has_states();
    if has_states {
        // Если модель содержит ровно одно состояние-реализацию без переходов
        // (например, `start Stacker = CR | MC | LC;`), делегируем в build_extend.
        let single_compound = {
            let borrowed = model.borrow();
            if borrowed.states.len() == 1 {
                borrowed.states.values().next().and_then(|state| {
                    if let StateNode::Implement {
                        implements,
                        references,
                        next,
                        ..
                    } = state
                    {
                        if references.is_empty() && next.is_none() {
                            Some(implements.clone())
                        } else {
                            None
                        }
                    } else {
                        None
                    }
                })
            } else {
                None
            }
        };
        if let Some(implements) = single_compound {
            build_extend(&implements, shared_parent)
        } else {
            build_node(model, shared_parent)
        }
    } else {
        let extends = model.borrow().implements.clone();
        build_extend(&extends, shared_parent)
    }
}

// ── Unit::Node ────────────────────────────────────────────────────────────────

fn build_node(
    model: Rc<RefCell<ModelNode>>,
    shared_parent: Option<Rc<RefCell<dyn Context>>>,
) -> Result<Unit, Diagnostic> {
    let start_name = {
        let borrowed = model.borrow();
        borrowed
            .states
            .iter()
            .find_map(|(name, state)| match state {
                StateNode::Simple {
                    kind: StateNodeKind::Start,
                    ..
                }
                | StateNode::Implement {
                    kind: StateNodeKind::Start,
                    ..
                } => Some(name.clone()),
                _ => None,
            })
            .ok_or_else(|| {
                Diagnostic::error(
                    Location::Builtin,
                    "Нет начального состояния в модели".to_string(),
                )
            })
    }?;

    let states_snapshot: Vec<(String, StateNode)> = {
        let borrowed = model.borrow();
        borrowed
            .states
            .iter()
            .map(|(k, v)| (k.clone(), v.clone()))
            .collect()
    };

    // Контекст: используем общий родительский если передан, иначе строим иерархию.
    let ctx_rc: Rc<RefCell<dyn Context>> =
        Rc::new(RefCell::new(if let Some(shared) = shared_parent {
            ModelNodeContext::new_with_parent(model.clone(), Some(shared))
        } else {
            ModelNodeContext::new(model.clone())
        }));

    let mut state_transitions: HashMap<String, Vec<(String, Predicate)>> = HashMap::new();
    let mut state_executions: HashMap<String, Executions> = HashMap::new();
    // 0044: инварианты модели (проверяются каждый такт) и по состояниям.
    let mut guards = crate::unit::Guards {
        model: build_guards(&model.borrow().formulas),
        per_state: HashMap::new(),
    };

    for (name, state_node) in &states_snapshot {
        state_transitions.insert(name.clone(), build_transitions(state_node)?);
        let state_guards = build_guards(state_node.formulas());
        if !state_guards.is_empty() {
            guards.per_state.insert(name.clone(), state_guards);
        }

        let mut execs: Executions = HashMap::new();
        for block in state_node.named_blocks() {
            let kind = block.name();
            if kind.is_empty() {
                continue;
            }
            if let Some(body) = block.statement() {
                let fns = compile_block_body(body, ctx_rc.clone());
                if !fns.is_empty() {
                    execs.entry(kind.to_string()).or_default().extend(fns);
                }
            }
        }
        state_executions.insert(name.clone(), execs);
    }

    Ok(Unit::from_kind(UnitKind::Node {
        entered_initial: false,
        context: Some(ctx_rc),
        state_transitions,
        state_executions,
        state: Some(start_name),
        executions: HashMap::new(),
        guards,
        last_transition: None,
    }))
}

/// Строит проверяемые обязательства из формул (фича 0044). `Formula::Guard` →
/// предикат условия + имя инварианта; `Formula::LTL` — статика, симулятором
/// игнорируется (проверяется верификатором, не здесь); `None`/`Formulas`
/// разворачиваются/пропускаются.
fn build_guards(formulas: &[grammar::semantic::formula::Formula]) -> Vec<crate::unit::Guard> {
    use grammar::semantic::formula::Formula;
    let mut out = Vec::new();
    for f in formulas {
        match f {
            Formula::Guard(cond, name) => out.push((create_predicate(cond), name.clone())),
            Formula::Formulas(inner) => out.extend(build_guards(inner)),
            Formula::LTL(_) | Formula::None => {}
        }
    }
    out
}

fn build_transitions(state: &StateNode) -> Result<Vec<(String, Predicate)>, Diagnostic> {
    state
        .references()
        .iter()
        .map(|r| {
            let pred = if matches!(r.cond, ConditionNode::None) {
                Predicate::new("Always", |_| Ok(true))
            } else {
                create_predicate(&r.cond)
            };
            Ok((r.name.clone(), pred))
        })
        .collect()
}

// ── Unit из Extend ────────────────────────────────────────────────────────────

fn build_extend(
    extend: &Extend,
    shared_parent: Option<Rc<RefCell<dyn Context>>>,
) -> Result<Unit, Diagnostic> {
    match extend {
        Extend::None | Extend::Unresolved(_) => Ok(Unit::default()),
        Extend::Model(rc, _) => build_impl(Rc::clone(rc), shared_parent),
        Extend::Parentless(inner) => build_extend(inner, shared_parent),
        Extend::Concatenation(items) => items.iter().try_fold(Unit::default(), |acc, item| {
            Ok(acc.add(&build_extend(item, shared_parent.clone())?))
        }),
        Extend::Parallel(items) => {
            // Все параллельные подмодели разделяют один общий родительский контекст —
            // это позволяет передавать shared-переменные (busy, tgt_*, lift_*) между ними.
            let shared: Option<Rc<RefCell<dyn Context>>> = if shared_parent.is_some() {
                shared_parent
            } else {
                items
                    .first()
                    .and_then(|first| extract_parent_model(first))
                    .map(|parent_model| {
                        Rc::new(RefCell::new(ModelNodeContext::new(parent_model)))
                            as Rc<RefCell<dyn Context>>
                    })
            };
            items.iter().try_fold(Unit::default(), |acc, item| {
                Ok(acc.union(&build_extend(item, shared.clone())?))
            })
        }
    }
}

/// Извлекает родительскую модель из Extend (нужна для построения shared-контекста).
fn extract_parent_model(extend: &Extend) -> Option<Rc<RefCell<ModelNode>>> {
    match extend {
        Extend::Model(rc, _) => rc.borrow().upper.as_ref()?.upgrade(),
        Extend::Parentless(inner) => extract_parent_model(inner),
        _ => None,
    }
}

// ── Тесты ────────────────────────────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use super::*;
    use grammar::parse;
    use grammar::semantic::tree::construct_model;

    // ── ModelNodeContext ──────────────────────────────────────────────────────

    #[test]
    fn test_model_node_context_missing_returns_none() {
        let model = Rc::new(RefCell::new(ModelNode::default()));
        let ctx = ModelNodeContext::new(model);
        assert!(ctx.get_value("x").is_none());
    }

    #[test]
    fn test_model_node_context_number_var() {
        let (ast, _) = parse("var x: u8 := 42;", 0).unwrap();
        let model_rc = construct_model(&ast, None, &[]).unwrap();
        let ctx = ModelNodeContext::new(Rc::clone(&model_rc));
        assert!(matches!(ctx.get_value("x"), Some(Value::Number(42))));
    }

    #[test]
    fn test_model_node_context_bool_var() {
        let (ast, _) = parse("var flag: bool := false;", 0).unwrap();
        let model_rc = construct_model(&ast, None, &[]).unwrap();
        let ctx = ModelNodeContext::new(Rc::clone(&model_rc));
        assert!(matches!(ctx.get_value("flag"), Some(Value::Boolean(false))));
    }

    #[test]
    fn test_model_node_context_cache_takes_priority() {
        let (ast, _) = parse("var x: u8 := 10;", 0).unwrap();
        let model_rc = construct_model(&ast, None, &[]).unwrap();
        let ctx = ModelNodeContext::new(Rc::clone(&model_rc));
        assert!(matches!(ctx.get_value("x"), Some(Value::Number(10))));
        ctx.cache
            .borrow_mut()
            .insert("x".to_string(), Value::Number(99));
        assert!(matches!(ctx.get_value("x"), Some(Value::Number(99))));
        assert!(model_rc.borrow().variables.contains_key("x"));
    }

    #[test]
    fn test_model_node_context_parent_hierarchy() {
        let (ast, _) = parse("var outer_var: u8 := 77; model Inner { start S; }", 0).unwrap();
        let root_rc = construct_model(&ast, None, &[]).unwrap();
        let inner_rc = root_rc.borrow().search_model("Inner").unwrap();
        let ctx = ModelNodeContext::new(Rc::clone(&inner_rc));
        let val = ctx.get_value("outer_var");
        assert!(
            matches!(val, Some(Value::Number(77))),
            "переменная из родительской модели должна быть доступна через иерархию контекста"
        );
    }

    #[test]
    fn test_model_node_context_set_value_delegates_to_parent() {
        let (ast, _) = parse("var shared: u8 := 0; model Inner { start S; }", 0).unwrap();
        let root_rc = construct_model(&ast, None, &[]).unwrap();
        let inner_rc = root_rc.borrow().search_model("Inner").unwrap();

        // Создаём shared parent context (как при построении Parallel)
        let shared_parent: Rc<RefCell<dyn Context>> =
            Rc::new(RefCell::new(ModelNodeContext::new(Rc::clone(&root_rc))));

        let mut inner_ctx =
            ModelNodeContext::new_with_parent(inner_rc, Some(shared_parent.clone()));

        // Записываем через inner_ctx — должно делегироваться в shared_parent
        inner_ctx.set_value("shared", Value::Number(42));

        // Читаем через shared_parent — должно вернуть 42
        assert!(
            matches!(
                shared_parent.borrow().get_value("shared"),
                Some(Value::Number(42))
            ),
            "set_value должен делегировать в shared parent для не-локальных переменных"
        );
    }

    // ── eval_expr ─────────────────────────────────────────────────────────────

    #[test]
    fn test_eval_expr_number() {
        assert!(matches!(
            eval_expr(&ExpressionNode::Number(7)),
            Some(Value::Number(7))
        ));
    }

    #[test]
    fn test_eval_expr_bool() {
        assert!(matches!(
            eval_expr(&ExpressionNode::Bool(true)),
            Some(Value::Boolean(true))
        ));
    }

    #[test]
    fn test_eval_expr_negate() {
        assert!(matches!(
            eval_expr(&ExpressionNode::Negate(Box::new(ExpressionNode::Number(5)))),
            Some(Value::Number(-5))
        ));
    }

    #[test]
    fn test_eval_expr_none_returns_none() {
        assert!(eval_expr(&ExpressionNode::None).is_none());
    }

    // ── build ─────────────────────────────────────────────────────────────────

    #[test]
    fn test_build_empty_model_returns_none() {
        let model = Rc::new(RefCell::new(ModelNode::default()));
        assert!(matches!(build(model).unwrap().kind(), UnitKind::None));
    }

    #[test]
    fn test_build_single_state_model() {
        let (ast, _) = parse("start S;", 0).unwrap();
        let model_rc = construct_model(&ast, None, &[]).unwrap();
        let result = build(model_rc).unwrap();
        let UnitKind::Node { state: Some(s), .. } = result.kind() else {
            panic!("ожидался Unit::Node");
        };
        assert_eq!(s, "S");
    }

    #[test]
    fn test_build_model_without_start_state_returns_err() {
        let mut model = ModelNode::default();
        model.states.insert(
            "S".to_string(),
            StateNode::Simple {
                upper: None,
                loc: Location::Implicit,
                named_blocks: vec![],
                name: "S".to_string(),
                references: vec![],
                kind: StateNodeKind::Simple,
                formulas: vec![],
            },
        );
        assert!(build(Rc::new(RefCell::new(model))).is_err());
    }

    #[test]
    fn test_build_unconditional_transition() {
        let (ast, _) = parse("start A { ref B; } state B;", 0).unwrap();
        let model_rc = construct_model(&ast, None, &[]).unwrap();
        let result = build(model_rc).unwrap();
        let UnitKind::Node {
            state_transitions, ..
        } = result.kind()
        else {
            panic!("ожидался Unit::Node");
        };
        let trans = state_transitions.get("A").unwrap();
        assert_eq!(trans.len(), 1);
        assert_eq!(trans[0].0, "B");
        assert!(trans[0].1.evaluate(&mut Unit::default()).unwrap());
    }

    #[test]
    fn test_build_node_has_context_with_variable() {
        let (ast, _) = parse("var x: u8 := 5; start S;", 0).unwrap();
        let model_rc = construct_model(&ast, None, &[]).unwrap();
        let result = build(model_rc).unwrap();
        assert!(matches!(result.get_value("x"), Some(Value::Number(5))));
    }

    /// 0032: у узла нет собственной карты значений — запись через `set_value`
    /// уходит в контекст модели и читается оттуда же (единый источник истины).
    /// Прежде тест проверял затенение картой узла; затенения больше нет.
    #[test]
    fn test_build_node_set_value_routes_to_context() {
        let (ast, _) = parse("var x: u8 := 5; start S;", 0).unwrap();
        let model_rc = construct_model(&ast, None, &[]).unwrap();
        let mut result = build(model_rc).unwrap();
        assert!(matches!(result.get_value("x"), Some(Value::Number(5))));
        result.set_value("x", Value::Number(99));
        assert!(matches!(result.get_value("x"), Some(Value::Number(99))));
    }

    #[test]
    fn test_build_concatenation_produces_sequential() {
        let src = "model A { start S; } model B { start S; } start Entry = A + B;";
        let (ast, _) = parse(src, 0).unwrap();
        let model_rc = construct_model(&ast, None, &[]).unwrap();
        let entry = model_rc.borrow().search_state("Entry").unwrap();
        let entry_ref = entry.borrow();
        let StateNode::Implement { implements, .. } = &*entry_ref else {
            panic!("Entry должен быть Implement");
        };
        let result = build_extend(implements, None).unwrap();
        let UnitKind::Sequential { units, .. } = result.kind() else {
            panic!("ожидался Unit::Sequential");
        };
        assert_eq!(units.len(), 2);
    }

    #[test]
    fn test_build_parallel_produces_parallel() {
        let src = "model A { start S; } model B { start S; } start Entry = A | B;";
        let (ast, _) = parse(src, 0).unwrap();
        let model_rc = construct_model(&ast, None, &[]).unwrap();
        let entry = model_rc.borrow().search_state("Entry").unwrap();
        let entry_ref = entry.borrow();
        let StateNode::Implement { implements, .. } = &*entry_ref else {
            panic!("Entry должен быть Implement");
        };
        let result = build_extend(implements, None).unwrap();
        let UnitKind::Parallel { units, .. } = result.kind() else {
            panic!("ожидался Unit::Parallel");
        };
        assert_eq!(units.len(), 2);
    }

    /// Enter-блок корректно записывает переменную через shared-контекст.
    #[test]
    fn test_build_enter_block_executes_on_transition() {
        let src = "var x: u8 := 0; start A { ref B; } state B { enter { x := 99; } }";
        let (ast, _) = parse(src, 0).unwrap();
        let model_rc = construct_model(&ast, None, &[]).unwrap();
        let mut unit = build(model_rc).unwrap();

        // До тика: x = 0 (дефолт)
        assert!(matches!(unit.get_value("x"), Some(Value::Number(0))));

        // Тик: A → B (enter { x = 99; } должен выполниться)
        unit.tick();

        // После тика: x = 99
        assert!(
            matches!(unit.get_value("x"), Some(Value::Number(99))),
            "enter-блок должен установить x=99 при входе в состояние B"
        );
    }

    /// Shared-переменные доступны между параллельными моделями.
    #[test]
    fn test_build_parallel_shared_variable() {
        // Модель A устанавливает shared в 1 при входе в B.
        // Модель B читает shared через свой контекст.
        let src = r#"
            var shared: u8 := 0;
            model A { start Idle { ref Done; } state Done { enter { shared := 7; } } }
            model B { start Check { ref End: shared = 7; } state End; }
            start Root = A | B;
        "#;
        let (ast, _) = parse(src, 0).unwrap();
        let model_rc = construct_model(&ast, None, &[]).unwrap();
        let mut unit = build(model_rc).unwrap();

        // Тик 1: A: Idle→Done (enter shared=7), B: Check (shared=7 → End)
        // A тикает первым, B тикает вторым и видит shared=7
        unit.tick();

        // shared должен быть 7
        assert!(
            matches!(unit.get_value("shared"), Some(Value::Number(7))),
            "shared-переменная должна быть доступна между параллельными моделями"
        );
    }

    #[test]
    fn test_enter_writes_output_port_via_context() {
        let src = r#"
            out cmd_ack: bit := 0;
            in task_valid: bit := 0;
            var busy: bit := 0;
            model CR {
                start Waiting { ref Accepting: task_valid; }
                state Accepting { enter { cmd_ack := 1; busy := 1; } next Done; }
                state Done;
            }
            start Main = CR;
        "#;
        let (ast, _) = parse(src, 0).unwrap();
        let model_rc = construct_model(&ast, None, &[]).unwrap();
        let mut unit = build(model_rc).unwrap();

        // Устанавливаем task_valid=1
        unit.set_value("task_valid", Value::Number(1));

        // До тика: cmd_ack должен быть 0 (дефолт)
        assert!(
            matches!(unit.get_value("cmd_ack"), Some(Value::Number(0)) | None),
            "cmd_ack должен быть 0 до тика, получено {:?}",
            unit.get_value("cmd_ack")
        );

        // Тик: Waiting→Accepting (enter: cmd_ack=1, busy=1)
        unit.tick();

        // После тика: cmd_ack = 1
        assert!(
            matches!(unit.get_value("cmd_ack"), Some(Value::Number(1))),
            "cmd_ack должен быть 1 после тика, получено {:?}",
            unit.get_value("cmd_ack")
        );
        assert!(
            matches!(unit.get_value("busy"), Some(Value::Number(1))),
            "busy должен быть 1 после тика, получено {:?}",
            unit.get_value("busy")
        );
    }

    /// Параллельная модель с Address-портами: enter-блок CR пишет cmd_ack=1.
    #[test]
    fn test_parallel_address_port_enter_write() {
        let src = r#"
            out cmd_ack: bit := 0x600:0;
            in task_valid: bit := 0x100:0;
            var busy: bit := 0;
            model CR {
                start Waiting { ref Accepting: task_valid; }
                state Accepting { enter { cmd_ack := 1; busy := 1; } }
            }
            model MC {
                start Idle { ref Active: busy; }
                state Active;
            }
            start Main = CR | MC;
        "#;
        let (ast, _) = parse(src, 0).unwrap();
        let model_rc = construct_model(&ast, None, &[]).unwrap();
        let mut unit = build(model_rc).unwrap();

        unit.set_value("task_valid", Value::Number(1));
        unit.tick();

        assert!(
            matches!(unit.get_value("cmd_ack"), Some(Value::Number(1))),
            "cmd_ack должен быть 1 после тика, получено {:?}",
            unit.get_value("cmd_ack")
        );
        assert!(
            matches!(unit.get_value("busy"), Some(Value::Number(1))),
            "busy должен быть 1 после тика, получено {:?}",
            unit.get_value("busy")
        );
    }
}
