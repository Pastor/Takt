//! Семантическое представление LTL-формул языка BuT.
//!
//! Содержит тип [`Formula`] и функцию преобразования [`condition_to_formula`],
//! которая переводит [`ConditionNode`] в [`Formula`].

use crate::semantic::ConditionNode;
use crate::verification::ltl::Ltl;

#[derive(Debug, Clone)]
pub enum Formula {
    None,
    /// Последовательность формул, разделённые запятой
    Formulas(Vec<Formula>),
    LTL(Ltl),
    Guard(ConditionNode),
}

impl PartialEq for Formula {
    fn eq(&self, other: &Self) -> bool {
        match (self, other) {
            (Self::LTL(a), Self::LTL(b)) => a == b,
            (Self::Formulas(a), Self::Formulas(b)) => a == b,
            (Self::Guard(a), Self::Guard(b)) => a == b,
            _ => false,
        }
    }
}

impl Eq for Formula {}

pub fn condition_to_formula(cond: &ConditionNode) -> Formula {
    match cond {
        ConditionNode::None => Formula::None,
        cond => Formula::Guard(cond.clone()),
    }
}
