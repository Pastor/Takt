use crate::parser::ast;
use crate::parser::ast::{Model, ModelElement, StateDefine, StateElement};
use crate::semantic::{Condition, ContextNode, Diagnostic, ModelNode, Reference, StateNode};
use std::collections::HashMap;
use std::rc::Rc;

pub fn construct_model(model: &Model) -> Result<ModelNode, Diagnostic> {
    let name = model.name.clone();
    let context = construct_context_model(model)?;
    let states = construct_states(model)?;
    Ok(ModelNode {
        context,
        name: name.map(|i| i.name.clone()),
        states,
        implements: (),
    })
}

pub fn construct_states(model: &Model) -> Result<HashMap<String, StateNode>, Diagnostic> {
    let states: &mut HashMap<String, Box<StateNode>> = &mut HashMap::new();
    for element in model.elements.iter() {
        if let ModelElement::State(def) = element {
            let name = def
                .clone()
                .name
                .ok_or_else(|| "Model state not naming".into())?
                .name;
            let context = construct_context_state(def)?;
            let implements = def.implements.clone();
            let mut references = Vec::new();
            let mut next = None;
            for element in def.elements.iter() {
                if let StateElement::Reference(_, id, cond) = element {
                    let name = id.name.clone();
                    let cond = if let Some(cond) = cond {
                        construct_condition(cond)?
                    } else {
                        Condition::None
                    };
                    references.push(Reference {
                        name,
                        cond,
                        object: Box::new(StateNode::Unresolved),
                    });
                } else if let StateElement::Next(id) = element {
                    let name = id.name.clone();
                    if next.is_some() {
                        return Err(format!("State '{}' already defined", &name).as_str().into());
                    }
                    next = Some(name);
                }
            }
            let state = if let Some(expr) = implements {
                let next = if next.is_none() {
                    None
                } else {
                    Some(Reference {
                        name: next.unwrap().clone(),
                        cond: Condition::None,
                        object: Box::new(StateNode::Unresolved),
                    })
                };
                StateNode::Implement {
                    context,
                    name: name.clone(),
                    references,
                    implements: (),
                    next,
                }
            } else {
                StateNode::Simple {
                    context,
                    name: name.clone(),
                    references,
                }
            };
            states.insert(name, Box::new(state));
        }
    }
    let new_states = &mut HashMap::new();
    for (_, state) in states.iter() {
        if let StateNode::Simple {
            context,
            name,
            references,
        } = *state.clone()
        {
            let mut new_references: &mut Vec<Reference<StateNode>> = &mut Vec::new();
            for reference in references {
                if let StateNode::Unresolved = *reference.object {
                    let state = states.get(&reference.name).ok_or_else(|| {
                        format!("Reference '{}' not found", &reference.name)
                            .as_str()
                            .into()
                    })?;
                    new_references.push(Reference {
                        name: reference.name,
                        cond: reference.cond,
                        object: state.clone(),
                    });
                } else {
                    new_references.push(reference)
                }
            }
            new_states.insert(
                name.clone(),
                StateNode::Simple {
                    context: context.clone(),
                    name: name.clone(),
                    references: new_references.clone(),
                },
            );
        } else if let StateNode::Implement {
            context,
            name,
            references,
            implements,
            next,
        } = *state.clone()
        {
            let mut new_references: &mut Vec<Reference<StateNode>> = &mut Vec::new();
            let mut next = next.clone();
            for reference in references {
                if let StateNode::Unresolved = *reference.object {
                    let state = states.get(&reference.name).ok_or_else(|| {
                        format!("Reference '{}' not found", &reference.name)
                            .as_str()
                            .into()
                    })?;
                    new_references.push(Reference {
                        name: reference.name,
                        cond: reference.cond,
                        object: state.clone(),
                    });
                } else {
                    new_references.push(reference)
                }
            }
            if let Some(next) = next.as_mut() {
                if let StateNode::Unresolved = *next.object {
                    let state = states.get(&next.name).ok_or_else(|| {
                        format!("Reference '{}' not found", &next.name)
                            .as_str()
                            .into()
                    })?;
                    *next = Reference {
                        name: name.clone(),
                        cond: next.cond.clone(),
                        object: state.clone(),
                    }
                }
            }

            new_states.insert(
                name.clone(),
                StateNode::Implement {
                    context: context.clone(),
                    name: name.clone(),
                    references: new_references.clone(),
                    implements: implements.clone(),
                    next: next.clone(),
                },
            );
        }
    }
    Ok(new_states.clone())
}

fn construct_context_model(model: &Model) -> Result<ContextNode, Diagnostic> {
    let mut models = HashMap::new();
    for element in model.elements.iter() {
        if let ModelElement::Model(def) = element {
            let model = construct_model(&def)?;
            models.insert(def.clone().name.unwrap().name.clone(), Rc::new(model));
        }
    }
    Ok(ContextNode {
        models,
        ..Default::default()
    })
}

fn construct_context_state(state: &StateDefine) -> Result<ContextNode, Diagnostic> {
    Ok(Default::default())
}

fn construct_condition(cond: &ast::Condition) -> Result<Condition, Diagnostic> {
    Ok(Condition::None)
}
