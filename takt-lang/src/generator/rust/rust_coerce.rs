//! Приведение значения к типу ПРИЁМНИКА — цель `rust`.
//!
//! Печатник выражений позиции не знает: одна и та же запись бывает верна
//! булевой в условии и числовой в присваивании, а вариант перечисления в Rust
//! числового представления не имеет вовсе. Знание о типе приёмника есть только
//! здесь, поэтому сюда стягиваются все правила такого рода — вариант
//! перечисления (0281), литерал `bit` (0148-01), широкий литерал бит-вектора
//! (0262), агрегат структуры (0293), разряд в позиции числа (0335).
//!
//! ⚠️ Модуль выделен из `rust_expr` фичей 0335 по границе **ответственности**,
//! а не ради размера: печать выражения и приведение к приёмнику — разные
//! вопросы, и второй задаётся ровно там, где известен приёмник.

use crate::diagnostics::Diagnostic;
use crate::generator::rust::rust_expr::{Scope, print_expression, unsupported, unwrap_outer};
use crate::generator::rust::rust_name::rust_type_name;
use crate::parser::ast::Member;
use crate::semantic::ExpressionNode;
use crate::semantic::type_node::TypeNode;

/// Тот же приём чинит `bit`-порт: `elevator_motor_up := 1` при `bool`-порте.
/// Печатает выражение с оглядкой на **целевой** тип.
///
/// Существует ради перечислений. `command := Up` приходит сюда как
/// `Assign(Variable(command), Number(2))`: семантика уже свернула вариант в его
/// числовое значение, и `ExpressionNode` варианта перечисления просто не имеет.
/// Цель `c` печатает `model->command = 2;`, и это **работает** — перечисление в
/// C есть целое. В Rust `Command` и `2` — разные типы, поэтому вариант нужно
/// восстановить по значению.
///
pub(crate) fn coerce_to(
    value: &ExpressionNode,
    target: &TypeNode,
    scope: &Scope,
) -> Result<String, Diagnostic> {
    match (target, value) {
        // Агрегат структуры (фича 0293): `var g: Gains := {2, 3};` печатается
        // литералом `Gains { kp: 2, ki: 3 }`. Без типа приёмника форма
        // неизвестна — общий печатник выражений печатал массив `[2, 3]`, то есть
        // невалидный Rust.
        (
            TypeNode::Struct(struct_name),
            ExpressionNode::Initializer(items) | ExpressionNode::Array(items),
        ) => crate::generator::rust::rust_struct::struct_literal(struct_name, items, scope),
        (TypeNode::Enum(enum_name), ExpressionNode::Number(n)) => {
            enum_variant_literal(enum_name, *n, scope)
        }
        // `bit`/`bool` в Takt принимает 0/1; в Rust это `false`/`true`.
        (TypeNode::Bit | TypeNode::Bool, ExpressionNode::Number(n)) => match n {
            0 => Ok("false".to_string()),
            1 => Ok("true".to_string()),
            other => Err(unsupported(&format!(
                "значение {} не представимо в bool (допустимо 0 или 1)",
                other
            ))),
        },
        // Вещественному полю целый литерал не подходит: `1` не является литералом f64.
        (TypeNode::Rational, ExpressionNode::Number(n)) => Ok(format!("{}.0", n)),
        // Массив СТРУКТУР (фича 0343): элементы печатаются литералом структуры,
        // а не вложенным массивом. Прежде выходило `[[1, 2], [3, 4]]` — `E0308`
        // при нулевом коде возврата `taktc`.
        (
            TypeNode::Array(_, elem),
            ExpressionNode::Initializer(items) | ExpressionNode::Array(items),
        ) if matches!(**elem, TypeNode::Struct(_)) => {
            let mut parts = Vec::new();
            for item in items {
                parts.push(coerce_to(item, elem, scope)?);
            }
            Ok(format!("[{}]", parts.join(", ")))
        }
        // Бит-вектор шире 64 бит — массив слов `[u64; K]` (0078), и целый
        // литерал ему не тип: `w: 0` давало `E0308` (фича 0262). Значение
        // достаётся младшему слову — литерал шире 64 бит язык не принимает
        // (`LE-009`).
        (TypeNode::Array(..), ExpressionNode::Number(n))
            if crate::generator::rust::rust_bit::words_of_type(target).is_some() =>
        {
            let count = crate::generator::rust::rust_bit::words_of_type(target)
                .expect("проверено охраной ветви");
            Ok(crate::generator::rust::rust_bit::word_literal(*n, count))
        }
        // Разряд `x.N` в позиции ЧИСЛОВОГО значения (фича 0335). Печатник
        // выражений отдаёт его **булевым** (`(… & 1) != 0`) — это верно в
        // условии и `E0308` в присваивании числу, при нулевом коде возврата
        // `taktc` (класс 0262). Эталон такую запись исполняет: разряд даёт 0
        // либо 1.
        (_, ExpressionNode::BitAccess(_, Member::Number(_)))
            if crate::generator::rust::rust_type::rust_type(target, "приёмник разряда")
                .is_ok_and(|name| INTEGER_TYPES.contains(&name.as_str())) =>
        {
            let name = crate::generator::rust::rust_type::rust_type(target, "приёмник разряда")?;
            // Внешние скобки снимаются: `u8::from((…))` — это `unused_parens`,
            // то есть отказ сборки порождённого кода под `-D warnings`.
            Ok(format!(
                "{name}::from({})",
                unwrap_outer(&print_expression(value, scope)?)
            ))
        }
        _ => print_expression(value, scope),
    }
}

/// Целочисленные типы Rust, у которых есть `From<bool>` (фича 0335).
///
/// ⚠️ Список **закрыт** намеренно: приведение печатается только туда, где
/// `Тип::from(bool)` существует. Приёмник иного вида (бит-вектор массивом слов,
/// структура, `f64`) уходит прежним путём — там разряд в позиции значения либо
/// невозможен, либо отвергается своей диагностикой.
const INTEGER_TYPES: [&str; 8] = ["u8", "u16", "u32", "u64", "i8", "i16", "i32", "i64"];

/// Имя варианта перечисления по его значению — ОДИН носитель на цель (0281).
///
/// Число, попавшее в позицию перечислимого типа, в Rust непредставимо: у
/// `enum` нет числового представления в выражении. Восстановление варианта
/// нужно **и присваиванию, и сравнению**: до фичи 0281 его знало только
/// присваивание, и `ref Done: c = 1;` давало `self.c == 1` — `E0308`
/// («expected `Command`, found integer») при нулевом коде возврата `taktc`.
///
/// ⚠️ Значение **вне** набора вариантов — честный отказ, а не догадка: в C оно
/// молча легло бы в переменную, здесь представить его нечем.
pub(crate) fn enum_variant_literal(
    enum_name: &str,
    value: i128,
    scope: &Scope,
) -> Result<String, Diagnostic> {
    let def = scope
        .model
        .search_enum(enum_name)
        .ok_or_else(|| unsupported(&format!("перечисление '{}' не найдено", enum_name)))?;
    let variant = def
        .variants
        .iter()
        .find(|(_, v)| *v == value)
        .ok_or_else(|| {
            unsupported(&format!(
                "значение {} не соответствует ни одному варианту перечисления '{}'",
                value, enum_name
            ))
        })?;
    Ok(format!(
        "{}::{}",
        rust_type_name(enum_name, def.loc)?,
        rust_type_name(&variant.0, def.loc)?
    ))
}
