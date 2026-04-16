//! Диагностика Ce13: обнаружение неиспользуемых переменных.
//!
//! Функция [`check_unused_variables`] обходит все выражения, операторы
//! и условия модели, собирает множество используемых переменных и
//! возвращает предупреждения для каждой объявленной, но неиспользуемой.
//!
//! Функция [`compute_usage`] вычисляет множество используемых имён
//! по всем категориям элементов модели.

use crate::diagnostics::Diagnostic;
use crate::semantic::{
    ConditionDefinitionNode, ConditionNode, ExpressionNode, FunctionDefinitionNode, ModelNode,
    NamedCodeBlockDefinitionNode, StatementNode, VariableNode,
};
use std::cell::RefCell;
use std::collections::HashSet;
use std::rc::Rc;

/// Множество использованных имён в модели.
#[derive(Debug)]
pub struct UsageSet {
    /// Используемые переменные (var)
    pub variables: HashSet<String>,
    /// Используемые константы (const)
    pub constants: HashSet<String>,
    /// Используемые порты (port)
    pub ports: HashSet<String>,
    /// Используемые функции (fn / extern fn)
    pub functions: HashSet<String>,
}

/// Вычисляет множество используемых имён для всех элементов модели.
///
/// Обходит те же элементы, что и [`check_unused_variables`]:
/// переменные, функции, именованные условия, блоки, состояния.
pub fn compute_usage(model: Rc<RefCell<ModelNode>>) -> UsageSet {
    let mut set = UsageSet {
        variables: HashSet::new(),
        constants: HashSet::new(),
        ports: HashSet::new(),
        functions: HashSet::new(),
    };
    collect_model_usage(model, &mut set);
    set
}

/// Рекурсивный обход модели для сбора множества используемых имён.
fn collect_model_usage(model: Rc<RefCell<ModelNode>>, set: &mut UsageSet) {
    let borrowed = model.borrow();

    // Инициализаторы переменных
    for var in borrowed.variables.values() {
        usage_from_var(var, set);
    }

    // Тела функций
    for func in borrowed.functions.values() {
        usage_from_func(func, set);
    }

    // Именованные условия
    for cond in borrowed.conditions.values() {
        usage_from_condition_node(cond, set);
    }

    // Именованные блоки модели
    for block in &borrowed.named_blocks {
        usage_from_named_block(block, set);
    }

    // Состояния
    let states: Vec<_> = borrowed.states.values().cloned().collect();
    drop(borrowed);

    for state in &states {
        usage_from_state(state, set);
    }

    // Рекурсивно для вложенных моделей
    let borrowed = model.borrow();
    let nested: Vec<Rc<RefCell<ModelNode>>> = borrowed.models.values().map(Rc::clone).collect();
    drop(borrowed);

    for nested_model in nested {
        collect_model_usage(nested_model, set);
    }
}

/// Записывает имена из инициализатора переменной в соответствующие множества.
fn usage_from_var(var: &VariableNode, set: &mut UsageSet) {
    match var {
        VariableNode::Simple { expr, .. }
        | VariableNode::Port { expr, .. }
        | VariableNode::Const { expr, .. } => usage_from_expr(expr, set),
        VariableNode::Unresolved => {}
    }
}

/// Записывает имена из выражения в соответствующие множества.
fn usage_from_expr(expr: &ExpressionNode, set: &mut UsageSet) {
    match expr {
        ExpressionNode::Variable(var_rc) => {
            let borrowed = var_rc.borrow();
            match &*borrowed {
                VariableNode::Simple { name, .. } => {
                    set.variables.insert(name.clone());
                }
                VariableNode::Port { name, .. } => {
                    set.ports.insert(name.clone());
                }
                VariableNode::Const { name, .. } => {
                    set.constants.insert(name.clone());
                }
                VariableNode::Unresolved => {}
            }
        }
        ExpressionNode::ArraySubscript(var_rc, _) | ExpressionNode::ArraySlice(var_rc, _, _) => {
            let borrowed = var_rc.borrow();
            match &*borrowed {
                VariableNode::Simple { name, .. } => {
                    set.variables.insert(name.clone());
                }
                VariableNode::Port { name, .. } => {
                    set.ports.insert(name.clone());
                }
                VariableNode::Const { name, .. } => {
                    set.constants.insert(name.clone());
                }
                VariableNode::Unresolved => {}
            }
        }
        ExpressionNode::Function(func_rc, args) => {
            // Регистрируем использованную функцию
            let func_name = func_rc.borrow().name().to_string();
            if !func_name.is_empty() {
                set.functions.insert(func_name);
            }
            for arg in args {
                usage_from_expr(arg, set);
            }
        }
        ExpressionNode::Not(e)
        | ExpressionNode::BitwiseNot(e)
        | ExpressionNode::UnaryPlus(e)
        | ExpressionNode::Negate(e)
        | ExpressionNode::Parenthesis(e)
        | ExpressionNode::BitAccess(e, _)
        | ExpressionNode::Cast(e, _) => usage_from_expr(e, set),
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
            usage_from_expr(l, set);
            usage_from_expr(r, set);
        }
        ExpressionNode::ConditionalOperator(cond, then_e, else_e) => {
            usage_from_expr(cond, set);
            usage_from_expr(then_e, set);
            usage_from_expr(else_e, set);
        }
        ExpressionNode::Array(items) | ExpressionNode::Initializer(items) => {
            for item in items {
                usage_from_expr(item, set);
            }
        }
        _ => {}
    }
}

