//! Q-арифметика fixed-point `q(m, n)` для цели Rust (фича 0061, задача 0061-03).
//!
//! Нормативные правила ADR 0061 — обязаны совпасть **побитово** с эталоном
//! симулятора ([`takt_sim::eval::fixed`]) и прочими целями:
//!
//! - `+`/`−` — сложение представлений с wraparound к `W = m + n` (сужение `as
//!   i{W}` — в Rust это усечение битов, а не паника: `i64`-промежуток не
//!   переполняется, `as i{W}` даёт two's-complement перенос, как `wrap(_, W)`);
//! - `*` — точное произведение в `i128`, затем `>> n`. В Rust `>>` знакового
//!   **арифметический** (floor к −∞) и **определён** — в отличие от C (C11
//!   6.5.7p5), где та же трансляция опиралась бы на implementation-defined;
//! - `/` — делимое `<< n` (в Rust `<<` знакового определён), затем деление
//!   (усечение к нулю).
//!
//! Здесь же живёт [`expression_type`] — вывод типа выражения для приведения к
//! `bool` в позиции условия и для восстановления вариантов перечисления; он
//! тематически рядом с детектором формата [`fixed_format`].

use crate::diagnostics::Diagnostic;
use crate::generator::rust::rust_expr::{Scope, print_expression, unsupported};
use crate::generator::rust::rust_type::rust_type;
use crate::semantic::type_node::{TypeNode, fixed_storage_bits};
use crate::semantic::{ExpressionNode, FunctionDefinitionNode};

/// Возвращает тип выражения, если он выводится статически.
///
/// Нужен для приведения к `bool` в позиции условия: в C `if (x)` при `x : u8`
/// законно, в Rust — ошибка типа. Без типа операнда угадывать нельзя — тот же
/// урок, что у ST (`ST-011`: «без типа операнда имя функции не построить»).
pub(crate) fn expression_type(expr: &ExpressionNode) -> Option<TypeNode> {
    match expr {
        ExpressionNode::Bool(_) => Some(TypeNode::Bool),
        ExpressionNode::Number(_) => Some(TypeNode::Integer {
            bits: 32,
            signed: true,
        }),
        ExpressionNode::Rational(_, _) => Some(TypeNode::Rational),
        ExpressionNode::Variable(var) => Some(var.borrow().ty().clone()),
        ExpressionNode::Parenthesis(inner) => expression_type(inner),
        ExpressionNode::Cast(_, ty) => Some(ty.clone()),
        // Сравнения и логические операции дают `bool` независимо от операндов.
        ExpressionNode::Less(_, _)
        | ExpressionNode::More(_, _)
        | ExpressionNode::LessEqual(_, _)
        | ExpressionNode::MoreEqual(_, _)
        | ExpressionNode::Equal(_, _)
        | ExpressionNode::NotEqual(_, _)
        | ExpressionNode::And(_, _)
        | ExpressionNode::Or(_, _)
        | ExpressionNode::Not(_)
        | ExpressionNode::BitAccess(_, _) => Some(TypeNode::Bool),
        ExpressionNode::ArraySubscript(var, _) => match var.borrow().ty() {
            TypeNode::Array(_, elem) => Some((**elem).clone()),
            _ => None,
        },
        // Тип вызова — ОБЪЯВЛЕННЫЙ возврат функции. Без этого `if is_ready()`
        // при `fn is_ready() -> bool` не приводится к bool: тип «не выводится»,
        // и честная диагностика RS-011 срабатывает там, где всё известно.
        ExpressionNode::Function(def, _) => function_return(&def.borrow()),
        _ => None,
    }
}

/// Возвращает объявленный тип результата функции.
pub(crate) fn function_return(def: &FunctionDefinitionNode) -> Option<TypeNode> {
    match def {
        FunctionDefinitionNode::Local { ret, .. }
        | FunctionDefinitionNode::External { ret, .. } => Some(ret.clone()),
        // У встроенных возврат описан тем же полем (`min`/`max`/… — Numeric,
        // `debug` — Unit): угадывать не требуется.
        FunctionDefinitionNode::Builtin(_, _, ret) => Some(ret.clone()),
        FunctionDefinitionNode::None | FunctionDefinitionNode::Unresolved(_) => None,
    }
}

