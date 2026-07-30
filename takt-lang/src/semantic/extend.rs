//! Построение и развёртка структуры [`Extend`] — реализации модели.
//!
//! Модуль предоставляет две публичные функции:
//! - [`unroll_extend_expression`] — разворачивает выражение в плоскую
//!   структуру [`Extend::Concatenation`] / [`Extend::Parallel`].

use crate::diagnostics::{Diagnostic, Location};
use crate::parser::ast;
use crate::semantic::extend_args;
use crate::semantic::{
    ConditionNode, ExpressionNode, ModelNode, ReferenceNode, StateNode, StateNodeKind,
};
use std::cell::RefCell;
use std::fmt::{Display, Formatter};
use std::rc::Rc;

/// Реализация модели: описывает, как состояние или корневой автомат
/// составлен из именованных моделей.
///
/// - [`Unresolved`](Extend::Unresolved) — временная заглушка до второго прохода.
/// - [`Model`](Extend::Model) — ссылка на конкретную именованную модель.
/// - [`Parentless`](Extend::Parentless) — обёртка без родителя (скобки).
/// - [`Concatenation`](Extend::Concatenation) — последовательная компоновка `A + B`.
/// - [`Parallel`](Extend::Parallel) — параллельная компоновка `A | B`.
#[derive(Default, Debug, Clone)]
pub enum Extend {
    /// Реализация не задана (значение по умолчанию для безымянной корневой модели).
    #[default]
    None,
    /// «Сырое» АСД-выражение реализации, ожидающее разрешения на этапе stage1.
    Unresolved(ast::Expression),
    /// Ссылка на конкретную именованную модель.
    ///
    /// Второе поле — позиция **использования** (use-site): где имя модели
    /// написано, а не где она объявлена. Разрешение стирало её, из-за чего
    /// переход к декларации на имени модели был невозможен — узла под курсором
    /// просто не существовало (фича 0056).
    ///
    /// У синтетической модели, собранной [`compact_implement`] для `M1 + M2`,
    /// исходной позиции нет: она несёт [`Location::Codegen`].
    ///
    /// Третье поле — **аргументы инстанцирования** `M(Y := 200)` (фича 0185).
    /// Они привязаны к месту использования, а не к модели: одна и та же модель
    /// в двух местах настраивается по-разному. Пустой вектор — вызов без
    /// аргументов, поведение прежнее.
    Model(Rc<RefCell<ModelNode>>, Location, Vec<ParameterArgument>),
    /// Скобочная группировка: `(реализация)`.
    Parentless(Box<Extend>),

    /// Последовательная компоновка: `левое + правое + ...`.
    Concatenation(Vec<Box<Extend>>),
    /// Параллельная компоновка: `левое | правое | ...`.
    Parallel(Vec<Box<Extend>>),
}

/// Аргумент инстанцирования модели: `M(ИМЯ := ВЫРАЖЕНИЕ)` (фича 0185).
///
/// Значение уже **вычислено** константным вычислителем (`const_eval`, задача
/// 0185-03) и понижено: за границей семантики выражения аргумента не
/// существует, есть литерал. Применяют его потребители дерева (0185-04/05) —
/// каждый своим печатником выражений, то есть общим для цели путём.
#[derive(Debug, Clone)]
pub struct ParameterArgument {
    /// Имя параметра целевой модели.
    pub name: String,
    /// Позиция **имени** в аргументе — для диагностик о самом аргументе.
    pub loc: Location,
    /// Вычисленное значение — понижённый литерал.
    pub value: ExpressionNode,
}

