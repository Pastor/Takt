use crate::diagnostics::Diagnostic;
use crate::semantic::{FunctionNode, TypeNode};
use phf::phf_map;
use std::convert::Into;

const BUILTIN_FUNCTIONS: phf::Map<&'static str, FunctionNode> = phf_map! {
    "debug" => FunctionNode::Builtin("debug", &[("text", TypeNode::BuiltinString)], TypeNode::Unit),
    "S" => FunctionNode::Builtin("S", &[("model", TypeNode::BuiltinModel)], TypeNode::BuiltinState),
};

pub fn builtin_function(name: &str) -> Result<&FunctionNode, Diagnostic> {
    Ok(BUILTIN_FUNCTIONS
        .get(name)
        .ok_or_else(|| format!("Неизвестная функция '{}'", name).as_str().into())?)
}
