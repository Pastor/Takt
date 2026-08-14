//! Q-арифметика fixed-point `q(m, n)` для цели C (фича 0061, задача 0061-03).
//!
//! Нормативные правила ADR 0061 — обязаны совпасть **побитово** с эталоном
//! симулятора ([`takt_sim::eval::fixed`]) и прочими целями:
//!
//! - `+`/`−` — сложение представлений, wraparound к `W = m + n` (сужение к
//!   типу хранения `int{W}_t` после каждой операции — как `wrap(_, W)` в сим);
//! - `*` — точное произведение, затем **floor к −∞** (правило 4);
//! - `/` — делимое ← `n` влево, целочисленное деление (усечение к нулю).
//!
//! ⚠️ **Ловушка C11 6.5.7p5** (правило 7 ADR): `>>` знакового отрицательного —
//! implementation-defined. Поэтому floor **не** эмитится сдвигом: используются
//! хелперы [`super::super::c_source`] на **floor-делении** (`/` и `%`
//! стандартно-определены для любого знака). В порождённом коде модели `>>` над
//! знаковым отсутствует вовсе — сторож T11.
//!
//! Тип операнда выводится общим слоем [`extract_type`]: `SE-059` гарантирует, что
//! оба операнда — один и тот же `q(m, n)`, поэтому формат берётся у любого.

use super::*;
use crate::semantic::type_inference::extract_type;
use crate::semantic::type_node::fixed_storage_bits;

/// Арифметическая операция над `q(m, n)`.
#[derive(Clone, Copy)]
pub(super) enum FixedOp {
    Add,
    Subtract,
    Multiply,
    Divide,
}

/// Формат `q(m, n)` выражения, если его тип статически выводится как `Fixed`.
///
/// Обёртка над [`extract_type`]: возвращает `(m, n)` только для `Fixed`. Модель
/// берётся у владельца — тот же приём, что в арме `Cast` печатника.
pub(super) fn fixed_of(map: &CMap, owner: &Element, expr: &ExpressionNode) -> Option<(u8, u8)> {
    let model = map.raw_model_at(owner.name()).ok()?;
    match extract_type(expr, model) {
        Ok(TypeNode::Fixed { m, n }) => Some((m, n)),
        _ => None,
    }
}

/// Тип хранения `q(m, n)` в C: `int{S}_t`, где `S` — округлённая вверх ширина.
fn storage_type(m: u8, n: u8) -> String {
    format!("int{}_t", fixed_storage_bits(m + n))
}

/// Истина, если ширина формата `W = m + n` **уже** равна ширине хранения.
///
/// ⚠️ Различие существенно (фикс 0061-01): правило 3 ADR 0061 требует переноса
/// к **W**, а приведение к типу хранения
/// сужает к `S = fixed_storage_bits(W)` — при `W = 12` это 16 бит, то есть
/// перенос происходит на другой границе, чем у эталона. Совпадают они только
/// когда `W ∈ {8, 16, 32, 64}` — а весь корпус ровно таков, поэтому расхождение
/// дожило от 0061 незамеченным.
fn width_is_storage(m: u8, n: u8) -> bool {
    m + n == fixed_storage_bits(m + n)
}

/// Открывает обёртку результата q-операции: приведение к типу хранения и, если
/// `W` уже ширины хранения, вызов `lam_q_wrap` для переноса к `W`.
///
/// ⚠️ Хелпер эмитится **только по нужде**: при `W = S` он был бы тождеством, а
/// вывод для всего корпуса изменился бы (снапшоты `examples/generated`).
fn open_wrap(printer: &mut Printer, m: u8, n: u8) {
    printer.print(&format!("({})(", storage_type(m, n)));
    if !width_is_storage(m, n) {
        printer.print("lam_q_wrap(");
    }
}

/// Закрывает обёртку, открытую [`open_wrap`].
fn close_wrap(printer: &mut Printer, m: u8, n: u8) {
    if !width_is_storage(m, n) {
        printer.print(&format!(", {})", m + n));
    }
    printer.print(")");
}

/// `2^n` как целочисленный литерал (для сдвигов через умножение/деление).
fn pow2(n: u8) -> u64 {
    1u64 << n
}

/// Печатает операнд, приведённый к `int64_t` для промежуточной арифметики.
fn widened(
    printer: &mut Printer,
    map: &CMap,
    owner: &Element,
    params: Vec<(String, TypeNode)>,
    expr: &ExpressionNode,
    has_model: bool,
) -> Result<(), Diagnostic> {
    printer.print("(int64_t)(");
    generate_expr(printer, map, owner, params, expr, 0, has_model)?;
    printer.print(")");
    Ok(())
}

