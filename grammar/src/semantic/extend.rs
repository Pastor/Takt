//! Построение и развёртка структуры [`Extend`] — реализации модели.
//!
//! Модуль предоставляет две публичные функции:
//! - [`unroll_extend_expression`] — разворачивает выражение в плоскую
//!   структуру [`Extend::Concatenation`] / [`Extend::Parallel`].

use crate::diagnostics::{Diagnostic, Location};
use crate::parser::ast;
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
#[derive(Default, Debug, PartialEq, Eq, Clone)]
pub enum Extend {
    /// Реализация не задана (значение по умолчанию для безымянной корневой модели).
    #[default]
    None,
    /// «Сырое» АСД-выражение реализации, ожидающее разрешения на этапе stage1.
    Unresolved(ast::Expression),
    /// Ссылка на конкретную именованную модель.
    Model(Rc<RefCell<ModelNode>>),
    /// Скобочная группировка: `(реализация)`.
    Parentless(Box<Extend>),

    /// Последовательная компоновка: `левое + правое + ...`.
    Concatenation(Vec<Box<Extend>>),
    /// Параллельная компоновка: `левое | правое | ...`.
    Parallel(Vec<Box<Extend>>),
}

impl Extend {
    /// Возвращает `true`, если вариант — конкретная ссылка на модель.
    #[inline]
    pub fn is_model(&self) -> bool {
        matches!(self, Extend::Model(_))
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
            Extend::Model(model) => model.clone().borrow().name().to_string(),
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
            Extend::Model(model) => {
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

/// Преобразует АСД-выражение в семантический [`ExpressionNode`],
/// разрешая переменные в конкретные модели через контекст `model`.
///
/// Поддерживаемые операции: `Variable`, `Parenthesis`, `+`, `|`.
/// Для остальных возвращает [`Diagnostic`].
fn unroll_expression_ast(
    expr: ast::Expression,
    model: Rc<RefCell<ModelNode>>,
) -> Result<ExpressionNode, Diagnostic> {
    match expr {
        ast::Expression::Variable(id) => {
            let borrowed = model.as_ref().borrow();
            let found = borrowed.search_model(&id.name).ok_or_else(|| {
                Diagnostic::error(id.loc, format!("Модель '{}' не найдена", &id.name))
            })?;
            Ok(ExpressionNode::Model(Rc::clone(&found)))
        }
        ast::Expression::Parenthesis(_, inner) => unroll_expression_ast(*inner, model),
        ast::Expression::Add(_, left, right) => {
            let left = unroll_expression_ast(*left, model.clone())?;
            let right = unroll_expression_ast(*right, model.clone())?;
            Ok(ExpressionNode::Add(Box::new(left), Box::new(right)))
        }
        ast::Expression::BitwiseOr(_, left, right) => {
            let left = unroll_expression_ast(*left, model.clone())?;
            let right = unroll_expression_ast(*right, model.clone())?;
            Ok(ExpressionNode::BitwiseOr(Box::new(left), Box::new(right)))
        }
        other => Err(
            format!("Выражение AST расширения не поддерживается: {:?}", other)
                .as_str()
                .into(),
        ),
    }
}

/// Упаковывает плоскую [`Extend::Concatenation`] в синтетическую [`Extend::Model`].
///
/// Для `M1 + M2` создаёт новую анонимную модель с состояниями `Step0 = M1 { next Step1 }`
/// и `Step1 = M2`, возвращая `Extend::Model(synthetic)`. Одноэлементная конкатенация
/// сворачивается рекурсивно. `Parentless` прозрачно делегирует внутрь.
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
                };
                seq_model.borrow_mut().states.insert(step_name, state);
            }
            Extend::Model(seq_model)
        }
        // Скобочная группировка — делегируем внутрь.
        Extend::Parentless(inner) => compact_implement(*inner, parent, state_name),
        // Остальное не требует обработки.
        other => other,
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
        ExpressionNode::Unresolved(expr) => {
            let unrolled = unroll_expression_ast(expr, model.clone())?;
            unroll_extend_expression(unrolled, model)
        }
        ExpressionNode::Model(model) => Ok(Extend::Model(Rc::clone(&model))),
        ExpressionNode::Parenthesis(expression) => unroll_extend_expression(*expression, model),
        ExpressionNode::Add(left, right) => {
            let left = unroll_extend_expression(*left, model.clone())?;
            let right = unroll_extend_expression(*right, model.clone())?;
            // Плоская конкатенация: если операнд уже Sequence — разворачиваем его элементы.
            let mut items: Vec<Box<Extend>> = Vec::new();
            match left {
                Extend::Concatenation(seq) => items.extend(seq),
                other => items.push(Box::new(other)),
            }
            match right {
                Extend::Concatenation(seq) => items.extend(seq),
                other => items.push(Box::new(other)),
            }
            Ok(Extend::Concatenation(items))
        }
        ExpressionNode::BitwiseOr(left, right) => {
            let left = unroll_extend_expression(*left, model.clone())?;
            let right = unroll_extend_expression(*right, model.clone())?;
            // Плоское объединение: если операнд уже Parallel — разворачиваем его элементы.
            let mut items: Vec<Box<Extend>> = Vec::new();
            match left {
                Extend::Parallel(p) => items.extend(p),
                other => items.push(Box::new(other)),
            }
            match right {
                Extend::Parallel(p) => items.extend(p),
                other => items.push(Box::new(other)),
            }
            Ok(Extend::Parallel(items))
        }
        other => Err(format!("Неизвестное выражение расширения: {:?}", other)
            .as_str()
            .into()),
    }
}

