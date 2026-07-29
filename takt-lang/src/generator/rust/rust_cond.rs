//! Трансляция **условий** Takt в Rust (`print_condition` и спутники).
//!
//! Вынесено из `rust_expr.rs` (фича 0088 — лимит размера модуля, ADR 0088):
//! чистое перемещение, вывод байт-в-байт неизменен. Печатник условий отделён от
//! печатника выражений намеренно (ADR 0019: у `=` разная семантика — равенство в
//! условии, присваивание в выражении); здесь — только ветвь условий.

use crate::diagnostics::{Diagnostic, Location};
use crate::generator::rust::rust_expr::{
    Scope, bit_mask, call_arguments, member_index, rational, unsupported, variable,
};
use crate::generator::rust::rust_fixed::function_return;
use crate::generator::rust::rust_name::{rust_type_name, rust_value_name};
use crate::semantic::type_node::TypeNode;
use crate::semantic::{ConditionNode, FunctionDefinitionNode};

/// Транслирует условие Takt в выражение `bool` Rust.
///
/// # Ошибки
/// [`RS-011`] на непереводимой конструкции.
pub(crate) fn print_condition(cond: &ConditionNode, scope: &Scope) -> Result<String, Diagnostic> {
    match cond {
        // Литерал длительности ВНЕ `after` — сравнение со значением типа
        // `duration` (фича 0183). Печатается **миллисекундами**, как и значение.
        ConditionNode::Duration(nanos) => Ok(crate::semantic::duration::value_millis(
            *nanos,
            Location::Codegen,
            "литерал длительности в условии",
        )?
        .to_string()),
        // Выдержка `after` (фича 0134): профиль «такты» → счётчик `takt_dwell`;
        // профиль «часы» → метка `now_ms` и сравнение разностью с обёрткой.
        ConditionNode::After(nanos) => {
            let units = crate::semantic::duration::units_or_diagnostic(
                *nanos,
                scope.time_profile,
                Location::Codegen,
                "выдержка 'after'",
            )?;
            match scope.time_profile {
                crate::semantic::duration::TimeProfile::Ticks { .. } => {
                    Ok(crate::generator::rust::rust_time::dwell_after_expr(units))
                }
                crate::semantic::duration::TimeProfile::Clock => {
                    let hal = scope.hal_receiver("выдержка 'after'")?;
                    Ok(crate::generator::rust::rust_time::clock_after_expr(
                        hal, units,
                    ))
                }
            }
        }
        // Тактовая выдержка `after Nt`: частота не нужна — счётчик и так считает такты.
        ConditionNode::AfterTicks(ticks) => Ok(
            crate::generator::rust::rust_time::dwell_after_expr(*ticks as u64),
        ),
        ConditionNode::Number(n) => Ok(n.to_string()),
        ConditionNode::Rational(text, negative) => Ok(rational(text, *negative)),
        ConditionNode::Bool(b) => Ok(b.to_string()),
        ConditionNode::Variable(var, _) => variable(&var.borrow(), scope),
        ConditionNode::Parenthesis(inner) => print_condition(inner, scope),
        ConditionNode::Not(a) => Ok(format!("(!{})", condition_as_bool(a, scope)?)),

        // `=` в УСЛОВИИ — равенство (ADR 0019). Именно ради этого различия
        // печатник условий отделён от печатника выражений.
        //
        // Спецформа `S(Модель) = Состояние` перехватывается ДО общего случая:
        // её операнды — модель и имя состояния, а не значения, и обычным
        // сравнением их не напечатать.
        ConditionNode::Equal(a, b) => match state_comparison(a, b, "==", scope)? {
            Some(text) => Ok(text),
            None => match boolean_comparison(a, "==", b, scope)? {
                Some(text) => Ok(text),
                None => cond_binary(a, "==", b, scope),
            },
        },
        ConditionNode::NotEqual(a, b) => match state_comparison(a, b, "!=", scope)? {
            Some(text) => Ok(text),
            None => match boolean_comparison(a, "!=", b, scope)? {
                Some(text) => Ok(text),
                None => cond_binary(a, "!=", b, scope),
            },
        },
        ConditionNode::Less(a, b) => cond_binary(a, "<", b, scope),
        ConditionNode::More(a, b) => cond_binary(a, ">", b, scope),
        ConditionNode::LessEqual(a, b) => cond_binary(a, "<=", b, scope),
        ConditionNode::MoreEqual(a, b) => cond_binary(a, ">=", b, scope),
        ConditionNode::Add(a, b) => cond_binary(a, "+", b, scope),
        ConditionNode::Subtract(a, b) => cond_binary(a, "-", b, scope),

        // `&`/`|` в условии Takt — побитовые (как в C). На `bool` в Rust они
        // определены и дают `bool`, поэтому трансляция один в один законна и
        // для булевых операндов, и для целых.
        ConditionNode::And(a, b) => cond_bool_binary(a, "&", b, scope),
        ConditionNode::Or(a, b) => cond_bool_binary(a, "|", b, scope),

        ConditionNode::EnumVariant(def, variant, _) => {
            let enum_name = rust_type_name(&def.borrow().name, def.borrow().loc)?;
            Ok(format!(
                "{}::{}",
                enum_name,
                rust_type_name(variant, def.borrow().loc)?
            ))
        }

        ConditionNode::ArraySubscript(var, index) => Ok(format!(
            "{}[{} as usize]",
            variable(&var.borrow(), scope)?,
            print_condition(index, scope)?
        )),

        ConditionNode::BitAccess(inner, member) => {
            let base = print_condition(inner, scope)?;
            Ok(bit_mask(&base, member_index(member)?))
        }

        ConditionNode::Function(def, args, loc) => {
            let printed = args
                .iter()
                .map(|a| print_condition(a, scope))
                .collect::<Result<Vec<_>, _>>()?;
            let borrowed = def.borrow();
            match &*borrowed {
                FunctionDefinitionNode::Builtin(name, _, _) => Err(Diagnostic::error(
                    *loc,
                    format!(
                        "Встроенная функция '{}' в условии перехода не транслируется \
                         в Rust: поддержано только 'S(Модель) = Состояние'",
                        name
                    ),
                )
                .with_code("RS-011")),
                local @ FunctionDefinitionNode::Local { name, .. } => Ok(format!(
                    "{}({})",
                    rust_value_name(name, *loc)?,
                    call_arguments(local, &printed, scope)?.join(", ")
                )),
                FunctionDefinitionNode::External { name, .. } => Ok(format!(
                    "{}.{}({})",
                    scope.hal_receiver(&format!("вызов внешней функции '{}'", name))?,
                    rust_value_name(name, *loc)?,
                    printed.join(", ")
                )),
                FunctionDefinitionNode::None | FunctionDefinitionNode::Unresolved(_) => {
                    Err(unsupported("неразрешённая функция в условии"))
                }
            }
        }

        ConditionNode::None => Err(unsupported("пустое условие")),
        ConditionNode::Unresolved(_) => Err(unsupported("неразрешённое условие")),
        ConditionNode::String(_) => Err(unsupported("строковый литерал в условии")),
        ConditionNode::Model(_, _) => Err(unsupported(
            "модель в позиции условия вне формы 'S(Модель) = Состояние'",
        )),
        ConditionNode::State(..) => Err(unsupported(
            "состояние в позиции условия вне формы 'S(Модель) = Состояние'",
        )),
    }
}