/// Равенство аргумента — это равенство **настройки**: имя и значение. Позиция
/// игнорируется (тот же довод, что у [`Extend`] ниже, и та же причина).
///
/// ⚠️ Автовыведённое равенство (редакция 0185-02) включало `loc`, и это был
/// дефект: цель `sv` сравнивает наборы аргументов, чтобы отвергнуть **разные**
/// настройки одной модели при уплощении (`SV-016`), — а с позицией в равенстве
/// два **текстуально одинаковых** вызова `Tuner(gain := 100) | Tuner(gain := 100)`
/// оказывались «разными наборами» и получали отказ. Заявленное поведение
/// («одинаковые наборы законны») не выполнялось; вскрыто прогоном всех целей в
/// двух режимах, задача 0185-07.
impl PartialEq for ParameterArgument {
    fn eq(&self, other: &Self) -> bool {
        self.name == other.name && self.value == other.value
    }
}

impl Eq for ParameterArgument {}

/// Равенство реализаций **игнорирует позицию использования**.
///
/// Не стиль, а условие корректности. `Extend` сравнивается транзитивно:
/// `ModelNode`/`StateNode` сравнивают своё поле `implements`, а `ConditionNode`
/// сравнивает `Rc<RefCell<ModelNode>>`. Оставь позицию в автовыведённом
/// равенстве — и две ссылки на **одну и ту же** модель из разных мест текста
/// стали бы разными узлами. Тот же приём и по той же причине уже применён к
/// [`ConditionNode::Variable`](crate::semantic::ConditionNode::Variable)
/// («Location (use-site) намеренно игнорируется»).
///
/// Узел определяется тем, **на что** он ссылается, а не тем, где написан.
impl PartialEq for Extend {
    fn eq(&self, other: &Self) -> bool {
        match (self, other) {
            (Extend::None, Extend::None) => true,
            (Extend::Unresolved(a), Extend::Unresolved(b)) => a == b,
            // Позиция (use-site) намеренно игнорируется — см. док выше.
            // ⚠️ А вот аргументы инстанцирования (фича 0185) игнорировать
            // НЕЛЬЗЯ: `M(Y := 100)` и `M(Y := 200)` ссылаются на одну модель,
            // но это разные реализации. Приравняв их, мы получили бы один
            // экземпляр там, где автор написал два разных.
            (Extend::Model(a, _, aa), Extend::Model(b, _, ba)) => a == b && aa == ba,
            (Extend::Parentless(a), Extend::Parentless(b)) => a == b,
            (Extend::Concatenation(a), Extend::Concatenation(b)) => a == b,
            (Extend::Parallel(a), Extend::Parallel(b)) => a == b,
            _ => false,
        }
    }
}

impl Eq for Extend {}

impl Extend {
    /// Возвращает `true`, если вариант — конкретная ссылка на модель.
    #[inline]
    pub fn is_model(&self) -> bool {
        matches!(self, Extend::Model(_, _, _))
    }
    /// Возвращает `true`, если вариант — скобочная группировка.
    #[inline]
    pub fn is_parentless(&self) -> bool {
        matches!(self, Extend::Parentless(_))
    }
    /// Возвращает `true`, если вариант — последовательная компоновка (`+`).
    #[inline]
    pub fn is_sequence(&self) -> bool {
        matches!(self, Extend::Concatenation(_))
    }
    /// Возвращает `true`, если вариант — параллельная компоновка (`|`).
    #[inline]
    pub fn is_parallel(&self) -> bool {
        matches!(self, Extend::Parallel(_))
    }
    /// Возвращает человекочитаемое имя варианта или имя модели.
    pub fn name(&self) -> String {
        match self {
            Extend::None => "None".to_string(),
            Extend::Unresolved(_) => "Unresolved".to_string(),
            Extend::Model(model, _, _) => model.clone().borrow().name().to_string(),
            Extend::Parentless(implement) => implement.name(),
            Extend::Concatenation(_) => "Concatenation".to_string(),
            Extend::Parallel(_) => "Parallel".to_string(),
        }
    }
}

