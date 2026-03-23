pub mod tree;

use std::collections::HashMap;
use std::fmt::Debug;
use std::rc::Rc;

#[derive(Debug)]
pub enum Diagnostic {
    Error(Vec<String>),
}

impl Into<Diagnostic> for &str {
    fn into(self) -> Diagnostic {
        Diagnostic::Error(vec![self.to_owned()])
    }
}

#[derive(Default, Debug, PartialEq, Eq, Clone)]
pub struct ContextNode {
    upper: Option<Rc<ContextNode>>,
    imports: HashMap<String, Rc<ModelNode>>,
    models: HashMap<String, Rc<ModelNode>>,
    named_blocks: HashMap<String, NamedBlockNode>,
    functions: HashMap<String, FunctionNode>,
    variables: HashMap<String, VariableNode>,
    types: HashMap<String, TypeNode>,
    conditions: HashMap<String, ConditionNode>,
}

#[derive(Default, Debug, PartialEq, Eq, Clone)]
pub struct ModelNode {
    context: ContextNode,
    name: Option<String>,
    states: HashMap<String, StateNode>,
    implements: (),
}

impl ModelNode {
    pub fn has_states(&self) -> bool {
        !self.states.is_empty()
    }
}

#[derive(Default, Debug, PartialEq, Eq, Clone)]
pub struct NamedBlockNode {}

#[derive(Default, Debug, PartialEq, Eq, Clone)]
pub struct FunctionNode {}

#[derive(Default, Debug, PartialEq, Eq, Clone)]
pub struct VariableNode {}

#[derive(Default, Debug, PartialEq, Eq, Clone)]
pub struct TypeNode {}

#[derive(Default, Debug, PartialEq, Eq, Clone)]
pub struct ConditionNode {}

#[derive(Default, Debug, PartialEq, Eq, Clone)]
pub enum StateNode {
    #[default]
    Unresolved,
    Simple {
        context: ContextNode,
        name: String,
        references: Vec<Reference<StateNode>>,
    },
    Implement {
        context: ContextNode,
        name: String,
        references: Vec<Reference<StateNode>>,
        implements: (),
        next: Option<Reference<StateNode>>,
    },
}

#[derive(Default, Debug, PartialEq, Eq, Clone)]
pub enum Condition {
    #[default]
    None,
}

#[derive(Default, Debug, PartialEq, Eq, Clone)]
pub struct Reference<T: Clone + PartialEq + Eq + Debug> {
    name: String,
    cond: Condition,
    object: Box<T>,
}
