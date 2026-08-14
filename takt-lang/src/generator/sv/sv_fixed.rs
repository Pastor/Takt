//! Q-арифметика fixed-point `q(m, n)` для цели SV (фича 0061, задача 0061-04).
//!
//! Ради этой цели фича и заведена: в синтезируемом RTL плавающей точки нет
//! (`SV-003`), а fixed-point даёт дробную арифметику в аппаратуре.
//!
//! Ширина в SV **не округляется** (в отличие от `c`/`rust`/`st`): `q(m, n)` —
//! `logic signed [W-1:0]`, `W = m + n`. Нормативные правила ADR 0061 совпадают
//! с эталоном симулятора (`eval::fixed`) и целями `c`/`rust`/`st` **побитово**:
//!
//! - `+`/`−` — сложение представлений, wraparound к `W` (size-cast `W'(…)`);
//! - `*` — точное произведение шириной `2W`, затем **арифметический** сдвиг
//!   `>>> n` (в SV `>>>` знакового определён — floor к −∞, правило 4);
//! - `/` — делимое `<<< n` в `2W`, затем знаковое деление (усечение к нулю).
//!   ⚠️ Синтезируется в аппаратный делитель (предупреждение — объём 0064).
//!
//! Знаковость восстанавливается на каждом уровне `$signed(…)`: операнды —
//! `logic signed`, но подвыражения-строки теряют её при вложении.

use crate::diagnostics::{Diagnostic, Location};
use crate::generator::sv::sv_expr::{Scope, print_expression, sv002};
use crate::semantic::type_node::TypeNode;
use crate::semantic::{ExpressionNode, VariableNode};

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
    match expr {
        ExpressionNode::Variable(var) => var_fixed(&var.borrow()),
        ExpressionNode::Cast(_, TypeNode::Fixed { m, n, .. }) => Some((*m, *n)),
        ExpressionNode::Parenthesis(a) | ExpressionNode::Negate(a) => fixed_format(a),
        ExpressionNode::Add(a, b)
        | ExpressionNode::Subtract(a, b)
        | ExpressionNode::Multiply(a, b)
        | ExpressionNode::Divide(a, b) => fixed_format(a).or_else(|| fixed_format(b)),
        _ => None,
    }
}

/// Формат `q(m, n)` переменной, если её тип — `Fixed`.
fn var_fixed(var: &VariableNode) -> Option<(u8, u8)> {
    match var.ty() {
        TypeNode::Fixed { m, n, .. } => Some((*m, *n)),
        _ => None,
    }
}

/// Знаковое W-битное значение операнда: `$signed(<printed>)`.
fn signed(printed: &str) -> String {
    format!("$signed({printed})")
}

/// Печатает бинарную q-операцию. Результат — `W'(…)` (wraparound к W).
pub(crate) fn binary(
    op: FixedOp,
    l: &ExpressionNode,
    r: &ExpressionNode,
    scope: &Scope,
    m: u8,
    n: u8,
) -> Result<String, Diagnostic> {
    let w = (m + n) as u32;
    let w2 = 2 * w;
    let (la, lb) = (print_expression(l, scope)?, print_expression(r, scope)?);
    match op {
        FixedOp::Add => Ok(format!("({w}'({} + {}))", signed(&la), signed(&lb))),
        FixedOp::Subtract => Ok(format!("({w}'({} - {}))", signed(&la), signed(&lb))),
        // Точное произведение 2W → floor к −∞ арифметическим `>>>` (правило 4).
        FixedOp::Multiply => Ok(format!(
            "({w}'((({w2}'({}) * {w2}'({})) >>> {n})))",
            signed(&la),
            signed(&lb)
        )),
        // Делимое ← n влево в 2W, знаковое деление (усечение к нулю, как сим).
        FixedOp::Divide => Ok(format!(
            "({w}'((({w2}'({}) <<< {n}) / {w2}'({}))))",
            signed(&la),
            signed(&lb)
        )),
    }
}

/// Печатает унарный минус над `q(m, n)`: `−repr` с wraparound к W.
pub(crate) fn negate(
    inner: &ExpressionNode,
    scope: &Scope,
    m: u8,
    n: u8,
) -> Result<String, Diagnostic> {
    let w = (m + n) as u32;
    Ok(format!(
        "({w}'(-{}))",
        signed(&print_expression(inner, scope)?)
    ))
}

/// Печатает приведение `expr as T`, когда источник **или** цель — `q(m, n)`.
///
/// `q ↔ int` масштабируют сдвигом на `2ⁿ` (в SV `>>>`/`<<<` знакового
/// определены). `q ↔ float` невозможно: в синтезируемом RTL плавающей точки нет
/// → `SV-003` (правило 8 ADR: `float` в цели `sv` остаётся запрещён).
pub(crate) fn cast(
    inner: &ExpressionNode,
    target: &TypeNode,
    scope: &Scope,
) -> Result<String, Diagnostic> {
    let src = fixed_format(inner);
    let printed = print_expression(inner, scope)?;
    match (src, target) {
        // q → q: пересчёт дробных разрядов (влево — сдвиг, вправо — floor `>>>`).
        (Some((_, from_n)), TypeNode::Fixed { m: tm, n: tn, .. }) => {
            let tw = (tm + tn) as u32;
            if tn >= &from_n {
                Ok(format!("({tw}'({} <<< {}))", signed(&printed), tn - from_n))
            } else {
                Ok(format!("({tw}'({} >>> {}))", signed(&printed), from_n - tn))
            }
        }
        // q ↔ float — недопустимо в синтезируемом RTL.
        (Some(_), TypeNode::Rational) => Err(sv003_cast()),
        // q → целое/бит: floor(repr / 2ⁿ) арифметическим сдвигом.
        (Some((_, from_n)), _) => {
            let bits = int_bits(target)?;
            Ok(format!("({bits}'({} >>> {from_n}))", signed(&printed)))
        }
        // float → q — источника float в синтезируемом RTL нет.
        (None, TypeNode::Fixed { .. }) if is_rational(inner) => Err(sv003_cast()),
        // целое/бит → q: repr = v · 2ⁿ с wraparound к W.
        (None, TypeNode::Fixed { m: tm, n: tn, .. }) => {
            let w = (tm + tn) as u32;
            Ok(format!("({w}'({} <<< {tn}))", signed(&printed)))
        }
        // Ни источник, ни цель не q — вызывающий не должен был звать сюда.
        (None, _) => Err(sv002("приведение типа (`as`)")),
    }
}

/// Разрядность целого/битового целевого типа приведения.
fn int_bits(target: &TypeNode) -> Result<u32, Diagnostic> {
    match target {
        TypeNode::Integer { bits, .. } => Ok(*bits as u32),
        TypeNode::Bit | TypeNode::Bool => Ok(1),
        _ => Err(sv002("приведение q → нецелого типа")),
    }
}

/// Истина, если тип источника приведения — вещественный (`float`).
fn is_rational(expr: &ExpressionNode) -> bool {
    match expr {
        ExpressionNode::Rational(_, _) => true,
        ExpressionNode::Variable(var) => matches!(var.borrow().ty(), TypeNode::Rational),
        ExpressionNode::Parenthesis(a) => is_rational(a),
        _ => false,
    }
}

/// `SV-003` — приведение между `q` и `float`: плавающей точки в RTL нет.
fn sv003_cast() -> Diagnostic {
    Diagnostic::error(
        Location::Codegen,
        "приведение между q(m, n) и float в цели 'sv': в синтезируемом RTL \
         плавающей точки нет (SV-003 для float в силе)"
            .to_string(),
    )
    .with_code("SV-003")
}
