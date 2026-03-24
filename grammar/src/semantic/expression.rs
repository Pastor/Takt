use crate::diagnostics::Diagnostic;
use crate::parser::ast::Expression;
use crate::semantic::{ExpressionNode, ModelNode};
use std::cell::RefCell;
use std::rc::Rc;

pub fn construct_expression(
    expr: Expression,
    model: Rc<RefCell<ModelNode>>,
) -> Result<ExpressionNode, Diagnostic> {
    Ok(ExpressionNode::None)
}
