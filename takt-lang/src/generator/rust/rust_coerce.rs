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
        // Массив: КАЖДЫЙ элемент печатается по типу элемента (фича 0343 —
        // структуры, фича 0368 — все остальные типы). Прежде правило знало
        // только структуру, и `var modes: [Mode; 2] := {Idle, Work};` давало
        // `[0, 1]` в поле `[Mode; 2]` — `E0308` при нулевом коде возврата
        // `taktc`, при том что та же запись СКАЛЯРОМ работает.
        //
        // ⚠️ Бит-вектор `[bit;N≤64]` исключён: это упакованный скаляр (правило
        // 0078), и поэлементная печать дала бы массив булевых.
        (
            TypeNode::Array(_, elem),
            ExpressionNode::Initializer(items) | ExpressionNode::Array(items),
        ) if crate::semantic::bit_vector::is_bit_vector(target).is_none() => {
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
        // Арифметика печатается В ТИПЕ ПРИЁМНИКА (фича 0360). Прежде операнды
        // печатались как есть, и `r: u16 := a + b;` при `a, b: u8` давало
        // **`E0308`** — вывод не компилировался вовсе, при нулевом коде
        // возврата `taktc`. Эталон и цель `c` (продвижение до `int`) считают
        // такую запись верно.
        //
        // ⚠️ Приводятся ОПЕРАНДЫ, а не результат: `(a + b) as u16` считало бы в
        // `u8` и обёртывало **до** расширения — 300 стало бы 44.
        (
            TypeNode::Integer { .. },
            ExpressionNode::Add(l, r)
            | ExpressionNode::Subtract(l, r)
            | ExpressionNode::Multiply(l, r)
            | ExpressionNode::Divide(l, r)
            | ExpressionNode::Modulo(l, r),
        ) if operands_need_cast(l, r, target) => {
            let op = match value {
                ExpressionNode::Add(..) => "+",
                ExpressionNode::Subtract(..) => "-",
                ExpressionNode::Multiply(..) => "*",
                ExpressionNode::Divide(..) => "/",
                _ => "%",
            };
            let name = crate::generator::rust::rust_type::rust_type(target, "приёмник арифметики")?;
            // Обёртка по правилу 0127 сохраняется: печатник арифметики её и
            // ставит, поэтому операнды приводятся, а операция печатается им же.
            Ok(format!(
                "({} {op} {})",
                cast_operand(l, &name, scope)?,
                cast_operand(r, &name, scope)?
            ))
        }
        // Именованное значение ИНОГО целого типа приводится к приёмнику
        // (фича 0360): `rr := r;` при `r: i16` и `rr: u16` давало `E0308` —
        // вывод не компилировался, тогда как эталон и цель `c` (неявное
        // преобразование) запись исполняют.
        (TypeNode::Integer { .. }, ExpressionNode::Variable(_))
            if crate::generator::mixed_sign::operand_type_expr(value)
                .is_some_and(|ty| matches!(ty, TypeNode::Integer { .. }) && ty != *target) =>
        {
            let name = crate::generator::rust::rust_type::rust_type(target, "приёмник значения")?;
            Ok(format!("({} as {name})", print_expression(value, scope)?))
        }
        // Целая степень печатается С ОГЛЯДКОЙ НА ПРИЁМНИК (фича 0415): тип
        // кладётся в контекст, а печать идёт ОБЫЧНЫМ путём — арифметика вокруг
        // степени сохраняет свою обёртку (`wrapping_add`, правило 0127).
        //
        // ⚠️ Ветвь охраняется признаком «в выражении есть степень»: без охраны
        // подсказка появлялась бы у всех выражений корпуса, а предмет фичи —
        // только степень.
        (_, _) if contains_power(value) => {
            print_expression(value, &scope.with_power_target(target))
        }
        _ => print_expression(value, scope),
    }
}

/// Нужно ли приводить операнды арифметики к типу приёмника (фича 0360).
///
/// Признак узкий: оба операнда — **именованные значения** целого типа, и хотя
/// бы у одного тип отличается от приёмника. Литерал сюда не входит (у него
/// типа нет, он подстраивается), выражение — тоже: приведение там означало бы
/// догадку о промежуточном типе.
fn operands_need_cast(l: &ExpressionNode, r: &ExpressionNode, target: &TypeNode) -> bool {
    let (Some(lt), Some(rt)) = (
        crate::generator::mixed_sign::operand_type_expr(l),
        crate::generator::mixed_sign::operand_type_expr(r),
    ) else {
        return false;
    };
    matches!(target, TypeNode::Integer { .. })
        && matches!(lt, TypeNode::Integer { .. })
        && matches!(rt, TypeNode::Integer { .. })
        && (lt != *target || rt != *target)
}

/// Печатает операнд с приведением к типу приёмника, если он иного типа.
fn cast_operand(
    node: &ExpressionNode,
    target_name: &str,
    scope: &Scope,
) -> Result<String, Diagnostic> {
    Ok(format!(
        "({} as {target_name})",
        print_expression(node, scope)?
    ))
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

/// Есть ли в выражении узел степени (фича 0415).
///
/// Обход **намеренно неисчерпывающий**: пропущенная форма даёт прежнюю печать,
/// а не порчу вывода — как у обхода ссылки вперёд (0246).
fn contains_power(value: &ExpressionNode) -> bool {
    match value {
        ExpressionNode::Power(..) => true,
        ExpressionNode::Add(l, r)
        | ExpressionNode::Subtract(l, r)
        | ExpressionNode::Multiply(l, r)
        | ExpressionNode::Divide(l, r)
        | ExpressionNode::Modulo(l, r) => contains_power(l) || contains_power(r),
        ExpressionNode::Parenthesis(inner) => contains_power(inner),
        _ => false,
    }
}
