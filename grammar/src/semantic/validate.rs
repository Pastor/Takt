use crate::diagnostics::Diagnostic;
use crate::semantic::{ModelNode, StateNode, StateNodeKind};
use std::cell::RefCell;
use std::rc::Rc;

fn model_only_one_start_state(model: Rc<RefCell<ModelNode>>) -> Result<(), Diagnostic> {
    let name = model.borrow().name.clone().unwrap_or_default();
    if model
        .borrow()
        .states
        .iter()
        .filter(|state| {
            let state = state.clone().1.clone();
            if let StateNode::Simple {
                kind: StateNodeKind::Start,
                ..
            } = state
            {
                true
            } else if let StateNode::Implement {
                kind: StateNodeKind::Start,
                ..
            } = state
            {
                true
            } else {
                false
            }
        })
        .count()
        != 1
    {
        return Err(format!(
            "В модели '{}' должно быть только одно начальное состояние",
            name
        )
        .as_str()
        .into());
    }
    Ok(())
}

pub fn validate_model(model: Rc<RefCell<ModelNode>>) -> Result<(), Diagnostic> {
    model_only_one_start_state(model.clone())?;
    let nested: Vec<(String, Rc<RefCell<ModelNode>>)> = model
        .borrow()
        .models
        .iter()
        .map(|(k, v)| (k.clone(), Rc::clone(v)))
        .collect();
    for (_, nested_model) in nested {
        model_only_one_start_state(nested_model)?;
    }
    Ok(())
}