/// Распознаёт спецформу `S(Модель) = Состояние` и печатает сравнение состояний.
///
/// Возвращает `None`, если условие к этой форме не относится — тогда печатается
/// обычное сравнение.
///
/// ## Почему имя состояния берётся строкой
///
/// Правая часть (`End` в `S(Ping) = End`) приходит **неразрешённой**: `End` —
/// состояние модели-аргумента, а не той, где записано условие, и семантика
/// разрешить его не может (`CLAUDE.md`: проход `resolve_state_references`
/// запрещён — он ломает ровно эту конструкцию, охраняется тестом
/// `syntax_simple`). Разрешение выполняется здесь, в области видимости
/// модели-аргумента. Цель `c` поступает так же (`generate_state_comparison`).
fn state_comparison(
    left: &ConditionNode,
    right: &ConditionNode,
    op: &str,
    scope: &Scope,
) -> Result<Option<String>, Diagnostic> {
    let Some(model) = model_of(left) else {
        return Ok(None);
    };
    let state_name = match right {
        ConditionNode::Variable(v, ..) => v.borrow().name().to_string(),
        // Неразрешённое имя — ШТАТНЫЙ случай (см. заголовок функции).
        ConditionNode::Unresolved(crate::parser::ast::Condition::Variable(id)) => id.name.clone(),
        // Имя, случайно совпавшее с состоянием объемлющей модели: семантика
        // разрешила его в ЧУЖОЙ области. Берём только имя.
        ConditionNode::State(state, _) => state.borrow().name().to_string(),
        _ => return Ok(None),
    };

    let unique = crate::semantic::minimap::Name::from(std::rc::Rc::clone(&model));
    let field = scope
        .instances
        .iter()
        .find(|(u, _)| u == unique.unique())
        .map(|(_, f)| f.clone())
        .ok_or_else(|| {
            unsupported(&format!(
                "условие по состоянию модели '{}': её экземпляр не найден среди \
                 под-моделей текущего состояния",
                unique.local()
            ))
        })?;

    // Поле `state` и перечисление состояний приватны, но лежат в ЭТОМ ЖЕ
    // модуле — обращение законно. Имя перечисления строится той же формулой,
    // что и в `rust_model::StateTable`.
    Ok(Some(format!(
        "(self.{}.state {} {}State::{})",
        field,
        op,
        unique.unique_camelcase(),
        rust_type_name(&state_name, Location::Codegen)?
    )))
}

