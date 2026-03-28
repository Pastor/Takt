//! Диагностика Ce13: обнаружение неиспользуемых переменных.
//!
//! Функция [`check_unused_variables`] обходит все выражения, операторы
//! и условия модели, собирает множество используемых переменных и
//! возвращает предупреждения для каждой объявленной, но неиспользуемой.

use crate::diagnostics::{Diagnostic, Location};
use crate::semantic::{
    Condition, ConditionNode, Expression, FunctionNode, ModelNode, NamedCodeBlock, Statement,
    VariableNode,
};
use std::cell::RefCell;
use std::collections::HashSet;
use std::rc::Rc;

/// Проверяет наличие неиспользуемых переменных в модели.
///
/// Возвращает список [`Diagnostic`] уровня Warning для каждой переменной,
/// объявленной через `var`, но ни разу не упомянутой в выражениях, условиях
/// или именованных блоках модели.
/// Порты и константы не проверяются — они могут быть внешним интерфейсом.
pub fn check_unused_variables(model: Rc<RefCell<ModelNode>>) -> Vec<Diagnostic> {
    let mut warnings = Vec::new();
    check_model_unused(model, &mut warnings);
    warnings
}

fn check_model_unused(model: Rc<RefCell<ModelNode>>, warnings: &mut Vec<Diagnostic>) {
    let borrowed = model.borrow();
    let mut used: HashSet<String> = HashSet::new();

    // Собираем использования из инициализаторов переменных
    for var in borrowed.variables.values() {
        collect_from_var(var, &mut used);
    }

    // Собираем использования из тел функций
    for func in borrowed.functions.values() {
        collect_from_func(func, &mut used);
    }

    // Собираем использования из именованных условий
    for cond in borrowed.conditions.values() {
        collect_from_condition_node(cond, &mut used);
    }

    // Собираем использования из именованных блоков модели
    for block in &borrowed.named_blocks {
        collect_from_named_block(block, &mut used);
    }

    // Собираем использования из состояний
    let states: Vec<_> = borrowed.states.values().cloned().collect();
    drop(borrowed);

    for state in &states {
        collect_from_state(state, &mut used);
    }

    // Проверяем каждую простую переменную (var)
    let borrowed = model.borrow();
    for (name, var) in &borrowed.variables {
        // Порты и константы не предупреждаем — они могут быть внешним интерфейсом
        if matches!(var, VariableNode::Port { .. } | VariableNode::Const { .. }) {
            continue;
        }
        if !used.contains(name.as_str()) {
            warnings.push(Diagnostic::warning(
                Location::Builtin,
                format!(
                    "Ce13: переменная '{}' объявлена, но нигде не используется",
                    name
                ),
            ));
        }
    }

    // Рекурсивно для вложенных моделей
    let nested: Vec<Rc<RefCell<ModelNode>>> =
        borrowed.models.values().map(Rc::clone).collect();
    drop(borrowed);

    for nested_model in nested {
        check_model_unused(nested_model, warnings);
    }
}

fn collect_from_var(var: &VariableNode, used: &mut HashSet<String>) {
    match var {
        VariableNode::Simple { expr, .. }
        | VariableNode::Port { expr, .. }
        | VariableNode::Const { expr, .. } => collect_from_expr(expr, used),
        VariableNode::Unresolved => {}
    }
}

fn collect_from_expr(expr: &Expression, used: &mut HashSet<String>) {
    match expr {
        Expression::Variable(var_rc) => {
            let borrowed = var_rc.borrow();
            if let VariableNode::Simple { name, .. }
            | VariableNode::Port { name, .. }
            | VariableNode::Const { name, .. } = &*borrowed
            {
                used.insert(name.clone());
            }
        }
        Expression::ArraySubscript(var_rc, _) | Expression::ArraySlice(var_rc, _, _) => {
            let borrowed = var_rc.borrow();
            if let VariableNode::Simple { name, .. }
            | VariableNode::Port { name, .. }
            | VariableNode::Const { name, .. } = &*borrowed
            {
                used.insert(name.clone());
            }
        }
        Expression::Not(e)
        | Expression::BitwiseNot(e)
        | Expression::UnaryPlus(e)
        | Expression::Negate(e)
        | Expression::Parenthesis(e)
        | Expression::BitAccess(e, _)
        | Expression::Cast(e, _) => collect_from_expr(e, used),
        Expression::Add(l, r)
        | Expression::Subtract(l, r)
        | Expression::Multiply(l, r)
        | Expression::Divide(l, r)
        | Expression::Modulo(l, r)
        | Expression::Power(l, r)
        | Expression::BitwiseAnd(l, r)
        | Expression::BitwiseXor(l, r)
        | Expression::BitwiseOr(l, r)
        | Expression::ShiftLeft(l, r)
        | Expression::ShiftRight(l, r)
        | Expression::And(l, r)
        | Expression::Or(l, r)
        | Expression::Equal(l, r)
        | Expression::NotEqual(l, r)
        | Expression::Less(l, r)
        | Expression::More(l, r)
        | Expression::LessEqual(l, r)
        | Expression::MoreEqual(l, r)
        | Expression::Assign(l, r) => {
            collect_from_expr(l, used);
            collect_from_expr(r, used);
        }
        Expression::ConditionalOperator(cond, then_e, else_e) => {
            collect_from_expr(cond, used);
            collect_from_expr(then_e, used);
            collect_from_expr(else_e, used);
        }
        Expression::Function(_, args) => {
            for arg in args {
                collect_from_expr(arg, used);
            }
        }
        Expression::Array(items) | Expression::Initializer(items) => {
            for item in items {
                collect_from_expr(item, used);
            }
        }
        _ => {}
    }
}

