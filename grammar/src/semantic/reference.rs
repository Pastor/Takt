use crate::diagnostics::Diagnostic;
use crate::semantic::condition::resolve_condition;
use crate::semantic::{Condition, ModelNode, Reference, StateNode};
use std::cell::RefCell;
use std::rc::Rc;

fn resolve_references(
    references: &Vec<Reference<StateNode>>,
    model: Rc<RefCell<ModelNode>>,
) -> Result<Vec<Reference<StateNode>>, Diagnostic> {
    let mut new_references = Vec::with_capacity(references.len());
    for reference in references {
        if let Condition::Unresolved(cond) = reference.cond.clone() {
            let resolved_cond = resolve_condition(&cond, model.clone())?;
            let resolved_reference = Reference {
                cond: resolved_cond,
                ..reference.clone()
            };
            new_references.push(resolved_reference);
        } else {
            new_references.push(reference.clone());
        }
    }
    Ok(new_references)
}

pub fn resolve_state_references(
    state: &StateNode,
    model: Rc<RefCell<ModelNode>>,
) -> Result<StateNode, Diagnostic> {
    if let StateNode::Simple {
        references,
        name,
        named_blocks,
        kind,
    } = state
    {
        Ok(StateNode::Simple {
            named_blocks: named_blocks.clone(),
            name: name.clone(),
            references: resolve_references(references, model.clone())?,
            kind: kind.clone(),
        })
    } else if let StateNode::Implement {
        references,
        name,
        named_blocks,
        implements,
        next,
        kind,
    } = state
    {
        Ok(StateNode::Implement {
            named_blocks: named_blocks.clone(),
            name: name.clone(),
            references: resolve_references(references, model.clone())?,
            implements: implements.clone(),
            next: next.clone(),
            kind: kind.clone(),
        })
    } else {
        Ok(state.clone())
    }
}
