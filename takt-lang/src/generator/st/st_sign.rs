//! Сравнение операндов РАЗНОЙ знаковости — цель `st` (фича 0359).
//!
//! Прежде печаталось как есть, и `iec2c` отвергал вывод: «Data type mismatch
//! for '<' expression» — при **нулевом** коде возврата `taktc`. Формы выбраны
//! **пробой** инструмента: преобразование `X_TO_Y` к общему типу и раскрытие
//! проверкой знака там, где общего типа нет (`u64` против знакового).
//!
//! Отдельным модулем, потому что `st_expr.rs` упирается в лимит размера
//! (правило `docs/CODE.md`): новое выносится, а не дописывается.
//!
//! ⚠️ Печатников ДВА — условий и выражений: условие ребра приходит
//! `ConditionNode`, условие `if` в теле — `ExpressionNode`. Правка одного
//! чинит половину входов.

use super::st_expr::{binary, binary_cond, wrap_cond, wrap_expr};
use crate::diagnostics::Diagnostic;
use crate::semantic::type_node::TypeNode;
use crate::semantic::{ConditionNode, ExpressionNode, ModelNode};

/// Сравнение операндов РАЗНОЙ знаковости (фича 0359).
///
/// Прежде печаталось как есть, и `iec2c` отвергал вывод: «Data type mismatch
/// for '<' expression» — при нулевом коде возврата `taktc`. Формы выбраны
/// **пробой** инструмента: преобразование `X_TO_Y` к общему типу и раскрытие
/// проверкой знака там, где общего типа нет.
pub(super) fn compare_cond(
    a: &ConditionNode,
    op: &str,
    b: &ConditionNode,
    model: &ModelNode,
) -> Result<String, Diagnostic> {
    let (ta, tb) = (
        crate::generator::mixed_sign::operand_type_cond(a),
        crate::generator::mixed_sign::operand_type_cond(b),
    );
    match crate::generator::mixed_sign::plan(ta.as_ref(), tb.as_ref()) {
        crate::generator::mixed_sign::Plan::AsIs => binary_cond(a, op, b, model),
        crate::generator::mixed_sign::Plan::Widen { bits } => {
            let Some(target) = crate::generator::st::st_type::iec_integer_name(bits as u8, true)
            else {
                return binary_cond(a, op, b, model);
            };
            let conv =
                |node: &ConditionNode, ty: &Option<TypeNode>| -> Result<String, Diagnostic> {
                    let text = wrap_cond(node, model)?;
                    let Some(TypeNode::Integer { bits, signed }) = ty else {
                        return Ok(text);
                    };
                    match crate::generator::st::st_type::iec_integer_name(*bits, *signed) {
                        Some(from) if from != target => Ok(format!("{from}_TO_{target}({text})")),
                        _ => Ok(text),
                    }
                };
            Ok(format!("{} {op} {}", conv(a, &ta)?, conv(b, &tb)?))
        }
        crate::generator::mixed_sign::Plan::SignGuard { signed_is_left } => {
            let (lt, rt) = (wrap_cond(a, model)?, wrap_cond(b, model)?);
            let (signed, unsigned) = if signed_is_left {
                (lt.as_str(), rt.as_str())
            } else {
                (rt.as_str(), lt.as_str())
            };
            // Операнд печатается дважды — в условии Takt эффектов не бывает
            // (присваивание есть оператор, 0187).
            let neg = format!("({signed} < 0)");
            let same = if signed_is_left {
                format!("(LINT_TO_ULINT({signed}) {op} {unsigned})")
            } else {
                format!("({unsigned} {op} LINT_TO_ULINT({signed}))")
            };
            let negative_wins = crate::generator::mixed_sign::negative_wins(op, signed_is_left);
            Ok(if negative_wins {
                format!("({neg} OR {same})")
            } else {
                format!("(NOT {neg} AND {same})")
            })
        }
    }
}
/// Сравнение операндов разной знаковости в ВЫРАЖЕНИИ (фича 0359).
///
/// Правило одно с печатником условий (`compare_cond`); здесь — путь тела, где
/// условие `if` приходит выражением. Прежде `iec2c` отвергал вывод.
pub(super) fn expr_compare(
    a: &ExpressionNode,
    op: &str,
    b: &ExpressionNode,
    model: &ModelNode,
) -> Result<String, Diagnostic> {
    let (ta, tb) = (
        crate::generator::mixed_sign::operand_type_expr(a),
        crate::generator::mixed_sign::operand_type_expr(b),
    );
    match crate::generator::mixed_sign::plan(ta.as_ref(), tb.as_ref()) {
        crate::generator::mixed_sign::Plan::AsIs => binary(a, op, b, model),
        crate::generator::mixed_sign::Plan::Widen { bits } => {
            let Some(target) = crate::generator::st::st_type::iec_integer_name(bits as u8, true)
            else {
                return binary(a, op, b, model);
            };
            let conv =
                |node: &ExpressionNode, ty: &Option<TypeNode>| -> Result<String, Diagnostic> {
                    let text = wrap_expr(node, model)?;
                    let Some(TypeNode::Integer { bits, signed }) = ty else {
                        return Ok(text);
                    };
                    match crate::generator::st::st_type::iec_integer_name(*bits, *signed) {
                        Some(from) if from != target => Ok(format!("{from}_TO_{target}({text})")),
                        _ => Ok(text),
                    }
                };
            Ok(format!("{} {op} {}", conv(a, &ta)?, conv(b, &tb)?))
        }
        crate::generator::mixed_sign::Plan::SignGuard { signed_is_left } => {
            let (lt, rt) = (wrap_expr(a, model)?, wrap_expr(b, model)?);
            let (signed, unsigned) = if signed_is_left {
                (lt.as_str(), rt.as_str())
            } else {
                (rt.as_str(), lt.as_str())
            };
            let neg = format!("({signed} < 0)");
            let same = if signed_is_left {
                format!("(LINT_TO_ULINT({signed}) {op} {unsigned})")
            } else {
                format!("({unsigned} {op} LINT_TO_ULINT({signed}))")
            };
            let negative_wins = crate::generator::mixed_sign::negative_wins(op, signed_is_left);
            Ok(if negative_wins {
                format!("({neg} OR {same})")
            } else {
                format!("(NOT {neg} AND {same})")
            })
        }
    }
}