#[cfg(test)]
pub mod tests {
    use crate::diagnostics::Location;
    use crate::parse;
    use crate::parser::ast;
    use crate::semantic::extend::{unroll_extend_expression, Extend};
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
        assert!(matches!(implement, Extend::Model(_)));
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
                Box::new(Extend::Model(model_rc.borrow().search_model("A").unwrap())),
                Box::new(Extend::Model(model_rc.borrow().search_model("B").unwrap())),
                Box::new(Extend::Concatenation(vec![
                    Box::new(Extend::Model(model_rc.borrow().search_model("A").unwrap())),
                    Box::new(Extend::Model(model_rc.borrow().search_model("B").unwrap())),
                ]))
            ])
        );
    }

    #[test]
    fn test_unroll_implement_expressions() {
        // После compact_implement все состояния с `+` на верхнем уровне дают Model(synthetic).
        // Это тест пост-компактного состояния семантического дерева.
        let (ast, _) = parse(SRC, 0).unwrap();
        let model_rc = construct_model(&ast, None, &[]).unwrap();

        // Next1..Next10: верхний уровень — конкатенация → упаковывается в Model(synthetic).
        let seq_states_with_step_count = [
            ("Next1", 3usize), // A + B + (A|B)      → 3 ступени
            ("Next2", 3),      // A + (B|A) + B       → 3 ступени
            ("Next3", 4),      // A + B + A + B        → 4 ступени
            ("Next4", 4),      // A + B + A + (B|A)   → 4 ступени
            ("Next5", 3),      // (A|B) + A + B        → 3 ступени
            ("Next6", 4),      // (A|B) + A + B + (A|B) → 4 ступени
            ("Next7", 6),      // (A|B)+A+B+(A|B)+A+B → 6 ступеней
            ("Next8", 7),      // … + (A|B)            → 7 ступеней
            ("Next9", 9),      // … + A + B            → 9 ступеней
            ("Next10", 11),    // … + A + B            → 11 ступеней
        ];
        for (name, expected_steps) in seq_states_with_step_count {
            let state = model_rc.borrow().search_state(name).unwrap();
            let StateNode::Implement { ref implements, .. } = *state.borrow() else {
                panic!("State {name} is not an Implement node")
            };
            let Extend::Model(seq_rc) = implements else {
                panic!(
                    "State {name}: после compact_implement ожидался Extend::Model, получили: {implements}"
                );
            };
            let step_count = seq_rc.borrow().states.len();
            assert_eq!(
                step_count, expected_steps,
                "State {name}: синтетическая модель должна содержать {expected_steps} ступеней, содержит {step_count}"
            );
            // Стартовая ступень существует и имеет kind=Start.
            let seq = seq_rc.borrow();
            let start = seq.get_start_state();
            assert!(
                start.is_some(),
                "State {name}: в синтетической модели должно быть стартовое состояние"
            );
        }

        // Entry = A | B | (A + B): верхний уровень — параллель → Parallel(_).
        // compact_implement не трогает Parallel, поэтому вложенная конкатенация
        // остаётся как Concatenation внутри Parallel.
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

        let extend_model = Extend::Model(model.clone());
        assert!(extend_model.is_model());
        assert!(!extend_model.is_parentless());
        assert!(!extend_model.is_sequence());
        assert!(!extend_model.is_parallel());

        let parentless = Extend::Parentless(Box::new(Extend::Model(model.clone())));
        assert!(!parentless.is_model());
        assert!(parentless.is_parentless());
        assert!(!parentless.is_sequence());
        assert!(!parentless.is_parallel());

        let seq = Extend::Concatenation(vec![Box::new(Extend::Model(model.clone()))]);
        assert!(!seq.is_model());
        assert!(!seq.is_parentless());
        assert!(seq.is_sequence());
        assert!(!seq.is_parallel());

        let par = Extend::Parallel(vec![Box::new(Extend::Model(model.clone()))]);
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
        assert_eq!(Extend::Model(model.clone()).name(), "MyModel");
        assert_eq!(
            Extend::Parentless(Box::new(Extend::Model(model.clone()))).name(),
            "MyModel"
        );
        assert_eq!(
            Extend::Concatenation(vec![Box::new(Extend::Model(model.clone()))]).name(),
            "Concatenation"
        );
        assert_eq!(
            Extend::Parallel(vec![Box::new(Extend::Model(model.clone()))]).name(),
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
        assert_eq!(format!("{}", Extend::Model(a.clone())), "A");
        assert_eq!(
            format!("{}", Extend::Parentless(Box::new(Extend::Model(a.clone())))),
            "(A)"
        );
        assert_eq!(
            format!(
                "{}",
                Extend::Concatenation(vec![
                    Box::new(Extend::Model(a.clone())),
                    Box::new(Extend::Model(b.clone())),
                ])
            ),
            "A + B"
        );
        assert_eq!(
            format!(
                "{}",
                Extend::Parallel(vec![
                    Box::new(Extend::Model(a.clone())),
                    Box::new(Extend::Model(b.clone())),
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
