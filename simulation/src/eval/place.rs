//! Обновление значения «по месту» (lvalue): запись в поле структуры (фича 0034).
//!
//! Адаптер ([`crate::unit::statement`]) раскладывает левую часть `p.x := …` в
//! корень + путь полей и зовёт [`update`]; семантика замены живёт **здесь**, в
//! ядре под `deny(wildcard_enum_match_arm)`. Точечность обязательна: `p.x := 7`
//! **не** пересоздаёт структуру и не трогает `p.y`.

use super::error::{EvalError, value_kind};
use super::value::Value;
use super::{StructRegistry, coerce_to_type_with};

/// Заменяет значение по пути полей `path` в структурном значении `value`,
/// возвращая обновлённое целое.
///
/// Лист пути приводится к **объявленному типу поля** (из `structs`): `p.y := 300`
/// при `y: u8` даёт `44` — усечение S9 действует внутри поля (T22). Несоответствие
/// селектора виду значения (поле у не-структуры) или неизвестное поле → [`EvalError`].
pub(crate) fn update(
    value: Value,
    path: &[String],
    new: Value,
    structs: &dyn StructRegistry,
) -> Result<Value, EvalError> {
    let Some((field, rest)) = path.split_first() else {
        // Пустой путь: замена целиком. Вызывается только с непустым путём —
        // присваивание всей переменной идёт мимо `update` (ветка `Variable`).
        return Ok(new);
    };
    let Value::Struct { name, mut fields } = value else {
        return Err(EvalError::FieldOfNonStruct {
            value: value_kind(&value),
        });
    };
    let Some(idx) = fields.iter().position(|(f, _)| f == field) else {
        return Err(EvalError::UnknownField {
            name,
            field: field.clone(),
        });
    };
    let new_field = if rest.is_empty() {
        // Лист: привести new к объявленному типу поля (усечение по типу, T22).
        // Если определение недоступно — без приведения (консервативно, как у
        // прочих значений без реестра).
        let field_ty = structs.find_struct(&name).and_then(|def| {
            def.fields
                .iter()
                .find(|(fname, _)| fname == field)
                .map(|(_, t)| t.clone())
        });
        match field_ty {
            Some(ty) => coerce_to_type_with(new, &ty, structs)?,
            None => new,
        }
    } else {
        // Вложенное поле (`o.i.v`): рекурсия по остатку пути.
        update(fields[idx].1.clone(), rest, new, structs)?
    };
    fields[idx].1 = new_field;
    Ok(Value::Struct { name, fields })
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

    #[test]
    fn update_field_is_pointwise() {
        // p.x := 7 не трогает p.y (без реестра — без приведения типа поля).
        let updated = update(
            point(1, 2),
            &["x".to_string()],
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
            &["z".to_string()],
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
            &["x".to_string()],
            Value::Number(0),
            &EmptyStructs,
        )
        .unwrap_err();
        assert_eq!(err.code(), "SIM-012");
    }
}
