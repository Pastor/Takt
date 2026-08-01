//! Анонимное обращение в **инициализаторе объявления** — `SE-099` (фича 0189).
//!
//! ## Почему это ошибка, а не значение
//!
//! Инициализатор объявления вычисляется **до первого такта**: эталон делает это
//! вычислителем начальных значений (`takt-sim/src/unit/initial.rs`), цели —
//! печатью в `_init` / `new()` / ветви сброса. Содержимое ячейки в этот момент
//! не известно никому: у эталона памяти ещё нет, у цели `c-hal` чтение регистра
//! на этапе инициализации — уже обращение к железу.
//!
//! Без запрета стороны разошлись бы **молча**: эталон дал бы ноль (обращение —
//! «не константа», значение по умолчанию), а `c-hal` напечатал бы настоящее
//! чтение регистра. Ровно тот класс, ради которого фича и заведена, — поэтому
//! форма отвергается, а не трактуется.
//!
//! Разрешённое место чтения — **тело**: `always { x := #0x100 as u8; }`.

use super::*;

/// Проверяет инициализаторы объявлений модели на анонимное обращение.
pub(super) fn validate_anon_in_initializers(
    model: Rc<RefCell<ModelNode>>,
) -> Result<(), Diagnostic> {
    let borrowed = model.borrow();
    for variable in borrowed.variables.values() {
        match variable {
            VariableNode::Unresolved => {}
            VariableNode::Simple { expr, loc, name, .. }
            | VariableNode::Const { expr, loc, name, .. } => {
                check(expr, *loc, name)?;
            }
            // У порта два выражения (фича 0187): размещение и начальное
            // значение. Обращение к ячейке незаконно в обоих — в адресе оно к
            // тому же не свернулось бы в константу (`SE-055`).
            VariableNode::Port {
                address,
                init,
                loc,
                name,
                ..
            } => {
                check(address, *loc, name)?;
                check(init, *loc, name)?;
            }
        }
    }
    Ok(())
}

/// Ищет обращение к ячейке в выражении инициализатора.
fn check(expr: &ExpressionNode, loc: Location, name: &str) -> Result<(), Diagnostic> {
    if !contains_anon(expr) {
        return Ok(());
    }
    Err(Diagnostic::error(
        loc,
        format!(
            "инициализатор '{name}' обращается к ячейке по адресу: содержимое памяти \
             до первого такта неизвестно, и эталон с целью разошлись бы молча. \
             Читайте ячейку в теле состояния — например, 'always {{ {name} := \
             #0xАДРЕС as ТИП; }}'"
        ),
    )
    .with_code("SE-099"))
}

/// Есть ли анонимное обращение в поддереве выражения.
///
/// Обход опирается на [`ExpressionNode::components`]-подобную рекурсию через
/// общий разбор: полноты добиваться незачем — форма запрещена целиком, и
/// достаточно найти хотя бы одно вхождение на любом уровне.
fn contains_anon(expr: &ExpressionNode) -> bool {
    match expr {
        ExpressionNode::AnonPort(_) => true,
        ExpressionNode::Parenthesis(inner)
        | ExpressionNode::BitAccess(inner, _)
        | ExpressionNode::CodeBlock(inner, _)
        | ExpressionNode::NamedFunctionBox(inner, _)
        | ExpressionNode::Not(inner)
        | ExpressionNode::UnaryPlus(inner)
        | ExpressionNode::Negate(inner)
        | ExpressionNode::Cast(inner, _)
        | ExpressionNode::BitwiseNot(inner) => contains_anon(inner),
        ExpressionNode::Power(left, right)
        | ExpressionNode::Multiply(left, right)
        | ExpressionNode::Divide(left, right)
        | ExpressionNode::Modulo(left, right)
        | ExpressionNode::Add(left, right)
        | ExpressionNode::Subtract(left, right)
        | ExpressionNode::ShiftLeft(left, right)
        | ExpressionNode::ShiftRight(left, right)
        | ExpressionNode::BitwiseAnd(left, right)
        | ExpressionNode::BitwiseXor(left, right)
        | ExpressionNode::BitwiseOr(left, right)
        | ExpressionNode::Less(left, right)
        | ExpressionNode::More(left, right)
        | ExpressionNode::LessEqual(left, right)
        | ExpressionNode::MoreEqual(left, right)
        | ExpressionNode::Equal(left, right)
        | ExpressionNode::NotEqual(left, right)
        | ExpressionNode::And(left, right)
        | ExpressionNode::Or(left, right)
        | ExpressionNode::Assign(left, right) => contains_anon(left) || contains_anon(right),
        ExpressionNode::ConditionalOperator(cond, then_, else_) => {
            contains_anon(cond) || contains_anon(then_) || contains_anon(else_)
        }
        ExpressionNode::Function(_, args)
        | ExpressionNode::Array(args)
        | ExpressionNode::Initializer(args) => args.iter().any(contains_anon),
        ExpressionNode::ArraySubscript(_, index) => contains_anon(index),
        // Прочее вложенных выражений не несёт.
        ExpressionNode::None
        | ExpressionNode::Unresolved(_)
        | ExpressionNode::ArraySlice(_, _, _)
        | ExpressionNode::Number(_)
        | ExpressionNode::Duration(_)
        | ExpressionNode::Rational(_, _)
        | ExpressionNode::String(_)
        | ExpressionNode::Type(_)
        | ExpressionNode::Address(_, _)
        | ExpressionNode::Bool(_)
        | ExpressionNode::Variable(_)
        | ExpressionNode::Model(_)
        | ExpressionNode::Condition(_)
        | ExpressionNode::List(_) => false,
    }
}
