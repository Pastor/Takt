use crate::context::Context;
use crate::predicate::create_predicate;
use crate::unit::{Execution, Predicate, Unit};
use crate::value::Value;
use grammar::diagnostics::{Diagnostic, Location};
use grammar::semantic::extend::Extend;
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
/// Это гарантирует, что runtime-изменения идут в кэш, а не в ModelNode-эталон.
struct ModelNodeContext {
    model: Rc<RefCell<ModelNode>>,
    cache: RefCell<HashMap<String, Value>>,
    parent: Option<Box<ModelNodeContext>>,
}

impl ModelNodeContext {
    fn new(model: Rc<RefCell<ModelNode>>) -> Self {
        let parent = {
            let borrowed = model.borrow();
            borrowed
                .upper
                .as_ref()
                .and_then(|w| w.upgrade())
                .map(|parent_rc| Box::new(ModelNodeContext::new(parent_rc)))
        };
        Self {
            model,
            cache: RefCell::new(HashMap::new()),
            parent,
        }
    }
}

impl Context for ModelNodeContext {
    fn get_value(&self, name: &str) -> Option<Value> {
        // Шаг 1: проверяем кэш.
        if let Some(v) = self.cache.borrow().get(name) {
            return Some(v.clone());
        }
        // Шаг 2: запрашиваем переменную напрямую из ModelNode.
        let value = {
            let borrowed = self.model.borrow();
            borrowed
                .variables
                .get(name)
                .and_then(|var| eval_expr(var_expr(var)))
        };
        if let Some(value) = value {
            // Шаг 3: копируем в кэш — последующие изменения идут сюда, ModelNode остаётся нетронутым.
            self.cache
                .borrow_mut()
                .insert(name.to_string(), value.clone());
            return Some(value);
        }
        // Шаг 4: поднимаемся к родительской модели (ModelNode.upper).
        self.parent.as_ref().and_then(|p| p.get_value(name))
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
        ExpressionNode::Array(items) => {
            let values: Option<Vec<Value>> = items.iter().map(eval_expr).collect();
            Some(Value::Array(values?))
        }
        _ => None,
    }
}

// ── Точка входа ───────────────────────────────────────────────────────────────

/// Строит дерево [`Unit`] из семантической модели.
///
/// - Модель с состояниями (`has_states()`) → [`Unit::Node`]
/// - Иначе: делегирует в `build_extend` по полю `implements`
pub(crate) fn build(model: Rc<RefCell<ModelNode>>) -> Result<Unit, Diagnostic> {
    let has_states = model.borrow().has_states();
    if has_states {
        build_node(model)
    } else {
        let extends = model.borrow().implements.clone();
        build_extend(&extends)
    }
}

// ── Unit::Node ────────────────────────────────────────────────────────────────

fn build_node(model: Rc<RefCell<ModelNode>>) -> Result<Unit, Diagnostic> {
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

    // Снимок состояний: освобождаем borrow перед созданием контекста.
    let states_snapshot: Vec<(String, StateNode)> = {
        let borrowed = model.borrow();
        borrowed
            .states
            .iter()
            .map(|(k, v)| (k.clone(), v.clone()))
            .collect()
    };

    let mut state_transitions: HashMap<String, Vec<(String, Predicate)>> = HashMap::new();
    let mut state_executions: HashMap<String, Executions> = HashMap::new();

    for (name, state_node) in &states_snapshot {
        state_transitions.insert(name.clone(), build_transitions(state_node)?);
        state_executions.insert(name.clone(), HashMap::new());
    }

    // Контекст: прямая ссылка на ModelNode, иерархия через ModelNode.upper.
    let ctx: Rc<RefCell<dyn Context>> = Rc::new(RefCell::new(ModelNodeContext::new(model)));

    Ok(Unit::Node {
        context: Some(ctx),
        state_transitions,
        state_executions,
        state: Some(start_name),
        variables: HashMap::new(),
        executions: HashMap::new(),
    })
}

fn build_transitions(state: &StateNode) -> Result<Vec<(String, Predicate)>, Diagnostic> {
    state
        .references()
        .iter()
        .map(|r| {
            // create_predicate паникует при ConditionNode::None — создаём предикат вручную.
            let pred = if matches!(r.cond, ConditionNode::None) {
                Predicate::new("Always", |_| true)
            } else {
                create_predicate(&r.cond)
            };
            Ok((r.name.clone(), pred))
        })
        .collect()
}

// ── Unit из Extend ────────────────────────────────────────────────────────────

fn build_extend(extend: &Extend) -> Result<Unit, Diagnostic> {
    match extend {
        Extend::None | Extend::Unresolved(_) => Ok(Unit::None),
        Extend::Model(rc) => build(Rc::clone(rc)),
        Extend::Parentless(inner) => build_extend(inner),
        // Concatenation → Sequential: Unit::None.add(&u) == u (нейтральный элемент).
        Extend::Concatenation(items) => items
            .iter()
            .try_fold(Unit::None, |acc, item| Ok(acc.add(&build_extend(item)?))),
        // Parallel: Unit::None.union(&u) == u (нейтральный элемент).
        Extend::Parallel(items) => items
            .iter()
            .try_fold(Unit::None, |acc, item| Ok(acc.union(&build_extend(item)?))),
    }
}

