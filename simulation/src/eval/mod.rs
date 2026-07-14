//! Ядро вычисления симулятора — **единственное** место семантики значений
//! (ADR 0025, Option B).
//!
//! # Зачем модуль существует
//!
//! Раньше семантика была продублирована в двух вычислителях
//! (`unit/builder.rs::eval_expression_rt` и `predicate.rs::flat`), которые
//! разошлись: один не умел арифметику вовсе, другой умел, но паниковал. Чинить их
//! порознь означало развести снова. Здесь семантика записана один раз; адаптеры
//! узлов (`ExpressionNode`/`ConditionNode`) — тонкие и семантики не содержат.
//!
//! # Как модуль удерживает инвариант
//!
//! [`deny(clippy::wildcard_enum_match_arm)`] запрещает подстановочные ветки по
//! вариантам перечислений: новый вариант `Value`/`TypeNode`/`BinOp` **ломает
//! сборку**, а не превращается молча в «невычислимо». Корневой причиной фичи 0025
//! была именно ветка `_ => None`. Линт настроен **точечно** на модуле — как
//! предписывает `docs/CODE.md` (запрещая при этом `#![deny(warnings)]` на крейт).
//!
//! # Разделение ответственности
//!
//! - [`ops`] — тип-**независимая** семантика: операции над значениями (S3, S4a, S5, S8).
//! - [`coerce_to_type`] — тип-**зависимая**: приведение и усечение по объявленному
//!   типу (S1, S2, S6, S7, S9). Вызывается на месте присваивания, где известен тип
//!   цели.
//! - [`error`] — структурированная ошибка без позиции; позицию добавляет адаптер.
//!
//! Арифметика ведётся в `i64`/`f64`, сужение — при записи. Это повторяет модель C
//! (продвижение операндов до `int`, сужение при присваивании), что и требует
//! критерий A8 фичи 0025.

#![deny(clippy::wildcard_enum_match_arm)]

pub(crate) mod error;
pub(crate) mod ops;
pub(crate) mod value;

use grammar::semantic::type_node::TypeNode;

use crate::eval::error::{EvalError, value_kind};
use crate::eval::value::Value;

