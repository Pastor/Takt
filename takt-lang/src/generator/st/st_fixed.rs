//! Q-арифметика fixed-point `q(m, n)` для цели ST (фича 0061, задача 0061-03).
//!
//! ⚠️ **В IEC 61131-3 сдвигов над числами нет** (`SHL`/`SHR` определены только
//! на битовых строках, арифметика над ними запрещена, `<<` не существует —
//! `CLAUDE.md`). Поэтому floor к −∞ у `*` и приведения `q → int` выражаются
//! **floor-делением** через эмитируемую `FUNCTION LAM_Q_FLOORDIV` (её MatIEC
//! принимает — проба 2026-07-19). Промежуток арифметики — `LINT` (64 бита):
//! операнды приводятся `{S}_TO_LINT`, результат сужается `LINT_TO_{S}`
//! (усечение битов = wraparound к W).
//!
//! ⚠️ Ограничение: точное произведение шириной `2W` обязано влезть в `LINT`,
//! т. е. `W ≤ 32`. Для `q(32, 32)` (`W = 64`) `*`/`/` дают честную `ST-013`, а
//! не молчаливое переполнение.
//!
//! Нормативные правила совпадают с эталоном симулятора (`eval::fixed`) и целями
//! `c`/`rust` — сверка идёт побитово через вещественный порт (`… as float`).

use crate::diagnostics::{Diagnostic, Location};
use crate::generator::st::st_expr::{inner_expr_type, print_expression};
use crate::semantic::type_node::{TypeNode, fixed_storage_bits};
use crate::semantic::{ExpressionNode, ModelNode};

/// Определение `FUNCTION LAM_Q_FLOORDIV` — floor-деление целых `LINT`.
///
/// `MOD` и сравнение `BOOL <> BOOL` (XOR знаков) MatIEC принимает. Обход ловушки
/// C11 в ST не нужен (сдвигов нет вовсе), но floor к −∞ у `/` в IEC отсутствует
/// (деление усекает к нулю), поэтому floor строится явно.
pub(crate) const LAM_Q_FLOORDIV: &str = "\
FUNCTION LAM_Q_FLOORDIV : LINT
VAR_INPUT
    x : LINT;
    d : LINT;
END_VAR
VAR
    q : LINT;
END_VAR
    q := x / d;
    IF (x MOD d <> 0) AND ((x < 0) <> (d < 0)) THEN
        LAM_Q_FLOORDIV := q - 1;
    ELSE
        LAM_Q_FLOORDIV := q;
    END_IF;
END_FUNCTION

";

/// Определение `FUNCTION LAM_Q_WRAP` — перенос к **W** битам (правило 3 ADR 0061).
///
/// ⚠️ Сужение `LINT_TO_{S}` переносит к ширине **хранения**, а не к `W = m + n`:
/// при `W = 12` это 16 бит против 12 — другая граница (фикс 0061-01). Совпадают
/// они лишь при `W ∈ {8, 16, 32, 64}`, каков весь корпус, — поэтому расхождение
/// с эталоном дожило от 0061 незамеченным.
///
/// ⚠️ Модуль `2^W` передаётся **аргументом**, а не считается: сдвигов над числами
/// в IEC нет вовсе (ловушка A-4 ADR 0061), а `SHL` определён лишь над битовыми
/// строками. `MOD` в IEC даёт остаток со знаком делимого — отсюда две поправки.
pub(crate) const LAM_Q_WRAP: &str = "\
FUNCTION LAM_Q_WRAP : LINT
VAR_INPUT
    x : LINT;
    m : LINT;
END_VAR
VAR
    r : LINT;
END_VAR
    r := x MOD m;
    IF r < 0 THEN
        r := r + m;
    END_IF;
    IF r >= m / 2 THEN
        r := r - m;
    END_IF;
    LAM_Q_WRAP := r;
END_FUNCTION

";

/// Истина, если `W = m + n` уже равна ширине хранения (перенос не нужен).
fn width_is_storage(m: u8, n: u8) -> bool {
    m + n == fixed_storage_bits(m + n)
}

/// Оборачивает выражение-`LINT` переносом к `W`, если `W` ≠ ширины хранения.
///
/// ⚠️ При `W = S` возвращает выражение **как есть**: вывод для корпуса обязан
/// остаться байт-в-байт прежним.
fn wrap_lint(expr: String, m: u8, n: u8) -> Result<String, Diagnostic> {
    if width_is_storage(m, n) {
        return Ok(expr);
    }
    let w = m + n;
    // 2^W обязан быть представим в LINT (знаковое 64): при W = 63 модуль равен
    // 2^63 и в LINT не влезает. Отказ называет причину — молча считать по
    // неверному модулю значило бы дать иной результат, чем у эталона.
    if w >= 63 {
        return Err(Diagnostic::error(
            Location::Codegen,
            format!(
                "перенос к {w} битам в цели st требует модуля 2^{w}, непредставимого \
                 в LINT (знаковое 64 бита); выберите q(m, n) с m + n ≤ 62 либо \
                 ширину, кратную 8"
            ),
        )
        .with_code("ST-021"));
    }
    Ok(format!("LAM_Q_WRAP({expr}, {})", 1u64 << w))
}