/// Извлекает модель из левой части: `Модель` либо `S(Модель)`.
fn model_of(
    cond: &ConditionNode,
) -> Option<std::rc::Rc<std::cell::RefCell<crate::semantic::ModelNode>>> {
    match cond {
        ConditionNode::Model(model, _) => Some(std::rc::Rc::clone(model)),
        ConditionNode::Function(fun, args, _) => {
            if !matches!(&*fun.borrow(), FunctionDefinitionNode::Builtin("S", ..)) {
                return None;
            }
            // Арность `S` — ровно один параметр, проверена семантикой.
            match args.first().map(|a| a.as_ref())? {
                ConditionNode::Model(model, _) => Some(std::rc::Rc::clone(model)),
                _ => None,
            }
        }
        _ => None,
    }
}

/// Печатает бинарное условие. Скобки — см. [`binary`](super::rust_expr).
fn cond_binary(
    a: &ConditionNode,
    op: &str,
    b: &ConditionNode,
    scope: &Scope,
) -> Result<String, Diagnostic> {
    Ok(format!(
        "({} {} {})",
        print_condition(a, scope)?,
        op,
        print_condition(b, scope)?
    ))
}

/// Печатает сравнение булева операнда (`bit`/`bool`) с литералом — фикс 0148-01.
///
/// Возвращает `None`, если форма не эта: тогда печатается обычное сравнение.
///
/// ## Зачем отдельная ветвь
///
/// В Takt `bit` — целое-однобитное, и `ref Next: btn = 1;` — естественная
/// запись. В Rust `bit` отображается на **`bool`** (порт читается
/// `hal.read_bit(...) -> bool`), поэтому дословный перевод даёт `bool == 1`,
/// а это **ошибка типов**: порождённый модуль не компилируется вовсе.
///
/// Форма с булевым литералом (`btn = true`) компилируется, но валит
/// `clippy::bool_comparison` — то есть не проходит политику `-D warnings`
/// гейта цели (ADR 0050, R9) и не соберётся у пользователя с той же политикой.
///
/// Обе формы сводятся к самому операнду: `x = 1` и `x = true` → `x`,
/// `x = 0` и `x = false` → `(!x)`; для `!=` — наоборот. Так же поступает
/// присваивание (`o := 1` → `write_bit(..., true)`), и разъезжаться этим двум
/// путям незачем.
fn boolean_comparison(
    a: &ConditionNode,
    op: &str,
    b: &ConditionNode,
    scope: &Scope,
) -> Result<Option<String>, Diagnostic> {
    /// Операнд булев по статическому типу?
    fn is_boolean(cond: &ConditionNode) -> bool {
        matches!(
            condition_type(cond),
            Some(TypeNode::Bool) | Some(TypeNode::Bit)
        )
    }
    /// Литерал, читаемый как булев: `true`/`false` и число (0 — ложь).
    fn literal(cond: &ConditionNode) -> Option<bool> {
        match cond {
            ConditionNode::Bool(v) => Some(*v),
            ConditionNode::Number(n) => Some(*n != 0),
            ConditionNode::Parenthesis(inner) => literal(inner),
            _ => None,
        }
    }

    // Литерал ищется с ОБЕИХ сторон: `btn = 1` и `1 = btn` одинаково законны.
    // Проверка «операнд не литерал» обязательна, иначе `true = false` попало бы
    // в первую ветвь и потеряло бы вторую половину.
    let (operand, value) = match (literal(a), literal(b)) {
        (None, Some(v)) if is_boolean(a) => (a, v),
        (Some(v), None) if is_boolean(b) => (b, v),
        _ => return Ok(None),
    };

    let printed = print_condition(operand, scope)?;
    let positive = (op == "==") == value;
    Ok(Some(if positive {
        printed
    } else {
        format!("(!{})", printed)
    }))
}

