//! Остаток в сравнении с нулём у цели `rust` (фича 0448).
//!
//! # Что было
//!
//! Цель печатала `x % k = 0` как `(x % k) == 0`, а `clippy` под `-D warnings`
//! — **флагами гейта самой цели** — отвечает `manual_is_multiple_of`. То есть
//! файл на диске есть, код возврата `taktc` нулевой, а собрать прошивку
//! нельзя.
//!
//! ⚠️ Гейт цели гоняет только корпус, а записи `x % k = 0` в `examples/` нет
//! вовсе — потому класс и не был виден. Нашёлся он пробой (фича 0447).
//!
//! # Правило
//!
//! Печатать методом там, и только там, где замена **тождественна**. Проверка
//! идёт по УЗЛАМ, а не по напечатанному тексту: форма записи (скобки, порядок
//! сторон) значения не имеет.

use crate::diagnostics::Diagnostic;
use crate::semantic::ExpressionNode;
use crate::semantic::type_node::TypeNode;

use super::rust_expr::{Scope, print_expression};
use super::rust_fixed::expression_type;

/// Печатает `x % k = 0` методом `is_multiple_of` (фича 0448).
///
/// # Зачем
///
/// `clippy` под `-D warnings` — флагами гейта самой цели — отвечает
/// `manual_is_multiple_of`: «manual implementation of `.is_multiple_of()`». То
/// есть цель клала на диск файл, который её собственный гейт отвергает. В
/// корпусе записи `n % 2 = 0` нет, поэтому гейт класса не видел; нашёлся он
/// пробой (фича 0447).
///
/// # Границы правила — они же условие тождественности
///
/// - **тип левого операнда беззнаковый**: `is_multiple_of` в стабильном Rust
///   есть только у беззнаковых целых, и `clippy` тем же ограничен;
/// - **делитель — ненулевой литерал**: `x % 0` паникует, а
///   `x.is_multiple_of(0)` возвращает `x == 0` — на переменной делителе замена
///   меняла бы поведение, а не форму;
/// - **сравнение с нулём** и только `=`/`!=`.
pub(super) fn multiple_of(
    a: &ExpressionNode,
    op: &str,
    b: &ExpressionNode,
    scope: &Scope,
) -> Result<Option<String>, Diagnostic> {
    let negate = match op {
        "==" => false,
        "!=" => true,
        _ => return Ok(None),
    };
    // Стороны равноправны: `0 = n % 2` даёт тот же отказ линтера, что
    // `n % 2 = 0` (замер 0448 — обе формы).
    let (remainder, zero) = match (unwrap_parens(a), unwrap_parens(b)) {
        (rem @ ExpressionNode::Modulo(_, _), z) => (rem, z),
        (z, rem @ ExpressionNode::Modulo(_, _)) => (rem, z),
        _ => return Ok(None),
    };
    if !matches!(zero, ExpressionNode::Number(0)) {
        return Ok(None);
    }
    let ExpressionNode::Modulo(value, divisor) = remainder else {
        return Ok(None);
    };
    let ExpressionNode::Number(k) = unwrap_parens(divisor) else {
        return Ok(None);
    };
    if *k <= 0 {
        return Ok(None);
    }
    if !matches!(
        expression_type(value),
        Some(TypeNode::Integer { signed: false, .. })
    ) {
        return Ok(None);
    }
    let text = format!("{}.is_multiple_of({k})", print_expression(value, scope)?);
    Ok(Some(if negate {
        format!("(!{text})")
    } else {
        format!("({text})")
    }))
}

/// Снимает скобки выражения — форма записи для правила значения не имеет.
fn unwrap_parens(expr: &ExpressionNode) -> &ExpressionNode {
    match expr {
        ExpressionNode::Parenthesis(inner) => unwrap_parens(inner),
        other => other,
    }
}
