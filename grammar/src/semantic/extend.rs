//! Построение и развёртка структуры [`Extend`] — реализации модели.
//!
//! Модуль предоставляет две публичные функции:
//! - [`construct_implement`] — строит [`Extend`] из семантического выражения [`ExpressionNode`];
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
/// - [`Add`](Extend::Add) — последовательная компоновка `A + B`.
/// - [`Or`](Extend::Or) — параллельная компоновка `A | B`.
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
    #[inline]
    pub fn is_model(&self) -> bool {
        matches!(self, Extend::Model(_))
    }
    #[inline]
    pub fn is_parentless(&self) -> bool {
        matches!(self, Extend::Parentless(_))
    }
    #[inline]
    pub fn is_sequence(&self) -> bool {
        matches!(self, Extend::Concatenation(_))
    }
    #[inline]
    pub fn is_parallel(&self) -> bool {
        matches!(self, Extend::Parallel(_))
    }
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

/// Разворачивает семантическое выражение расширения в плоскую структуру [`Extend`],
/// объединяя цепочки `+` в [`Extend::Concatenation`] и `|` в [`Extend::Parallel`].
pub fn unroll_extend_expression(
    prefix_name: String,
    expression: ExpressionNode,
    model: Rc<RefCell<ModelNode>>,
) -> Result<Extend, Diagnostic> {
    let model = Rc::clone(&model);
    match expression {
        ExpressionNode::Unresolved(expr) => {
            let unrolled = unroll_expression_ast(expr, model.clone())?;
            unroll_extend_expression(prefix_name, unrolled, model)
        }
        ExpressionNode::Model(model) => Ok(Extend::Model(Rc::clone(&model))),
        ExpressionNode::Parenthesis(expression) => {
            unroll_extend_expression(prefix_name, *expression, model)
        }
        ExpressionNode::Add(left, right) => {
            let left = unroll_extend_expression(prefix_name.clone(), *left, model.clone())?;
            let right = unroll_extend_expression(prefix_name.clone(), *right, model.clone())?;
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
            let left = unroll_extend_expression(prefix_name.clone(), *left, model.clone())?;
            let right = unroll_extend_expression(prefix_name.clone(), *right, model.clone())?;
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

pub fn compact_extend(
    prefix_name: String,
    extend: &Extend,
    owned: Rc<RefCell<ModelNode>>,
) -> Result<Extend, Diagnostic> {
    match extend {
        Extend::None | Extend::Unresolved(_) => Err("Неизвестная реализация".into()),
        Extend::Model(model) => {
            // Копируем содержимое модели и меняем его владельца, добавляем в список моделей.
            // Для анонимных моделей (корневой import) имя состоит только из префикса.
            let base = model.borrow().name.clone().unwrap_or_default();
            let model_name = format!("{}{}", prefix_name, base);
            let model_node = owned.borrow().search_model(&model_name);
            if model_node.is_some() {
                return Ok(Extend::Model(model_node.unwrap()));
            }
            let model_node = model.borrow().copy(&model_name, Some(owned.clone()));
            let mode_rc = Rc::new(RefCell::new(model_node));
            owned
                .borrow_mut()
                .models
                .insert(model_name.clone(), mode_rc.clone());
            Ok(Extend::Model(mode_rc))
        }
        Extend::Parentless(extend_item) => compact_extend(prefix_name, extend_item, owned.clone()),
        Extend::Concatenation(extends) => {
            let prefix_name = format!("{}_Concatenation", prefix_name.clone());
            let model = ModelNode::new(prefix_name.clone().as_str(), Some(owned.clone()));
            let mut prev = None;
            let mut n = 0;
            let max_sequence_length: usize = extends.len();
            for implement in extends.iter().rev() {
                let model_name = format!("{}_{}", prefix_name, n);
                let extend_model = compact_extend(model_name.clone(), implement, owned.clone())?;
                n += 1;
                let kind = if n >= max_sequence_length {
                    StateNodeKind::Start
                } else {
                    StateNodeKind::Simple
                };
                let state = StateNode::Implement {
                    upper: Some(Rc::downgrade(&model)),
                    loc: Location::Codegen,
                    named_blocks: vec![],
                    name: model_name.clone(),
                    references: vec![],
                    implements: extend_model.clone(),
                    next: prev.clone(),
                    kind,
                };
                prev = Some(ReferenceNode {
                    location: Location::Codegen,
                    name: state.name().to_string(),
                    cond: ConditionNode::None,
                    object: Box::new(state.clone()),
                });
                model.borrow_mut().states.insert(model_name.clone(), state);
            }
            Ok(Extend::Model(model))
        }
        Extend::Parallel(extends) => {
            let extends = extends
                .iter()
                .map(|implement| {
                    Box::new(compact_extend(prefix_name.clone(), implement, owned.clone()).unwrap())
                })
                .collect::<Vec<_>>();
            Ok(Extend::Parallel(extends))
        }
    }
}

mod minimalistic {
    use crate::diagnostics::{Diagnostic, Location};
    use crate::semantic::ModelNode;
    use std::cell::RefCell;
    use std::collections::HashMap;
    use std::rc::Rc;

    #[derive(Debug, Clone, PartialEq, Eq, Hash)]
    struct Name {
        local: String,
        unique: String,
    }

    impl Name {
        fn new(local: String, unique: String) -> Self {
            Name { local, unique }
        }
    }

    #[derive(Debug, Clone)]
    enum Element {
        Model {
            name: Name,
            states: Vec<Name>,
            start: Name,
        },
        StateExtend {
            name: Name,
            extend: Vec<Element>,
            next: Name,
        },
        State {
            name: Name,
            references: Vec<Name>,
        },
    }

    struct Snapshot {
        root: Rc<RefCell<ModelNode>>,
        used_elements: HashMap<Name, Element>,
        start: Name,
    }

    impl Snapshot {
        pub(crate) fn create(model: Rc<RefCell<ModelNode>>) -> Result<Self, Diagnostic> {
            let mut used_elements = HashMap::new();
            let Some(start) = model.borrow().get_start_state() else {
                return Err(Diagnostic::error(
                    Location::Implicit,
                    "Model must have a start state".to_string(),
                ));
            };
            //TODO: Рекурсивный спуск для создания упрощенной модели
            let local_name = start.name();
            Ok(Snapshot {
                root: model.clone(),
                used_elements,
                start: Name::new(
                    local_name.to_string(),
                    unique_state_name(local_name, model.clone()),
                ),
            })
        }

        #[inline]
        fn model_at(&self, name: &str) -> Option<Rc<RefCell<ModelNode>>> {
            model_by_unique_name(name, self.root.clone())
        }

        #[inline]
        fn used_models(&self) -> Vec<Element> {
            self.used_elements
                .values()
                .filter(|e| matches!(e, Element::Model { .. }))
                .cloned()
                .collect::<Vec<Element>>()
        }
    }

    fn unique_model_name(model: Rc<RefCell<ModelNode>>) -> String {
        let name = model.borrow().name.clone().unwrap_or_default();
        if let Some(upper) = model.borrow().upper.as_ref() {
            return format!("{}:{}", unique_model_name(upper.upgrade().unwrap()), name);
        }
        name
    }

    fn unique_state_name(local_name: &str, model: Rc<RefCell<ModelNode>>) -> String {
        let name = unique_model_name(model);
        format!("{}:{}", &name, local_name)
    }

    fn model_by_unique_name(
        model_name: &str,
        owned: Rc<RefCell<ModelNode>>,
    ) -> Option<Rc<RefCell<ModelNode>>> {
        if let Some(index) = model_name.find(":") {
            let (upper_name, lower_name) = model_name.split_at(index);
            if owned.borrow().name() == upper_name {
                return model_by_unique_name(&lower_name[2..], owned.clone());
            } else {
                if let Some(model) = owned.borrow().search_model(upper_name) {
                    if lower_name.is_empty() {
                        return Some(model.clone());
                    }
                    return model_by_unique_name(&lower_name[2..], model.clone());
                }
            }
        } else {
            if owned.borrow().name() == model_name {
                return Some(owned);
            } else if let Some(model) = owned.borrow().search_model(model_name) {
                return Some(model.clone());
            }
        }
        None
    }

    #[cfg(test)]
    mod tests {
        use super::*;

        #[test]
        fn test_unique_model_name() {
            let model = ModelNode::new("A", None);
            assert_eq!(unique_model_name(model.clone()), "A");

            let model = ModelNode::new("B", Some(model.clone()));
            assert_eq!(unique_model_name(model.clone()), "A:B");
        }

        #[test]
        fn test_model_by_unique_name() {
            let global = ModelNode::new("A", None);
            let model = ModelNode::new("B", Some(global.clone()));
            let _ = ModelNode::new("C", Some(model.clone()));
            assert_eq!(
                model_by_unique_name("A", global.clone())
                    .unwrap()
                    .borrow()
                    .name(),
                "A"
            );
            assert_eq!(
                model_by_unique_name("A:B", global.clone())
                    .unwrap()
                    .borrow()
                    .name(),
                "B"
            );
            assert_eq!(
                model_by_unique_name("A:B:C", global.clone())
                    .unwrap()
                    .borrow()
                    .name(),
                "C"
            );
        }
    }
}

#[cfg(test)]
mod tests {
    use crate::diagnostics::Location;
    use crate::parse;
    use crate::parser::ast;
    use crate::semantic::extend::{Extend, compact_extend, unroll_extend_expression};
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

        let implement = unroll_extend_expression(
            String::from("O"),
            ExpressionNode::Unresolved(ast::Expression::Variable(ast::Identifier::new("A"))),
            model_rc.clone(),
        )
        .unwrap();
        assert!(matches!(implement, Extend::Model(_)));
        let implement = unroll_extend_expression(
            String::from("O"),
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
        let (ast, _) = parse(SRC, 0).unwrap();
        let model_rc = construct_model(&ast, None, &[]).unwrap();

        let ma = || Box::new(Extend::Model(model_rc.borrow().search_model("A").unwrap()));
        let mb = || Box::new(Extend::Model(model_rc.borrow().search_model("B").unwrap()));
        let par_ab = || Extend::Parallel(vec![ma(), mb()]);

        let pairs = vec![
            // Next1 = A + B + (A | B)
            (
                "Next1",
                Extend::Concatenation(vec![ma(), mb(), Box::new(par_ab())]),
                "Next1_Sequence",
            ),
            // Next2 = A + (B | A) + B
            (
                "Next2",
                Extend::Concatenation(vec![
                    ma(),
                    Box::new(Extend::Parallel(vec![mb(), ma()])),
                    mb(),
                ]),
                "Next2_Sequence",
            ),
            // Next3 = A + (B + A) + B  →  все элементы разворачиваются в одну последовательность
            (
                "Next3",
                Extend::Concatenation(vec![ma(), mb(), ma(), mb()]),
                "Next3_Sequence",
            ),
            // Next4 = A + (B + A) + (B | A)
            (
                "Next4",
                Extend::Concatenation(vec![
                    ma(),
                    mb(),
                    ma(),
                    Box::new(Extend::Parallel(vec![mb(), ma()])),
                ]),
                "Next4_Sequence",
            ),
            // Next5 = (A | B) + (A + B)
            (
                "Next5",
                Extend::Concatenation(vec![Box::new(par_ab()), ma(), mb()]),
                "Next5_Sequence",
            ),
            // Next6 = (A | B) + (A + B) + (A | B)
            (
                "Next6",
                Extend::Concatenation(vec![Box::new(par_ab()), ma(), mb(), Box::new(par_ab())]),
                "Next6_Sequence",
            ),
            // Next7 = (A | B) + (A + B) + (A | B) + (A + B)
            (
                "Next7",
                Extend::Concatenation(vec![
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
                Extend::Concatenation(vec![
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
                Extend::Concatenation(vec![
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
                Extend::Concatenation(vec![
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
                Extend::Parallel(vec![
                    ma(),
                    mb(),
                    Box::new(Extend::Concatenation(vec![ma(), mb()])),
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

        let result = compact_extend(String::from("O"), &Extend::None, model_rc.clone());
        assert!(result.is_err());

        let result = compact_extend(
            String::from("O"),
            &Extend::Unresolved(ast::Expression::Variable(ast::Identifier::new("A"))),
            model_rc.clone(),
        );
        assert!(result.is_err());

        let model_a = model_rc.borrow().search_model("A").unwrap();
        let model_b = model_rc.borrow().search_model("B").unwrap();
        {
            let result = compact_extend(
                String::from("O"),
                &Extend::Model(model_a.clone()),
                model_rc.clone(),
            );
            assert!(result.is_ok());
            let Extend::Model(result) = result.unwrap() else {
                panic!("Result is not a model")
            };
            let result = result.borrow();
            assert_eq!(
                format!("O{}", model_a.borrow().name.clone().unwrap()),
                result.name.clone().unwrap()
            );
        }
        {
            let result = compact_extend(
                String::from("O"),
                &Extend::Concatenation(vec![
                    Box::new(Extend::Model(model_a.clone())),
                    Box::new(Extend::Model(model_b.clone())),
                ]),
                model_rc.clone(),
            );
            assert!(result.is_ok());
            let result = result.unwrap();
            let Extend::Model(result) = result else {
                panic!("Result is not a model")
            };
            let result = result.borrow();
            assert_eq!(result.name(), "O_Concatenation".to_string());
            assert!(result.search_state("O_Concatenation_1").is_some());
            assert!(result.search_state("O_Concatenation_0").is_some());
        }
        {
            let result = compact_extend(
                String::from("O"),
                &Extend::Parallel(vec![
                    Box::new(Extend::Model(model_a.clone())),
                    Box::new(Extend::Model(model_b.clone())),
                ]),
                model_rc.clone(),
            );
            assert!(result.is_ok());
            let result = result.unwrap();
            let Extend::Parallel(implements) = result.clone() else {
                panic!("Result is not a parallel implement")
            };
            assert_eq!(result.name(), "Parallel".to_string());
            assert_eq!(implements.len(), 2);
            assert!(implements[0].is_model());
            assert!(implements[1].is_model());
        }
    }
}
