//! Сдвиг на величину, не меньшую ширины типа (фича 0326).
//!
//! # Что было
//!
//! `var a: i8 := -8; v := a >> 8;` — эталон даёт **−1** (арифметический сдвиг
//! заполняет разряды знаком), цель `c` печатает `model->a >> 8` и даёт то же
//! (операнды в C продвигаются до `int`), цель `st` — floor-деление на `2⁸`,
//! тоже −1, цель `sv` — `>>> 8`, тоже −1 (проверено прогоном verilator).
//!
//! А цель `rust` печатала `self.a >> 8`, и **`rustc` отвергал** такой код:
//! «attempt to shift right by `8_i32`, which would overflow». Код возврата
//! `taktc` при этом **ноль** — класс «инструмент рапортует об успехе, а вывод
//! невалиден» (0262, 0287).
//!
//! # Что делается
//!
//! При **литеральной** величине сдвига, не меньшей ширины типа, печатается то
//! же значение, что даёт эталон, но выразимой формой:
//!
//! - беззнаковый тип — `0`: все разряды ушли;
//! - знаковый — сдвиг на `ширина − 1`: там остаётся только знак, то есть −1
//!   для отрицательного и 0 для неотрицательного.
//!
//! ⚠️ **Переменная величина сдвига не покрывается:** её значение известно
//! только в такте, а `checked_shr` в каждом выражении стоил бы дороже пользы.
//! Это названная граница, вынесенная кандидатом, а не забытый случай.
//!
//! Здесь же живёт печать **целой степени** (фича 0329): у неё та же природа —
//! операция языка, которую целевой язык выражает не тем оператором, каким её
//! записал автор.

use crate::diagnostics::Diagnostic;
use crate::generator::rust::rust_expr::{Scope, print_expression};
use crate::semantic::type_node::TypeNode;
use crate::semantic::{ExpressionNode, VariableNode};

/// Печать сдвига вправо, если величина заведомо не меньше ширины типа.
///
/// `Ok(None)` — обычный случай: печатает вызывающий.
pub(crate) fn saturating_right(
    value: &ExpressionNode,
    amount: &ExpressionNode,
    scope: &Scope,
) -> Result<Option<String>, Diagnostic> {
    let (Some(bits), Some(shift)) = (width_of(value), literal(amount)) else {
        return Ok(None);
    };
    if shift < i128::from(bits) {
        return Ok(None);
    }
    let printed = print_expression(value, scope)?;
    Ok(Some(if signed_of(value) {
        // Знак остаётся один: сдвиг на `bits − 1` даёт −1 либо 0 — ровно то,
        // что вычисляет эталон.
        format!("({printed} >> {})", bits - 1)
    } else {
        String::from("0")
    }))
}

/// Ширина типа выражения в битах, если она известна статически.
fn width_of(expr: &ExpressionNode) -> Option<u8> {
    match type_of(expr)? {
        TypeNode::Integer { bits, .. } => Some(bits),
        _ => None,
    }
}

/// Знаковый ли тип выражения.
fn signed_of(expr: &ExpressionNode) -> bool {
    matches!(type_of(expr), Some(TypeNode::Integer { signed: true, .. }))
}

/// Тип выражения — по объявлению переменной либо по явному приведению.
fn type_of(expr: &ExpressionNode) -> Option<TypeNode> {
    match expr {
        ExpressionNode::Variable(var) => match &*var.borrow() {
            VariableNode::Simple { ty, .. } | VariableNode::Const { ty, .. } => Some(ty.clone()),
            _ => None,
        },
        ExpressionNode::Cast(_, ty) => Some(ty.clone()),
        ExpressionNode::Parenthesis(inner) => type_of(inner),
        _ => None,
    }
}

/// Целое значение литерала величины сдвига.
fn literal(expr: &ExpressionNode) -> Option<i128> {
    match expr {
        ExpressionNode::Number(v) => Some(*v),
        ExpressionNode::Parenthesis(inner) => literal(inner),
        _ => None,
    }
}

/// Целая степень — `wrapping_pow` (фича 0329).
///
/// # Почему `wrapping_pow`
///
/// Он даёт **ровно** семантику эталона: обёртка `mod 2ⁿ` (правило ADR 0127).
/// Обычный `pow` паникует при переполнении в отладке, то есть на том же входе
/// прошивка и прогон разошлись бы — молча в релизе и падением в отладке.
///
/// Прежде цель отказывала (`RS-011`) с текстом про `f64::powf` — вещественную
/// степень, которой в этой позиции нет вовсе.
///
/// # Ошибки
///
/// `RS-011` — показатель отрицателен: у целой степени его быть не может, а
/// `wrapping_pow` принимает `u32`.
pub(crate) fn power(
    base: &ExpressionNode,
    exp: &ExpressionNode,
    scope: &Scope,
) -> Result<String, Diagnostic> {
    if let Some(value) = literal(exp)
        && value < 0
    {
        return Err(crate::generator::rust::rust_expr::unsupported(
            "возведение в ОТРИЦАТЕЛЬНУЮ степень: результат дробный, а целая \
             степень в Rust принимает беззнаковый показатель",
        ));
    }
    Ok(format!(
        "({}).wrapping_pow(({}) as u32)",
        print_expression(base, scope)?,
        print_expression(exp, scope)?
    ))
}
