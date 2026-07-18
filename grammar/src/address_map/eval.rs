//! Вычислитель выражения адреса (фича 0042): свёртка в константу `(addr, bit)`.
//!
//! Тема самостоятельна и держит **инвариант единственной арифметики**:
//! [`apply_binary`] обслуживает **оба** матчера (сырой АСД у оператора `address`
//! и понижённый [`ExpressionNode`] у inline). `CLAUDE.md` предупреждает —
//! «разъехавшись, они дали бы **разный адрес для одного текста**».

use super::env::AddressEnv;
use crate::diagnostics::{Diagnostic, Location};
use crate::semantic::{ExpressionNode, ModelNode, VariableNode};
use std::cell::RefCell;
use std::rc::Rc;

/// Значение выражения адреса: число и необязательный бит (`0xADDR:bit`).
type AddrValue = (i64, Option<i64>);

/// `SE-054` — имя в выражении адреса не разрешается ни define'ом, ни `const`.
fn undefined_symbol(loc: Location, name: &str) -> Diagnostic {
    Diagnostic::error(
        loc,
        format!(
            "выражение адреса ссылается на неопределённый символ '{}' \
             (нет ни `--define {}=…`, ни `const {}` в модели)",
            name, name, name
        ),
    )
    .with_code("SE-054")
}

/// `SE-055` — выражение адреса не сворачивается в константу.
fn not_constant(loc: Location, reason: &str) -> Diagnostic {
    Diagnostic::error(
        loc,
        format!("выражение адреса не сворачивается в константу: {}", reason),
    )
    .with_code("SE-055")
}

/// Применяет бинарную операцию к вычисленным операндам.
///
/// Единственное место арифметики адреса — им пользуются **оба** матчера (по
/// [`ExpressionNode`] и по сырому АСД). Разъехавшись, они дали бы разный адрес
/// для одного и того же текста в зависимости от того, каким путём выражение
/// попало в разрешение (inline против оператора `address`).
fn apply_binary(
    op: &str,
    left: AddrValue,
    right: AddrValue,
    loc: Location,
) -> Result<AddrValue, Diagnostic> {
    // Бит — свойство записи адреса (`0x1000:3`), а не число: арифметика над ним
    // бессмысленна, и молча его терять нельзя.
    if left.1.is_some() || right.1.is_some() {
        return Err(not_constant(
            loc,
            "операнд формы `адрес:бит` не участвует в арифметике",
        ));
    }
    let (a, b) = (left.0, right.0);
    let value = match op {
        "+" => a.wrapping_add(b),
        "-" => a.wrapping_sub(b),
        "*" => a.wrapping_mul(b),
        "/" => {
            if b == 0 {
                return Err(not_constant(loc, "деление на ноль"));
            }
            a.wrapping_div(b)
        }
        "%" => {
            if b == 0 {
                return Err(not_constant(loc, "остаток от деления на ноль"));
            }
            a.wrapping_rem(b)
        }
        "<<" | ">>" => {
            // Сдвиг на отрицательное или ≥ 64 в Rust — паника; у адреса это
            // всегда опечатка, поэтому диагностика, а не молчание.
            if !(0..64).contains(&b) {
                return Err(not_constant(loc, "сдвиг допустим только на 0..63 бит"));
            }
            if op == "<<" { a << b } else { a >> b }
        }
        "&" => a & b,
        "|" => a | b,
        "^" => a ^ b,
        _ => {
            return Err(not_constant(
                loc,
                "операция не поддержана в выражении адреса",
            ));
        }
    };
    Ok((value, None))
}

