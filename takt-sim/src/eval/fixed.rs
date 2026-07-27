//! Эталонная Q-арифметика fixed-point `q(m, n)` (фича 0061, задача 0061-02).
//!
//! Нормативные правила ADR 0061 — обязаны совпасть **побитово** с целями
//! (`c`/`rust`/`st`/`sv`), иначе сверка проваливается by design:
//!
//! - `+`/`−` — сложение представлений, **wraparound** к `W = m + n` бит (как у
//!   целых языка; насыщение — вне объёма, кандидат);
//! - `*` — точное произведение шириной `2W` (в `i128`), затем **арифметический**
//!   сдвиг вправо на `n` — округление **floor к −∞** (правило 4; самый дешёвый в
//!   аппаратуре и однозначный). ⚠️ Проверять на **отрицательных**: на
//!   положительных floor и trunc совпадают и дефект был бы невидим (T9).
//! - `/` — делимое расширяется до `2W`, сдвигается влево на `n`, затем
//!   **целочисленное** деление (усечение к нулю, как в C — правило 5);
//! - приведения только явные (правило 6): `int`/`float` ↔ `q` масштабируют на 2ⁿ.
//!
//! Значение самоописательно ([`Value::Fixed`] несёт `m`/`n`), поэтому тип в
//! операцию протаскивать не нужно — семантика живёт **здесь** (фича 0025), а не
//! в адаптерах.

use crate::eval::error::{EvalError, value_kind};
use crate::eval::ops::BinOp;
use crate::eval::value::Value;

/// Приводит `v` к знаковому `intW` в дополнительном коде (wraparound, правило 3).
pub(crate) fn wrap(v: i128, w: u8) -> i64 {
    let modulo = 1i128 << w; // 2^W (W ≤ 64 гарантирован построением типа)
    let masked = v.rem_euclid(modulo); // 0..2^W
    let signed = if masked >= (modulo >> 1) {
        masked - modulo
    } else {
        masked
    };
    signed as i64
}

/// Пересчитывает представление между дробными разрядностями: влево — точно,
/// вправо — **арифметический** сдвиг (floor к −∞).
fn rescale(repr: i128, from_n: u8, to_n: u8) -> i128 {
    if to_n >= from_n {
        repr << (to_n - from_n)
    } else {
        repr >> (from_n - to_n)
    }
}

/// Значение `Value::Fixed` из «сырого» (возможно, переполненного) представления.
fn make(repr: i128, m: u8, n: u8) -> Value {
    Value::Fixed {
        repr: wrap(repr, m + n),
        m,
        n,
    }
}

/// Ошибка «операция не определена для этих операндов» (не тихий неверный ответ).
fn mismatch(op: BinOp, lhs: &Value, rhs: &Value) -> EvalError {
    EvalError::TypeMismatch {
        op: op.symbol(),
        lhs: value_kind(lhs),
        rhs: Some(value_kind(rhs)),
    }
}

/// Q-арифметика бинарной операции, когда хотя бы один операнд — [`Value::Fixed`].
///
/// Оба операнда обязаны быть `q` **одного формата** (гарантирует страж `SE-059`);
/// иначе — [`EvalError::TypeMismatch`], а не молчаливо неверный результат.
pub(crate) fn binary(op: BinOp, lhs: &Value, rhs: &Value) -> Result<Value, EvalError> {
    let (
        Value::Fixed { repr: ra, m, n },
        Value::Fixed {
            repr: rb,
            m: m2,
            n: n2,
        },
    ) = (lhs, rhs)
    else {
        return Err(mismatch(op, lhs, rhs));
    };
    if m != m2 || n != n2 {
        return Err(mismatch(op, lhs, rhs));
    }
    let (m, n) = (*m, *n);
    let (a, b) = (*ra as i128, *rb as i128);
    match op {
        BinOp::Add => Ok(make(a + b, m, n)),
        BinOp::Subtract => Ok(make(a - b, m, n)),
        // Точное произведение 2W → сдвиг на n вправо, floor к −∞ (арифм. сдвиг).
        BinOp::Multiply => Ok(make((a * b) >> n, m, n)),
        // Делимое ← n влево, затем целочисленное деление (усечение к нулю).
        BinOp::Divide => {
            if b == 0 {
                return Err(EvalError::DivisionByZero);
            }
            Ok(make((a << n) / b, m, n))
        }
        BinOp::Less => Ok(Value::Boolean(a < b)),
        BinOp::More => Ok(Value::Boolean(a > b)),
        BinOp::LessEqual => Ok(Value::Boolean(a <= b)),
        BinOp::MoreEqual => Ok(Value::Boolean(a >= b)),
        BinOp::Equal => Ok(Value::Boolean(a == b)),
        BinOp::NotEqual => Ok(Value::Boolean(a != b)),
        // Не определены для q: остаток, степень, сдвиги, битовые, логические.
        BinOp::Modulo
        | BinOp::Power
        | BinOp::ShiftLeft
        | BinOp::ShiftRight
        | BinOp::BitwiseAnd
        | BinOp::BitwiseOr
        | BinOp::BitwiseXor
        | BinOp::LogicalAnd
        | BinOp::LogicalOr => Err(mismatch(op, lhs, rhs)),
    }
}

/// Унарный минус: `−(repr)` с wraparound (значение остаётся `q(m, n)`).
pub(crate) fn negate(repr: i64, m: u8, n: u8) -> Value {
    make(-(repr as i128), m, n)
}

