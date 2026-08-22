//! Понижение q-литерала в ТЕЛАХ и УСЛОВИЯХ (фича 0381).
//!
//! # Что было
//!
//! Дробный литерал понижался в представление `q(m, n)` только в **объявлении**
//! (`var gain: q(8, 8) := 1.5;` → `Number(384)`, фича 0061 и её продолжения
//! 0368/0370). В теле и в условии литерал доезжал до целей **как написан**, и
//! замер 2026-08-22 дал расхождение **значений**:
//!
//! | Запись | эталон | цель `c` |
//! |---|---|---|
//! | `gain := 2.0;` | `2.0` | `model->gain = 2.0;` → **2**, а не 512 |
//! | `take(3.0)` при параметре `q(8, 8)` | `3.0` | `Qpos_take(model, 3.0)` → **3** |
//! | `ref Done: gain > 1.0;` при `gain = 0.5` | ребро НЕ срабатывает | `128 > 1.0` → **срабатывает** |
//!
//! Последняя строка — **другой автомат** при нулевом коде возврата `taktc`.
//! Цели `st` и `sv` тот же вход отвергают (`iec2c`: «Incompatible data types»,
//! `SV-002`), то есть потребители расходились ещё и между собой.
//!
//! # Правило
//!
//! Литерал понижается там, где **известен тип приёмника**: присваивание,
//! сравнение с q-операндом, аргумент вызова, возврат функции. Это те же
//! «позиции приёмника», что у 0335/0336, только вопрос решается **в
//! семантике** — за её границей формы не существует, и потребителям не по чему
//! расходиться (приём 0143/0185/0192).
//!
//! ⚠️ **Условие ребра приходит СЫРЫМ АСД** (`Condition::Unresolved`,
//! инвариант проекта): цели печатают его, разрешая имена против модели. Поэтому
//! обход идёт и по `ast::Condition` — тип операнда там берётся **по имени** из
//! карты переменных, ровно как это делает цель.
//!
//! ⚠️ Носитель представления — прежний `lower_fixed_literal` (0061): второго
//! знания о масштабе 2ⁿ не заводится.

use std::cell::RefCell;
use std::collections::HashSet;
use std::rc::Rc;

use crate::diagnostics::{Diagnostic, Location};
use crate::parser::ast;
use crate::semantic::type_node::TypeNode;
use crate::semantic::type_node::type_fixed::lower_fixed_literal;
use crate::semantic::{
    ConditionNode, ExpressionNode, FunctionDefinitionNode, ModelNode, NamedCodeBlockDefinitionNode,
    StateNode, StatementNode, VariableNode,
};

/// Понижает q-литералы в телах и условиях модели и её под-моделей.
pub(crate) fn lower_fixed_literals(model: &Rc<RefCell<ModelNode>>) -> Result<(), Diagnostic> {
    let mut visited = HashSet::new();
    lower_model(model, &mut visited)
}

fn lower_model(
    model: &Rc<RefCell<ModelNode>>,
    visited: &mut HashSet<*const RefCell<ModelNode>>,
) -> Result<(), Diagnostic> {
    if !visited.insert(Rc::as_ptr(model)) {
        return Ok(()); // разделяемая под-модель уже обработана
    }
    let nested: Vec<Rc<RefCell<ModelNode>>> = model.borrow().models.values().cloned().collect();
    {
        // Снимок типов переменных: обход условий-АСД спрашивает тип ПО ИМЕНИ —
        // так же, как это делает цель, печатая неразрешённое условие ребра.
        let names = fixed_names(&model.borrow());
        let mut b = model.borrow_mut();
        for func in b.functions.values_mut() {
            lower_function(func, &names)?;
        }
        for blk in b.named_blocks.iter_mut() {
            lower_block(blk, &names)?;
        }
        for cond in b.conditions.values_mut() {
            lower_condition(&mut cond.value, &names)?;
        }
        for state in b.states.values_mut() {
            lower_state(state, &names)?;
        }
    }
    for child in &nested {
        lower_model(child, visited)?;
    }
    Ok(())
}

/// Имена переменных q-типа и их формат.
fn fixed_names(model: &ModelNode) -> Vec<(String, (u8, u8))> {
    model
        .variables
        .iter()
        .filter_map(|(name, var)| match var {
            VariableNode::Simple {
                ty: TypeNode::Fixed { m, n, .. },
                ..
            }
            | VariableNode::Const {
                ty: TypeNode::Fixed { m, n, .. },
                ..
            }
            | VariableNode::Port {
                ty: TypeNode::Fixed { m, n, .. },
                ..
            } => Some((name.clone(), (*m, *n))),
            _ => None,
        })
        .collect()
}