/// Вычисляет **сырое АСД**-выражение адреса — путь оператора `address`.
///
/// Значение оператора хранится как `ExpressionNode::Unresolved(ast::Expression)`
/// и семантикой не понижается (`tree.rs`): ни один из 7 проходов к
/// `address_defs` не обращается. Поэтому здесь разбирается АСД напрямую.
fn eval_ast_addr(
    expr: &crate::parser::ast::Expression,
    scope: &Rc<RefCell<ModelNode>>,
    env: &AddressEnv,
    seen: &mut Vec<String>,
) -> Result<AddrValue, Diagnostic> {
    use crate::parser::ast::Expression as E;
    // Бинарная операция разворачивается явно: замыкание заимствовало бы `seen`
    // изменяемо на всё тело match.
    macro_rules! bin {
        ($op:literal, $l:expr, $r:expr, $loc:expr) => {{
            let left = eval_ast_addr($l, scope, env, seen)?;
            let right = eval_ast_addr($r, scope, env, seen)?;
            apply_binary($op, left, right, $loc)
        }};
    }
    match expr {
        E::Number(_, n) => Ok((*n, None)),
        E::Address(_, a, b) => Ok((*a, Some(*b))),
        E::Variable(id) => resolve_symbol(&id.name, id.loc, scope, env, seen),
        E::Parenthesis(_, inner) => eval_ast_addr(inner, scope, env, seen),
        E::UnaryPlus(_, inner) => eval_ast_addr(inner, scope, env, seen),
        E::Negate(loc, inner) => {
            let (v, bit) = eval_ast_addr(inner, scope, env, seen)?;
            unary_bitless(v.wrapping_neg(), bit, *loc)
        }
        E::BitwiseNot(loc, inner) => {
            let (v, bit) = eval_ast_addr(inner, scope, env, seen)?;
            unary_bitless(!v, bit, *loc)
        }
        E::Add(loc, l, r) => bin!("+", l, r, *loc),
        E::Subtract(loc, l, r) => bin!("-", l, r, *loc),
        E::Multiply(loc, l, r) => bin!("*", l, r, *loc),
        E::Divide(loc, l, r) => bin!("/", l, r, *loc),
        E::Modulo(loc, l, r) => bin!("%", l, r, *loc),
        E::ShiftLeft(loc, l, r) => bin!("<<", l, r, *loc),
        E::ShiftRight(loc, l, r) => bin!(">>", l, r, *loc),
        E::BitwiseAnd(loc, l, r) => bin!("&", l, r, *loc),
        E::BitwiseOr(loc, l, r) => bin!("|", l, r, *loc),
        E::BitwiseXor(loc, l, r) => bin!("^", l, r, *loc),
        other => Err(not_constant(
            other.loc(),
            "поддержаны только целочисленные литералы, символы и операции \
             `+ - * / % << >> & | ^ ~`",
        )),
    }
}

/// Унарная операция: бит в ней бессмыслен — см. [`apply_binary`].
fn unary_bitless(value: i64, bit: Option<i64>, loc: Location) -> Result<AddrValue, Diagnostic> {
    if bit.is_some() {
        return Err(not_constant(
            loc,
            "операнд формы `адрес:бит` не участвует в арифметике",
        ));
    }
    Ok((value, None))
}

/// Разрешает имя в выражении адреса: **сначала** define, **затем** `const` модели.
///
/// Порядок — решение D2 ADR 0042: платформенный слой главнее модели (прямая
/// симметрия с оверлеем внешней карты `SE-050`). Предупреждение о перекрытии
/// (`SE-053`) выдаёт вызывающий: здесь нет позиции объявления `const`.
fn resolve_symbol(
    name: &str,
    loc: Location,
    scope: &Rc<RefCell<ModelNode>>,
    env: &AddressEnv,
    seen: &mut Vec<String>,
) -> Result<AddrValue, Diagnostic> {
    if let Some(v) = env.lookup(name) {
        // Define побеждает `const` (решение D2), но перекрытие обязано быть
        // ЗАМЕТНЫМ — прямая симметрия с оверлеем внешней карты (`SE-050`):
        // платформенный слой главнее модели, однако молча её не подменяет.
        if let Some(VariableNode::Const { loc: const_loc, .. }) = scope.borrow().search_var(name) {
            env.note_override(name, const_loc);
        }
        return Ok(v);
    }
    // Цикл `const A := B; const B := A;` — иначе вычислитель зациклится.
    if seen.iter().any(|s| s == name) {
        return Err(not_constant(
            loc,
            &format!("циклическая ссылка через '{}'", name),
        ));
    }
    let Some(var) = scope.borrow().search_var(name) else {
        return Err(undefined_symbol(loc, name));
    };
    let VariableNode::Const { expr, .. } = &var else {
        // Переменная или порт: их значение известно только в рантайме.
        return Err(not_constant(
            loc,
            &format!(
                "'{}' — не константа (адрес обязан быть известен при сборке)",
                name
            ),
        ));
    };
    seen.push(name.to_string());
    let value = eval_addr_value(expr, scope, env, seen);
    seen.pop();
    value
}