impl Display for Extend {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        match self {
            Extend::None => write!(f, "None"),
            Extend::Unresolved(_) => write!(f, "Unresolved"),
            Extend::Model(model, _, _) => {
                write!(f, "{}", model.borrow().name.clone().unwrap_or_default())
            }
            Extend::Parentless(extends) => write!(f, "({})", extends),
            Extend::Concatenation(extends) => write!(
                f,
                "{}",
                extends
                    .iter()
                    .map(|i| i.to_string())
                    .collect::<Vec<String>>()
                    .join(" + ")
            ),
            Extend::Parallel(implements) => write!(
                f,
                "{}",
                implements
                    .iter()
                    .map(|i| i.to_string())
                    .collect::<Vec<String>>()
                    .join(" | ")
            ),
        }
    }
}

/// Упаковывает плоскую [`Extend::Concatenation`] в синтетическую [`Extend::Model`].
///
/// Для `M1 + M2` создаёт новую анонимную модель с состояниями `Step0 = M1 { next Step1 }`
/// и `Step1 = M2`, возвращая `Extend::Model(synthetic, …)`. Одноэлементная
/// конкатенация сворачивается рекурсивно. `Parentless` прозрачно делегирует внутрь.
///
/// Вызывается сразу после [`unroll_extend_expression`] в стадии stage1.
pub fn compact_implement(
    extend: Extend,
    parent: Rc<RefCell<ModelNode>>,
    state_name: &str,
) -> Extend {
    match extend {
        // Одноэлементная последовательность — прозрачно разворачиваем.
        Extend::Concatenation(mut items) if items.len() == 1 => {
            compact_implement(*items.remove(0), parent, state_name)
        }
        // Несколько элементов: создаём синтетическую модель со ступенями Step0…StepN-1.
        Extend::Concatenation(items) => {
            let seq_name = format!("{}_Sequence", state_name);
            let seq_model = ModelNode::new(&seq_name, Some(Rc::clone(&parent)));
            let n = items.len();
            for (i, item) in items.into_iter().enumerate() {
                let step_name = format!("Step{}", i);
                let next: Option<ReferenceNode<StateNode>> = if i + 1 < n {
                    Some(ReferenceNode {
                        location: Location::Codegen,
                        name: format!("Step{}", i + 1),
                        cond: ConditionNode::None,
                        object: Box::new(StateNode::Unresolved),
                    })
                } else {
                    None
                };
                let kind = if i == 0 {
                    StateNodeKind::Start
                } else {
                    StateNodeKind::Simple
                };
                let inner = compact_implement(*item, Rc::clone(&seq_model), &step_name);
                let state = StateNode::Implement {
                    upper: Some(Rc::downgrade(&seq_model)),
                    loc: Location::Codegen,
                    named_blocks: vec![],
                    name: step_name.clone(),
                    references: vec![],
                    implements: inner,
                    next,
                    kind,
                    formulas: vec![],
                };
                seq_model.borrow_mut().states.insert(step_name, state);
            }
            // Модель придумал компилятор: исходной позиции у неё нет.
            Extend::Model(seq_model, Location::Codegen, Vec::new())
        }
        // Скобочная группировка — делегируем внутрь.
        Extend::Parentless(inner) => compact_implement(*inner, parent, state_name),
        // Остальное не требует обработки.
        other => other,
    }
}

/// Плоская конкатенация: операнд, уже являющийся цепочкой, разворачивается.
fn concatenate(left: Extend, right: Extend) -> Extend {
    let mut items: Vec<Box<Extend>> = Vec::new();
    for side in [left, right] {
        match side {
            Extend::Concatenation(seq) => items.extend(seq),
            other => items.push(Box::new(other)),
        }
    }
    Extend::Concatenation(items)
}

/// Плоское объединение: операнд, уже являющийся параллелью, разворачивается.
fn parallelize(left: Extend, right: Extend) -> Extend {
    let mut items: Vec<Box<Extend>> = Vec::new();
    for side in [left, right] {
        match side {
            Extend::Parallel(p) => items.extend(p),
            other => items.push(Box::new(other)),
        }
    }
    Extend::Parallel(items)
}

