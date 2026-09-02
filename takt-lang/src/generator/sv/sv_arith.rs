//! Арифметика в ширине приёмника — цель `sv` (фича 0360).
//!
//! # Что было
//!
//! `r := a + b;` при `a, b: u8` и `r: u16` давало verilator **`WIDTHEXPAND`**
//! («Operator ADD expects 16 bits on the LHS, but LHS's VARREF generates 8
//! bits»), а гейт цели считает предупреждение ошибкой. Смешанная знаковость
//! (`i8` + `u8`) — тот же класс. Эталон и цель `c` (продвижение до `int`) обе
//! записи считают верно.
//!
//! # Почему расширяются операнды
//!
//! Сложение в восьми битах обернулось бы по модулю **до** расширения: 200 + 100
//! дало бы 44 вместо 300. Обёртка по правилу 0127 остаётся — но на ширине
//! приёмника, как у эталона.

use crate::diagnostics::Diagnostic;
use crate::semantic::ExpressionNode;
use crate::semantic::type_node::TypeNode;

use super::sv_expr::print_expression;
use super::sv_scope::Scope;

/// Печатает арифметику с операндами в ширине приёмника.
///
/// `None` — печать прежняя: правило узкое, оба операнда обязаны быть
/// **именованными значениями** целого типа, и хотя бы один — иного типа, чем
/// приёмник. Литерал сюда не входит: он подстраивается под контекст сам.
pub(super) fn in_target(
    value: &ExpressionNode,
    target: &TypeNode,
    scope: &Scope,
) -> Result<Option<String>, Diagnostic> {
    let (op, l, r) = match value {
        ExpressionNode::Add(l, r) => ("+", l, r),
        ExpressionNode::Subtract(l, r) => ("-", l, r),
        ExpressionNode::Multiply(l, r) => ("*", l, r),
        ExpressionNode::Divide(l, r) => ("/", l, r),
        ExpressionNode::Modulo(l, r) => ("%", l, r),
        _ => return Ok(None),
    };
    let TypeNode::Integer { bits, signed } = target else {
        return Ok(None);
    };
    let (Some(lt), Some(rt)) = (
        crate::generator::mixed_sign::operand_type_expr(l),
        crate::generator::mixed_sign::operand_type_expr(r),
    ) else {
        return Ok(None);
    };
    if !matches!(lt, TypeNode::Integer { .. }) || !matches!(rt, TypeNode::Integer { .. }) {
        return Ok(None);
    }
    if lt == *target && rt == *target {
        return Ok(None);
    }
    let width = u32::from(*bits);
    // Знаковая цель обязана расширяться СО ЗНАКОМ, иначе отрицательное станет
    // большим положительным (урок 0323).
    let cast = |node: &ExpressionNode, ty: &TypeNode| -> Result<String, Diagnostic> {
        let text = print_expression(node, scope)?;
        let signed_operand = matches!(ty, TypeNode::Integer { signed: true, .. });
        Ok(if *signed {
            if signed_operand {
                format!("{width}'($signed({text}))")
            } else {
                format!("$signed({width}'({text}))")
            }
        } else {
            format!("{width}'({text})")
        })
    };
    Ok(Some(format!("({} {op} {})", cast(l, &lt)?, cast(r, &rt)?)))
}

/// Печатает именованное значение в ширине приёмника (фича 0360).
///
/// `None` — печать прежняя: значение уже нужного типа либо тип не выводится.
pub(super) fn value_in_target(
    value: &ExpressionNode,
    target: &TypeNode,
    scope: &Scope,
) -> Result<Option<String>, Diagnostic> {
    // Явное приведение автора приёмника НЕ отменяет (фича 0495): `probe :=
    // wide as u32;` при `out probe: u8` давало `32'(wide)` в 8-битном
    // приёмнике, и verilator отвечал `WIDTHTRUNC` — а гейт цели считает
    // предупреждение ошибкой. Эталон такую запись исполняет: приведение автора
    // даёт промежуточное значение, присваивание усекает его по приёмнику.
    if !matches!(
        value,
        ExpressionNode::Variable(_) | ExpressionNode::Cast(_, _)
    ) {
        return Ok(None);
    }
    let TypeNode::Integer { bits, signed } = target else {
        return Ok(None);
    };
    // Тип значения: у приведения — названный автором, у имени — объявленный.
    let from = match value {
        ExpressionNode::Cast(_, cast_ty) => cast_ty.clone(),
        _ => match crate::generator::mixed_sign::operand_type_expr(value) {
            Some(ty) => ty,
            None => return Ok(None),
        },
    };
    if !matches!(from, TypeNode::Integer { .. }) || from == *target {
        return Ok(None);
    }
    let width = u32::from(*bits);
    let text = print_expression(value, scope)?;
    let from_signed = matches!(from, TypeNode::Integer { signed: true, .. });
    Ok(Some(if *signed {
        if from_signed {
            format!("{width}'($signed({text}))")
        } else {
            format!("$signed({width}'({text}))")
        }
    } else {
        format!("{width}'({text})")
    }))
}
