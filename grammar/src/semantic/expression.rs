//! Построение семантических выражений языка BuT.
//!
//! Текущая реализация является заглушкой: [`construct_expression`] сохраняет
//! АСД-выражение в варианте [`Expression::Unresolved`] без разрешения переменных.
//!
//! TODO: реализовать полное разрешение — переменные, условия, типы.

use crate::diagnostics::Diagnostic;
use crate::parser::ast;
use crate::semantic::{Expression, ModelNode};
use std::cell::RefCell;
use std::rc::Rc;

/// Строит семантическое выражение из АСД-выражения.
///
/// # Заглушка
///
/// Текущая реализация возвращает [`Expression::Unresolved`] для любого входа.
/// **Не используйте эту функцию там, где требуется реальное разрешение** —
/// вместо этого напрямую обрабатывайте [`ast::Expression`]-ветки.
///
/// В частности, [`crate::semantic::tree::construct_implement`] обходит эту функцию
/// для корректной обработки выражений реализации.
#[allow(dead_code)]
pub fn construct_expression(
    expr: ast::Expression,
    _model: Rc<RefCell<ModelNode>>,
) -> Result<Expression, Diagnostic> {
    // TODO: разрезолвить переменные, условия, типы
    Ok(Expression::Unresolved(expr))
}