/// Приводит значение к объявленному типу переменной (S1, S2, S6, S7, S9).
///
/// Вызывается **на месте присваивания**: там известен тип цели
/// (`VariableNode::ty()`), и там же результат может быть отвергнут. Внутрь
/// `Context::set_value` приведение не убрано намеренно — метод объявлен без
/// `Result`, а S2 обязан уметь отказать; см. `docs/development/0025-01-eval-core.md`.
///
/// # Соответствие C
///
/// - Беззнаковые (S1): обёртка mod 2^bits — как в C (`uint8_t x = 255; x + 1` → `0`).
/// - Знаковые (S2): выход за диапазон — UB в C, поэтому **ошибка**, а не обёртка.
// Единственная подстановочная ветка в модуле — и она вынужденная, а не забытая.
//
// `TypeNode` в крейте `grammar` помечен `#[non_exhaustive]` (как предписывает
// `docs/CODE.md` для публичных перечислений), поэтому Rust **требует** от
// внешнего крейта ветку `_`, даже когда перечислены все известные варианты.
// Механизм ADR 0025 («новый вариант ломает сборку») здесь не работает — но он и
// не был нужен для типов: корневая причина фичи жила в разборе
// `ExpressionNode`/`ConditionNode`, а те `#[non_exhaustive]` **не** помечены,
// значит для адаптеров (задачи 0025-02/03) гарантия сохраняется полностью.
//
// Ветка `_` здесь безопасна: она возвращает **ошибку**, а не `None`. Неизвестный
// тип приведёт к диагностике, а не к тихому пропуску — то есть к тому же
// наблюдаемому поведению, что и явно перечисленный неподдерживаемый тип.
#[allow(clippy::wildcard_enum_match_arm)]
pub(crate) fn coerce_to_type(value: Value, ty: &TypeNode) -> Result<Value, EvalError> {
    match ty {
        TypeNode::Integer { bits, signed } => coerce_integer(value, *bits, *signed),
        // S7: вариант enum — целое; разрядность подбирает генератор C по максимуму.
        TypeNode::Enum(_) => coerce_integer(value, 64, true),
        // S6: `bit` — один бит. Расходится с генератором C (`Bit` → `int`), но
        // сверка по `Bit` анализом исключена: эталон C для него дефектен.
        TypeNode::Bit => Ok(Value::Number(to_integer(&value, ty)? & 1)),
        TypeNode::Bool => match &value {
            Value::Boolean(b) => Ok(Value::Boolean(*b)),
            Value::Number(n) => Ok(Value::Boolean(*n != 0)),
            Value::Real(_) | Value::Array(_) => Err(EvalError::NotCoercible {
                value: value_kind(&value),
                ty: "bool".to_string(),
            }),
        },
        TypeNode::Rational => match &value {
            Value::Real(f) => Ok(Value::Real(*f)),
            Value::Number(n) => Ok(Value::Real(*n as f64)),
            Value::Boolean(_) | Value::Array(_) => Err(EvalError::NotCoercible {
                value: value_kind(&value),
                ty: "float".to_string(),
            }),
        },
        TypeNode::Array(size, elem) => coerce_array(value, *size, elem),
        // Адресный тип порта: значение порта — целое машинное слово.
        TypeNode::Address(_, _) => match &value {
            Value::Number(n) => Ok(Value::Number(*n)),
            Value::Boolean(b) => Ok(Value::Number(i64::from(*b))),
            Value::Real(_) | Value::Array(_) => Err(EvalError::NotCoercible {
                value: value_kind(&value),
                ty: "адресный порт".to_string(),
            }),
        },
        // Пробел `Value`: структуры симулятором не представимы. Явная диагностика
        // вместо тихого `None` — контрпример T22 тест-плана.
        TypeNode::Struct(name) => Err(EvalError::UnsupportedType {
            ty: format!("структура '{name}'"),
        }),
        TypeNode::Inference => Err(EvalError::UnsupportedType {
            ty: "невыведенный тип".to_string(),
        }),
        TypeNode::Unit => Err(EvalError::UnsupportedType {
            ty: "пустой тип".to_string(),
        }),
        TypeNode::Unsupported => Err(EvalError::UnsupportedType {
            ty: "неподдерживаемый тип".to_string(),
        }),
        TypeNode::BuiltinString => Err(EvalError::UnsupportedType {
            ty: "строка".to_string(),
        }),
        TypeNode::BuiltinModel => Err(EvalError::UnsupportedType {
            ty: "модель".to_string(),
        }),
        TypeNode::BuiltinState => Err(EvalError::UnsupportedType {
            ty: "состояние".to_string(),
        }),
        TypeNode::BuiltinNumeric => match &value {
            Value::Number(_) | Value::Real(_) => Ok(value),
            Value::Boolean(_) | Value::Array(_) => Err(EvalError::NotCoercible {
                value: value_kind(&value),
                ty: "числовой тип".to_string(),
            }),
        },
        // Вынужденная ветка: `TypeNode` — `#[non_exhaustive]` (см. комментарий
        // над функцией). Отказ с диагностикой, а не тихий пропуск.
        _ => Err(EvalError::UnsupportedType {
            ty: format!("{ty:?}"),
        }),
    }
}

/// Целочисленное значение из [`Value`] (вещественное усекается к нулю, как в C).
fn to_integer(value: &Value, ty: &TypeNode) -> Result<i64, EvalError> {
    match value {
        Value::Number(n) => Ok(*n),
        Value::Boolean(b) => Ok(i64::from(*b)),
        // C усекает float→int в сторону нуля.
        Value::Real(f) => Ok(*f as i64),
        Value::Array(_) => Err(EvalError::NotCoercible {
            value: value_kind(value),
            ty: format!("{ty:?}"),
        }),
    }
}

