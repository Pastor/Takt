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
//! Фича 0334 достроила правило до **обоих** направлений и **переменной**
//! величины. Довод «`checked_shr` в каждом выражении стоил бы дороже пользы»
//! опровергнут замером: цена умолчания — не лишняя инструкция, а **другой
//! автомат**. При `n = 8` и `a: u8` эталон даёт `0`, а порождённый Rust
//! **паникует** в отладке и даёт `200` в релизе (величина маскируется до
//! `n & 7 = 0`) — то есть прошивку собирают именно в том режиме, где значение
//! молча неверно. Сдвиг **влево** на литеральную величину при этом вовсе не
//! собирался: `rustc` отвечает «attempt to shift left by `8_i32`, which would
//! overflow» при **нулевом** коде возврата `taktc` (класс 0262).
//!
//! ⚠️ **Отрицательная величина сдвига под правило не подпадает**: эталон
//! отвечает `SIM-002` и останавливает прогон, то есть у записи нет верного
//! значения вовсе. Прошивка считает её молча — это описанное разделение
//! обязанностей (фича 0333), а не расхождение.
//!
//! Здесь же живёт печать **целой степени** (фича 0329): у неё та же природа —
//! операция языка, которую целевой язык выражает не тем оператором, каким её
//! записал автор.

use crate::diagnostics::Diagnostic;
use crate::generator::rust::rust_expr::{Scope, print_expression};
use crate::semantic::type_node::TypeNode;
use crate::semantic::{ExpressionNode, VariableNode};

/// Направление сдвига: у них общая природа, но разные насыщения.
#[derive(Clone, Copy, PartialEq, Eq)]
pub(crate) enum Direction {
    Left,
    Right,
}

/// Печать сдвига, если результат может не помещаться в `<<`/`>>` языка Rust.
///
/// `Ok(None)` — обычный случай: печатает вызывающий обычным оператором.
///
/// # Что печатается
///
/// | Форма | Направление | Печать |
/// |---|---|---|
/// | литерал `≥ ширины` | влево | `0` |
/// | литерал `≥ ширины` | вправо, беззнаковый | `0` |
/// | литерал `≥ ширины` | вправо, знаковый | `v >> (ширина − 1)` |
/// | переменная | влево | `v.checked_shl(n).unwrap_or(0)` |
/// | переменная | вправо, беззнаковый | `v.checked_shr(n).unwrap_or(0)` |
/// | переменная | вправо, знаковый | `v >> n.min(ширина − 1)` |
///
/// ⚠️ Знаковый сдвиг вправо выражен **`min`**, а не `checked_shr`: у него
/// насыщение зависит от самого значения (`v >> (ширина − 1)` есть знак), и
/// `unwrap_or` напечатал бы `v` **дважды** — а вычисление операнда в языке
/// Takt бывает с эффектом (вызов функции пишет в переменные модели).
pub(crate) fn guarded(
    direction: Direction,
    value: &ExpressionNode,
    amount: &ExpressionNode,
    scope: &Scope,
) -> Result<Option<String>, Diagnostic> {
    let Some(bits) = width_of(value) else {
        return Ok(None);
    };
    let signed = signed_of(value);
    if let Some(shift) = literal(amount) {
        if shift < i128::from(bits) {
            return Ok(None);
        }
        let printed = print_expression(value, scope)?;
        return Ok(Some(match (direction, signed) {
            // Знак остаётся один: сдвиг на `bits − 1` даёт −1 либо 0 — ровно
            // то, что вычисляет эталон.
            (Direction::Right, true) => format!("({printed} >> {})", bits - 1),
            _ => String::from("0"),
        }));
    }

    let printed = print_expression(value, scope)?;
    let shift = shift_amount(amount, scope)?;
    Ok(Some(match (direction, signed) {
        (Direction::Right, true) => {
            format!("({printed} >> ({shift}).min({}))", u32::from(bits) - 1)
        }
        (Direction::Left, _) => format!("({printed}).checked_shl({shift}).unwrap_or(0)"),
        (Direction::Right, false) => format!("({printed}).checked_shr({shift}).unwrap_or(0)"),
    }))
}

/// Величина сдвига как `u32` — тип, которого требуют `checked_sh*`.
///
/// ⚠️ Приведение печатается **по нужде**: у величины, уже имеющей тип `u32`,
/// `x as u32` — это `clippy::unnecessary_cast`, то есть **отказ** сборки под
/// `-D warnings` (тот же класс, что фича 0263).
fn shift_amount(amount: &ExpressionNode, scope: &Scope) -> Result<String, Diagnostic> {
    let printed = print_expression(amount, scope)?;
    if matches!(
        type_of(amount),
        Some(TypeNode::Integer {
            bits: 32,
            signed: false
        })
    ) {
        return Ok(printed);
    }
    Ok(format!("({printed}) as u32"))
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
