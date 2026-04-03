//! Построение и развёртка структуры [`Implement`] — реализации модели.
//!
//! Модуль предоставляет две публичные функции:
//! - [`construct_implement`] — строит [`Implement`] из семантического выражения [`ExpressionNode`];
//! - [`unroll_implement_expression`] — разворачивает выражение в плоскую
//!   структуру [`Implement::Sequence`] / [`Implement::Parallel`].

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
/// - [`Unresolved`](Implement::Unresolved) — временная заглушка до второго прохода.
/// - [`Model`](Implement::Model) — ссылка на конкретную именованную модель.
/// - [`Parentless`](Implement::Parentless) — обёртка без родителя (скобки).
/// - [`Add`](Implement::Add) — последовательная компоновка `A + B`.
/// - [`Or`](Implement::Or) — параллельная компоновка `A | B`.
#[derive(Default, Debug, PartialEq, Eq, Clone)]
pub enum Implement {
    /// Реализация не задана (значение по умолчанию для безымянной корневой модели).
    #[default]
    None,
    /// «Сырое» АСД-выражение реализации, ожидающее разрешения на этапе stage1.
    Unresolved(ast::Expression),
    /// Ссылка на конкретную именованную модель.
    Model(Rc<RefCell<ModelNode>>),
    /// Скобочная группировка: `(реализация)`.
    Parentless(Box<Implement>),

    /// Последовательная компоновка: `левое + правое + ...`.
    Sequence(Vec<Box<Implement>>),
    /// Параллельная компоновка: `левое | правое | ...`.
    Parallel(Vec<Box<Implement>>),
}

impl Implement {
    pub fn is_model(&self) -> bool {
        matches!(self, Implement::Model(_))
    }
    pub fn is_parentless(&self) -> bool {
        matches!(self, Implement::Parentless(_))
    }
    pub fn is_sequence(&self) -> bool {
        matches!(self, Implement::Sequence(_))
    }
    pub fn is_parallel(&self) -> bool {
        matches!(self, Implement::Parallel(_))
    }
    pub fn name(&self) -> String {
        match self {
            Implement::None => "None".to_string(),
            Implement::Unresolved(_) => "Unresolved".to_string(),
            Implement::Model(model) => model.clone().borrow().name().to_string(),
            Implement::Parentless(implement) => implement.name(),
            Implement::Sequence(_) => "Sequence".to_string(),
            Implement::Parallel(_) => "Parallel".to_string(),
        }
    }
}

