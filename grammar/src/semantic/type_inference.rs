use crate::diagnostics::Diagnostic;
use crate::semantic::expression::construct_expression;
use crate::semantic::{ExpressionNode, ModelNode, TypeNode, VariableNode};
use std::cell::RefCell;
use std::collections::HashMap;
use std::rc::Rc;

pub fn type_inference(
    variables: &mut HashMap<String, VariableNode>,
    model: Rc<RefCell<ModelNode>>,
) -> Result<HashMap<String, VariableNode>, Diagnostic> {
    for (name, var) in variables.clone() {
        match var {
            VariableNode::Simple(_, TypeNode::Inference, Some(expr)) => {
                let expr_node = construct_expression(expr.clone(), model.clone())?;
                let typ = extract_type(expr_node)?;
                variables.insert(
                    name.clone(),
                    VariableNode::Simple(name.clone(), typ, Some(expr)),
                );
            }
            VariableNode::Const(_, TypeNode::Inference, expr) => {
                let expr_node = construct_expression(expr.clone(), model.clone())?;
                let typ = extract_type(expr_node)?;
                variables.insert(name.clone(), VariableNode::Const(name.clone(), typ, expr));
            }
            _ => {}
        }
    }
    Ok(variables.clone())
}

fn extract_type(expr: ExpressionNode) -> Result<TypeNode, Diagnostic> {
    Ok(TypeNode::Unsupported)
}