fn lower_function(
    func: &mut FunctionDefinitionNode,
    names: &[(String, (u8, u8))],
) -> Result<(), Diagnostic> {
    let FunctionDefinitionNode::Local { ret, body, .. } = func else {
        return Ok(());
    };
    let ret_fixed = fixed_of_type(ret);
    lower_stmt(body, names, ret_fixed)
}

fn lower_block(
    blk: &mut NamedCodeBlockDefinitionNode,
    names: &[(String, (u8, u8))],
) -> Result<(), Diagnostic> {
    match blk {
        NamedCodeBlockDefinitionNode::Enter { body, .. }
        | NamedCodeBlockDefinitionNode::Exit { body, .. }
        | NamedCodeBlockDefinitionNode::Always { body, .. }
        | NamedCodeBlockDefinitionNode::Unknown { body, .. }
        | NamedCodeBlockDefinitionNode::Every { body, .. } => lower_stmt(body, names, None),
        // Пустой и неразрешённый блоки тела не несут.
        NamedCodeBlockDefinitionNode::None | NamedCodeBlockDefinitionNode::Unresolved(_, _) => {
            Ok(())
        }
    }
}

fn lower_state(state: &mut StateNode, names: &[(String, (u8, u8))]) -> Result<(), Diagnostic> {
    let (blocks, refs) = match state {
        StateNode::Simple {
            named_blocks,
            references,
            ..
        }
        | StateNode::Implement {
            named_blocks,
            references,
            ..
        } => (named_blocks, references),
        StateNode::Unresolved => return Ok(()),
    };
    for blk in blocks.iter_mut() {
        lower_block(blk, names)?;
    }
    // Условие ребра `ref` — сырой АСД (инвариант проекта), и цели печатают его,
    // разрешая имена против модели: понижение обязано идти туда же.
    for reference in refs.iter_mut() {
        lower_condition(&mut reference.cond, names)?;
    }
    Ok(())
}

/// Формат `q(m, n)` у типа, если он такой.
fn fixed_of_type(ty: &TypeNode) -> Option<(u8, u8)> {
    match ty {
        TypeNode::Fixed { m, n, .. } => Some((*m, *n)),
        _ => None,
    }
}

/// Формат q у ИМЕНОВАННОГО значения выражения.
fn fixed_of_expr(expr: &ExpressionNode) -> Option<(u8, u8)> {
    match expr {
        ExpressionNode::Variable(cell) => match &*cell.borrow() {
            VariableNode::Simple { ty, .. }
            | VariableNode::Const { ty, .. }
            | VariableNode::Port { ty, .. } => fixed_of_type(ty),
            VariableNode::Unresolved => None,
        },
        ExpressionNode::Parenthesis(inner) => fixed_of_expr(inner),
        _ => None,
    }
}

/// Понижает литерал на месте, если он числовой; прочее не трогает.
fn lower_literal(expr: &mut ExpressionNode, (m, n): (u8, u8)) -> Result<(), Diagnostic> {
    if !matches!(
        expr,
        ExpressionNode::Number(_) | ExpressionNode::Rational(_, _)
    ) {
        return Ok(());
    }
    if let Some(v) = lower_fixed_literal(expr, m, n, Location::Codegen)? {
        *expr = ExpressionNode::Number(v);
    }
    Ok(())
}

fn lower_stmt(
    stmt: &mut StatementNode,
    names: &[(String, (u8, u8))],
    ret: Option<(u8, u8)>,
) -> Result<(), Diagnostic> {
    match stmt {
        StatementNode::Block(items) => {
            for s in items.iter_mut() {
                lower_stmt(s, names, ret)?;
            }
        }
        StatementNode::Expression(e, _) => lower_expr(e, names)?,
        StatementNode::If { cond, then_, else_ } => {
            lower_expr(cond, names)?;
            lower_stmt(then_, names, ret)?;
            if let Some(e) = else_ {
                lower_stmt(e, names, ret)?;
            }
        }
        StatementNode::Loop { cond, body } => {
            if let Some(c) = cond {
                lower_expr(c, names)?;
            }
            lower_stmt(body, names, ret)?;
        }
        StatementNode::For {
            init,
            cond,
            step,
            body,
        } => {
            if let Some(s) = init {
                lower_stmt(s, names, ret)?;
            }
            if let Some(c) = cond {
                lower_expr(c, names)?;
            }
            if let Some(s) = step {
                lower_expr(s, names)?;
            }
            lower_stmt(body, names, ret)?;
        }
        // Локальное объявление: приёмник — объявленный тип.
        StatementNode::Variable(_, ty, init) => {
            let fixed = fixed_of_type(ty);
            if let Some(e) = init {
                lower_expr(e, names)?;
                if let Some(f) = fixed {
                    lower_literal(e, f)?;
                }
            }
        }
        // Возврат: приёмник — объявленный тип функции.
        StatementNode::Return(Some(e)) => {
            lower_expr(e, names)?;
            if let Some(f) = ret {
                lower_literal(e, f)?;
            }
        }
        StatementNode::Match { expr, arms } => {
            lower_expr(expr, names)?;
            for arm in arms.iter_mut() {
                lower_stmt(&mut arm.body, names, ret)?;
            }
        }
        StatementNode::None
        | StatementNode::Unresolved(_)
        | StatementNode::Return(None)
        | StatementNode::Continue
        | StatementNode::Break
        | StatementNode::InlineFormula(_) => {}
    }
    Ok(())
}