impl Display for Implement {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        match self {
            Implement::None => write!(f, "None"),
            Implement::Unresolved(_) => write!(f, "Unresolved"),
            Implement::Model(model) => {
                write!(f, "{}", model.borrow().name.clone().unwrap_or_default())
            }
            Implement::Parentless(implement) => write!(f, "({})", implement),
            Implement::Sequence(implements) => write!(
                f,
                "{}",
                implements
                    .iter()
                    .map(|i| i.to_string())
                    .collect::<Vec<String>>()
                    .join(" + ")
            ),
            Implement::Parallel(implements) => write!(
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

fn unroll_implement_expression_ast(
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
        ast::Expression::Parenthesis(_, inner) => unroll_implement_expression_ast(*inner, model),
        ast::Expression::Add(_, left, right) => {
            let left = unroll_implement_expression_ast(*left, model.clone())?;
            let right = unroll_implement_expression_ast(*right, model.clone())?;
            Ok(ExpressionNode::Add(Box::new(left), Box::new(right)))
        }
        ast::Expression::BitwiseOr(_, left, right) => {
            let left = unroll_implement_expression_ast(*left, model.clone())?;
            let right = unroll_implement_expression_ast(*right, model.clone())?;
            Ok(ExpressionNode::BitwiseOr(Box::new(left), Box::new(right)))
        }
        other => Err(
            format!("Выражение реализации не поддерживается: {:?}", other)
                .as_str()
                .into(),
        ),
    }
}

/// Разворачивает семантическое выражение реализации в плоскую структуру [`Implement`],
/// объединяя цепочки `+` в [`Implement::Sequence`] и `|` в [`Implement::Parallel`].
pub fn unroll_implement_expression(
    prefix_name: String,
    expression: ExpressionNode,
    model: Rc<RefCell<ModelNode>>,
) -> Result<Implement, Diagnostic> {
    let model = Rc::clone(&model);
    match expression {
        ExpressionNode::Unresolved(expr) => {
            let unrolled = unroll_implement_expression_ast(expr, model.clone())?;
            unroll_implement_expression(prefix_name, unrolled, model)
        }
        ExpressionNode::Model(model) => Ok(Implement::Model(Rc::clone(&model))),
        ExpressionNode::Parenthesis(expression) => {
            unroll_implement_expression(prefix_name, *expression, model)
        }
        ExpressionNode::Add(left, right) => {
            let left = unroll_implement_expression(prefix_name.clone(), *left, model.clone())?;
            let right = unroll_implement_expression(prefix_name.clone(), *right, model.clone())?;
            // Плоская конкатенация: если операнд уже Sequence — разворачиваем его элементы.
            let mut items: Vec<Box<Implement>> = Vec::new();
            match left {
                Implement::Sequence(seq) => items.extend(seq),
                other => items.push(Box::new(other)),
            }
            match right {
                Implement::Sequence(seq) => items.extend(seq),
                other => items.push(Box::new(other)),
            }
            Ok(Implement::Sequence(items))
        }
        ExpressionNode::BitwiseOr(left, right) => {
            let left = unroll_implement_expression(prefix_name.clone(), *left, model.clone())?;
            let right = unroll_implement_expression(prefix_name.clone(), *right, model.clone())?;
            // Плоское объединение: если операнд уже Parallel — разворачиваем его элементы.
            let mut items: Vec<Box<Implement>> = Vec::new();
            match left {
                Implement::Parallel(p) => items.extend(p),
                other => items.push(Box::new(other)),
            }
            match right {
                Implement::Parallel(p) => items.extend(p),
                other => items.push(Box::new(other)),
            }
            Ok(Implement::Parallel(items))
        }
        other => Err(format!("Неизвестное выражение реализации: {:?}", other)
            .as_str()
            .into()),
    }
}

pub fn compact_implement(
    prefix_name: String,
    implement: &Implement,
    owned: Rc<RefCell<ModelNode>>,
) -> Result<Implement, Diagnostic> {
    match implement {
        Implement::None | Implement::Unresolved(_) => Err("Неизвестная реализация".into()),
        Implement::Model(model) => {
            // Копируем содержимое модели и меняем его владельца, добавляем в список моделей
            let model_name = format!("{}{}", prefix_name, model.borrow().name.clone().unwrap());
            let model_node = owned.borrow().search_model(&model_name);
            if model_node.is_some() {
                return Ok(Implement::Model(model_node.unwrap()));
            }
            let model_node = model.borrow().copy(&model_name, Some(owned.clone()));
            let mode_rc = Rc::new(RefCell::new(model_node));
            owned
                .borrow_mut()
                .models
                .insert(model_name.clone(), mode_rc.clone());
            Ok(Implement::Model(mode_rc))
        }
        Implement::Parentless(implement) => {
            compact_implement(prefix_name, implement, owned.clone())
        }
        Implement::Sequence(implements) => {
            let prefix_name = format!("{}Sequence", prefix_name.clone());
            let model = ModelNode::new(prefix_name.clone().as_str(), Some(owned.clone()));
            let model_rc = Rc::new(RefCell::new(model));
            let mut prev = None;
            let mut n = 0;
            let mut last_name = String::new();
            let max_sequence_length: usize = implements.len();
            for implement in implements.iter().rev() {
                let model_name = format!("{}_{}", prefix_name, n);
                last_name = model_name.clone();
                let implement_model =
                    compact_implement(model_name.clone(), implement, owned.clone())?;
                n += 1;
                let kind = if n >= max_sequence_length {
                    StateNodeKind::Start
                } else {
                    StateNodeKind::Simple
                };
                let state = StateNode::Implement {
                    upper: Some(Rc::downgrade(&model_rc)),
                    loc: Location::Codegen,
                    named_blocks: vec![],
                    name: model_name.clone(),
                    references: vec![],
                    implements: implement_model.clone(),
                    next: prev.clone(),
                    kind,
                };
                prev = Some(ReferenceNode {
                    location: Location::Codegen,
                    name: state.name().to_string(),
                    cond: ConditionNode::None,
                    object: Box::new(state.clone()),
                });
                model_rc
                    .borrow_mut()
                    .states
                    .insert(model_name.clone(), state);
            }
            Ok(Implement::Model(model_rc))
        }
        Implement::Parallel(implements) => {
            let implements = implements
                .iter()
                .map(|implement| {
                    Box::new(
                        compact_implement(prefix_name.clone(), implement, owned.clone()).unwrap(),
                    )
                })
                .collect::<Vec<_>>();
            Ok(Implement::Parallel(implements))
        }
    }
}

#[cfg(test)]
mod tests {
    use crate::diagnostics::Location;
    use crate::parse;
    use crate::parser::ast;
    use crate::semantic::implement::{Implement, compact_implement, unroll_implement_expression};
    use crate::semantic::tree::construct_model;
    use crate::semantic::{ExpressionNode, StateNode};