/// Разворачивает **АСД**-выражение расширения прямо в [`Extend`].
///
/// # Почему напрямую, без промежуточного `ExpressionNode`
///
/// Прежде путь был `ast::Expression` → `ExpressionNode` → `Extend`, причём
/// промежуточный узел строился только чтобы тут же быть разобранным. На этом
/// шаге и **терялась позиция использования**: у `ExpressionNode::Model` поля для
/// неё нет, а заводить его значило бы тащить позицию через ~40 вариантов
/// перечисления, где она никому не нужна. Разворот напрямую из АСД сохраняет
/// `id.loc` (фича 0056) и попутно убирает лишнее звено.
fn unroll_ast_extend(
    expr: ast::Expression,
    model: Rc<RefCell<ModelNode>>,
) -> Result<Extend, Diagnostic> {
    match expr {
        ast::Expression::Variable(id) => {
            let found = model
                .as_ref()
                .borrow()
                .search_model(&id.name)
                .ok_or_else(|| {
                    Diagnostic::error(id.loc, format!("Модель '{}' не найдена", id.name))
                        .with_code("SE-001")
                })?;
            // Позиция имени — то, ради чего разворот идёт по АСД.
            Ok(Extend::Model(found, id.loc, Vec::new()))
        }
        // Инстанцирование с аргументами: `M(Y := 200)` (фича 0185).
        // Грамматика видит здесь вызов функции — моделью его делает позиция
        // (выражение реализации), поэтому имя ищется среди моделей.
        ast::Expression::Function(call_loc, id, args) => {
            let found = model
                .as_ref()
                .borrow()
                .search_model(&id.name)
                .ok_or_else(|| {
                    Diagnostic::error(id.loc, format!("Модель '{}' не найдена", id.name))
                        .with_code("SE-001")
                })?;
            let arguments =
                extend_args::parse_arguments(&found, &id.name, &args, call_loc, &model)?;
            Ok(Extend::Model(found, id.loc, arguments))
        }
        ast::Expression::Parenthesis(_, inner) => unroll_ast_extend(*inner, model),
        ast::Expression::Add(_, left, right) => {
            let left = unroll_ast_extend(*left, model.clone())?;
            let right = unroll_ast_extend(*right, model)?;
            Ok(concatenate(left, right))
        }
        ast::Expression::BitwiseOr(_, left, right) => {
            let left = unroll_ast_extend(*left, model.clone())?;
            let right = unroll_ast_extend(*right, model)?;
            Ok(parallelize(left, right))
        }
        // Прочие формы реализацией быть не могут. Диагностика **с кодом и
        // позицией**: прежде здесь печатался `Debug` узла АСД без того и
        // другого — сообщение о внутреннем устройстве вместо ошибки автора.
        other => Err(Diagnostic::error(
            arg_loc(&other).unwrap_or(Location::Implicit),
            "Реализация модели задаётся именем модели, композицией '+'/'|' или \
             инстанцированием 'M(параметр := значение)'"
                .to_string(),
        )
        .with_code("SE-081")),
    }
}

/// Позиция выражения, если она у варианта есть (для диагностики выше).
fn arg_loc(expr: &ast::Expression) -> Option<Location> {
    match expr {
        ast::Expression::Variable(id) => Some(id.loc),
        ast::Expression::Assign(loc, _, _)
        | ast::Expression::Number(loc, _)
        | ast::Expression::Function(loc, _, _)
        | ast::Expression::Parenthesis(loc, _) => Some(*loc),
        _ => None,
    }
}