/// Арифметическая операция над `q(m, n)`.
#[derive(Clone, Copy)]
pub(crate) enum FixedOp {
    Add,
    Subtract,
    Multiply,
    Divide,
}

/// Формат `q(m, n)` выражения, если его тип — `Fixed` (рекурсивно по арифметике;
/// `SE-059` гарантирует единый формат операндов).
pub(crate) fn fixed_format(expr: &ExpressionNode) -> Option<(u8, u8)> {
    if let Some(TypeNode::Fixed { m, n }) = inner_expr_type(expr) {
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

/// Знаковый целый тип IEC для разрядности хранения (`m ≥ 1` → всегда знаковый).
fn iec_signed(bits: u8) -> &'static str {
    match bits {
        0..=8 => "SINT",
        9..=16 => "INT",
        17..=32 => "DINT",
        _ => "LINT",
    }
}

/// Целый тип IEC произвольной знаковости (для приведений `q ↔ int`).
fn iec_int(bits: u8, signed: bool) -> &'static str {
    match (bits, signed) {
        (8, false) => "USINT",
        (16, false) => "UINT",
        (32, false) => "UDINT",
        (64, false) => "ULINT",
        (8, true) => "SINT",
        (16, true) => "INT",
        (32, true) => "DINT",
        _ => "LINT",
    }
}

/// `LINT`-представление операнда `q` (тип хранения `S`): `{S}_TO_LINT(expr)`.
fn to_lint(printed: &str, s: &str) -> String {
    format!("{s}_TO_LINT({printed})")
}

/// Печатает бинарную q-операцию. Результат сужается `LINT_TO_{S}` (wraparound).
pub(crate) fn binary(
    op: FixedOp,
    a: &ExpressionNode,
    b: &ExpressionNode,
    model: &ModelNode,
    m: u8,
    n: u8,
) -> Result<String, Diagnostic> {
    let bits = fixed_storage_bits(m + n);
    let s = iec_signed(bits);
    let (la, lb) = (print_expression(a, model)?, print_expression(b, model)?);
    let (la, lb) = (to_lint(&la, s), to_lint(&lb, s));
    let pow = 1u64 << n;
    let inner = match op {
        FixedOp::Add => format!("{la} + {lb}"),
        FixedOp::Subtract => format!("{la} - {lb}"),
        FixedOp::Multiply | FixedOp::Divide if bits == 64 => return Err(too_wide(m, n)),
        // Точное произведение 2W → floor к −∞ (LAM_Q_FLOORDIV, правило 4).
        FixedOp::Multiply => format!("LAM_Q_FLOORDIV({la} * {lb}, {pow})"),
        // Делимое ← n влево (умножением), деление IEC усекает к нулю (как сим).
        FixedOp::Divide => format!("({la} * {pow}) / {lb}"),
    };
    Ok(format!("LINT_TO_{s}({})", wrap_lint(inner, m, n)?))
}

/// Печатает унарный минус над `q(m, n)`: `−repr` с wraparound к W.
pub(crate) fn negate(
    inner: &ExpressionNode,
    model: &ModelNode,
    m: u8,
    n: u8,
) -> Result<String, Diagnostic> {
    let s = iec_signed(fixed_storage_bits(m + n));
    let li = to_lint(&print_expression(inner, model)?, s);
    Ok(format!(
        "LINT_TO_{s}({})",
        wrap_lint(format!("-{li}"), m, n)?
    ))
}

