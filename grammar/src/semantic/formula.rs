//! Семантическое представление LTL-формул языка Lam.
//!
//! Содержит тип [`Formula`] и функцию преобразования [`condition_to_formula`],
//! которая переводит [`ConditionNode`] в [`Formula`].

use crate::parser::ast::LtlExpr;
use crate::semantic::ConditionNode;
use crate::verification::ltl::Ltl;
use std::rc::Rc;

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

pub fn ltl_ast_to_semantic(expr: &LtlExpr) -> Ltl {
    match expr {
        LtlExpr::True(_) => Ltl::True,
        LtlExpr::False(_) => Ltl::False,
        LtlExpr::Atom(id) => Ltl::Atom(id.name.clone()),
        LtlExpr::Not(_, inner) => Ltl::Not(Rc::new(ltl_ast_to_semantic(inner))),
        LtlExpr::Next(_, inner) => Ltl::Next(Rc::new(ltl_ast_to_semantic(inner))),
        LtlExpr::Finally(_, inner) => Ltl::Finally(Rc::new(ltl_ast_to_semantic(inner))),
        LtlExpr::Globally(_, inner) => Ltl::Globally(Rc::new(ltl_ast_to_semantic(inner))),
        LtlExpr::And(_, l, r) => Ltl::And(
            Rc::new(ltl_ast_to_semantic(l)),
            Rc::new(ltl_ast_to_semantic(r)),
        ),
        LtlExpr::Or(_, l, r) => Ltl::Or(
            Rc::new(ltl_ast_to_semantic(l)),
            Rc::new(ltl_ast_to_semantic(r)),
        ),
        LtlExpr::Until(_, l, r) => Ltl::Until(
            Rc::new(ltl_ast_to_semantic(l)),
            Rc::new(ltl_ast_to_semantic(r)),
        ),
        LtlExpr::Release(_, l, r) => Ltl::Release(
            Rc::new(ltl_ast_to_semantic(l)),
            Rc::new(ltl_ast_to_semantic(r)),
        ),
        LtlExpr::Implies(_, l, r) => Ltl::Implies(
            Rc::new(ltl_ast_to_semantic(l)),
            Rc::new(ltl_ast_to_semantic(r)),
        ),
        LtlExpr::Parenthesis(_, inner) => ltl_ast_to_semantic(inner),
    }
}
