//! Проверка доступа к полю структуры (фича 0080, дефект 3, `SE-061`).
//!
//! Доступ `p.field` к несуществующему полю структуры прежде **не проверялся**
//! семантикой: генератор C печатал `model->p.NOSUCHFIELD` молча, и ошибку ловил
//! лишь `cc` («no member named …») — поздно и невнятно. Симулятор ловит то же
//! на исполнении (`SIM-027`). Этот проход даёт **компайл-тайм** диагностику в
//! `takt-lang`, единую для всех целей.
//!
//! Проверка **консервативна**: срабатывает только когда база доступа надёжно
//! разрешается в **структурный** тип, а поля в нём нет. Неразрешимую базу
//! (напр. элемент массива структур) пропускаем — ложное срабатывание хуже
//! пропуска, а `cc`/симулятор остаются страховкой.

use super::*;
use crate::parser::ast::{Identifier, Member};
use crate::semantic::type_node::TypeNode;

/// Ce19 (`SE-061`): доступ к несуществующему полю структуры.
///
/// Обходит инициализаторы переменных, условия, тела именованных блоков модели и
/// состояний, тела функций и условия рёбер `ref`; на каждом `x.field` (где `x`
/// разрешается в структуру) проверяет наличие поля.
pub fn check_struct_field_access(model: Rc<RefCell<ModelNode>>) -> Result<(), Diagnostic> {
    let borrowed = model.borrow();

    for var in borrowed.variables.values() {
        if let Some(expr) = var_initializer(var) {
            check_expr(expr, &borrowed)?;
        }
    }
    for cond in borrowed.conditions.values() {
        check_cond(&cond.value, &borrowed)?;
    }
    for block in &borrowed.named_blocks {
        if let Some(stmt) = block.statement() {
            check_stmt(stmt, &borrowed)?;
        }
    }
    for func in borrowed.functions.values() {
        if let FunctionDefinitionNode::Local { body, .. } = func {
            check_stmt(body, &borrowed)?;
        }
    }
    for state in borrowed.states.values() {
        for block in state.named_blocks() {
            if let Some(stmt) = block.statement() {
                check_stmt(stmt, &borrowed)?;
            }
        }
        for reference in state.references() {
            check_cond(&reference.cond, &borrowed)?;
        }
    }
    Ok(())
}

fn var_initializer(var: &VariableNode) -> Option<&ExpressionNode> {
    match var {
        VariableNode::Simple { expr, .. } | VariableNode::Const { expr, .. } => {
            if matches!(expr, ExpressionNode::None) {
                None
            } else {
                Some(expr)
            }
        }
        _ => None,
    }
}

/// Разрешает тип выражения-базы доступа к члену — только для цепочки
/// `переменная(.поле)*`. Возвращает `None`, если тип надёжно не выводится
/// (консервативно: тогда проверка не срабатывает).
fn base_type(expr: &ExpressionNode, model: &ModelNode) -> Option<TypeNode> {
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
        _ => None,
    }
}

/// Если `x.field` обращается к структуре без такого поля — диагностика `SE-061`.
fn check_member(
    inner: &ExpressionNode,
    field: &Identifier,
    model: &ModelNode,
) -> Result<(), Diagnostic> {
    if let Some(TypeNode::Struct(name)) = base_type(inner, model)
        && let Some(s) = model.search_struct(&name)
        && !s.fields.iter().any(|(f, _)| *f == field.name)
    {
        return Err(Diagnostic::error(
            field.loc,
            format!("структура '{}' не содержит поля '{}'", name, field.name),
        )
        .with_code("SE-061"));
    }
    Ok(())
}

fn check_expr(expr: &ExpressionNode, model: &ModelNode) -> Result<(), Diagnostic> {
    match expr {
        ExpressionNode::BitAccess(inner, Member::Identifier(field)) => {
            check_member(inner, field, model)?;
            check_expr(inner, model)?;
        }
        ExpressionNode::Not(e)
        | ExpressionNode::BitwiseNot(e)
        | ExpressionNode::UnaryPlus(e)
        | ExpressionNode::Negate(e)
        | ExpressionNode::Parenthesis(e)
        | ExpressionNode::BitAccess(e, _)
        | ExpressionNode::CodeBlock(e, _)
        | ExpressionNode::NamedFunctionBox(e, _)
        | ExpressionNode::Cast(e, _) => check_expr(e, model)?,
        ExpressionNode::Add(l, r)
        | ExpressionNode::Subtract(l, r)
        | ExpressionNode::Multiply(l, r)
        | ExpressionNode::Divide(l, r)
        | ExpressionNode::Modulo(l, r)
        | ExpressionNode::Power(l, r)
        | ExpressionNode::BitwiseAnd(l, r)
        | ExpressionNode::BitwiseXor(l, r)
        | ExpressionNode::BitwiseOr(l, r)
        | ExpressionNode::ShiftLeft(l, r)
        | ExpressionNode::ShiftRight(l, r)
        | ExpressionNode::And(l, r)
        | ExpressionNode::Or(l, r)
        | ExpressionNode::Equal(l, r)
        | ExpressionNode::NotEqual(l, r)
        | ExpressionNode::Less(l, r)
        | ExpressionNode::More(l, r)
        | ExpressionNode::LessEqual(l, r)
        | ExpressionNode::MoreEqual(l, r)
        | ExpressionNode::Assign(l, r) => {
            check_expr(l, model)?;
            check_expr(r, model)?;
        }
        ExpressionNode::ConditionalOperator(c, t, e) => {
            check_expr(c, model)?;
            check_expr(t, model)?;
            check_expr(e, model)?;
        }
        ExpressionNode::Array(items)
        | ExpressionNode::Initializer(items)
        | ExpressionNode::Function(_, items) => {
            for item in items {
                check_expr(item, model)?;
            }
        }
        _ => {}
    }
    Ok(())
}