/// Записывает имена из оператора в соответствующие множества.
fn usage_from_stmt(stmt: &StatementNode, set: &mut UsageSet) {
    match stmt {
        StatementNode::Block(stmts) => {
            for s in stmts {
                usage_from_stmt(s, set);
            }
        }
        StatementNode::Expression(e) => usage_from_expr(e, set),
        StatementNode::If { cond, then_, else_ } => {
            usage_from_expr(cond, set);
            usage_from_stmt(then_, set);
            if let Some(e) = else_ {
                usage_from_stmt(e, set);
            }
        }
        StatementNode::Loop { cond, body } => {
            if let Some(c) = cond {
                usage_from_expr(c, set);
            }
            usage_from_stmt(body, set);
        }
        StatementNode::For {
            init,
            cond,
            step,
            body,
        } => {
            if let Some(s) = init {
                usage_from_stmt(s, set);
            }
            if let Some(c) = cond {
                usage_from_expr(c, set);
            }
            if let Some(s) = step {
                usage_from_expr(s, set);
            }
            usage_from_stmt(body, set);
        }
        StatementNode::Variable(_, _, Some(e)) => usage_from_expr(e, set),
        StatementNode::Return(Some(e)) => usage_from_expr(e, set),
        _ => {}
    }
}

/// Записывает имена из условия в соответствующие множества.
fn usage_from_condition(cond: &ConditionNode, set: &mut UsageSet) {
    match cond {
        ConditionNode::Variable(var_rc, _) => {
            let borrowed = var_rc.borrow();
            match &*borrowed {
                VariableNode::Simple { name, .. } => {
                    set.variables.insert(name.clone());
                }
                VariableNode::Port { name, .. } => {
                    set.ports.insert(name.clone());
                }
                VariableNode::Const { name, .. } => {
                    set.constants.insert(name.clone());
                }
                VariableNode::Unresolved => {}
            }
        }
        ConditionNode::Not(c) | ConditionNode::Parenthesis(c) => usage_from_condition(c, set),
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
            usage_from_condition(l, set);
            usage_from_condition(r, set);
        }
        ConditionNode::Function(func_rc, args, _) => {
            // Регистрируем использованную функцию
            let func_name = func_rc.borrow().name().to_string();
            if !func_name.is_empty() {
                set.functions.insert(func_name);
            }
            for arg in args {
                usage_from_condition(arg, set);
            }
        }
        ConditionNode::ArraySubscript(var_rc, _) => {
            let borrowed = var_rc.borrow();
            match &*borrowed {
                VariableNode::Simple { name, .. } => {
                    set.variables.insert(name.clone());
                }
                VariableNode::Port { name, .. } => {
                    set.ports.insert(name.clone());
                }
                VariableNode::Const { name, .. } => {
                    set.constants.insert(name.clone());
                }
                VariableNode::Unresolved => {}
            }
        }
        _ => {}
    }
}

/// Записывает имена из именованного условия в соответствующие множества.
fn usage_from_condition_node(cond_node: &ConditionDefinitionNode, set: &mut UsageSet) {
    usage_from_condition(&cond_node.value, set);
}

/// Записывает имена из именованного блока кода в соответствующие множества.
fn usage_from_named_block(block: &NamedCodeBlockDefinitionNode, set: &mut UsageSet) {
    if let Some(stmt) = block.statement() {
        usage_from_stmt(stmt, set);
    }
}

/// Записывает имена из тела функции в соответствующие множества.
fn usage_from_func(func: &FunctionDefinitionNode, set: &mut UsageSet) {
    if let FunctionDefinitionNode::Local { body, .. } = func {
        usage_from_stmt(body, set);
    }
}

/// Записывает имена из состояния в соответствующие множества.
fn usage_from_state(state: &crate::semantic::StateNode, set: &mut UsageSet) {
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
                usage_from_named_block(block, set);
            }
            for reference in references {
                usage_from_condition(&reference.cond, set);
            }
        }
        StateNode::Unresolved => {}
    }
}

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
            warnings.push(
                Diagnostic::warning(
                    var.loc(),
                    format!(
                        "переменная '{}' объявлена, но нигде не используется",
                        name
                    ),
                )
                .with_code("SE-036"),
            );
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