/// Вычисляет выражение адреса, уже понижённое семантикой, — путь inline.
fn eval_addr_value(
    expr: &ExpressionNode,
    scope: &Rc<RefCell<ModelNode>>,
    env: &AddressEnv,
    seen: &mut Vec<String>,
) -> Result<AddrValue, Diagnostic> {
    macro_rules! bin {
        ($op:literal, $l:expr, $r:expr) => {{
            let left = eval_addr_value($l, scope, env, seen)?;
            let right = eval_addr_value($r, scope, env, seen)?;
            apply_binary($op, left, right, Location::Implicit)
        }};
    }
    match expr {
        ExpressionNode::Number(n) => Ok((*n, None)),
        ExpressionNode::Address(a, b) => Ok((*a, Some(*b))),
        // Сырой АСД: значение оператора `address` семантикой не понижается.
        ExpressionNode::Unresolved(ast_expr) => eval_ast_addr(ast_expr, scope, env, seen),
        // Имя, разрешённое семантикой в объявление (путь inline).
        ExpressionNode::Variable(var_rc) => {
            let borrowed = var_rc.borrow();
            let name = borrowed.name().to_string();
            let loc = borrowed.loc();
            drop(borrowed);
            resolve_symbol(&name, loc, scope, env, seen)
        }
        ExpressionNode::Parenthesis(inner) => eval_addr_value(inner, scope, env, seen),
        ExpressionNode::UnaryPlus(inner) => eval_addr_value(inner, scope, env, seen),
        ExpressionNode::Negate(inner) => {
            let (v, bit) = eval_addr_value(inner, scope, env, seen)?;
            unary_bitless(v.wrapping_neg(), bit, Location::Implicit)
        }
        ExpressionNode::BitwiseNot(inner) => {
            let (v, bit) = eval_addr_value(inner, scope, env, seen)?;
            unary_bitless(!v, bit, Location::Implicit)
        }
        ExpressionNode::Add(l, r) => bin!("+", l, r),
        ExpressionNode::Subtract(l, r) => bin!("-", l, r),
        ExpressionNode::Multiply(l, r) => bin!("*", l, r),
        ExpressionNode::Divide(l, r) => bin!("/", l, r),
        ExpressionNode::Modulo(l, r) => bin!("%", l, r),
        ExpressionNode::ShiftLeft(l, r) => bin!("<<", l, r),
        ExpressionNode::ShiftRight(l, r) => bin!(">>", l, r),
        ExpressionNode::BitwiseAnd(l, r) => bin!("&", l, r),
        ExpressionNode::BitwiseOr(l, r) => bin!("|", l, r),
        ExpressionNode::BitwiseXor(l, r) => bin!("^", l, r),
        _ => Err(not_constant(
            Location::Implicit,
            "поддержаны только целочисленные литералы, символы и операции \
             `+ - * / % << >> & | ^ ~`",
        )),
    }
}

/// Вычисляет выражение адреса в константу `(addr, bit)`.
///
/// # Чем отличается от прежнего понижения
///
/// Раньше здесь стояла `lower_addr_expr`, принимавшая **только** литералы:
///
/// ```text
/// _ => None,   // символ, арифметика, всё прочее — «адреса нет», без диагностики
/// ```
///
/// То есть `address BTN = BTN_ADDR;` и `address BTN = 0x100000 + 4;` **молча
/// теряли адрес**, а пользователь получал `SE-052` «порт не имеет адреса» —
/// диагностику, называющую следствие вместо причины. Тот же класс «тихого
/// пропуска», что дал восемь дефектов в фиче 0025.
///
/// Теперь неудача **всегда** названа: `SE-054` (неопределённый символ) либо
/// `SE-055` (не сворачивается в константу).
///
/// # Возврат
///
/// - `Ok(None)` — выражения адреса нет вовсе (порт без инициализатора и без
///   оператора `address`); полноту проверяет вызывающий (`SE-052`).
/// - `Ok(Some(v))` — вычислено.
/// - `Err(d)` — `SE-054`/`SE-055`.
pub(super) fn eval_addr_expr(
    expr: &ExpressionNode,
    scope: &Rc<RefCell<ModelNode>>,
    env: &AddressEnv,
) -> Result<Option<AddrValue>, Diagnostic> {
    if matches!(expr, ExpressionNode::None) {
        return Ok(None);
    }
    let mut seen = Vec::new();
    eval_addr_value(expr, scope, env, &mut seen).map(Some)
}