/// Понижает литералы в выражении: присваивание, сравнение, аргумент вызова.
fn lower_expr(expr: &mut ExpressionNode, names: &[(String, (u8, u8))]) -> Result<(), Diagnostic> {
    // Сначала вглубь: приёмник виден на своём уровне.
    for child in children_mut(expr) {
        lower_expr(child, names)?;
    }
    match expr {
        ExpressionNode::Assign(target, value) => {
            if let Some(f) = fixed_of_expr(target) {
                lower_literal(value, f)?;
            }
        }
        ExpressionNode::Equal(l, r)
        | ExpressionNode::NotEqual(l, r)
        | ExpressionNode::Less(l, r)
        | ExpressionNode::More(l, r)
        | ExpressionNode::LessEqual(l, r)
        | ExpressionNode::MoreEqual(l, r) => {
            // ⚠️ **Арифметика сюда НЕ входит:** `gain + 1.5` отвергает `SE-059`
            // (неявное смешение q и float), и понижение литерала там лишь
            // ухудшало бы сообщение — вместо «'q(8, 8)' и 'float'» автор
            // получал «'q(8, 8)' и '[bit;16]'», то есть тип, которого он не
            // писал. Сравнение — другое дело: оно законно и работает.
            match (fixed_of_expr(l), fixed_of_expr(r)) {
                (Some(f), None) => lower_literal(r, f)?,
                (None, Some(f)) => lower_literal(l, f)?,
                _ => {}
            }
        }
        ExpressionNode::Function(def, args) => {
            let params: Vec<Option<(u8, u8)>> = match &*def.borrow() {
                FunctionDefinitionNode::Local { params, .. } => {
                    params.iter().map(|(_, ty)| fixed_of_type(ty)).collect()
                }
                _ => Vec::new(),
            };
            for (arg, param) in args.iter_mut().zip(params) {
                if let Some(f) = param {
                    lower_literal(arg, f)?;
                }
            }
        }
        _ => {}
    }
    let _ = names;
    Ok(())
}

/// Понижает литералы в РАЗРЕШЁННОМ условии (`cond`, формулы).
fn lower_condition(
    cond: &mut ConditionNode,
    names: &[(String, (u8, u8))],
) -> Result<(), Diagnostic> {
    use ConditionNode as C;
    match cond {
        // Ребро `ref` хранит СЫРОЙ АСД (инвариант проекта), `cond` — уже
        // разрешённое условие: путей два, и правило у них одно.
        C::Unresolved(raw) => lower_ast_condition(raw, names),
        C::Equal(l, r)
        | C::NotEqual(l, r)
        | C::Less(l, r)
        | C::More(l, r)
        | C::LessEqual(l, r)
        | C::MoreEqual(l, r) => {
            lower_condition(l, names)?;
            lower_condition(r, names)?;
            match (cond_fixed_of(l), cond_fixed_of(r)) {
                (Some(f), None) => lower_cond_literal(r, f),
                (None, Some(f)) => lower_cond_literal(l, f),
                _ => Ok(()),
            }
        }
        C::And(l, r) | C::Or(l, r) => {
            lower_condition(l, names)?;
            lower_condition(r, names)
        }
        C::Not(inner) | C::Parenthesis(inner) => lower_condition(inner, names),
        _ => Ok(()),
    }
}

