//! Разворот цикла `for` со **статическими** границами (фича 0321).
//!
//! # Зачем
//!
//! В синтезируемом RTL цикла как такового нет: он обязан разворачиваться в
//! схему. Цель отказывала на **любом** `for` — в том числе на таком, у которого
//! границы известны при компиляции:
//!
//! ```takt
//! for var k: u8 := 0; k < 3; k := k + 1 { sum := sum + k; }
//! ```
//!
//! Замер 2026-08-20: эталон, `c`, `c-hal`, `st`, `st-at`, `rust` и `plantuml`
//! этот вход исполняют, `sv` и `sv-mmio` отвечали `SV-002`. Причина в тексте
//! отказа была названа верно («границы, известные на этапе синтеза»), но
//! **проверялась** она никак: отказ выдавался всем.
//!
//! # Что считается статическим
//!
//! - `init` — объявление переменной с **литеральным** начальным значением;
//! - `cond` — сравнение **той же** переменной с литералом (`<`, `<=`, `>`,
//!   `>=`, `!=`);
//! - `step` — присваивание вида `k := k + литерал` либо `k := k - литерал`.
//!
//! Всё прочее — прежний отказ: разворачивать то, чьи границы неизвестны,
//! значит гадать.
//!
//! ⚠️ **Предел итераций назван и мал** ([`MAX_ITERATIONS`]): развёрнутый цикл
//! есть **схема**, и тысяча итераций — тысяча копий тела. Отказ с числом
//! честнее, чем модуль, который не помещается в кристалл.
//!
//! ⚠️ Значения считаются **тем же** способом, что исполнил бы эталон: шаг
//! прибавляется к текущему значению, условие проверяется перед телом. Ошибка
//! здесь дала бы молча другое число итераций — расхождение, которого не увидит
//! ни один линтер (урок 0045).

use crate::semantic::{ExpressionNode, StatementNode};

/// Предел числа итераций разворота.
///
/// Цифра — не свойство языка, а граница разумного размера схемы: 64 копии тела
/// ещё читаемы в порождённом модуле, дальше отказ полезнее.
pub(in crate::generator::sv) const MAX_ITERATIONS: usize = 64;

/// Развёрнутый цикл: имя переменной и значения, которые она принимает.
pub(in crate::generator::sv) struct Unrolled {
    /// Имя переменной цикла — ей присваивается значение перед каждой копией.
    pub name: String,
    /// Значения по итерациям, в порядке исполнения.
    pub values: Vec<i128>,
}

/// Разбирает `for` на статические границы; `None` — границы неизвестны.
pub(in crate::generator::sv) fn unroll(
    init: Option<&StatementNode>,
    cond: Option<&ExpressionNode>,
    step: Option<&ExpressionNode>,
) -> Option<Unrolled> {
    let StatementNode::Variable(name, _, Some(start)) = init? else {
        return None;
    };
    let mut value = literal(start)?;
    let (bound, compare) = comparison(cond?, name)?;
    let delta = increment(step?, name)?;
    let mut values = Vec::new();
    while compare(value, bound) {
        values.push(value);
        if values.len() > MAX_ITERATIONS {
            return None;
        }
        value = value.checked_add(delta)?;
    }
    Some(Unrolled {
        name: name.clone(),
        values,
    })
}

/// Целое значение литерала.
fn literal(expr: &ExpressionNode) -> Option<i128> {
    match expr {
        ExpressionNode::Number(v) => Some(*v),
        ExpressionNode::Parenthesis(inner) => literal(inner),
        _ => None,
    }
}

/// Предикат продолжения цикла: «текущее значение против границы».
type Continues = fn(i128, i128) -> bool;

/// Разбирает условие продолжения: граница и предикат сравнения.
fn comparison(cond: &ExpressionNode, name: &str) -> Option<(i128, Continues)> {
    let (left, right, compare): (_, _, Continues) = match cond {
        ExpressionNode::Less(l, r) => (l, r, |a, b| a < b),
        ExpressionNode::LessEqual(l, r) => (l, r, |a, b| a <= b),
        ExpressionNode::More(l, r) => (l, r, |a, b| a > b),
        ExpressionNode::MoreEqual(l, r) => (l, r, |a, b| a >= b),
        ExpressionNode::NotEqual(l, r) => (l, r, |a, b| a != b),
        _ => return None,
    };
    if !is_variable(left, name) {
        return None;
    }
    Some((literal(right)?, compare))
}

/// Разбирает шаг: `k := k + литерал` либо `k := k - литерал`.
fn increment(step: &ExpressionNode, name: &str) -> Option<i128> {
    let ExpressionNode::Assign(target, value) = step else {
        return None;
    };
    if !is_variable(target, name) {
        return None;
    }
    match value.as_ref() {
        ExpressionNode::Add(l, r) if is_variable(l, name) => literal(r),
        ExpressionNode::Subtract(l, r) if is_variable(l, name) => Some(-literal(r)?),
        _ => None,
    }
}

/// Ссылается ли выражение на переменную цикла.
fn is_variable(expr: &ExpressionNode, name: &str) -> bool {
    match expr {
        ExpressionNode::Variable(var) => var.borrow().name() == name,
        ExpressionNode::Parenthesis(inner) => is_variable(inner, name),
        _ => false,
    }
}