fn check_cond(cond: &ConditionNode, model: &ModelNode) -> Result<(), Diagnostic> {
    match cond {
        ConditionNode::BitAccess(inner, Member::Identifier(field)) => {
            // База условия — выражение; для проверки поля переиспользуем
            // разбор выражений через мостовую конвертацию не нужен: доступ к полю
            // в условии несёт `ConditionNode`, у которого база — тоже условие.
            check_cond_member(inner, field, model)?;
            check_cond(inner, model)?;
        }
        ConditionNode::Not(c) | ConditionNode::Parenthesis(c) | ConditionNode::BitAccess(c, _) => {
            check_cond(c, model)?
        }
        ConditionNode::And(l, r)
        | ConditionNode::Or(l, r)
        | ConditionNode::Equal(l, r)
        | ConditionNode::NotEqual(l, r)
        | ConditionNode::Less(l, r)
        | ConditionNode::More(l, r)
        | ConditionNode::LessEqual(l, r)
        | ConditionNode::MoreEqual(l, r)
        | ConditionNode::Add(l, r)
        | ConditionNode::Subtract(l, r) => {
            check_cond(l, model)?;
            check_cond(r, model)?;
        }
        ConditionNode::Function(_, args, _) => {
            for arg in args {
                check_cond(arg, model)?;
            }
        }
        _ => {}
    }
    Ok(())
}

/// Тип базы доступа в условии (аналог [`base_type`] для `ConditionNode`).
fn cond_base_type(cond: &ConditionNode, model: &ModelNode) -> Option<TypeNode> {
    match cond {
        ConditionNode::Variable(var_rc, _) => Some(var_rc.borrow().ty().clone()),
        ConditionNode::Parenthesis(inner) => cond_base_type(inner, model),
        ConditionNode::BitAccess(inner, Member::Identifier(field)) => {
            let TypeNode::Struct(name) = cond_base_type(inner, model)? else {
                return None;
            };
            let s = model.search_struct(&name)?;
            s.fields
                .iter()
                .find(|(f, _)| *f == field.name)
                .map(|(_, t)| t.clone())
        }
        _ => None,
    }
}

fn check_cond_member(
    inner: &ConditionNode,
    field: &Identifier,
    model: &ModelNode,
) -> Result<(), Diagnostic> {
    if let Some(TypeNode::Struct(name)) = cond_base_type(inner, model)
        && let Some(s) = model.search_struct(&name)
        && !s.fields.iter().any(|(f, _)| *f == field.name)
    {
        return Err(Diagnostic::error(
            field.loc,
            format!("структура '{}' не содержит поля '{}'", name, field.name),
        )
        .with_code("SE-061"));
    }
    Ok(())
}

fn check_stmt(stmt: &StatementNode, model: &ModelNode) -> Result<(), Diagnostic> {
    match stmt {
        StatementNode::Block(stmts) => {
            for s in stmts {
                check_stmt(s, model)?;
            }
        }
        StatementNode::Expression(e, _) => check_expr(e, model)?,
        StatementNode::If { cond, then_, else_ } => {
            check_expr(cond, model)?;
            check_stmt(then_, model)?;
            if let Some(e) = else_ {
                check_stmt(e, model)?;
            }
        }
        StatementNode::Loop { cond, body } => {
            if let Some(c) = cond {
                check_expr(c, model)?;
            }
            check_stmt(body, model)?;
        }
        StatementNode::For {
            init,
            cond,
            step,
            body,
        } => {
            if let Some(s) = init {
                check_stmt(s, model)?;
            }
            if let Some(c) = cond {
                check_expr(c, model)?;
            }
            if let Some(s) = step {
                check_expr(s, model)?;
            }
            check_stmt(body, model)?;
        }
        StatementNode::Variable(_, _, Some(e)) => check_expr(e, model)?,
        StatementNode::Return(Some(e)) => check_expr(e, model)?,
        StatementNode::Match { expr, arms } => {
            check_expr(expr, model)?;
            for arm in arms {
                check_stmt(&arm.body, model)?;
            }
        }
        _ => {}
    }
    Ok(())
}