/// Арифметическая операция над `q(m, n)`.
#[derive(Clone, Copy)]
pub(crate) enum FixedOp {
    Add,
    Subtract,
    Multiply,
    Divide,
}

/// Формат `q(m, n)` выражения, если его тип — `Fixed`.
///
/// `SE-059` гарантирует единый формат обоих операндов арифметики, поэтому у
/// бинарного узла достаточно взять формат любой стороны, у которой он выводится.
pub(crate) fn fixed_format(expr: &ExpressionNode) -> Option<(u8, u8)> {
    if let Some(TypeNode::Fixed { m, n }) = expression_type(expr) {
        return Some((m, n));
    }
    match expr {
        ExpressionNode::Add(a, b)
        | ExpressionNode::Subtract(a, b)
        | ExpressionNode::Multiply(a, b)
        | ExpressionNode::Divide(a, b) => fixed_format(a).or_else(|| fixed_format(b)),
        ExpressionNode::Negate(a) | ExpressionNode::Parenthesis(a) => fixed_format(a),
        _ => None,
    }
}

/// Тип хранения `q(m, n)` в Rust: `i{S}`, где `S` — округлённая вверх ширина.
fn storage(m: u8, n: u8) -> String {
    format!("i{}", fixed_storage_bits(m + n))
}

/// Оборачивает готовое выражение-представление в перенос к **W** и хранение `i{S}`.
///
/// ⚠️ Приведение `as i{S}` переносит к ширине **хранения**, а правило 3 ADR 0061
/// требует переноса к `W = m + n`: при `W = 12` это 16 бит против 12, то есть
/// другая граница (фикс 0061-01). Совпадают они лишь при `W ∈ {8, 16, 32, 64}` —
/// таков весь корпус, поэтому расхождение дожило от 0061 незамеченным.
///
/// ⚠️ При `W = S` форма печати **прежняя байт-в-байт**: иначе поедут снапшоты
/// `examples/generated`. Перенос делается парой сдвигов (в Rust `<<` не паникует
/// на потере старших бит, `>>` знакового — арифметический), а не маской: так
/// знак восстанавливается тем же выражением.
/// ⚠️ `wide` говорит, что `inner` посчитан в `i128` (произведение, деление,
/// пересчёт `q → q`): такому выражению нужен явный `as i64` перед сдвигами, а
/// уже-`i64` его давать **нельзя** — `clippy::unnecessary_cast` под `-D warnings`.
fn wrap_to(inner: &str, m: u8, n: u8, wide: bool) -> String {
    let s = storage(m, n);
    let w = m + n;
    if w == fixed_storage_bits(w) {
        return format!("(({inner}) as {s})");
    }
    let shift = 64 - w;
    let narrowed = if wide {
        format!("({inner}) as i64")
    } else {
        inner.to_string()
    };
    format!("((({narrowed}) << {shift} >> {shift}) as {s})")
}

/// `2^n` как целочисленный литерал.
fn pow2(n: u8) -> u64 {
    1u64 << n
}

