//! Доступ к члену значения: поле структуры (`p.x`) или бит целого (`BTN.0`).
//!
//! Узел АСД `a.b` (`ExpressionNode::BitAccess`/`ConditionNode::BitAccess`)
//! покрывает **оба** смысла: `Member::Identifier` — поле структуры (фича 0034),
//! `Member::Number` — бит целого/логического (фича бит-портов). Различение — по
//! **вычисленному значению** левой части, а не по имени узла.
//!
//! Логика — здесь, в ядре под `deny(wildcard_enum_match_arm)`, чтобы адаптеры
//! [`crate::expression`] (тела блоков) и [`crate::predicate`] (условия рёбер) её
//! **не дублировали**: именно расхождение двух вычислителей было корневой
//! причиной фичи 0025.

use takt_lang::parser::ast::Member;

use crate::eval::error::{EvalError, value_kind};
use crate::eval::value::Value;

/// Читает член `value.member`: поле структуры или бит целого.
///
/// Диспетчеризация по паре (вид значения, вид селектора):
///
/// | Значение | Селектор | Результат |
/// |---|---|---|
/// | `Struct` | `Identifier(f)` | поле `f` (или `UnknownField`) |
/// | `Struct` | `Number(_)` | `BitIndexOfStruct` — бита у структуры нет |
/// | `Number`/`Boolean` | `Number(b)` | бит `b` (как прежде) |
/// | `Number`/`Boolean` | `Identifier(_)` | `FieldOfNonStruct` |
/// | `Real`/`Array`/`Fixed` | `Number(_)` | `BitOfNonInteger` |
/// | `Real`/`Array`/`Fixed` | `Identifier(_)` | `FieldOfNonStruct` |
pub(crate) fn read_member(value: &Value, member: &Member) -> Result<Value, EvalError> {
    match (value, member) {
        (Value::Struct { name, fields }, Member::Identifier(field)) => fields
            .iter()
            .find(|(f, _)| *f == field.name)
            .map(|(_, v)| v.clone())
            .ok_or_else(|| EvalError::UnknownField {
                name: name.clone(),
                field: field.name.clone(),
            }),
        (Value::Struct { name, .. }, Member::Number(_)) => {
            Err(EvalError::BitIndexOfStruct { name: name.clone() })
        }
        (Value::Number(n), Member::Number(bit)) => read_bit(*n, *bit),
        (Value::Boolean(b), Member::Number(bit)) => read_bit(i64::from(*b), *bit),
        // Бит-вектор `[bit;N]` при N > 64 (фича 0078) — упакован в массив
        // 64-битных слов: бит `k` живёт в слове `k / 64`, смещение `k % 64`.
        (Value::Array(words), Member::Number(bit)) => read_bit_words(words, *bit),
        (
            Value::Number(_)
            | Value::Boolean(_)
            | Value::Real(_)
            | Value::Array(_)
            | Value::Fixed { .. },
            Member::Identifier(_),
        ) => Err(EvalError::FieldOfNonStruct {
            value: value_kind(value),
        }),
        (Value::Real(_) | Value::Fixed { .. }, Member::Number(_)) => {
            Err(EvalError::BitOfNonInteger {
                value: value_kind(value),
            })
        }
    }
}

/// Извлекает бит `bit` целочисленного значения `bits` как логическое.
fn read_bit(bits: i64, bit: i64) -> Result<Value, EvalError> {
    if !(0..64).contains(&bit) {
        return Err(EvalError::BitIndexOutOfRange { bit });
    }
    Ok(Value::Boolean(bits & (1 << bit) != 0))
}

/// Извлекает бит `bit` из бит-вектора `[bit;N]` (N > 64), упакованного в массив
/// 64-битных слов (фича 0078): слово `bit / 64`, смещение `bit % 64`. Бит вне
/// набранных слов или слово-не-целое → `BitIndexOutOfRange`.
fn read_bit_words(words: &[Value], bit: i64) -> Result<Value, EvalError> {
    let Ok(k) = u32::try_from(bit) else {
        return Err(EvalError::BitIndexOutOfRange { bit });
    };
    let (w, off) = takt_lang::semantic::bit_vector::bit_slot(k);
    match words.get(usize::from(w)) {
        Some(Value::Number(word)) => Ok(Value::Boolean(word & (1 << off) != 0)),
        _ => Err(EvalError::BitIndexOutOfRange { bit }),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn point() -> Value {
        Value::Struct {
            name: "Point".to_string(),
            fields: vec![
                ("x".to_string(), Value::Number(1)),
                ("y".to_string(), Value::Number(2)),
            ],
        }
    }

    #[test]
    fn field_read_by_name() {
        assert_eq!(
            read_member(
                &point(),
                &Member::Identifier(takt_lang::parser::ast::Identifier::new("y"))
            ),
            Ok(Value::Number(2))
        );
    }

    #[test]
    fn unknown_field_is_error() {
        let err = read_member(
            &point(),
            &Member::Identifier(takt_lang::parser::ast::Identifier::new("z")),
        )
        .unwrap_err();
        assert_eq!(err.code(), "SIM-027");
    }

    #[test]
    fn bit_index_on_struct_is_error() {
        let err = read_member(&point(), &Member::Number(0)).unwrap_err();
        assert_eq!(err.code(), "SIM-029");
    }

    #[test]
    fn bit_read_on_integer_unchanged() {
        // Регресс: BTN.0 — бит целого (фича бит-портов).
        assert_eq!(
            read_member(&Value::Number(0b101), &Member::Number(2)),
            Ok(Value::Boolean(true))
        );
        assert_eq!(
            read_member(&Value::Number(0b101), &Member::Number(1)),
            Ok(Value::Boolean(false))
        );
    }

    #[test]
    fn field_on_non_struct_is_error() {
        let err = read_member(
            &Value::Number(5),
            &Member::Identifier(takt_lang::parser::ast::Identifier::new("x")),
        )
        .unwrap_err();
        assert_eq!(err.code(), "SIM-012");
    }
}