/// Формат q у именованного значения РАЗРЕШЁННОГО условия.
fn cond_fixed_of(cond: &ConditionNode) -> Option<(u8, u8)> {
    match cond {
        ConditionNode::Variable(cell, _) => match &*cell.borrow() {
            VariableNode::Simple { ty, .. }
            | VariableNode::Const { ty, .. }
            | VariableNode::Port { ty, .. } => fixed_of_type(ty),
            VariableNode::Unresolved => None,
        },
        ConditionNode::Parenthesis(inner) => cond_fixed_of(inner),
        _ => None,
    }
}

/// Понижает числовой литерал разрешённого условия.
fn lower_cond_literal(cond: &mut ConditionNode, (m, n): (u8, u8)) -> Result<(), Diagnostic> {
    let expr = match cond {
        ConditionNode::Number(k) => ExpressionNode::Number(*k),
        ConditionNode::Rational(s, neg) => ExpressionNode::Rational(s.clone(), *neg),
        _ => return Ok(()),
    };
    if let Some(v) = lower_fixed_literal(&expr, m, n, Location::Codegen)? {
        *cond = ConditionNode::Number(v);
    }
    Ok(())
}

/// Понижает литералы в СЫРОМ условии АСД (ребро `ref`, инвариант проекта).
///
/// Тип операнда берётся **по имени** из карты переменных — так же, как это
/// делает цель, печатая неразрешённое условие.
fn lower_ast_condition(
    cond: &mut ast::Condition,
    names: &[(String, (u8, u8))],
) -> Result<(), Diagnostic> {
    use ast::Condition as C;
    match cond {
        C::Equal(_, l, r)
        | C::NotEqual(_, l, r)
        | C::Less(_, l, r)
        | C::More(_, l, r)
        | C::LessEqual(_, l, r)
        | C::MoreEqual(_, l, r) => {
            lower_ast_condition(l, names)?;
            lower_ast_condition(r, names)?;
            match (ast_fixed_of(l, names), ast_fixed_of(r, names)) {
                (Some(f), None) => lower_ast_literal(r, f)?,
                (None, Some(f)) => lower_ast_literal(l, f)?,
                _ => {}
            }
        }
        C::And(_, l, r) | C::Or(_, l, r) => {
            lower_ast_condition(l, names)?;
            lower_ast_condition(r, names)?;
        }
        C::Not(_, inner) | C::Parenthesis(_, inner) | C::AfterExpr(_, inner) => {
            lower_ast_condition(inner, names)?;
        }
        _ => {}
    }
    Ok(())
}

/// Формат q у имени в сыром условии.
fn ast_fixed_of(cond: &ast::Condition, names: &[(String, (u8, u8))]) -> Option<(u8, u8)> {
    match cond {
        ast::Condition::Variable(id) => names
            .iter()
            .find(|(name, _)| *name == id.name)
            .map(|(_, f)| *f),
        ast::Condition::Parenthesis(_, inner) => ast_fixed_of(inner, names),
        _ => None,
    }
}

/// Понижает числовой литерал сырого условия.
fn lower_ast_literal(cond: &mut ast::Condition, (m, n): (u8, u8)) -> Result<(), Diagnostic> {
    let (loc, expr) = match cond {
        ast::Condition::Number(loc, k) => (*loc, ExpressionNode::Number(*k)),
        ast::Condition::Rational(loc, s, neg) => (*loc, ExpressionNode::Rational(s.clone(), *neg)),
        _ => return Ok(()),
    };
    if let Some(v) = lower_fixed_literal(&expr, m, n, loc)? {
        *cond = ast::Condition::Number(loc, v);
    }
    Ok(())
}

/// Дети выражения — для спуска вглубь.
fn children_mut(expr: &mut ExpressionNode) -> Vec<&mut ExpressionNode> {
    use ExpressionNode as E;
    match expr {
        E::Assign(a, b)
        | E::Equal(a, b)
        | E::NotEqual(a, b)
        | E::Less(a, b)
        | E::More(a, b)
        | E::LessEqual(a, b)
        | E::MoreEqual(a, b)
        | E::Add(a, b)
        | E::Subtract(a, b)
        | E::Multiply(a, b)
        | E::Divide(a, b)
        | E::Modulo(a, b)
        | E::Power(a, b)
        | E::And(a, b)
        | E::Or(a, b)
        | E::BitwiseAnd(a, b)
        | E::BitwiseOr(a, b)
        | E::BitwiseXor(a, b)
        | E::ShiftLeft(a, b)
        | E::ShiftRight(a, b)
        | E::ArraySubscript(a, b) => vec![a, b],
        E::Parenthesis(a) | E::Not(a) | E::BitwiseNot(a) | E::Negate(a) | E::Cast(a, _) => vec![a],
        E::Function(_, args) => args.iter_mut().collect(),
        _ => Vec::new(),
    }
}