// ── Тесты ────────────────────────────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use super::*;
    use grammar::parse;
    use grammar::semantic::tree::construct_model;

    // ── ModelNodeContext ──────────────────────────────────────────────────────

    /// get_value возвращает None для переменной, которой нет ни в модели, ни в родителе.
    #[test]
    fn test_model_node_context_missing_returns_none() {
        let model = Rc::new(RefCell::new(ModelNode::default()));
        let ctx = ModelNodeContext::new(model);
        assert!(ctx.get_value("x").is_none());
    }

    /// get_value читает Number из ModelNode напрямую (не из копии).
    #[test]
    fn test_model_node_context_number_var() {
        let (ast, _) = parse("var x: u8 = 42;", 0).unwrap();
        let model_rc = construct_model(&ast, None, &[]).unwrap();
        let ctx = ModelNodeContext::new(Rc::clone(&model_rc));
        assert!(matches!(ctx.get_value("x"), Some(Value::Number(42))));
    }

    /// get_value читает Boolean из ModelNode напрямую.
    #[test]
    fn test_model_node_context_bool_var() {
        let (ast, _) = parse("var flag: bool = false;", 0).unwrap();
        let model_rc = construct_model(&ast, None, &[]).unwrap();
        let ctx = ModelNodeContext::new(Rc::clone(&model_rc));
        assert!(matches!(ctx.get_value("flag"), Some(Value::Boolean(false))));
    }

    /// Кэш перекрывает значение из ModelNode (ленивое копирование работает).
    #[test]
    fn test_model_node_context_cache_takes_priority() {
        let (ast, _) = parse("var x: u8 = 10;", 0).unwrap();
        let model_rc = construct_model(&ast, None, &[]).unwrap();
        let ctx = ModelNodeContext::new(Rc::clone(&model_rc));
        assert!(matches!(ctx.get_value("x"), Some(Value::Number(10))));
        // Подменяем кэш — имитация runtime-изменения.
        ctx.cache
            .borrow_mut()
            .insert("x".to_string(), Value::Number(99));
        assert!(matches!(ctx.get_value("x"), Some(Value::Number(99))));
        // ModelNode не изменён: исходное значение сохранено.
        assert!(model_rc.borrow().variables.contains_key("x"));
    }

    /// Иерархия: переменная из родительской модели доступна через цепочку parent.
    #[test]
    fn test_model_node_context_parent_hierarchy() {
        let (ast, _) = parse("var outer_var: u8 = 77; model Inner { start S; }", 0).unwrap();
        let root_rc = construct_model(&ast, None, &[]).unwrap();
        let inner_rc = root_rc.borrow().search_model("Inner").unwrap();
        // Inner.upper должен указывать на root.
        let ctx = ModelNodeContext::new(Rc::clone(&inner_rc));
        // Переменной outer_var нет в Inner → должна найтись в parent (root).
        let val = ctx.get_value("outer_var");
        assert!(
            matches!(val, Some(Value::Number(77))),
            "переменная из родительской модели должна быть доступна через иерархию контекста"
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

    /// Пустая модель → Unit::None.
    #[test]
    fn test_build_empty_model_returns_none() {
        let model = Rc::new(RefCell::new(ModelNode::default()));
        assert!(matches!(build(model).unwrap(), Unit::None));
    }

    /// "start S;" → Unit::Node { state: Some("S") }.
    #[test]
    fn test_build_single_state_model() {
        let (ast, _) = parse("start S;", 0).unwrap();
        let model_rc = construct_model(&ast, None, &[]).unwrap();
        let result = build(model_rc).unwrap();
        let Unit::Node { state: Some(s), .. } = &result else {
            panic!("ожидался Unit::Node");
        };
        assert_eq!(s, "S");
    }

    /// ModelNode без start-состояния → build_node → Err.
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

    /// Безусловный `ref B` → Predicate всегда true.
    #[test]
    fn test_build_unconditional_transition() {
        let (ast, _) = parse("start A { ref B; } state B;", 0).unwrap();
        let model_rc = construct_model(&ast, None, &[]).unwrap();
        let result = build(model_rc).unwrap();
        let Unit::Node {
            state_transitions, ..
        } = &result
        else {
            panic!("ожидался Unit::Node");
        };
        let trans = state_transitions.get("A").unwrap();
        assert_eq!(trans.len(), 1);
        assert_eq!(trans[0].0, "B");
        assert!(trans[0].1.evaluate(&Unit::None));
    }

    /// Переменная из ModelNode доступна через context при get_value (Unit::Node.variables пуст).
    #[test]
    fn test_build_node_has_context_with_variable() {
        let (ast, _) = parse("var x: u8 = 5; start S;", 0).unwrap();
        let model_rc = construct_model(&ast, None, &[]).unwrap();
        let result = build(model_rc).unwrap();
        assert!(matches!(result.get_value("x"), Some(Value::Number(5))));
    }

    /// Запись в Unit::Node.variables перекрывает значение из context.
    #[test]
    fn test_build_node_variables_shadow_context() {
        let (ast, _) = parse("var x: u8 = 5; start S;", 0).unwrap();
        let model_rc = construct_model(&ast, None, &[]).unwrap();
        let mut result = build(model_rc).unwrap();
        if let Unit::Node { variables, .. } = &mut result {
            variables.insert("x".to_string(), Value::Number(99));
        }
        assert!(matches!(result.get_value("x"), Some(Value::Number(99))));
    }

    /// Extend::Concatenation → Unit::Sequential с двумя дочерними.
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
        let result = build_extend(implements).unwrap();
        let Unit::Sequential { units, .. } = &result else {
            panic!("ожидался Unit::Sequential");
        };
        assert_eq!(units.len(), 2);
    }

    /// Extend::Parallel → Unit::Parallel с двумя дочерними.
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
        let result = build_extend(implements).unwrap();
        let Unit::Parallel { units, .. } = &result else {
            panic!("ожидался Unit::Parallel");
        };
        assert_eq!(units.len(), 2);
    }
}