/// Печатает `&`/`|`, приводя операнды к `bool`.
///
/// Операнд-порт (`ElevatorMotor_SensorU`) в Takt используется как условие
/// напрямую; в Rust `bool & bool` законно, но `u8 & bool` — нет.
fn cond_bool_binary(
    a: &ConditionNode,
    op: &str,
    b: &ConditionNode,
    scope: &Scope,
) -> Result<String, Diagnostic> {
    Ok(format!(
        "({} {} {})",
        condition_as_bool(a, scope)?,
        op,
        condition_as_bool(b, scope)?
    ))
}

/// Возвращает тип условия, если он выводится статически.
pub(crate) fn condition_type(cond: &ConditionNode) -> Option<TypeNode> {
    match cond {
        ConditionNode::Bool(_) => Some(TypeNode::Bool),
        ConditionNode::Number(_) => Some(TypeNode::Integer {
            bits: 32,
            signed: true,
        }),
        ConditionNode::Rational(_, _) => Some(TypeNode::Rational),
        ConditionNode::Variable(var, _) => Some(var.borrow().ty().clone()),
        ConditionNode::Parenthesis(inner) => condition_type(inner),
        ConditionNode::Equal(_, _)
        | ConditionNode::NotEqual(_, _)
        | ConditionNode::Less(_, _)
        | ConditionNode::More(_, _)
        | ConditionNode::LessEqual(_, _)
        | ConditionNode::MoreEqual(_, _)
        | ConditionNode::Not(_)
        | ConditionNode::And(_, _)
        | ConditionNode::Or(_, _)
        // Выдержка `after` — булево условие (фича 0134): `takt_dwell >= N` либо
        // сравнение метки `now_ms` разностью, оба дают `bool`.
        | ConditionNode::After(_)
        | ConditionNode::AfterTicks(_)
        | ConditionNode::BitAccess(_, _) => Some(TypeNode::Bool),
        ConditionNode::ArraySubscript(var, _) => match var.borrow().ty() {
            TypeNode::Array(_, elem) => Some((**elem).clone()),
            _ => None,
        },
        ConditionNode::Function(def, _, _) => function_return(&def.borrow()),
        // Вариант перечисления имеет тип своего перечисления: нужен, чтобы
        // `command = Up` сравнивалось, а не приводилось к bool.
        ConditionNode::EnumVariant(def, _, _) => Some(TypeNode::Enum(def.borrow().name.clone())),
        _ => None,
    }
}

/// Печатает условие, приводя его к `bool`.
///
/// Точка входа для guard'ов рёбер: `ref Next: x;` при `x : u8` в C означает
/// `if (x)`, в Rust — `if x != 0`.
pub(crate) fn condition_as_bool(cond: &ConditionNode, scope: &Scope) -> Result<String, Diagnostic> {
    let printed = print_condition(cond, scope)?;
    match condition_type(cond) {
        Some(TypeNode::Bool) | Some(TypeNode::Bit) => Ok(printed),
        Some(TypeNode::Rational) => Ok(format!("({} != 0.0)", printed)),
        Some(TypeNode::Integer { .. }) => Ok(format!("({} != 0)", printed)),
        _ => Err(unsupported(&format!(
            "условие '{}': тип не выводится, приведение к bool построить нельзя",
            printed
        ))),
    }
}
