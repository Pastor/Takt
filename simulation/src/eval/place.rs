//! Обновление значения «по месту» (lvalue): запись в поле структуры (фича 0034)
//! и в элемент массива (фича 0076).
//!
//! Адаптер ([`crate::unit::statement`]) раскладывает левую часть `p.x := …` /
//! `data[i] := …` в корень + путь **сегментов** ([`PlaceSegment`]) и зовёт
//! [`update`]; семантика замены живёт **здесь**, в ядре под
//! `deny(wildcard_enum_match_arm)`. Точечность обязательна: `p.x := 7` **не**
//! пересоздаёт структуру и не трогает `p.y`; `data[0] := 7` не трогает `data[1]`.

use grammar::semantic::type_node::TypeNode;

use super::error::{EvalError, value_kind};
use super::value::Value;
use super::{StructRegistry, coerce_to_type_with};

/// Сегмент пути к месту записи: поле структуры или индекс массива (фича 0076).
///
/// Индекс уже **вычислен** адаптером (у него есть контекст) в `usize` — ядро
/// АСД не знает. Поле — имя, как в структурах 0034.
#[derive(Debug, Clone, PartialEq)]
pub(crate) enum PlaceSegment {
    /// `.имя` — поле структуры.
    Field(String),
    /// `[i]` — элемент массива по вычисленному индексу.
    Index(usize),
}

/// Заменяет значение по пути сегментов `path` в `value`, возвращая обновлённое
/// целое. `ty` — **объявленный** тип `value` (тип корневой переменной на верхнем
/// вызове), протаскивается вниз, чтобы лист приводился к типу поля/элемента.
///
/// Лист приводится к объявленному типу (усечение S9): `p.y := 300` при `y: u8`
/// даёт `44`; `data[0] := 300` при `[u8;4]` — `44` (T22 действует и в элементе).
/// Несоответствие селектора виду значения (поле у не-структуры, индекс у
/// не-массива), неизвестное поле, выход за границы → [`EvalError`]. Если тип
/// недоступен (`None`) — лист без приведения (консервативно, как без реестра).
pub(crate) fn update(
    value: Value,
    ty: Option<&TypeNode>,
    path: &[PlaceSegment],
    new: Value,
    structs: &dyn StructRegistry,
) -> Result<Value, EvalError> {
    let Some((segment, rest)) = path.split_first() else {
        // Пустой путь: замена целиком. Вызывается только с непустым путём —
        // присваивание всей переменной идёт мимо `update` (ветка `Variable`).
        return Ok(new);
    };
    match segment {
        PlaceSegment::Field(field) => update_field(value, field, rest, new, structs),
        PlaceSegment::Index(index) => update_index(value, ty, *index, rest, new, structs),
    }
}

/// Запись в поле структуры (фича 0034). Тип поля берётся из реестра `structs`
/// (в `TypeNode::Struct` полей нет — только имя), поэтому `ty` здесь не нужен.
fn update_field(
    value: Value,
    field: &str,
    rest: &[PlaceSegment],
    new: Value,
    structs: &dyn StructRegistry,
) -> Result<Value, EvalError> {
    let Value::Struct { name, mut fields } = value else {
        return Err(EvalError::FieldOfNonStruct {
            value: value_kind(&value),
        });
    };
    let Some(idx) = fields.iter().position(|(f, _)| f == field) else {
        return Err(EvalError::UnknownField {
            name,
            field: field.to_string(),
        });
    };
    let field_ty = structs.find_struct(&name).and_then(|def| {
        def.fields
            .iter()
            .find(|(fname, _)| fname == field)
            .map(|(_, t)| t.clone())
    });
    let new_field = if rest.is_empty() {
        match field_ty {
            Some(ty) => coerce_to_type_with(new, &ty, structs)?,
            None => new,
        }
    } else {
        update(fields[idx].1.clone(), field_ty.as_ref(), rest, new, structs)?
    };
    fields[idx].1 = new_field;
    Ok(Value::Struct { name, fields })
}

