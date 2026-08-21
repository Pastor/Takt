//! Тип выражения-базы постфиксной операции — общий носитель (фича 0358).
//!
//! # Зачем
//!
//! Индексация и срез стали постфиксами **над выражением**, а не над именем:
//! `b.data[1]` прежде не разбирался вовсе (`SY-002`), тогда как обратная
//! цепочка `ps[1].x` работала. Чтобы проверки границ (`SE-028`) и «это не
//! массив» (`SE-030`) остались в силе, нужен тип базы — а он теперь выводится
//! по цепочке, а не читается у переменной.
//!
//! Носитель один: тем же знанием пользуется проверка доступа к полю
//! (`validate::member_access`, `SE-061`), где эта функция и появилась.
//!
//! # Консервативность — часть контракта
//!
//! `None` означает «тип надёжно не выводится», и проверка тогда **не
//! срабатывает**. Ложное срабатывание хуже пропуска: за пропуском стоят
//! диагностики целей и эталона, за ложным отказом — незаконно отвергнутая
//! программа.

use crate::parser::ast::Member;
use crate::semantic::ModelNode;
use crate::semantic::expression_node::ExpressionNode;
use crate::semantic::type_node::TypeNode;

/// Тип выражения `expr` в контексте модели `model`.
///
/// Разбирается цепочка `переменная(.поле | [индекс])*` — то, из чего состоит
/// **место** в языке. Всё прочее даёт `None`.
pub(crate) fn base_type(expr: &ExpressionNode, model: &ModelNode) -> Option<TypeNode> {
    match expr {
        ExpressionNode::Variable(var_rc) => Some(var_rc.borrow().ty().clone()),
        ExpressionNode::Parenthesis(inner) => base_type(inner, model),
        ExpressionNode::BitAccess(inner, Member::Identifier(field)) => {
            let TypeNode::Struct(name) = base_type(inner, model)? else {
                return None;
            };
            let s = model.search_struct(&name)?;
            s.fields
                .iter()
                .find(|(f, _)| *f == field.name)
                .map(|(_, t)| t.clone())
        }
        // Элемент массива — тип элемента; так `b.data[1].x` и `ps[1].x`
        // разбираются одним правилом.
        ExpressionNode::ArraySubscript(base, _) => match base_type(base, model)? {
            TypeNode::Array(_, elem) => Some(*elem),
            _ => None,
        },
        _ => None,
    }
}