/// S1/S2/S9: усечение (беззнаковые) либо проверка диапазона (знаковые).
fn coerce_integer(value: Value, bits: u8, signed: bool) -> Result<Value, EvalError> {
    let ty = TypeNode::Integer { bits, signed };
    let n = to_integer(&value, &ty)?;
    if bits >= 64 {
        // Известное ограничение: значения хранятся в i64, поэтому u64 со старшим
        // битом не представим. На приёмку (примеры на u8) не влияет.
        return Ok(Value::Number(n));
    }
    if signed {
        // S2: выход за диапазон знакового типа — UB в C, не воспроизводим.
        let min = -(1_i64 << (bits - 1));
        let max = (1_i64 << (bits - 1)) - 1;
        if n < min || n > max {
            return Err(EvalError::SignedOverflow { value: n, bits });
        }
        Ok(Value::Number(n))
    } else {
        // S1: обёртка mod 2^bits — определённое поведение C.
        let mask = (1_i64 << bits) - 1;
        Ok(Value::Number(n & mask))
    }
}

/// Поэлементное приведение массива с проверкой длины.
fn coerce_array(value: Value, size: u16, elem: &TypeNode) -> Result<Value, EvalError> {
    let Value::Array(items) = value else {
        return Err(EvalError::NotCoercible {
            value: value_kind(&value),
            ty: format!("массив [{size}]"),
        });
    };
    if items.len() != usize::from(size) {
        return Err(EvalError::NotCoercible {
            value: "массив другой длины",
            ty: format!("массив [{size}]"),
        });
    }
    let coerced = items
        .into_iter()
        .map(|item| coerce_to_type(item, elem))
        .collect::<Result<Vec<Value>, EvalError>>()?;
    Ok(Value::Array(coerced))
}

#[cfg(test)]
mod tests {
    use super::*;

    fn u(bits: u8) -> TypeNode {
        TypeNode::Integer {
            bits,
            signed: false,
        }
    }

    fn i(bits: u8) -> TypeNode {
        TypeNode::Integer { bits, signed: true }
    }

    // ── S1: обёртка беззнакового (сверено с cc -std=c11) ──────────────────────

    #[test]
    fn s1_u8_wraps_on_overflow() {
        // C: uint8_t y = 255; y = y + 1; → 0 (проверено пробой на cc).
        assert_eq!(
            coerce_to_type(Value::Number(256), &u(8)),
            Ok(Value::Number(0))
        );
    }

    #[test]
    fn s9_u8_truncates_large_value() {
        // 300 mod 256 = 44 (T17 тест-плана).
        assert_eq!(
            coerce_to_type(Value::Number(300), &u(8)),
            Ok(Value::Number(44))
        );
    }

    #[test]
    fn s1_u8_negative_wraps_like_c_cast() {
        // C: (uint8_t)-1 == 255.
        assert_eq!(
            coerce_to_type(Value::Number(-1), &u(8)),
            Ok(Value::Number(255))
        );
    }

    #[test]
    fn s4_shift_result_truncates_to_u8() {
        // S4: u8 `x << 8` → в C определено: 1<<8 = 256, запись в uint8_t → 0.
        assert_eq!(
            coerce_to_type(Value::Number(256), &u(8)),
            Ok(Value::Number(0))
        );
    }

    #[test]
    fn s1_u16_wraps() {
        assert_eq!(
            coerce_to_type(Value::Number(65_536), &u(16)),
            Ok(Value::Number(0))
        );
    }

    #[test]
    fn u8_in_range_is_unchanged() {
        assert_eq!(
            coerce_to_type(Value::Number(42), &u(8)),
            Ok(Value::Number(42))
        );
    }

    // ── S2: знаковое переполнение → ошибка ────────────────────────────────────

    #[test]
    fn s2_i8_overflow_is_error_not_wrap() {
        // В C это UB — не воспроизводим (принцип ADR).
        assert_eq!(
            coerce_to_type(Value::Number(128), &i(8)),
            Err(EvalError::SignedOverflow {
                value: 128,
                bits: 8
            })
        );
    }

    #[test]
    fn s2_i8_underflow_is_error() {
        assert!(matches!(
            coerce_to_type(Value::Number(-129), &i(8)),
            Err(EvalError::SignedOverflow { .. })
        ));
    }