    const SRC: &str = r#"
model A {
    start Start;
}
model B {
    start Start;
}
start Entry = A | B | (A + B);
state Next1 = A + B + (A | B);
state Next2 = A + (B | A) + B;
state Next3 = A + (B + A) + B;
state Next4 = A + (B + A) + (B | A);
state Next5 = (A | B) + (A + B);
state Next6 = (A | B) + (A + B) + (A | B);
state Next7 = (A | B) + (A + B) + (A | B) + (A + B);
state Next8 = (A | B) + (A + B) + (A | B) + (A + B) + (A | B);
state Next9 = (A | B) + (A + B) + (A | B) + (A + B) + (A | B) + (A + B);
state Next10 = (A | B) + (A + B) + (A | B) + (A + B) + (A | B) + (A + B) + (A + B);
"#;

    #[test]
    fn test_unroll_implement_expression() {
        let (ast, _) = parse(SRC, 0).unwrap();
        let model_rc = construct_model(&ast, None, &[]).unwrap();

        let implement = unroll_implement_expression(
            String::from("O"),
            ExpressionNode::Unresolved(ast::Expression::Variable(ast::Identifier::new("A"))),
            model_rc.clone(),
        )
        .unwrap();
        assert!(matches!(implement, Implement::Model(_)));
        let implement = unroll_implement_expression(
            String::from("O"),
            ExpressionNode::Unresolved(ast::Expression::BitwiseOr(
                Location::Implicit,
                Box::new(ast::Expression::Variable(ast::Identifier::new("A"))),
                Box::new(ast::Expression::Variable(ast::Identifier::new("B"))),
            )),
            model_rc.clone(),
        )
        .unwrap();
        assert!(matches!(implement, Implement::Parallel(_)));
        // start Entry = A | B | (A + B)  →  Parallel([A, B, Sequence([A, B])])
        let implement = unroll_implement_expression(
            String::from("O"),
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
            Implement::Parallel(vec![
                Box::new(Implement::Model(
                    model_rc.borrow().search_model("A").unwrap()
                )),
                Box::new(Implement::Model(
                    model_rc.borrow().search_model("B").unwrap()
                )),
                Box::new(Implement::Sequence(vec![
                    Box::new(Implement::Model(
                        model_rc.borrow().search_model("A").unwrap()
                    )),
                    Box::new(Implement::Model(
                        model_rc.borrow().search_model("B").unwrap()
                    )),
                ]))
            ])
        );
    }