/// Печатает приведение `expr as T`, когда источник **или** цель — `q(m, n)`.
pub(crate) fn cast(
    inner: &ExpressionNode,
    target: &TypeNode,
    model: &ModelNode,
) -> Result<String, Diagnostic> {
    let src = fixed_format(inner);
    let printed = print_expression(inner, model)?;
    match (src, target) {
        // q → q: пересчёт дробных разрядов.
        (Some((_, from_n)), TypeNode::Fixed { m: tm, n: tn }) => {
            let li = to_lint(&printed, iec_signed(storage_of(inner)));
            rescale(&li, from_n, *tn, *tm)
        }
        // q → float: repr / 2^n (точно представимо в LREAL).
        (Some((_, from_n)), TypeNode::Rational) => {
            let s = iec_signed(storage_of(inner));
            Ok(format!("({s}_TO_LREAL({printed}) / {}.0)", 1u64 << from_n))
        }
        // q → целое/бит: floor(repr / 2^n) = целая часть.
        (Some((_, from_n)), _) => {
            let s = iec_signed(storage_of(inner));
            let tgt = int_name_of_target(target)?;
            let li = to_lint(&printed, s);
            Ok(format!(
                "LINT_TO_{tgt}(LAM_Q_FLOORDIV({li}, {}))",
                1u64 << from_n
            ))
        }
        // float → q: floor(f · 2^n) — LREAL_TO_INT в IEC ОКРУГЛЯЕТ, не floor.
        (None, TypeNode::Fixed { .. })
            if matches!(inner_expr_type(inner), Some(TypeNode::Rational)) =>
        {
            Err(Diagnostic::error(
                Location::Codegen,
                "приведение float → q в цели st: LREAL_TO_INT округляет к ближайшему, \
                 а q требует floor; литеральный float понижается на этапе компиляции"
                    .to_string(),
            )
            .with_code("ST-014"))
        }
        // целое/бит → q: repr = v · 2^n с wraparound к W.
        (None, TypeNode::Fixed { m: tm, n: tn }) => {
            let ts = iec_signed(fixed_storage_bits(tm + tn));
            let src_ty = inner_expr_type(inner).ok_or_else(untyped_source)?;
            let src_name = match src_ty {
                TypeNode::Integer { bits, signed } => iec_int(bits, signed),
                TypeNode::Bit | TypeNode::Bool => "BOOL",
                _ => return Err(untyped_source()),
            };
            let li = if src_name == "BOOL" {
                format!("BOOL_TO_LINT({printed})")
            } else {
                format!("{src_name}_TO_LINT({printed})")
            };
            Ok(format!(
                "LINT_TO_{ts}({})",
                wrap_lint(format!("{li} * {}", 1u64 << tn), *tm, *tn)?
            ))
        }
        (None, _) => Ok(printed),
    }
}

/// Тип хранения (в битах) выражения-`q` — по его выведенному формату.
fn storage_of(expr: &ExpressionNode) -> u8 {
    match fixed_format(expr) {
        Some((m, n)) => fixed_storage_bits(m + n),
        None => 64,
    }
}

/// Пересчёт представления `q` между дробными разрядностями с сужением к `S2`.
fn rescale(li: &str, from_n: u8, to_n: u8, to_m: u8) -> Result<String, Diagnostic> {
    let s2 = iec_signed(fixed_storage_bits(to_m + to_n));
    let inner = if to_n >= from_n {
        format!("{li} * {}", 1u64 << (to_n - from_n))
    } else {
        format!("LAM_Q_FLOORDIV({li}, {})", 1u64 << (from_n - to_n))
    };
    Ok(format!("LINT_TO_{s2}({})", wrap_lint(inner, to_m, to_n)?))
}

/// Имя целого IEC-типа цели приведения `q → int`.
fn int_name_of_target(target: &TypeNode) -> Result<&'static str, Diagnostic> {
    match target {
        TypeNode::Integer { bits, signed } => Ok(iec_int(*bits, *signed)),
        TypeNode::Bit | TypeNode::Bool => Ok("BOOL"),
        _ => Err(Diagnostic::error(
            Location::Codegen,
            "приведение q → нецелого типа не поддержано".to_string(),
        )
        .with_code("ST-011")),
    }
}

/// `ST-013` — `q` шире 32 бит: промежуток `2W` не влезает в `LINT`.
fn too_wide(m: u8, n: u8) -> Diagnostic {
    Diagnostic::error(
        Location::Codegen,
        format!(
            "q({m}, {n}): W = {} > 32 — точное произведение шириной 2W не влезает в LINT",
            m + n
        ),
    )
    .with_code("ST-013")
}

/// `ST-011` — тип источника приведения в `q` не выводится.
fn untyped_source() -> Diagnostic {
    Diagnostic::error(
        Location::Codegen,
        "приведение в q: тип источника не выводится статически".to_string(),
    )
    .with_code("ST-011")
}

/// Вставляет `FUNCTION LAM_Q_FLOORDIV` перед первым POU, если она вызвана в
/// `program`. Эмитится по факту вызова (без лишней POU); корпус без `q`
/// неизменен (T14). Опережающие ссылки в ST — расширение `iec2c -p`, которым
/// цель уже пользуется, поэтому позиция «перед первым FUNCTION_BLOCK» безопасна.
pub(crate) fn insert_helper(program: String) -> String {
    let mut helpers = String::new();
    // Порядок значим: `LAM_Q_WRAP` зовёт только себя, `LAM_Q_FLOORDIV` — тоже,
    // но объявление обязано стоять до использования, а вставляются они разом
    // перед первым POU.
    if program.contains("LAM_Q_WRAP(") {
        helpers.push_str(LAM_Q_WRAP);
    }
    if program.contains("LAM_Q_FLOORDIV(") {
        helpers.push_str(LAM_Q_FLOORDIV);
    }
    if helpers.is_empty() {
        return program;
    }
    match program.find("FUNCTION_BLOCK") {
        Some(i) => {
            let mut s = program;
            s.insert_str(i, &helpers);
            s
        }
        None => format!("{helpers}{program}"),
    }
}