    #[test]
    fn i8_boundaries_are_accepted() {
        assert_eq!(
            coerce_to_type(Value::Number(127), &i(8)),
            Ok(Value::Number(127))
        );
        assert_eq!(
            coerce_to_type(Value::Number(-128), &i(8)),
            Ok(Value::Number(-128))
        );
    }

    // ── S6: bit ───────────────────────────────────────────────────────────────

    #[test]
    fn s6_bit_truncates_to_single_bit() {
        // T14: `var f: bit := 1; f := f + 1;` → 2 & 1 → 0.
        assert_eq!(
            coerce_to_type(Value::Number(2), &TypeNode::Bit),
            Ok(Value::Number(0))
        );
        assert_eq!(
            coerce_to_type(Value::Number(3), &TypeNode::Bit),
            Ok(Value::Number(1))
        );
    }

    // ── S7: enum ──────────────────────────────────────────────────────────────

    #[test]
    fn s7_enum_variant_is_integer() {
        assert_eq!(
            coerce_to_type(Value::Number(1), &TypeNode::Enum("Mode".to_string())),
            Ok(Value::Number(1))
        );
    }

    // ── bool / float ──────────────────────────────────────────────────────────

    #[test]
    fn bool_from_number_follows_c() {
        assert_eq!(
            coerce_to_type(Value::Number(2), &TypeNode::Bool),
            Ok(Value::Boolean(true))
        );
        assert_eq!(
            coerce_to_type(Value::Number(0), &TypeNode::Bool),
            Ok(Value::Boolean(false))
        );
    }

    #[test]
    fn rational_accepts_int_and_real() {
        assert_eq!(
            coerce_to_type(Value::Number(3), &TypeNode::Rational),
            Ok(Value::Real(3.0))
        );
        assert_eq!(
            coerce_to_type(Value::Real(0.5), &TypeNode::Rational),
            Ok(Value::Real(0.5))
        );
    }

    #[test]
    fn int_from_real_truncates_toward_zero_like_c() {
        assert_eq!(
            coerce_to_type(Value::Real(2.9), &u(8)),
            Ok(Value::Number(2))
        );
    }

    // ── Массивы ───────────────────────────────────────────────────────────────

    #[test]
    fn array_coerces_elementwise() {
        let value = Value::Array(vec![Value::Number(256), Value::Number(1)]);
        let ty = TypeNode::Array(2, Box::new(u(8)));
        assert_eq!(
            coerce_to_type(value, &ty),
            Ok(Value::Array(vec![Value::Number(0), Value::Number(1)]))
        );
    }

    #[test]
    fn array_length_mismatch_is_error() {
        let value = Value::Array(vec![Value::Number(1)]);
        let ty = TypeNode::Array(2, Box::new(u(8)));
        assert!(matches!(
            coerce_to_type(value, &ty),
            Err(EvalError::NotCoercible { .. })
        ));
    }

    // ── Контрпримеры: явная диагностика вместо тихого пропуска ────────────────

    #[test]
    fn t22_struct_type_is_explicit_diagnostic() {
        // Контрпример T22: структуры не поддерживаются — но об этом сообщается.
        let err = coerce_to_type(Value::Number(1), &TypeNode::Struct("P".to_string()));
        assert!(matches!(err, Err(EvalError::UnsupportedType { .. })));
        assert!(err.unwrap_err().message().contains("структура"));
    }

    #[test]
    fn array_to_scalar_is_error() {
        assert!(matches!(
            coerce_to_type(Value::Array(vec![]), &u(8)),
            Err(EvalError::NotCoercible { .. })
        ));
    }

    #[test]
    fn unsupported_types_report_reason() {
        for ty in [
            TypeNode::Inference,
            TypeNode::Unit,
            TypeNode::Unsupported,
            TypeNode::BuiltinString,
            TypeNode::BuiltinModel,
            TypeNode::BuiltinState,
        ] {
            assert!(
                matches!(
                    coerce_to_type(Value::Number(1), &ty),
                    Err(EvalError::UnsupportedType { .. })
                ),
                "ожидалась диагностика для {ty:?}"
            );
        }
    }
}