/// Разворачивает семантическое выражение расширения в плоскую структуру [`Extend`],
/// объединяя цепочки `+` в [`Extend::Concatenation`] и `|` в [`Extend::Parallel`].
pub fn unroll_extend_expression(
    expression: ExpressionNode,
    model: Rc<RefCell<ModelNode>>,
) -> Result<Extend, Diagnostic> {
    let model = Rc::clone(&model);
    match expression {
        // Путь продукта: реализация приходит сырым АСД (`tree.rs`, stage1).
        ExpressionNode::Unresolved(expr) => unroll_ast_extend(expr, model),
        // Уже разрешённая модель: позиции использования у неё нет и взять негде.
        // Разрешённая модель: ни позиции использования, ни аргументов у неё нет
        // и взять негде — этим путём идут узлы, собранные кодогеном.
        ExpressionNode::Model(model) => Ok(Extend::Model(
            Rc::clone(&model),
            Location::Implicit,
            Vec::new(),
        )),
        ExpressionNode::Parenthesis(expression) => unroll_extend_expression(*expression, model),
        ExpressionNode::Add(left, right) => {
            let left = unroll_extend_expression(*left, model.clone())?;
            let right = unroll_extend_expression(*right, model)?;
            Ok(concatenate(left, right))
        }
        ExpressionNode::BitwiseOr(left, right) => {
            let left = unroll_extend_expression(*left, model.clone())?;
            let right = unroll_extend_expression(*right, model)?;
            Ok(parallelize(left, right))
        }
        other => Err(format!("Неизвестное выражение расширения: {:?}", other)
            .as_str()
            .into()),
    }
}

#[cfg(test)]
mod tests {
    use crate::diagnostics::Location;
    use crate::parse;
    use crate::parser::ast;
    use crate::semantic::extend::{Extend, unroll_extend_expression};
    use crate::semantic::test_constants::tests::SRC;
    use crate::semantic::tree::construct_model;
    use crate::semantic::{ExpressionNode, StateNode};

    #[test]
    fn test_unroll_implement_expression() {
        let (ast, _) = parse(SRC, 0).unwrap();
        let model_rc = construct_model(&ast, None, &[]).unwrap();

        let implement = unroll_extend_expression(
            ExpressionNode::Unresolved(ast::Expression::Variable(ast::Identifier::new("A"))),
            model_rc.clone(),
        )
        .unwrap();
        assert!(matches!(implement, Extend::Model(_, _, _)));
        let implement = unroll_extend_expression(
            ExpressionNode::Unresolved(ast::Expression::BitwiseOr(
                Location::Implicit,
                Box::new(ast::Expression::Variable(ast::Identifier::new("A"))),
                Box::new(ast::Expression::Variable(ast::Identifier::new("B"))),
            )),
            model_rc.clone(),
        )
        .unwrap();
        assert!(matches!(implement, Extend::Parallel(_)));
        // start Entry = A | B | (A + B)  →  Parallel([A, B, Sequence([A, B])])
        let implement = unroll_extend_expression(
            ExpressionNode::Unresolved(ast::Expression::BitwiseOr(
                Location::Implicit,
                Box::new(ast::Expression::BitwiseOr(
                    Location::Implicit,
                    Box::new(ast::Expression::Variable(ast::Identifier::new("A"))),
                    Box::new(ast::Expression::Variable(ast::Identifier::new("B"))),
                )),
                Box::new(ast::Expression::Parenthesis(
                    Location::Implicit,
                    Box::new(ast::Expression::Add(
                        Location::Implicit,
                        Box::new(ast::Expression::Variable(ast::Identifier::new("A"))),
                        Box::new(ast::Expression::Variable(ast::Identifier::new("B"))),
                    )),
                )),
            )),
            model_rc.clone(),
        )
        .unwrap();
        assert_eq!(
            implement,
            Extend::Parallel(vec![
                Box::new(Extend::Model(
                    model_rc.borrow().search_model("A").unwrap(),
                    Location::Implicit,
                    Vec::new(),
                )),
                Box::new(Extend::Model(
                    model_rc.borrow().search_model("B").unwrap(),
                    Location::Implicit,
                    Vec::new(),
                )),
                Box::new(Extend::Concatenation(vec![
                    Box::new(Extend::Model(
                        model_rc.borrow().search_model("A").unwrap(),
                        Location::Implicit,
                        Vec::new(),
                    )),
                    Box::new(Extend::Model(
                        model_rc.borrow().search_model("B").unwrap(),
                        Location::Implicit,
                        Vec::new(),
                    )),
                ]))
            ])
        );
    }

