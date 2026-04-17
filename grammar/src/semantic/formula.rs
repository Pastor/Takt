//! Семантическое представление LTL-формул языка BuT.
//!
//! Содержит тип [`Formula`] и функцию преобразования [`condition_to_formula`],
//! которая переводит [`ConditionNode`] в [`Formula`].

use crate::parser::ast::Member;
use crate::semantic::enum_node::EnumDefinitionNode;
use crate::semantic::minimap::{Element, Name};
use crate::semantic::{ConditionNode, FunctionDefinitionNode, ModelNode, StateNode, VariableNode};
use std::cell::RefCell;
use std::rc::Rc;

/// Семантическое представление LTL-формулы.
///
/// # Шаблоны LTL
///
/// - Инвариант: `G (p → q)` — всегда, если p истинно, то q тоже истинно
/// - Живучесть: `G (p → F q)` — всегда, если случается p, то затем обязательно случится q
/// - Ответ: `G (req → F ack)` — каждый запрос рано или поздно получит подтверждение
/// - Строгая очередность: `G (p → X q)` — за p всегда немедленно следует q
/// - Отсутствие блокировки: `G ~deadlock` — никогда не возникает deadlock
/// - Предшествование: `(~q) U p` — p случается до того, как случится q
///
/// # Темпоральные операторы
///
/// - `X φ` («next φ») — в следующем такте φ истинно
/// - `F φ` («eventually φ») — когда-нибудь φ станет истинным
/// - `G φ` («globally φ») — φ истинно всегда
/// - `φ U ψ` («φ until ψ») — ψ рано или поздно случится, до тех пор φ истинно
/// - `φ R ψ` («φ release ψ») — ψ истинно, пока не случится φ
#[derive(Debug, Clone)]
pub enum Formula {
    /// Последовательность формул, разделённые запятой
    Formulas(Vec<Formula>),
    /// (`a`)
    Parenthesis(Box<Formula>),
    /// `~a`
    Not(Box<Formula>),
    /// `X a -> next a`
    Next(Box<Formula>),
    /// `F a -> finally a`
    Finally(Box<Formula>),
    /// `G a -> globally a`
    Globally(Box<Formula>),
    /// `a U b -> a until b`
    Until(Box<Formula>, Box<Formula>),
    /// `a R b -> a release b`
    Release(Box<Formula>, Box<Formula>),
    /// `a & b -> a and b`
    And(Box<Formula>, Box<Formula>),
    /// `a | b -> a or b`
    Or(Box<Formula>, Box<Formula>),
    /// `a -> b`
    Implies(Box<Formula>, Box<Formula>),
    /// `a = b`
    Equals(Box<Formula>, Box<Formula>),
    /// `a != b`
    NotEqual(Box<Formula>, Box<Formula>),
    /// `a > b`
    Great(Box<Formula>, Box<Formula>),
    /// `a < b`
    Less(Box<Formula>, Box<Formula>),
    /// `a >= b`
    MoreEqual(Box<Formula>, Box<Formula>),
    /// `a <= b`
    LessEqual(Box<Formula>, Box<Formula>),
    /// `a + b`
    Add(Box<Formula>, Box<Formula>),
    /// `a - b`
    Subtract(Box<Formula>, Box<Formula>),
    /// Константа `true` или `false`
    Boolean(bool),
    /// Переменная модели или состояния или локальная
    Variable(Rc<RefCell<VariableNode>>),
    /// Числовая константа
    Number(i64),
    /// Рациональная константа
    Rational(String),
    /// Доступ к элементу массива: `id[n]`
    ArraySubscript(Rc<RefCell<VariableNode>>, i64),
    /// Доступ к биту: `условие.член`
    BitAccess(Box<Formula>, Member),
    /// Вызов функции: `id(аргументы,*)`
    FunctionCall(Rc<RefCell<FunctionDefinitionNode>>, Vec<Formula>),
    /// Вариант перечисления (Ce4/NI6)
    EnumVariant(Rc<RefCell<EnumDefinitionNode>>, String, i64),
    /// Ссылка на состояние
    State(Rc<RefCell<StateNode>>),
    /// Ссылка на модель
    Model(Rc<RefCell<ModelNode>>),
    /// Строковый литерал
    String(Vec<String>),
    /// Проверка состояния модели
    /// `Model::SubModel::State = Model::SubModel::State::End`
    #[allow(private_interfaces)]
    EqualsState {
        /// Элемент карты модели.
        model: Element,
        /// Имя состояния.
        state: Name,
    },
}

