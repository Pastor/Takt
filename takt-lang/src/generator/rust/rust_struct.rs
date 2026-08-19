//! Литерал структуры для цели `rust` (фича 0293).
//!
//! Отдельный модуль, потому что `rust_expr.rs` пришпилен лимитом размера, а
//! знание «как выглядит агрегат структуры» самостоятельно: оно повторяет
//! правило именования полей из `rust_decl` и обязано меняться вместе с ним.

use crate::diagnostics::{Diagnostic, Location};
use crate::generator::rust::rust_expr::{Scope, coerce_to, unsupported};
use crate::generator::rust::rust_name::rust_type_name;
use crate::semantic::ExpressionNode;

/// Литерал структуры: `Gains { kp: 2, ki: 3 }` (фича 0293).
///
/// Порядок значений — **объявленный** (инициализатор языка позиционный), имена
/// полей нормируются тем же правилом, что и объявление в `rust_decl`.
///
/// # Ошибки
/// `RS-011`, если структура не объявлена либо число значений не совпало с
/// числом полей: молча дополнять умолчаниями нельзя — это тихо иное значение.
pub(crate) fn struct_literal(
    name: &str,
    items: &[ExpressionNode],
    scope: &Scope,
) -> Result<String, Diagnostic> {
    let def = scope
        .model
        .search_struct(name)
        .ok_or_else(|| unsupported(&format!("структура '{name}' не объявлена")))?;
    if def.fields.len() != items.len() {
        return Err(unsupported(&format!(
            "инициализатор структуры '{name}': объявлено полей {}, значений {}",
            def.fields.len(),
            items.len()
        )));
    }
    let mut parts = Vec::with_capacity(items.len());
    for ((field, field_ty), value) in def.fields.iter().zip(items) {
        parts.push(format!(
            "{}: {}",
            crate::semantic::naming::normalize_lowercase_snakecase(field.clone()),
            coerce_to(value, field_ty, scope)?
        ));
    }
    Ok(format!(
        "{} {{ {} }}",
        rust_type_name(name, Location::Codegen)?,
        parts.join(", ")
    ))
}
