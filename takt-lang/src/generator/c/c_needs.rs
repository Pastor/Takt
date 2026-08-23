//! Нужен ли порождённой функции указатель на состояние (фича 0396).
//!
//! # Что было
//!
//! Протокол вызова цели `c` единообразен: пользовательская функция получает
//! `model`, под-модель — `main`. Тело пользуется ими не всегда, и фича 0260
//! погасила предупреждения заглушкой `(void)параметр;` — параметр в сигнатуре
//! остался. Замер 2026-08-22 по корпусу: **53** таких места.
//!
//! Класс **косметический** (оба гейта цели зелены: заглушка делает вывод
//! валидным), цена — в интерфейсе и читаемости порождённого кода.
//!
//! # Правило
//!
//! Указатель нужен функции, если её тело обращается к состоянию модели —
//! читает или пишет переменную, порт, зовёт `debug`/`extern fn` — **либо**
//! зовёт функцию, которой он нужен.
//!
//! ⚠️ **Носитель признака ОДИН на две цели.** Тот же вопрос решает
//! `rust_needs::function_needs` (фича 0050): у цели `rust` он раскладывается
//! на «нужен ли HAL» и «какие переменные передать», у `c` обе нужды сводятся к
//! одному указателю. Своё знание здесь разошлось бы с тем — класс
//! 0084/0193/0195, ровно тот, о котором предупреждает ADR 0396.
//!
//! ⚠️ **Транзитивность бесплатна:** рекурсия в языке запрещена (`SE-053`),
//! граф вызовов — ДАГ, и предикат считается одним обходом снизу вверх.

use std::cell::RefCell;
use std::collections::{BTreeSet, HashSet};
use std::rc::Rc;

use crate::diagnostics::Diagnostic;
use crate::semantic::{
    ConditionNode, ExpressionNode, FunctionDefinitionNode, ModelNode, StatementNode, VariableNode,
};

/// Нужен ли функции указатель на состояние.
///
/// `false` у `extern fn` и встроенных: тела у них нет, и передавать нечего.
///
/// # Ошибки
///
/// Диагностика носителя (`rust_needs`), если тело не разбирается.
pub(in crate::generator::c) fn needs_state(
    def: &FunctionDefinitionNode,
    model: &ModelNode,
) -> Result<bool, Diagnostic> {
    let mut seen = BTreeSet::new();
    let needs = crate::generator::rust::rust_needs::function_needs(def, model, &mut seen)?;
    // У цели `c` обе нужды выражены одним указателем: и порт, и переменная
    // модели живут за `model`/`main`.
    Ok(needs.hal || !needs.vars.is_empty())
}

/// Нужен ли ПОД-МОДЕЛИ указатель на корень (`main`).
///
/// Нужен, если её тела обращаются к объявлению, живущему **выше** по дереву
/// (общая переменная корня, его порт), либо если он нужен любой её
/// под-модели — та получает `main` от неё же.
///
/// ⚠️ Признак считается по СЕМАНТИКЕ, а не по напечатанному тексту (в отличие
/// от заглушки 0260): сигнатура печатается раньше тела и в трёх местах —
/// прототип, определение, вызов, — и текст к тому моменту ещё не готов.
///
/// ⚠️ Ошибка признака **громкая**: сказав «не нужен» там, где тело `main`
/// упоминает, получим отказ `cc` («undeclared identifier»), а не молчание.
/// Гейт корпуса гоняет `cc` по всем примерам и такой промах поймает.
pub(in crate::generator::c) fn model_needs_root(model: &Rc<RefCell<ModelNode>>) -> bool {
    let mut seen = HashSet::new();
    needs_root_inner(model, &mut seen)
}