fn collect_from_stmt(stmt: &Statement, used: &mut HashSet<String>) {
    match stmt {
        Statement::Block(stmts) => {
            for s in stmts {
                collect_from_stmt(s, used);
            }
        }
        Statement::Expression(e) => collect_from_expr(e, used),
        Statement::If { cond, then_, else_ } => {
            collect_from_expr(cond, used);
            collect_from_stmt(then_, used);
            if let Some(e) = else_ {
                collect_from_stmt(e, used);
            }
        }
        Statement::Loop { cond, body } => {
            if let Some(c) = cond {
                collect_from_expr(c, used);
            }
            collect_from_stmt(body, used);
        }
        Statement::For { init, cond, step, body } => {
            if let Some(s) = init {
                collect_from_stmt(s, used);
            }
            if let Some(c) = cond {
                collect_from_expr(c, used);
            }
            if let Some(s) = step {
                collect_from_expr(s, used);
            }
            collect_from_stmt(body, used);
        }
        Statement::Variable(_, _, Some(e)) => collect_from_expr(e, used),
        Statement::Return(Some(e)) => collect_from_expr(e, used),
        _ => {}
    }
}

fn collect_from_condition(cond: &Condition, used: &mut HashSet<String>) {
    match cond {
        Condition::Variable(var_rc) => {
            let borrowed = var_rc.borrow();
            if let VariableNode::Simple { name, .. }
            | VariableNode::Port { name, .. }
            | VariableNode::Const { name, .. } = &*borrowed
            {
                used.insert(name.clone());
            }
        }
        Condition::Not(c) | Condition::Parenthesis(c) => collect_from_condition(c, used),
        Condition::And(l, r)
        | Condition::Or(l, r)
        | Condition::Equal(l, r)
        | Condition::NotEqual(l, r)
        | Condition::Less(l, r)
        | Condition::More(l, r)
        | Condition::LessEqual(l, r)
        | Condition::MoreEqual(l, r)
        | Condition::Add(l, r)
        | Condition::Subtract(l, r) => {
            collect_from_condition(l, used);
            collect_from_condition(r, used);
        }
        Condition::Function(_, args) => {
            for arg in args {
                collect_from_condition(arg, used);
            }
        }
        Condition::ArraySubscript(var_rc, _) => {
            let borrowed = var_rc.borrow();
            if let VariableNode::Simple { name, .. }
            | VariableNode::Port { name, .. }
            | VariableNode::Const { name, .. } = &*borrowed
            {
                used.insert(name.clone());
            }
        }
        Condition::BitAccess(_, _) => {}
        _ => {}
    }
}

fn collect_from_condition_node(cond_node: &ConditionNode, used: &mut HashSet<String>) {
    collect_from_condition(&cond_node.value, used);
}

fn collect_from_named_block(block: &NamedCodeBlock, used: &mut HashSet<String>) {
    if let Some(stmt) = block.statement() {
        collect_from_stmt(stmt, used);
    }
}

fn collect_from_func(func: &FunctionNode, used: &mut HashSet<String>) {
    if let FunctionNode::Local { body, .. } = func {
        collect_from_stmt(body, used);
    }
}

fn collect_from_state(
    state: &crate::semantic::StateNode,
    used: &mut HashSet<String>,
) {
    use crate::semantic::{Reference, StateNode};
    match state {
        StateNode::Simple {
            named_blocks,
            references,
            ..
        }
        | StateNode::Implement {
            named_blocks,
            references,
            ..
        } => {
            for block in named_blocks {
                collect_from_named_block(block, used);
            }
            for reference in references {
                collect_from_condition(&reference.cond, used);
            }
        }
        StateNode::Unresolved => {}
    }
}
