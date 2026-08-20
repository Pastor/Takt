//! Приведение `as` к целочисленному типу в цели `sv` (фича 0323).
//!
//! Вынесено из `sv_expr.rs` по границе ответственности: печать выражения
//! отвечает на вопрос «как выглядит операция», а этот модуль — «как выглядит
//! смена типа». Поводом был гейт размера модуля, границей — смысл.

use crate::diagnostics::Diagnostic;
use crate::generator::sv::sv_expr::{Scope, print_expression};
use crate::generator::sv::sv_fsm::sv002;
use crate::semantic::ExpressionNode;
use crate::semantic::type_node::TypeNode;

/// Приведение к **целочисленному** типу — размерная форма `<W>'(выражение)`
/// (фича 0323).
///
/// # Зачем
///
/// Цель не переводила `as` вовсе: замер 2026-08-20 на `w := a as u16;` —
/// эталон и шесть целей исполняют, `sv`/`sv-mmio` отвечают `SV-002`.
/// Приведение — базовая операция языка, и её отсутствие тянуло за собой всё,
/// что через неё выражается (`[bit;N] as u8`, сравнение с числом и прочее).
///
/// # Форма
///
/// - беззнаковая цель — `W'(expr)`: усечение старших разрядов и дополнение
///   нулями заданы стандартом, и это ровно правило ADR 0127 (обёртка `mod 2ⁿ`);
/// - знаковая — `$signed(W'(expr))`: без `$signed` сравнение и арифметический
///   сдвиг работали бы как беззнаковые.
///
/// ⚠️ Ширина берётся у **цели** приведения. Форма без числа перед апострофом
/// (`'(…)`) означает «ширина по контексту» — молчаливое расширение до ширины
/// приёмника, а не то, что просит автор.
///
/// # Ошибки
///
/// `SV-002` — цель не скалярная (массив, структура): у такого приведения нет
/// одной ширины, и печатать его нечем.
pub(in crate::generator::sv) fn integer_cast(
    inner: &ExpressionNode,
    ty: &TypeNode,
    scope: &Scope,
) -> Result<String, Diagnostic> {
    let Some(width) = crate::generator::sv::sv_type::scalar_width(ty) else {
        return Err(sv002(
            "приведение типа (`as`) к нескалярному типу: у массива и структуры \
             нет одной ширины",
        ));
    };
    let value = print_expression(inner, scope)?;
    let sized = format!("{width}'({value})");
    Ok(match ty {
        TypeNode::Integer { signed: true, .. } => format!("$signed({sized})"),
        _ => sized,
    })
}

/// Знаково ли выражение — для выбора арифметического сдвига (фича 0324).
///
/// ⚠️ Признак **синтаксический и осторожный**: он смотрит на объявленный тип
/// операнда и на явное приведение. Не узнав знака, отвечает `false`, то есть
/// печатается прежний логический сдвиг — ошибка в сторону прежнего поведения,
/// а не в сторону нового.
pub(in crate::generator::sv) fn is_signed_expression(expr: &ExpressionNode) -> bool {
    match expr {
        ExpressionNode::Variable(var) => matches!(
            var.borrow().ty(),
            TypeNode::Integer { signed: true, .. } | TypeNode::Fixed { .. }
        ),
        ExpressionNode::Cast(_, ty) => matches!(
            ty,
            TypeNode::Integer { signed: true, .. } | TypeNode::Fixed { .. }
        ),
        ExpressionNode::Parenthesis(inner) | ExpressionNode::Negate(inner) => {
            is_signed_expression(inner)
        }
        _ => false,
    }
}

/// Оператор сдвига вправо: арифметический для знакового (фича 0324).
///
/// ⚠️ В SystemVerilog `>>` — **логический** сдвиг даже над `logic signed`:
/// проба verilator 2026-08-20 дала `-8 >> 1 = 124` против `-8 >>> 1 = -4`.
/// Эталон, `c` и `rust` дают −4, то есть цель расходилась значением молча.
pub(in crate::generator::sv) fn shift_right_operator(left: &ExpressionNode) -> &'static str {
    if is_signed_expression(left) {
        ">>>"
    } else {
        ">>"
    }
}
