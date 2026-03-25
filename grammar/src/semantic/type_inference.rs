use crate::diagnostics::Diagnostic;
use crate::semantic::{Expression, ModelNode, TypeNode, VariableNode};
use std::cell::RefCell;
use std::collections::HashMap;
use std::rc::Rc;

pub fn type_inference(
    variables: &mut HashMap<String, VariableNode>,
    model: Rc<RefCell<ModelNode>>,
) -> Result<HashMap<String, VariableNode>, Diagnostic> {
    for (name, var) in variables.clone() {
        match var {
            VariableNode::Simple(_, TypeNode::Inference, expr) => {
                let typ = extract_type(&expr, model.clone())?;
                variables.insert(name.clone(), VariableNode::Simple(name.clone(), typ, expr));
            }
            VariableNode::Const(_, TypeNode::Inference, expr) => {
                let typ = extract_type(&expr, model.clone())?;
                variables.insert(name.clone(), VariableNode::Const(name.clone(), typ, expr));
            }
            _ => {}
        }
    }
    Ok(variables.clone())
}

fn extract_type(_expr: &Expression, _model: Rc<RefCell<ModelNode>>) -> Result<TypeNode, Diagnostic> {
    //TODO: необходимо пройтись по всему выражению и разрезолвить типы если они не разрезолвлены и потом вывести тип выражения или ошибку
    Ok(TypeNode::Unsupported)
}