    #[test]
    fn test_unroll_implement_expressions() {
        let (ast, _) = parse(SRC, 0).unwrap();
        let model_rc = construct_model(&ast, None, &[]).unwrap();

        let ma = || {
            Box::new(Implement::Model(
                model_rc.borrow().search_model("A").unwrap(),
            ))
        };
        let mb = || {
            Box::new(Implement::Model(
                model_rc.borrow().search_model("B").unwrap(),
            ))
        };
        let par_ab = || Implement::Parallel(vec![ma(), mb()]);

        let pairs = vec![
            // Next1 = A + B + (A | B)
            (
                "Next1",
                Implement::Sequence(vec![ma(), mb(), Box::new(par_ab())]),
                "Next1_Sequence",
            ),
            // Next2 = A + (B | A) + B
            (
                "Next2",
                Implement::Sequence(vec![
                    ma(),
                    Box::new(Implement::Parallel(vec![mb(), ma()])),
                    mb(),
                ]),
                "Next2_Sequence",
            ),
            // Next3 = A + (B + A) + B  →  все элементы разворачиваются в одну последовательность
            (
                "Next3",
                Implement::Sequence(vec![ma(), mb(), ma(), mb()]),
                "Next3_Sequence",
            ),
            // Next4 = A + (B + A) + (B | A)
            (
                "Next4",
                Implement::Sequence(vec![
                    ma(),
                    mb(),
                    ma(),
                    Box::new(Implement::Parallel(vec![mb(), ma()])),
                ]),
                "Next4_Sequence",
            ),
            // Next5 = (A | B) + (A + B)
            (
                "Next5",
                Implement::Sequence(vec![Box::new(par_ab()), ma(), mb()]),
                "Next5_Sequence",
            ),
            // Next6 = (A | B) + (A + B) + (A | B)
            (
                "Next6",
                Implement::Sequence(vec![Box::new(par_ab()), ma(), mb(), Box::new(par_ab())]),
                "Next6_Sequence",
            ),
            // Next7 = (A | B) + (A + B) + (A | B) + (A + B)
            (
                "Next7",
                Implement::Sequence(vec![
                    Box::new(par_ab()),
                    ma(),
                    mb(),
                    Box::new(par_ab()),
                    ma(),
                    mb(),
                ]),
                "Next7_Sequence",
            ),
            // Next8 = (A | B) + (A + B) + (A | B) + (A + B) + (A | B)
            (
                "Next8",
                Implement::Sequence(vec![
                    Box::new(par_ab()),
                    ma(),
                    mb(),
                    Box::new(par_ab()),
                    ma(),
                    mb(),
                    Box::new(par_ab()),
                ]),
                "Next8_Sequence",
            ),
            // Next9 = (A | B) + (A + B) + (A | B) + (A + B) + (A | B) + (A + B)
            (
                "Next9",
                Implement::Sequence(vec![
                    Box::new(par_ab()),
                    ma(),
                    mb(),
                    Box::new(par_ab()),
                    ma(),
                    mb(),
                    Box::new(par_ab()),
                    ma(),
                    mb(),
                ]),
                "Next9_Sequence",
            ),
            // Next10 = (A | B) + (A + B) + (A | B) + (A + B) + (A | B) + (A + B) + (A + B)
            (
                "Next10",
                Implement::Sequence(vec![
                    Box::new(par_ab()),
                    ma(),
                    mb(),
                    Box::new(par_ab()),
                    ma(),
                    mb(),
                    Box::new(par_ab()),
                    ma(),
                    mb(),
                    ma(),
                    mb(),
                ]),
                "Next10_Sequence",
            ),
            // Entry = A | B | (A + B)
            (
                "Entry",
                Implement::Parallel(vec![
                    ma(),
                    mb(),
                    Box::new(Implement::Sequence(vec![ma(), mb()])),
                ]),
                "EntryA | EntryB | Entry_Sequence",
            ),
        ];
        for (name, _expected, expected_name) in pairs {
            let state = model_rc.borrow().search_state(name).unwrap();
            let StateNode::Implement { ref implements, .. } = *state.borrow() else {
                panic!("State is not an implement")
            };
            assert_eq!(
                implements.to_string(),
                expected_name,
                "State {} is not unrolled. {} != {}",
                name,
                implements,
                expected_name
            );
        }
    }

    #[test]
    fn test_implement_to_model() {
        let (ast, _) = parse(SRC, 0).unwrap();
        let model_rc = construct_model(&ast, None, &[]).unwrap();

        let result = compact_implement(String::from("O"), &Implement::None, model_rc.clone());
        assert!(result.is_err());

        let result = compact_implement(
            String::from("O"),
            &Implement::Unresolved(ast::Expression::Variable(ast::Identifier::new("A"))),
            model_rc.clone(),
        );
        assert!(result.is_err());

        let model_a = model_rc.borrow().search_model("A").unwrap();
        let model_b = model_rc.borrow().search_model("B").unwrap();
        {
            let result = compact_implement(
                String::from("O"),
                &Implement::Model(model_a.clone()),
                model_rc.clone(),
            );
            assert!(result.is_ok());
            let Implement::Model(result) = result.unwrap() else {
                panic!("Result is not a model")
            };
            let result = result.borrow();
            assert_eq!(
                format!("O{}", model_a.borrow().name.clone().unwrap()),
                result.name.clone().unwrap()
            );
        }
        {
            let result = compact_implement(
                String::from("O"),
                &Implement::Sequence(vec![
                    Box::new(Implement::Model(model_a.clone())),
                    Box::new(Implement::Model(model_b.clone())),
                ]),
                model_rc.clone(),
            );
            assert!(result.is_ok());
            let result = result.unwrap();
            let Implement::Model(result) = result else {
                panic!("Result is not a model")
            };
            let result = result.borrow();
            assert_eq!(result.name(), "O_Sequence".to_string());
            assert!(result.search_state("O_Sequence_1").is_some());
            assert!(result.search_state("O_Sequence_0").is_some());
        }
        {
            let result = compact_implement(
                String::from("O"),
                &Implement::Parallel(vec![
                    Box::new(Implement::Model(model_a.clone())),
                    Box::new(Implement::Model(model_b.clone())),
                ]),
                model_rc.clone(),
            );
            assert!(result.is_ok());
            let result = result.unwrap();
            let Implement::Parallel(implements) = result.clone() else {
                panic!("Result is not a parallel implement")
            };
            assert_eq!(result.name(), "Parallel".to_string());
            assert_eq!(implements.len(), 2);
            assert!(implements[0].is_model());
            assert!(implements[1].is_model());
        }
    }
}