    #[test]
    fn test_unroll_implement_expressions() {
        // compact_implement отключён в tree.rs, поэтому состояния с `+` остаются
        // как Extend::Concatenation(items) с плоским списком элементов.
        // unroll_extend_expression раскрывает цепочки + и () в плоский Concatenation:
        // A + (B + C) + D  →  Concatenation([A, B, C, D]) (скобки прозрачны).
        let (ast, _) = parse(SRC, 0).unwrap();
        let model_rc = construct_model(&ast, None, &[]).unwrap();

        // Next1..Next10: верхний уровень — конкатенация → плоский Concatenation.
        // Количество элементов = ожидаемое число ступеней.
        let seq_states_with_item_count = [
            ("Next1", 3usize), // A + B + (A|B)                         → [A, B, Par]
            ("Next2", 3),      // A + (B|A) + B                         → [A, Par, B]
            ("Next3", 4),      // A + (B + A) + B  (скобки прозрачны)   → [A, B, A, B]
            ("Next4", 4),      // A + (B + A) + (B|A)                   → [A, B, A, Par]
            ("Next5", 3),      // (A|B) + (A + B)                       → [Par, A, B]
            ("Next6", 4),      // (A|B) + (A + B) + (A|B)               → [Par, A, B, Par]
            ("Next7", 6),      // … + (A + B)                           → [Par,A,B,Par,A,B]
            ("Next8", 7),      // … + (A|B)                             → + Par
            ("Next9", 9),      // … + (A + B)                           → + A, B
            ("Next10", 11),    // … + (A + B)                           → + A, B
        ];
        for (name, expected_items) in seq_states_with_item_count {
            let state = model_rc.borrow().search_state(name).unwrap();
            let StateNode::Implement { ref implements, .. } = *state.borrow() else {
                panic!("State {name} is not an Implement node")
            };
            let Extend::Concatenation(items) = implements else {
                panic!("State {name}: ожидался Extend::Concatenation, получили: {implements}");
            };
            assert_eq!(
                items.len(),
                expected_items,
                "State {name}: конкатенация должна содержать {expected_items} элементов, содержит {}",
                items.len()
            );
        }

        // Entry = A | B | (A + B): верхний уровень — параллель → Parallel.
        {
            let state = model_rc.borrow().search_state("Entry").unwrap();
            let StateNode::Implement { ref implements, .. } = *state.borrow() else {
                panic!("State Entry is not an Implement node")
            };
            assert!(
                matches!(implements, Extend::Parallel(_)),
                "Entry = A | B | (A + B): ожидался Extend::Parallel, получили: {implements}"
            );
        }
    }

    #[test]
    fn test_extend_predicates() {
        use crate::semantic::ModelNode;

        let model = ModelNode::new("A", None);

        assert!(!Extend::None.is_model());
        assert!(!Extend::None.is_parentless());
        assert!(!Extend::None.is_sequence());
        assert!(!Extend::None.is_parallel());

        let extend_model = Extend::Model(model.clone(), Location::Implicit, Vec::new());
        assert!(extend_model.is_model());
        assert!(!extend_model.is_parentless());
        assert!(!extend_model.is_sequence());
        assert!(!extend_model.is_parallel());

        let parentless = Extend::Parentless(Box::new(Extend::Model(
            model.clone(),
            Location::Implicit,
            Vec::new(),
        )));
        assert!(!parentless.is_model());
        assert!(parentless.is_parentless());
        assert!(!parentless.is_sequence());
        assert!(!parentless.is_parallel());

        let seq = Extend::Concatenation(vec![Box::new(Extend::Model(
            model.clone(),
            Location::Implicit,
            Vec::new(),
        ))]);
        assert!(!seq.is_model());
        assert!(!seq.is_parentless());
        assert!(seq.is_sequence());
        assert!(!seq.is_parallel());

        let par = Extend::Parallel(vec![Box::new(Extend::Model(
            model.clone(),
            Location::Implicit,
            Vec::new(),
        ))]);
        assert!(!par.is_model());
        assert!(!par.is_parentless());
        assert!(!par.is_sequence());
        assert!(par.is_parallel());
    }

