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
//! # Представления МЕСТА, знание одно (фича 0382)
//!
//! Одно и то же место автор записывает в двух разных деревьях, и оба доходят
//! до потребителей:
//!
//! | Вход | Представление | Откуда берётся |
//! |---|---|---|
//! | [`base_type`] | [`ExpressionNode`] | тело блока, тело функции |
//! | [`cond_base_type`] | [`ConditionNode`] | `cond`, формулы, инвариант, условие ребра (разрешается стадией 6) |
//!
//! Спуск по типу (структура → поле, массив → элемент) у них **общий**: копии
//! этого правила разъехались бы молча — компилятор о расхождении копий не
//! скажет (класс 0084/0193/0195). Различаются входы только формой дерева; тип
//! корня оба берут у ячейки ссылки.
//!
//! ⚠️ Третьего входа — по сырому `ast::Condition` — здесь нет намеренно:
//! неразрешённым до целей доезжает только паттерн `S(Модель) = Состояние`, а в
//! нём обе стороны суть имена (замер фичи 0382).
//!
//! # Консервативность — часть контракта
//!
//! `None` означает «тип надёжно не выводится», и проверка тогда **не
//! срабатывает**. Ложное срабатывание хуже пропуска: за пропуском стоят
//! диагностики целей и эталона, за ложным отказом — незаконно отвергнутая
//! программа.

use crate::parser::ast::Member;
use crate::semantic::ModelNode;
use crate::semantic::condition_node::ConditionNode;
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
            field_type(base_type(inner, model)?, &field.name, model)
        }
        // Элемент массива — тип элемента; так `b.data[1].x` и `ps[1].x`
        // разбираются одним правилом.
        ExpressionNode::ArraySubscript(base, _) => element_type(base_type(base, model)?),
        _ => None,
    }
}

/// Тип места в РАЗРЕШЁННОМ условии (`cond`, формулы) — фича 0382.
///
/// Аналог [`base_type`] для [`ConditionNode`]: спуск тот же, отличается только
/// представление дерева.
pub(crate) fn cond_base_type(cond: &ConditionNode, model: &ModelNode) -> Option<TypeNode> {
    match cond {
        ConditionNode::Variable(var_rc, _) => Some(var_rc.borrow().ty().clone()),
        ConditionNode::Parenthesis(inner) => cond_base_type(inner, model),
        ConditionNode::BitAccess(inner, Member::Identifier(field)) => {
            field_type(cond_base_type(inner, model)?, &field.name, model)
        }
        ConditionNode::ArraySubscript(base, _) => element_type(cond_base_type(base, model)?),
        _ => None,
    }
}

/// Тип поля `field` у типа базы — общий шаг спуска для всех трёх входов.
fn field_type(base: TypeNode, field: &str, model: &ModelNode) -> Option<TypeNode> {
    let TypeNode::Struct(name) = base else {
        return None;
    };
    let s = model.search_struct(&name)?;
    s.fields
        .iter()
        .find(|(f, _)| f == field)
        .map(|(_, t)| t.clone())
}

/// Тип элемента массива — общий шаг спуска для всех трёх входов.
fn element_type(base: TypeNode) -> Option<TypeNode> {
    match base {
        TypeNode::Array(_, elem) => Some(*elem),
        _ => None,
    }
}
