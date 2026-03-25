use crate::diagnostics::Diagnostic;
use crate::parser::ast::Type;
use crate::semantic::TypeNode;
use std::collections::HashMap;

/// Строит [`TypeNode`] из опционального АСД-типа [`Type`].
///
/// Используется как в построении модели, так и в выводе типов
/// (например, для выражения `as T`).
pub(crate) fn construct_type(
    typ: Option<Type>,
    map: &HashMap<String, TypeNode>,
) -> Result<TypeNode, Diagnostic> {
    if typ.is_none() {
        return Ok(TypeNode::Inference);
    }
    match typ.unwrap() {
        Type::Address { address, bit } => Ok(TypeNode::Address(address, bit)),
        Type::Bit => Ok(TypeNode::Bit),
        Type::Bool => Ok(TypeNode::Bool),
        Type::Rational => Ok(TypeNode::Rational),
        Type::Alias(def) => match def.name.as_str() {
            "bit" => Ok(TypeNode::Bit),
            "bool" => Ok(TypeNode::Bool),
            "float" => Ok(TypeNode::Rational),
            "unit" => Ok(TypeNode::Unit),
            local => Ok(map
                .get(local)
                .ok_or_else(|| {
                    format!("Локальный тип '{}' не найден", &def.name)
                        .as_str()
                        .into()
                })?
                .clone()),
        },
        Type::Array {
            element_type,
            element_count,
            ..
        } => Ok(TypeNode::Array(
            element_count,
            Box::new(construct_type(Some(*element_type), map)?),
        )),
        Type::Function { .. } => Ok(TypeNode::Unsupported),
        Type::Unit => Ok(TypeNode::Unit),
    }
}