/// `expr as q(m, n)` (узел `Cast`): **масштабирует** значение к представлению
/// (правило 6). `int`/`bool` умножаются на 2ⁿ (сдвиг влево), `float` — floor от
/// `f · 2ⁿ`, `q(m₂, n₂)` — пересчитывается на разницу дробных разрядов.
pub(crate) fn cast_to_fixed(value: &Value, m: u8, n: u8) -> Result<Value, EvalError> {
    let repr: i128 = match value {
        Value::Number(i) => (*i as i128) << n,
        Value::Boolean(b) => (i64::from(*b) as i128) << n,
        Value::Real(f) => (f * pow2(n)).floor() as i128,
        Value::Fixed { repr, n: n2, .. } => rescale(*repr as i128, *n2, n),
        Value::Array(_) | Value::Struct { .. } | Value::Duration(_) => {
            return Err(EvalError::NotCoercible {
                value: value_kind(value),
                ty: format!("q({m}, {n})"),
            });
        }
    };
    Ok(make(repr, m, n))
}

/// Целая часть `q(m, n)` (floor): `repr >> n` — для `q as int`/`q as bit`.
pub(crate) fn to_integer_part(repr: i64, n: u8) -> i64 {
    repr >> n
}

/// Вещественное значение `q(m, n)`: `repr / 2ⁿ` — для `q as float`.
pub(crate) fn to_real(repr: i64, n: u8) -> f64 {
    repr as f64 / pow2(n)
}

/// `2ⁿ` как `f64` (n ≤ 63 по построению типа).
fn pow2(n: u8) -> f64 {
    (1u64 << n) as f64
}

#[cfg(test)]
mod tests {
    use super::*;

    fn q(repr: i64, m: u8, n: u8) -> Value {
        Value::Fixed { repr, m, n }
    }

    /// T4: 1.5 в q(8, 8) — представление 384; сложение представлений.
    #[test]
    fn add_representations() {
        // 1.5 + 0.5 = 2.0 → 384 + 128 = 512
        assert_eq!(
            binary(BinOp::Add, &q(384, 8, 8), &q(128, 8, 8)),
            Ok(q(512, 8, 8))
        );
    }

    /// T9 (ключевой): −1.5 · 2.0 в q(8, 8) → −3.0 (−768). Округление floor к −∞
    /// проверяется именно на **отрицательном** — на положительных floor и trunc
    /// совпадают, и дефект был бы невидим.
    #[test]
    fn multiply_negative_is_floor() {
        // −1.5 → −384, 2.0 → 512; (−384·512) >> 8 = −196608 >> 8 = −768 = −3.0
        assert_eq!(
            binary(BinOp::Multiply, &q(-384, 8, 8), &q(512, 8, 8)),
            Ok(q(-768, 8, 8))
        );
    }

    /// Произведение, дающее дробный хвост, округляется к −∞ (не к нулю).
    #[test]
    fn multiply_floor_rounds_down_for_negative() {
        // 0.5 · (−0.00390625) = −0.001953…; репрезентации 128 · (−1) = −128;
        // −128 >> 8 = −1 (floor), тогда как усечение к нулю дало бы 0.
        assert_eq!(
            binary(BinOp::Multiply, &q(128, 8, 8), &q(-1, 8, 8)),
            Ok(q(-1, 8, 8))
        );
    }

    /// T19: переполнение `+` — wraparound (перенос), а не насыщение.
    #[test]
    fn add_overflow_wraps() {
        // q(8, 8): max repr = 32767 (127.996…). 32767 + 1 = 32768 → −32768.
        assert_eq!(
            binary(BinOp::Add, &q(32767, 8, 8), &q(1, 8, 8)),
            Ok(q(-32768, 8, 8))
        );
    }

    /// Деление: 3.0 / 2.0 в q(8, 8) → 1.5 (384). (768 << 8) / 512 = 384.
    #[test]
    fn divide_scales() {
        assert_eq!(
            binary(BinOp::Divide, &q(768, 8, 8), &q(512, 8, 8)),
            Ok(q(384, 8, 8))
        );
    }

    /// Деление на ноль — ошибка, а не паника.
    #[test]
    fn divide_by_zero_is_error() {
        assert_eq!(
            binary(BinOp::Divide, &q(384, 8, 8), &q(0, 8, 8)),
            Err(EvalError::DivisionByZero)
        );
    }

    /// Сравнение q идёт по представлениям (тот же формат).
    #[test]
    fn compare_representations() {
        assert_eq!(
            binary(BinOp::Less, &q(384, 8, 8), &q(512, 8, 8)),
            Ok(Value::Boolean(true))
        );
    }

    /// Смешение форматов на уровне значений — ошибка (страж на случай, если
    /// SE-059 обошли).
    #[test]
    fn mixed_formats_is_error() {
        assert!(binary(BinOp::Add, &q(384, 8, 8), &q(384, 4, 4)).is_err());
    }

    /// Приведение int → q(8, 8) масштабирует: 3 → 3.0 = 768.
    #[test]
    fn cast_int_scales() {
        assert_eq!(cast_to_fixed(&Value::Number(3), 8, 8), Ok(q(768, 8, 8)));
    }

    /// Приведение q → целая часть (floor): 1.5 (384) → 1.
    #[test]
    fn cast_fixed_to_integer_part_floors() {
        assert_eq!(to_integer_part(384, 8), 1);
        assert_eq!(to_integer_part(-384, 8), -2); // −1.5 → floor = −2
    }

    /// Приведение q → float: 1.5 (384) → 1.5.
    #[test]
    fn cast_fixed_to_real() {
        assert!((to_real(384, 8) - 1.5).abs() < 1e-9);
    }

    /// wrap на границе W=64 не паникует и даёт корректный i64.
    #[test]
    fn wrap_width_64() {
        assert_eq!(wrap(i64::MAX as i128, 64), i64::MAX);
        assert_eq!(wrap(i64::MIN as i128, 64), i64::MIN);
    }
}