fn needs_root_inner(
    model: &Rc<RefCell<ModelNode>>,
    seen: &mut HashSet<*const RefCell<ModelNode>>,
) -> bool {
    if !seen.insert(Rc::as_ptr(model)) {
        return false; // разделяемая под-модель уже учтена
    }
    let b = model.borrow();
    let mut needed = false;
    for block in &b.named_blocks {
        if let Some(stmt) = block.statement() {
            needed |= stmt_touches_outside(stmt, model);
        }
    }
    for state in b.states.values() {
        for block in state.named_blocks() {
            if let Some(stmt) = block.statement() {
                needed |= stmt_touches_outside(stmt, model);
            }
        }
        for reference in state.references() {
            needed |= cond_touches_outside(&reference.cond, model);
        }
    }
    for cond in b.conditions.values() {
        needed |= cond_touches_outside(&cond.value, model);
    }
    for func in b.functions.values() {
        if let FunctionDefinitionNode::Local { body, .. } = func {
            needed |= stmt_touches_outside(body, model);
        }
    }
    let nested: Vec<Rc<RefCell<ModelNode>>> = b.models.values().cloned().collect();
    drop(b);
    for child in &nested {
        // Под-модель получает `main` от этой: нужен ей — нужен и здесь.
        needed |= needs_root_inner(child, seen);
    }
    needed
}

/// Обращается ли оператор к объявлению вне модели `owner`.
///
/// ⚠️ Обход **не** исчерпывающий по узлам, и это осознанно: пропущенная форма
/// даёт «параметр нужен» — прежнее поведение, то есть ошибку в безопасную
/// сторону. Обратная ошибка была бы отказом `cc`, и её ловит гейт корпуса.
fn stmt_touches_outside(stmt: &StatementNode, owner: &Rc<RefCell<ModelNode>>) -> bool {
    match stmt {
        StatementNode::Block(items) => items.iter().any(|s| stmt_touches_outside(s, owner)),
        StatementNode::Expression(expr, _) => expr_touches_outside(expr, owner),
        StatementNode::If { cond, then_, else_ } => {
            expr_touches_outside(cond, owner)
                || stmt_touches_outside(then_, owner)
                || else_
                    .as_ref()
                    .is_some_and(|alt| stmt_touches_outside(alt, owner))
        }
        StatementNode::Loop { cond, body } => {
            cond.as_ref()
                .is_some_and(|c| expr_touches_outside(c, owner))
                || stmt_touches_outside(body, owner)
        }
        StatementNode::For {
            init,
            cond,
            step,
            body,
        } => {
            init.as_ref()
                .is_some_and(|i| stmt_touches_outside(i, owner))
                || cond
                    .as_ref()
                    .is_some_and(|c| expr_touches_outside(c, owner))
                || step
                    .as_ref()
                    .is_some_and(|s| expr_touches_outside(s, owner))
                || stmt_touches_outside(body, owner)
        }
        StatementNode::Match { expr, arms } => {
            expr_touches_outside(expr, owner)
                || arms.iter().any(|a| stmt_touches_outside(&a.body, owner))
        }
        StatementNode::Return(Some(expr)) => expr_touches_outside(expr, owner),
        StatementNode::Variable(_, _, Some(init), _) => expr_touches_outside(init, owner),
        _ => false,
    }
}

