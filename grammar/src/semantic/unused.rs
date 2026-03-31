//! Диагностика Ce13: обнаружение неиспользуемых переменных.
//!
//! Функция [`check_unused_variables`] обходит все выражения, операторы
//! и условия модели, собирает множество используемых переменных и
//! возвращает предупреждения для каждой объявленной, но неиспользуемой.

use crate::diagnostics::{Diagnostic, Location};
use crate::semantic::{
    ConditionDefinitionNode, ConditionNode, ExpressionNode, FunctionDefinitionNode, ModelNode,
    NamedCodeBlockDefinitionNode, StatementNode, VariableNode,
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
    let nested: Vec<Rc<RefCell<ModelNode>>> = borrowed.models.values().map(Rc::clone).collect();
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

fn collect_from_expr(expr: &ExpressionNode, used: &mut HashSet<String>) {
    match expr {
        ExpressionNode::Variable(var_rc) => {
            let borrowed = var_rc.borrow();
            if let VariableNode::Simple { name, .. }
            | VariableNode::Port { name, .. }
            | VariableNode::Const { name, .. } = &*borrowed
            {
                used.insert(name.clone());
            }
        }
        ExpressionNode::ArraySubscript(var_rc, _) | ExpressionNode::ArraySlice(var_rc, _, _) => {
            let borrowed = var_rc.borrow();
            if let VariableNode::Simple { name, .. }
            | VariableNode::Port { name, .. }
            | VariableNode::Const { name, .. } = &*borrowed
            {
                used.insert(name.clone());
            }
        }
        ExpressionNode::Not(e)
        | ExpressionNode::BitwiseNot(e)
        | ExpressionNode::UnaryPlus(e)
        | ExpressionNode::Negate(e)
        | ExpressionNode::Parenthesis(e)
        | ExpressionNode::BitAccess(e, _)
        | ExpressionNode::Cast(e, _) => collect_from_expr(e, used),
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
            collect_from_expr(l, used);
            collect_from_expr(r, used);
        }
        ExpressionNode::ConditionalOperator(cond, then_e, else_e) => {
            collect_from_expr(cond, used);
            collect_from_expr(then_e, used);
            collect_from_expr(else_e, used);
        }
        ExpressionNode::Function(_, args) => {
            for arg in args {
                collect_from_expr(arg, used);
            }
        }
        ExpressionNode::Array(items) | ExpressionNode::Initializer(items) => {
            for item in items {
                collect_from_expr(item, used);
            }
        }
        _ => {}
    }
}

fn collect_from_stmt(stmt: &StatementNode, used: &mut HashSet<String>) {
    match stmt {
        StatementNode::Block(stmts) => {
            for s in stmts {
                collect_from_stmt(s, used);
            }
        }
        StatementNode::Expression(e) => collect_from_expr(e, used),
        StatementNode::If { cond, then_, else_ } => {
            collect_from_expr(cond, used);
            collect_from_stmt(then_, used);
            if let Some(e) = else_ {
                collect_from_stmt(e, used);
            }
        }
        StatementNode::Loop { cond, body } => {
            if let Some(c) = cond {
                collect_from_expr(c, used);
            }
            collect_from_stmt(body, used);
        }
        StatementNode::For {
            init,
            cond,
            step,
            body,
        } => {
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
        StatementNode::Variable(_, _, Some(e)) => collect_from_expr(e, used),
        StatementNode::Return(Some(e)) => collect_from_expr(e, used),
        _ => {}
    }
}

fn collect_from_condition(cond: &ConditionNode, used: &mut HashSet<String>) {
    match cond {
        ConditionNode::Variable(var_rc, _) => {
            let borrowed = var_rc.borrow();
            if let VariableNode::Simple { name, .. }
            | VariableNode::Port { name, .. }
            | VariableNode::Const { name, .. } = &*borrowed
            {
                used.insert(name.clone());
            }
        }
        ConditionNode::Not(c) | ConditionNode::Parenthesis(c) => collect_from_condition(c, used),
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
            collect_from_condition(l, used);
            collect_from_condition(r, used);
        }
        ConditionNode::Function(_, args, _) => {
            for arg in args {
                collect_from_condition(arg, used);
            }
        }
        ConditionNode::ArraySubscript(var_rc, _) => {
            let borrowed = var_rc.borrow();
            if let VariableNode::Simple { name, .. }
            | VariableNode::Port { name, .. }
            | VariableNode::Const { name, .. } = &*borrowed
            {
                used.insert(name.clone());
            }
        }
        ConditionNode::BitAccess(_, _) => {}
        _ => {}
    }
}

fn collect_from_condition_node(cond_node: &ConditionDefinitionNode, used: &mut HashSet<String>) {
    collect_from_condition(&cond_node.value, used);
}

fn collect_from_named_block(block: &NamedCodeBlockDefinitionNode, used: &mut HashSet<String>) {
    if let Some(stmt) = block.statement() {
        collect_from_stmt(stmt, used);
    }
}

fn collect_from_func(func: &FunctionDefinitionNode, used: &mut HashSet<String>) {
    if let FunctionDefinitionNode::Local { body, .. } = func {
        collect_from_stmt(body, used);
    }
}

fn collect_from_state(state: &crate::semantic::StateNode, used: &mut HashSet<String>) {
    use crate::semantic::StateNode;
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