/// Печатает бинарную q-операцию: результат сужается к `int{W}_t` (wraparound к W).
#[allow(clippy::too_many_arguments)]
pub(super) fn binary(
    printer: &mut Printer,
    map: &CMap,
    owner: &Element,
    params: Vec<(String, TypeNode)>,
    op: FixedOp,
    l: &ExpressionNode,
    r: &ExpressionNode,
    m: u8,
    n: u8,
    has_model: bool,
) -> Result<(), Diagnostic> {
    open_wrap(printer, m, n);
    match op {
        FixedOp::Add | FixedOp::Subtract => {
            let sym = if matches!(op, FixedOp::Add) {
                " + "
            } else {
                " - "
            };
            widened(printer, map, owner, params.clone(), l, has_model)?;
            printer.print(sym);
            widened(printer, map, owner, params, r, has_model)?;
        }
        FixedOp::Multiply => {
            printer.print("lam_q_mul(");
            widened(printer, map, owner, params.clone(), l, has_model)?;
            printer.print(", ");
            widened(printer, map, owner, params, r, has_model)?;
            printer.print(&format!(", {})", n));
        }
        FixedOp::Divide => {
            printer.print("lam_q_div(");
            widened(printer, map, owner, params.clone(), l, has_model)?;
            printer.print(", ");
            widened(printer, map, owner, params, r, has_model)?;
            printer.print(&format!(", {})", n));
        }
    }
    close_wrap(printer, m, n);
    Ok(())
}

/// Печатает унарный минус над `q(m, n)`: `−repr` с wraparound к W.
#[allow(clippy::too_many_arguments)]
pub(super) fn negate(
    printer: &mut Printer,
    map: &CMap,
    owner: &Element,
    params: Vec<(String, TypeNode)>,
    inner: &ExpressionNode,
    m: u8,
    n: u8,
    has_model: bool,
) -> Result<(), Diagnostic> {
    open_wrap(printer, m, n);
    printer.print("-");
    widened(printer, map, owner, params, inner, has_model)?;
    close_wrap(printer, m, n);
    Ok(())
}

/// Печатает приведение `expr as T`, когда источник **или** цель — `q(m, n)`.
///
/// Масштабирование (правило 6 ADR): `int`/`bool` ↔ `q` — умножение/floor-деление
/// на `2^n`, `float` ↔ `q` — деление/умножение на `2^n`, `q` ↔ `q` — пересчёт
/// разницы дробных разрядов. Сдвиги не используются (ловушка C11 и UB `<<`
/// отрицательного).
#[allow(clippy::too_many_arguments)]
pub(super) fn cast(
    printer: &mut Printer,
    map: &CMap,
    owner: &Element,
    params: Vec<(String, TypeNode)>,
    inner: &ExpressionNode,
    target: &TypeNode,
    target_c: &str,
    has_model: bool,
) -> Result<(), Diagnostic> {
    let src = fixed_of(map, owner, inner);
    match (src, target) {
        // q → q: пересчёт дробных разрядов (влево — умножение, вправо — floor).
        (Some((_, from_n)), TypeNode::Fixed { m: tm, n: tn }) => {
            open_wrap(printer, *tm, *tn);
            rescale(printer, map, owner, params, inner, from_n, *tn, has_model)?;
            close_wrap(printer, *tm, *tn);
        }
        // q → float: repr / 2^n (точно представимо в double).
        (Some((_, from_n)), TypeNode::Rational) => {
            printer.print("((double)(");
            generate_expr(printer, map, owner, params, inner, 0, has_model)?;
            printer.print(&format!(") / {}.0)", pow2(from_n)));
        }
        // q → целое/бит: floor(repr / 2^n) = целая часть.
        (Some((_, from_n)), _) => {
            printer.print(&format!("({})lam_q_floordiv(", target_c));
            widened(printer, map, owner, params, inner, has_model)?;
            printer.print(&format!(", (int64_t)1 << {})", from_n));
        }
        // float → q: floor(f * 2^n).
        // ⚠️ Форма печати здесь своя, не через `open_wrap`: при `W = S` вывод
        // обязан остаться прежним байт-в-байт — `(int16_t)floor(…)`, без лишней
        // скобки, иначе поедут снапшоты `examples/generated` (фикс 0061-01).
        (None, TypeNode::Fixed { m: tm, n: tn }) if source_is_real(map, owner, inner) => {
            let wraps = !width_is_storage(*tm, *tn);
            printer.print(&format!("({})", storage_type(*tm, *tn)));
            if wraps {
                printer.print("lam_q_wrap(");
            }
            printer.print("floor((");
            generate_expr(printer, map, owner, params, inner, 0, has_model)?;
            printer.print(&format!(") * {}.0)", pow2(*tn)));
            if wraps {
                printer.print(&format!(", {})", tm + tn));
            }
        }
        // целое/бит → q: (repr = v * 2^n) с wraparound к W.
        (None, TypeNode::Fixed { m: tm, n: tn }) => {
            open_wrap(printer, *tm, *tn);
            widened(printer, map, owner, params, inner, has_model)?;
            printer.print(&format!(" * ((int64_t)1 << {})", tn));
            close_wrap(printer, *tm, *tn);
        }
        // Ни источник, ни цель не q — сюда не попадаем (страж вызова).
        (None, _) => {
            printer.print(&format!("({})", target_c));
            generate_expr(printer, map, owner, params, inner, 13, has_model)?;
        }
    }
    Ok(())
}