/// То же для выражения.
fn expr_touches_outside(expr: &ExpressionNode, owner: &Rc<RefCell<ModelNode>>) -> bool {
    match expr {
        ExpressionNode::Variable(cell) => cell_is_outside(cell, owner),
        ExpressionNode::Parenthesis(a)
        | ExpressionNode::Not(a)
        | ExpressionNode::Negate(a)
        | ExpressionNode::BitwiseNot(a)
        | ExpressionNode::UnaryPlus(a)
        | ExpressionNode::Cast(a, _)
        | ExpressionNode::BitAccess(a, _)
        | ExpressionNode::ArraySlice(a, _, _)
        | ExpressionNode::NamedFunctionBox(a, _) => expr_touches_outside(a, owner),
        ExpressionNode::Add(a, b)
        | ExpressionNode::Subtract(a, b)
        | ExpressionNode::Multiply(a, b)
        | ExpressionNode::Divide(a, b)
        | ExpressionNode::Modulo(a, b)
        | ExpressionNode::Power(a, b)
        | ExpressionNode::ShiftLeft(a, b)
        | ExpressionNode::ShiftRight(a, b)
        | ExpressionNode::BitwiseAnd(a, b)
        | ExpressionNode::BitwiseXor(a, b)
        | ExpressionNode::BitwiseOr(a, b)
        | ExpressionNode::Less(a, b)
        | ExpressionNode::More(a, b)
        | ExpressionNode::LessEqual(a, b)
        | ExpressionNode::MoreEqual(a, b)
        | ExpressionNode::Equal(a, b)
        | ExpressionNode::NotEqual(a, b)
        | ExpressionNode::And(a, b)
        | ExpressionNode::Or(a, b)
        | ExpressionNode::Assign(a, b)
        | ExpressionNode::ArraySubscript(a, b) => {
            expr_touches_outside(a, owner) || expr_touches_outside(b, owner)
        }
        ExpressionNode::ConditionalOperator(a, b, c) => {
            expr_touches_outside(a, owner)
                || expr_touches_outside(b, owner)
                || expr_touches_outside(c, owner)
        }
        ExpressionNode::Array(items)
        | ExpressionNode::Initializer(items)
        | ExpressionNode::Function(_, items) => {
            items.iter().any(|i| expr_touches_outside(i, owner))
        }
        ExpressionNode::CodeBlock(e, stmt) => {
            expr_touches_outside(e, owner) || stmt_touches_outside(stmt, owner)
        }
        // Обращение по адресу и порт-литерал живут вне модели по существу:
        // доступ к ним идёт через корень.
        ExpressionNode::AnonPort(_) | ExpressionNode::Address(_, _) => true,
        _ => false,
    }
}

/// То же для условия.
fn cond_touches_outside(cond: &ConditionNode, owner: &Rc<RefCell<ModelNode>>) -> bool {
    match cond {
        ConditionNode::Variable(cell, _) => cell_is_outside(cell, owner),
        ConditionNode::Parenthesis(a)
        | ConditionNode::Not(a)
        | ConditionNode::BitAccess(a, _)
        | ConditionNode::AfterExpr(a) => cond_touches_outside(a, owner),
        ConditionNode::Add(a, b)
        | ConditionNode::Subtract(a, b)
        | ConditionNode::And(a, b)
        | ConditionNode::Or(a, b)
        | ConditionNode::Less(a, b)
        | ConditionNode::More(a, b)
        | ConditionNode::LessEqual(a, b)
        | ConditionNode::MoreEqual(a, b)
        | ConditionNode::Equal(a, b)
        | ConditionNode::NotEqual(a, b)
        | ConditionNode::ArraySubscript(a, b) => {
            cond_touches_outside(a, owner) || cond_touches_outside(b, owner)
        }
        ConditionNode::Function(_, args, _) => args.iter().any(|a| cond_touches_outside(a, owner)),
        // Состояние СОСЕДА читается через корень (0267), обращение по адресу —
        // тоже: обе формы требуют `main`.
        ConditionNode::Model(_, _) | ConditionNode::State(..) | ConditionNode::AnonPort(_) => true,
        _ => false,
    }
}

/// Живёт ли объявление ячейки ВЫШЕ модели `owner`.
///
/// `false` у ячейки без владельца: гадать нельзя, а прежнее поведение
/// (параметр печатается) безопаснее.
fn cell_is_outside(cell: &Rc<RefCell<VariableNode>>, owner: &Rc<RefCell<ModelNode>>) -> bool {
    // ⚠️ ПОРТ требует указателя на корень ВСЕГДА, даже объявленный в самой
    // под-модели: колбэки HAL (`write_numeric`, `userdata`) живут в структуре
    // корня, и доступ печатается `main->write_numeric(…)`. Замер 2026-08-23:
    // без этой ветви порождённый C не компилировался — «use of undeclared
    // identifier 'main'» (поймал сторож 0184).
    if matches!(&*cell.borrow(), VariableNode::Port { .. }) {
        return true;
    }
    let Some(holder) = cell.borrow().upper() else {
        return false;
    };
    !Rc::ptr_eq(&holder, owner)
}