impl PartialEq for Formula {
    fn eq(&self, other: &Self) -> bool {
        match (self, other) {
            (Self::Formulas(a), Self::Formulas(b)) => a == b,
            (Self::Parenthesis(a), Self::Parenthesis(b)) => a == b,
            (Self::Not(a), Self::Not(b)) => a == b,
            (Self::Next(a), Self::Next(b)) => a == b,
            (Self::Finally(a), Self::Finally(b)) => a == b,
            (Self::Globally(a), Self::Globally(b)) => a == b,
            (Self::Until(l1, r1), Self::Until(l2, r2)) => l1 == l2 && r1 == r2,
            (Self::Release(l1, r1), Self::Release(l2, r2)) => l1 == l2 && r1 == r2,
            (Self::And(l1, r1), Self::And(l2, r2)) => l1 == l2 && r1 == r2,
            (Self::Or(l1, r1), Self::Or(l2, r2)) => l1 == l2 && r1 == r2,
            (Self::Implies(l1, r1), Self::Implies(l2, r2)) => l1 == l2 && r1 == r2,
            (Self::Equals(l1, r1), Self::Equals(l2, r2)) => l1 == l2 && r1 == r2,
            (Self::NotEqual(l1, r1), Self::NotEqual(l2, r2)) => l1 == l2 && r1 == r2,
            (Self::Great(l1, r1), Self::Great(l2, r2)) => l1 == l2 && r1 == r2,
            (Self::Less(l1, r1), Self::Less(l2, r2)) => l1 == l2 && r1 == r2,
            (Self::MoreEqual(l1, r1), Self::MoreEqual(l2, r2)) => l1 == l2 && r1 == r2,
            (Self::LessEqual(l1, r1), Self::LessEqual(l2, r2)) => l1 == l2 && r1 == r2,
            (Self::Add(l1, r1), Self::Add(l2, r2)) => l1 == l2 && r1 == r2,
            (Self::Subtract(l1, r1), Self::Subtract(l2, r2)) => l1 == l2 && r1 == r2,
            (Self::Boolean(a), Self::Boolean(b)) => a == b,
            (Self::Variable(v1), Self::Variable(v2)) => v1 == v2,
            (Self::Number(a), Self::Number(b)) => a == b,
            (Self::Rational(a), Self::Rational(b)) => a == b,
            (Self::ArraySubscript(v1, n1), Self::ArraySubscript(v2, n2)) => v1 == v2 && n1 == n2,
            (Self::BitAccess(a, ma), Self::BitAccess(b, mb)) => a == b && ma == mb,
            (Self::FunctionCall(f1, args1), Self::FunctionCall(f2, args2)) => {
                f1 == f2 && args1 == args2
            }
            (Self::EnumVariant(e1, n1, v1), Self::EnumVariant(e2, n2, v2)) => {
                e1 == e2 && n1 == n2 && v1 == v2
            }
            (Self::State(a), Self::State(b)) => a == b,
            (Self::Model(a), Self::Model(b)) => a == b,
            (Self::String(a), Self::String(b)) => a == b,
            (
                Self::EqualsState {
                    model: m1,
                    state: s1,
                },
                Self::EqualsState {
                    model: m2,
                    state: s2,
                },
            ) => m1 == m2 && s1 == s2,
            _ => false,
        }
    }
}

impl Eq for Formula {}

/// Преобразует [`ConditionNode`] в [`Formula`].
///
/// Выполняет структурное отображение всех вариантов условия в соответствующие
/// варианты формулы. `ConditionNode::None` отображается в `Formula::Boolean(true)`
/// (безусловный переход — истина), `ConditionNode::Unresolved` — в `Formula::Boolean(false)`
/// (заглушка).
pub fn condition_to_formula(cond: &ConditionNode) -> Formula {
    match cond {
        ConditionNode::None => Formula::Boolean(true),
        ConditionNode::Unresolved(_) => Formula::Boolean(false),
        ConditionNode::Parenthesis(c) => Formula::Parenthesis(Box::new(condition_to_formula(c))),
        ConditionNode::Not(c) => Formula::Not(Box::new(condition_to_formula(c))),
        ConditionNode::Add(l, r) => Formula::Add(
            Box::new(condition_to_formula(l)),
            Box::new(condition_to_formula(r)),
        ),
        ConditionNode::Subtract(l, r) => Formula::Subtract(
            Box::new(condition_to_formula(l)),
            Box::new(condition_to_formula(r)),
        ),
        ConditionNode::And(l, r) => Formula::And(
            Box::new(condition_to_formula(l)),
            Box::new(condition_to_formula(r)),
        ),
        ConditionNode::Or(l, r) => Formula::Or(
            Box::new(condition_to_formula(l)),
            Box::new(condition_to_formula(r)),
        ),
        ConditionNode::Less(l, r) => Formula::Less(
            Box::new(condition_to_formula(l)),
            Box::new(condition_to_formula(r)),
        ),
        ConditionNode::More(l, r) => Formula::Great(
            Box::new(condition_to_formula(l)),
            Box::new(condition_to_formula(r)),
        ),
        ConditionNode::LessEqual(l, r) => Formula::LessEqual(
            Box::new(condition_to_formula(l)),
            Box::new(condition_to_formula(r)),
        ),
        ConditionNode::MoreEqual(l, r) => Formula::MoreEqual(
            Box::new(condition_to_formula(l)),
            Box::new(condition_to_formula(r)),
        ),
        ConditionNode::Equal(l, r) => Formula::Equals(
            Box::new(condition_to_formula(l)),
            Box::new(condition_to_formula(r)),
        ),
        ConditionNode::NotEqual(l, r) => Formula::NotEqual(
            Box::new(condition_to_formula(l)),
            Box::new(condition_to_formula(r)),
        ),
        ConditionNode::Number(n) => Formula::Number(*n),
        ConditionNode::Rational(s, neg) => {
            Formula::Rational(if *neg { format!("-{}", s) } else { s.clone() })
        }
        ConditionNode::Bool(b) => Formula::Boolean(*b),
        ConditionNode::Variable(v, _) => Formula::Variable(Rc::clone(v)),
        ConditionNode::ArraySubscript(v, n) => Formula::ArraySubscript(Rc::clone(v), *n),
        ConditionNode::BitAccess(c, m) => {
            Formula::BitAccess(Box::new(condition_to_formula(c)), m.clone())
        }
        ConditionNode::Function(f, args, _) => Formula::FunctionCall(
            Rc::clone(f),
            args.iter().map(|a| condition_to_formula(a)).collect(),
        ),
        ConditionNode::EnumVariant(e, name, val) => {
            Formula::EnumVariant(Rc::clone(e), name.clone(), *val)
        }
        ConditionNode::State(s) => Formula::State(Rc::clone(s)),
        ConditionNode::Model(m) => Formula::Model(Rc::clone(m)),
        ConditionNode::String(parts) => Formula::String(parts.clone()),
    }
}
