//! Печать ПРИСВАИВАНИЯ и обёрточной арифметики — цель `rust`.
//!
//! Выделено из `rust_expr` фичей 0415 по границе **ответственности**: печать
//! выражения отвечает на вопрос «как выглядит значение», а этот модуль — на
//! вопрос «как выглядит запись значения в место» (порт, разряд, переменная) и
//! «когда арифметика печатается обёрткой» (правило 0127).
//!
//! ⚠️ Свёртка `x := x + 1` живёт здесь же: она есть свойство записи, а не
//! значения — `clippy::assign_op_pattern` отвергает развёрнутую форму под
//! `-D warnings`.

use super::rust_expr::{
    Scope, coerce_to, expression_type, is_wrapping_arith, print_expression, unwrap_outer,
    write_port,
};
use crate::diagnostics::Diagnostic;
use crate::generator::rust::rust_fixed;
use crate::parser::ast::Member;
use crate::semantic::ExpressionNode;
use crate::semantic::VariableNode;

/// Печатает присваивание; запись в порт превращает в вызов HAL.
pub(crate) fn assign(
    target: &ExpressionNode,
    value: &ExpressionNode,
    scope: &Scope,
) -> Result<String, Diagnostic> {
    if let ExpressionNode::Variable(var) = target {
        let borrowed = var.borrow();
        if let VariableNode::Port {
            name,
            ty,
            direction,
            loc,
            ..
        } = &*borrowed
        {
            let printed = coerce_to(value, ty, scope)?;
            return write_port(name, ty, *direction, unwrap_outer(&printed), scope, *loc);
        }
        if let VariableNode::Const { name, loc, .. } = &*borrowed {
            return Err(Diagnostic::error(
                *loc,
                format!("Присваивание в константу '{}' недопустимо", name),
            )
            .with_code("RS-019"));
        }
    }
    // Запись одного разряда (фича 0250). Прежде эта ветви не было, и печатник
    // левой части выдавал ЧТЕНИЕ бита: `(((self.b >> 2) & 1) != 0) = true;` —
    // `rustc` отвечал E0070, то есть цель рапортовала об успехе и клала на
    // диск файл, который не собирается.
    if let ExpressionNode::BitAccess(inner, Member::Number(bit)) = target {
        return crate::generator::rust::rust_bit::assign_bit(inner, *bit, value, scope);
    }
    let target_text = print_expression(target, scope)?;
    // `x := x + 1` → `x += 1`. Не косметика: clippy считает `x = x + 1` ручной
    // реализацией составного присваивания (`assign_op_pattern`) и под
    // `-D warnings` отвергает. Совпадение операнда проверяется по НАПЕЧАТАННОМУ
    // тексту, а не по узлам: текст — это ровно то, что увидит компилятор.
    if let Some(compound) = compound_assign(&target_text, value, scope)? {
        return Ok(compound);
    }
    let ty = expression_type(target);
    let printed = match &ty {
        Some(ty) => coerce_to(value, ty, scope)?,
        None => print_expression(value, scope)?,
    };
    // Присваиваемое значение — ещё одна позиция, где внешние скобки лишние:
    // `x = (a - b);` даёт `unnecessary parentheses around assigned value`.
    Ok(format!("{} = {}", target_text, unwrap_outer(&printed)))
}

/// Строит составное присваивание (`x += 1`), если значение имеет форму `x op …`.
fn compound_assign(
    target_text: &str,
    value: &ExpressionNode,
    scope: &Scope,
) -> Result<Option<String>, Diagnostic> {
    // Q-арифметика (0061) НЕ сворачивается: `x := x * y` над q — это масштабный
    // `takt_q`-путь, а не нативное `x *= y` (то дало бы целочисленное умножение
    // представлений без сдвига на n — молча неверный результат и паника на
    // переполнении в debug).
    if rust_fixed::fixed_format_in(value, scope.model).is_some() {
        return Ok(None);
    }
    // Беззнаковая арифметика печатается обёрткой (`wrapping_*`, фича 0127):
    // свернуть её в `x += 1` нельзя — `+=` в debug паникует на переполнении, а
    // правило языка требует обёртки mod 2^N.
    if is_wrapping_arith(value) {
        return Ok(None);
    }
    let (op, lhs, rhs) = match value {
        ExpressionNode::Add(a, b) => ("+=", a, b),
        ExpressionNode::Subtract(a, b) => ("-=", a, b),
        ExpressionNode::Multiply(a, b) => ("*=", a, b),
        ExpressionNode::Divide(a, b) => ("/=", a, b),
        ExpressionNode::Modulo(a, b) => ("%=", a, b),
        ExpressionNode::BitwiseAnd(a, b) => ("&=", a, b),
        ExpressionNode::BitwiseOr(a, b) => ("|=", a, b),
        ExpressionNode::BitwiseXor(a, b) => ("^=", a, b),
        ExpressionNode::ShiftLeft(a, b) => ("<<=", a, b),
        ExpressionNode::ShiftRight(a, b) => (">>=", a, b),
        _ => return Ok(None),
    };
    // ⚠️ Печать операнда здесь — ПРЕДИКАТ, а не вывод: её отказ значит «свернуть
    // нельзя», и настоящую диагностику даст печать значения ниже по пути, где
    // известен приёмник (фича 0415). Прежде отказ пробрасывался отсюда, и
    // `v := (2 ** 3) + 1;` падал ещё до того, как приёмник вступал в дело, —
    // то есть правило действовало только в позиции «степень прямо в приёмнике».
    //
    // Глотание ограничено ОДНИМ сравнением: печать самого значения (`rhs_text`
    // и путь `assign`) ошибку по-прежнему пробрасывает, иначе повторился бы
    // класс 0189 — печатник, теряющий оператор молча.
    match print_expression(lhs, scope) {
        Ok(text) if text == target_text => {}
        _ => return Ok(None),
    }
    let rhs_text = print_expression(rhs, scope)?;
    Ok(Some(format!(
        "{} {} {}",
        target_text,
        op,
        unwrap_outer(&rhs_text)
    )))
}