    #[test]
    fn test_extend_name() {
        use crate::semantic::ModelNode;

        let model = ModelNode::new("MyModel", None);

        assert_eq!(Extend::None.name(), "None");
        assert_eq!(
            Extend::Unresolved(ast::Expression::Variable(ast::Identifier::new("X"))).name(),
            "Unresolved"
        );
        assert_eq!(
            Extend::Model(model.clone(), Location::Implicit, Vec::new()).name(),
            "MyModel"
        );
        assert_eq!(
            Extend::Parentless(Box::new(Extend::Model(
                model.clone(),
                Location::Implicit,
                Vec::new()
            )))
            .name(),
            "MyModel"
        );
        assert_eq!(
            Extend::Concatenation(vec![Box::new(Extend::Model(
                model.clone(),
                Location::Implicit,
                Vec::new(),
            ))])
            .name(),
            "Concatenation"
        );
        assert_eq!(
            Extend::Parallel(vec![Box::new(Extend::Model(
                model.clone(),
                Location::Implicit,
                Vec::new(),
            ))])
            .name(),
            "Parallel"
        );
    }

    #[test]
    fn test_extend_display() {
        use crate::semantic::ModelNode;

        let a = ModelNode::new("A", None);
        let b = ModelNode::new("B", None);

        assert_eq!(format!("{}", Extend::None), "None");
        assert_eq!(
            format!(
                "{}",
                Extend::Unresolved(ast::Expression::Variable(ast::Identifier::new("X")))
            ),
            "Unresolved"
        );
        assert_eq!(
            format!(
                "{}",
                Extend::Model(a.clone(), Location::Implicit, Vec::new())
            ),
            "A"
        );
        assert_eq!(
            format!(
                "{}",
                Extend::Parentless(Box::new(Extend::Model(
                    a.clone(),
                    Location::Implicit,
                    Vec::new()
                )))
            ),
            "(A)"
        );
        assert_eq!(
            format!(
                "{}",
                Extend::Concatenation(vec![
                    Box::new(Extend::Model(a.clone(), Location::Implicit, Vec::new())),
                    Box::new(Extend::Model(b.clone(), Location::Implicit, Vec::new())),
                ])
            ),
            "A + B"
        );
        assert_eq!(
            format!(
                "{}",
                Extend::Parallel(vec![
                    Box::new(Extend::Model(a.clone(), Location::Implicit, Vec::new())),
                    Box::new(Extend::Model(b.clone(), Location::Implicit, Vec::new())),
                ])
            ),
            "A | B"
        );
    }

    #[test]
    fn test_unroll_model_not_found() {
        let (ast, _) = parse(SRC, 0).unwrap();
        let model_rc = construct_model(&ast, None, &[]).unwrap();

        let result = unroll_extend_expression(
            ExpressionNode::Unresolved(ast::Expression::Variable(ast::Identifier::new(
                "NonExistent",
            ))),
            model_rc.clone(),
        );
        assert!(result.is_err());
        let err = result.unwrap_err();
        assert!(err.message.contains("NonExistent"));
    }

    #[test]
    fn test_unroll_unsupported_expression() {
        use crate::semantic::ModelNode;

        let model = ModelNode::new("Root", None);

        // ExpressionNode::BitwiseAnd не поддерживается — должна вернуть ошибку
        let result = unroll_extend_expression(
            ExpressionNode::BitwiseAnd(
                Box::new(ExpressionNode::Model(model.clone())),
                Box::new(ExpressionNode::Model(model.clone())),
            ),
            model.clone(),
        );
        assert!(result.is_err());
    }
}