/// Пересчёт представления `q` между дробными разрядностями (без сужения — его
/// делает вызывающий приведением к типу хранения цели).
#[allow(clippy::too_many_arguments)]
fn rescale(
    printer: &mut Printer,
    map: &CMap,
    owner: &Element,
    params: Vec<(String, TypeNode)>,
    inner: &ExpressionNode,
    from_n: u8,
    to_n: u8,
    has_model: bool,
) -> Result<(), Diagnostic> {
    if to_n >= from_n {
        widened(printer, map, owner, params, inner, has_model)?;
        printer.print(&format!(" * ((int64_t)1 << {})", to_n - from_n));
    } else {
        printer.print("lam_q_floordiv(");
        widened(printer, map, owner, params, inner, has_model)?;
        printer.print(&format!(", (int64_t)1 << {})", from_n - to_n));
    }
    Ok(())
}

/// Определение `lam_q_floordiv` — floor-деление (`/`/`%` стандартно-определены
/// при любом знаке), которым и `*`, и приведение `q → int` обходят ловушку C11
/// 6.5.7p5 (правило 7 ADR): `>>` знакового отрицательного не эмитится.
const LAM_Q_FLOORDIV: &str = "static int64_t lam_q_floordiv(int64_t x, int64_t d) {\n    \
    int64_t q = x / d;\n    return ((x % d != 0) && ((x < 0) != (d < 0))) ? q - 1 : q;\n}\n";
/// `lam_q_mul` — точное произведение, floor к −∞ (зовёт `lam_q_floordiv`).
const LAM_Q_MUL: &str = "static int64_t lam_q_mul(int64_t a, int64_t b, unsigned n) {\n    \
    return lam_q_floordiv(a * b, (int64_t)1 << n);\n}\n";
/// `lam_q_div` — делимое ← n влево, целочисленное деление (усечение к нулю).
const LAM_Q_DIV: &str = "static int64_t lam_q_div(int64_t a, int64_t b, unsigned n) {\n    \
    return (a * ((int64_t)1 << n)) / b;\n}\n";
/// `lam_q_wrap` — перенос к **W** битам (правило 3 ADR 0061), а не к ширине
/// хранения. Считается в **беззнаковом**: сужение знакового вне диапазона
/// implementation-defined, а `uint64_t` определён стандартом при любом значении.
const LAM_Q_WRAP: &str = "static int64_t lam_q_wrap(int64_t v, unsigned w) {\n    \
    uint64_t mask = (w >= 64) ? ~(uint64_t)0 : (((uint64_t)1 << w) - 1);\n    \
    uint64_t bits = (uint64_t)v & mask;\n    \
    uint64_t sign = (uint64_t)1 << (w - 1);\n    \
    return (int64_t)((bits & sign) ? (bits | ~mask) : bits);\n}\n";

/// Вставляет определения Q-хелперов (0061), фактически вызванных в `source`,
/// сразу после `#include`. Эмитятся ровно нужные (без `-Wunused-function`);
/// корпус без `q` остаётся байт-в-байт прежним (T14). `lam_q_mul` тянет
/// `lam_q_floordiv`; порядок определений — floordiv → mul → div (C требует
/// объявления до использования).
pub(in crate::generator::c) fn insert_fixed_helpers(source: String) -> String {
    let uses_mul = source.contains("lam_q_mul(");
    let uses_div = source.contains("lam_q_div(");
    let uses_floordiv = uses_mul || source.contains("lam_q_floordiv(");
    let uses_wrap = source.contains("lam_q_wrap(");
    if !uses_floordiv && !uses_div && !uses_wrap {
        return source;
    }
    let mut helpers = String::new();
    if uses_wrap {
        helpers.push_str(LAM_Q_WRAP);
    }
    if uses_floordiv {
        helpers.push_str(LAM_Q_FLOORDIV);
    }
    if uses_mul {
        helpers.push_str(LAM_Q_MUL);
    }
    if uses_div {
        helpers.push_str(LAM_Q_DIV);
    }
    const ANCHOR: &str = "#include <math.h>\n";
    match source.find(ANCHOR) {
        Some(i) => {
            let mut s = source;
            s.insert_str(i + ANCHOR.len(), &helpers);
            s
        }
        None => format!("{helpers}{source}"),
    }
}

/// Истина, если тип источника приведения — вещественный (`float`).
fn source_is_real(map: &CMap, owner: &Element, expr: &ExpressionNode) -> bool {
    matches!(
        map.raw_model_at(owner.name())
            .ok()
            .and_then(|model| extract_type(expr, model).ok()),
        Some(TypeNode::Rational)
    )
}