/// Запись в элемент массива по индексу (фича 0076). Тип элемента берётся из
/// `ty = TypeNode::Array(_, elem)` — реестр тут ни при чём (массив имён полей не
/// имеет). Выход за границы → `ArrayIndexOutOfBounds`; запись индекса в
/// не-массив → `IndexOfNonArray`.
fn update_index(
    value: Value,
    ty: Option<&TypeNode>,
    index: usize,
    rest: &[PlaceSegment],
    new: Value,
    structs: &dyn StructRegistry,
) -> Result<Value, EvalError> {
    let Value::Array(mut items) = value else {
        return Err(EvalError::IndexOfNonArray {
            value: value_kind(&value),
        });
    };
    if index >= items.len() {
        return Err(EvalError::ArrayIndexOutOfBounds {
            index,
            len: items.len(),
        });
    }
    let elem_ty = match ty {
        Some(TypeNode::Array(_, elem)) => Some(elem.as_ref()),
        _ => None,
    };
    let new_elem = if rest.is_empty() {
        match elem_ty {
            Some(t) => coerce_to_type_with(new, t, structs)?,
            None => new,
        }
    } else {
        update(items[index].clone(), elem_ty, rest, new, structs)?
    };
    items[index] = new_elem;
    Ok(Value::Array(items))
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::eval::EmptyStructs;

    fn point(x: i64, y: i64) -> Value {
        Value::Struct {
            name: "Point".to_string(),
            fields: vec![
                ("x".to_string(), Value::Number(x)),
                ("y".to_string(), Value::Number(y)),
            ],
        }
    }

    fn field(name: &str) -> Vec<PlaceSegment> {
        vec![PlaceSegment::Field(name.to_string())]
    }

    #[test]
    fn update_field_is_pointwise() {
        // p.x := 7 не трогает p.y (без реестра — без приведения типа поля).
        let updated = update(
            point(1, 2),
            None,
            &field("x"),
            Value::Number(7),
            &EmptyStructs,
        )
        .unwrap();
        assert_eq!(updated, point(7, 2));
    }

    #[test]
    fn update_unknown_field_is_error() {
        let err = update(
            point(1, 2),
            None,
            &field("z"),
            Value::Number(0),
            &EmptyStructs,
        )
        .unwrap_err();
        assert_eq!(err.code(), "SIM-027");
    }

    #[test]
    fn update_field_on_non_struct_is_error() {
        let err = update(
            Value::Number(5),
            None,
            &field("x"),
            Value::Number(0),
            &EmptyStructs,
        )
        .unwrap_err();
        assert_eq!(err.code(), "SIM-012");
    }

    // ── Массивы (фича 0076) ──────────────────────────────────────────────────

    fn arr(vals: &[i64]) -> Value {
        Value::Array(vals.iter().map(|&n| Value::Number(n)).collect())
    }

    fn index(i: usize) -> Vec<PlaceSegment> {
        vec![PlaceSegment::Index(i)]
    }

    #[test]
    fn update_array_element_is_pointwise() {
        // data[1] := 9 не трогает соседей (без типа — без приведения элемента).
        let updated = update(
            arr(&[1, 2, 3]),
            None,
            &index(1),
            Value::Number(9),
            &EmptyStructs,
        )
        .unwrap();
        assert_eq!(updated, arr(&[1, 9, 3]));
    }

    #[test]
    fn update_array_out_of_bounds_is_error() {
        let err = update(
            arr(&[1, 2]),
            None,
            &index(5),
            Value::Number(0),
            &EmptyStructs,
        )
        .unwrap_err();
        assert_eq!(err.code(), "SIM-010");
    }

    #[test]
    fn update_index_on_non_array_is_error() {
        let err = update(
            Value::Number(5),
            None,
            &index(0),
            Value::Number(0),
            &EmptyStructs,
        )
        .unwrap_err();
        assert_eq!(err.code(), "SIM-010");
    }

    #[test]
    fn update_array_element_coerced_to_elem_type() {
        // data[0] := 300 при [u8;2] → 44 (усечение по типу элемента, T22).
        let u8_ty = TypeNode::Integer {
            bits: 8,
            signed: false,
        };
        let arr_ty = TypeNode::Array(2, Box::new(u8_ty));
        let updated = update(
            arr(&[0, 0]),
            Some(&arr_ty),
            &index(0),
            Value::Number(300),
            &EmptyStructs,
        )
        .unwrap();
        assert_eq!(updated, arr(&[44, 0]));
    }
}