/// Печатает бинарную q-операцию: результат сужается к `i{W}` (wraparound к W).
pub(crate) fn binary(
    op: FixedOp,
    a: &ExpressionNode,
    b: &ExpressionNode,
    scope: &Scope,
    m: u8,
    n: u8,
) -> Result<String, Diagnostic> {
    // Операнды НЕ оборачиваются в скобки: `print_expression` уже скобкует
    // составные узлы, а лишняя пара → clippy::double_parens под `-D warnings`.
    // Приоритет `as` выше `+`/`*`/`>>`, поэтому `la as i64 + lb as i64` группирует
    // верно и без скобок вокруг операндов.
    let (la, lb) = (print_expression(a, scope)?, print_expression(b, scope)?);
    let inner = match op {
        FixedOp::Add => format!("{la} as i64 + {lb} as i64"),
        FixedOp::Subtract => format!("{la} as i64 - {lb} as i64"),
        // Точное произведение 2W, floor к −∞ через арифметический `>>` (в Rust
        // определён для знакового — правило 4 ADR, C-цель обходит C11 хелпером).
        FixedOp::Multiply => format!("({la} as i128 * {lb} as i128) >> {n}"),
        // Делимое ← n влево (в Rust `<<` знакового определён), деление к нулю.
        // Скобка вокруг `(la as i128)` обязательна: `la as i128 << n` Rust парсит
        // как generic `i128<...>`, а не сдвиг (E0747-подобная ошибка).
        FixedOp::Divide => format!("(({la} as i128) << {n}) / {lb} as i128"),
    };
    let wide = matches!(op, FixedOp::Multiply | FixedOp::Divide);
    Ok(wrap_to(&inner, m, n, wide))
}

/// Печатает унарный минус над `q(m, n)`: `−repr` с wraparound к W.
pub(crate) fn negate(
    inner: &ExpressionNode,
    scope: &Scope,
    m: u8,
    n: u8,
) -> Result<String, Diagnostic> {
    Ok(wrap_to(
        &format!("-{} as i64", print_expression(inner, scope)?),
        m,
        n,
        false,
    ))
}

/// Печатает приведение `expr as T`, когда источник **или** цель — `q(m, n)`.
///
/// Масштабирование (правило 6 ADR): `int`/`bool` ↔ `q` — сдвиг на `2^n` (в Rust
/// `<<`/`>>` знакового определены), `float` → `q` — недоступно в `no_std` (нет
/// `f64::floor` без `libm`) → честная `RS-011`.
pub(crate) fn cast(
    inner: &ExpressionNode,
    target: &TypeNode,
    scope: &Scope,
) -> Result<String, Diagnostic> {
    let src = fixed_format(inner);
    let printed = print_expression(inner, scope)?;
    match (src, target) {
        // q → q: пересчёт дробных разрядов (влево — сдвиг, вправо — floor `>>`).
        (Some((_, from_n)), TypeNode::Fixed { m: tm, n: tn }) => {
            // Скобка вокруг `(printed as i128)` обязательна перед `<<`/`>>`
            // (иначе Rust парсит `i128<...>` как generic, не сдвиг).
            let inner = if tn >= &from_n {
                format!("({printed} as i128) << {}", tn - from_n)
            } else {
                format!("({printed} as i128) >> {}", from_n - tn)
            };
            Ok(wrap_to(&inner, *tm, *tn, true))
        }
        // q → float: repr / 2^n (точно представимо в f64).
        (Some((_, from_n)), TypeNode::Rational) => {
            Ok(format!("({printed} as f64 / {}.0)", pow2(from_n)))
        }
        // q → целое/бит: floor(repr / 2^n) = целая часть (арифметический `>>`).
        (Some((_, from_n)), _) => {
            let t = rust_type(target, "приведение q → целое")?;
            Ok(format!("((({printed} as i64) >> {from_n}) as {t})"))
        }
        // float → q: floor(f · 2^n) — нет в no_std без libm.
        (None, TypeNode::Fixed { .. })
            if matches!(expression_type(inner), Some(TypeNode::Rational)) =>
        {
            Err(unsupported(
                "приведение float → q в рантайме: нужен floor, которого нет в \
                 no_std без libm (литеральный float понижается на этапе компиляции)",
            ))
        }
        // целое/бит → q: repr = v · 2^n с wraparound к W.
        (None, TypeNode::Fixed { m: tm, n: tn }) => Ok(wrap_to(
            &format!("({printed} as i64) << {tn}"),
            *tm,
            *tn,
            false,
        )),
        // Ни источник, ни цель не q — вызывающий не должен был звать сюда.
        (None, _) => {
            let t = rust_type(target, "приведение типа")?;
            Ok(format!("({printed} as {t})"))
        }
    }
}
