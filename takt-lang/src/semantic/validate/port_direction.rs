//! Направление порта проверяется **во всех** позициях (фича 0188).
//!
//! ## Зачем
//!
//! Язык объявляет: во входной порт писать нельзя (`SE-026`), из выходного читать
//! нельзя (`SE-027`). Правило было реализовано, но зовётся только из проверок
//! **условий** — тела блоков, тела функций и инициализаторы не обходились.
//!
//! Проба (2026-07-31) на входе `always { A := 5; t := B; }` при `in A`, `out B`:
//! диагностики нет, а шесть потребителей расходятся — `rust` отказывает
//! (`RS-018`), `c`/`st`/симулятор молча исполняют, `sv` печатает присваивание
//! входному порту модуля (невалидный SystemVerilog), а `c-hal` берёт индекс
//! перечисления из **чужой** таблицы (нумерация идёт внутри направления) и пишет
//! **по адресу другого порта**. Гейт `cc -c` этот код принимает.
//!
//! ## Как устроено
//!
//! Модуль **не решает заново**, что законно: он лишь доставляет выражения
//! существующему судье [`super::common::validate_expression`]. Иначе правило
//! оказалось бы в двух местах и разошлось бы — ровно тот дефект, который фича
//! закрывает.
//!
//! Накопление — по выражениям (одна ошибка на выражение), по образцу
//! `literal_range` (фича 0157): редактор подчёркивает все нарушения, а не первое.

use crate::diagnostics::Diagnostic;
use crate::semantic::{
    ExpressionNode, FunctionDefinitionNode, ModelNode, StateNode, StatementNode, VariableNode,
};
use std::cell::RefCell;
use std::rc::Rc;

/// Проверяет направление портов во всех телах модели и её под-моделей.
pub(super) fn check_port_directions(model: Rc<RefCell<ModelNode>>) -> Vec<Diagnostic> {
    let mut found = Vec::new();
    check_model(&model, &mut found);
    found
}

fn check_model(model: &Rc<RefCell<ModelNode>>, found: &mut Vec<Diagnostic>) {
    let (vars, funcs, blocks, states, nested) = {
        let b = model.borrow();
        (
            b.variables.values().cloned().collect::<Vec<_>>(),
            b.functions.values().cloned().collect::<Vec<_>>(),
            b.named_blocks.clone(),
            b.states.values().cloned().collect::<Vec<_>>(),
            b.models.values().map(Rc::clone).collect::<Vec<_>>(),
        )
    };

    // Инициализатор переменной: `var x: u8 := led;` — тоже чтение выхода.
    for var in &vars {
        if let VariableNode::Simple { expr, .. } | VariableNode::Const { expr, .. } = var {
            check_expr(expr, model, found);
        }
    }
    for func in &funcs {
        if let FunctionDefinitionNode::Local { body, .. } = func {
            check_stmt(body, model, found);
        }
    }
    for block in &blocks {
        if let Some(stmt) = block.statement() {
            check_stmt(stmt, model, found);
        }
    }
    for state in &states {
        let named_blocks = match state {
            StateNode::Simple { named_blocks, .. } | StateNode::Implement { named_blocks, .. } => {
                named_blocks
            }
            StateNode::Unresolved => continue,
        };
        for block in named_blocks {
            if let Some(stmt) = block.statement() {
                check_stmt(stmt, model, found);
            }
        }
    }
    for child in &nested {
        check_model(child, found);
    }
}

/// Обход оператора: каждое выражение отдаётся судье направления.
fn check_stmt(stmt: &StatementNode, model: &Rc<RefCell<ModelNode>>, found: &mut Vec<Diagnostic>) {
    match stmt {
        StatementNode::Block(items) => {
            for item in items {
                check_stmt(item, model, found);
            }
        }
        StatementNode::Expression(expr) => check_expr(expr, model, found),
        StatementNode::If { cond, then_, else_ } => {
            check_expr(cond, model, found);
            check_stmt(then_, model, found);
            if let Some(other) = else_ {
                check_stmt(other, model, found);
            }
        }
        StatementNode::Loop { cond, body } => {
            if let Some(cond) = cond {
                check_expr(cond, model, found);
            }
            check_stmt(body, model, found);
        }
        StatementNode::For {
            init,
            cond,
            step,
            body,
        } => {
            if let Some(init) = init {
                check_stmt(init, model, found);
            }
            if let Some(cond) = cond {
                check_expr(cond, model, found);
            }
            if let Some(step) = step {
                check_expr(step, model, found);
            }
            check_stmt(body, model, found);
        }
        StatementNode::Variable(_, _, Some(expr)) => check_expr(expr, model, found),
        StatementNode::Return(Some(expr)) => check_expr(expr, model, found),
        StatementNode::Match { expr, arms } => {
            check_expr(expr, model, found);
            for arm in arms {
                check_stmt(&arm.body, model, found);
            }
        }
        // Прочие операторы выражений не несут: объявление без инициализатора,
        // `break`/`continue`, пустой `return`, встроенная формула, `None` и
        // неразрешённый АСД-оператор.
        _ => {}
    }
}

/// Отдаёт выражение судье направления; ошибку копит, а не прерывает обход.
fn check_expr(expr: &ExpressionNode, model: &Rc<RefCell<ModelNode>>, found: &mut Vec<Diagnostic>) {
    if let Err(diagnostic) = super::common::validate_expression(expr, Rc::clone(model)) {
        found.push(diagnostic);
    }
}
