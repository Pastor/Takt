//! Диагностики LTL-формул (фича 0035): предупреждения о неверифицируемых
//! формулах и неизвестных атомах.
//!
//! LTL-формула `: [LTL] φ;` разбирается на всех трёх уровнях (модель, состояние,
//! блок кода — паритет обеспечен 0035-01), но **не верифицируется**: `build_buchi`
//! из исходника не вызывается ниоткуда, а генератор C её не эмитит. Чтобы ни один
//! путь LTL не заканчивался тишиной (принцип «громкий отказ», ADR 0025/0024),
//! здесь собираются предупреждения:
//!
//! - **SE-055** — на КАЖДУЮ LTL-формулу: разобрана и сохранена, но верификация
//!   LTL не реализована (ни одна цель её не проверяет).
//! - **SE-056** — на атом формулы, не совпадающий ни с одним именем модели
//!   (переменная/порт/константа, состояние, `cond`). Это **проверка имени**, а не
//!   связывание: семантика атома остаётся вопросом расширения фичи 0010. Литералы
//!   `true`/`false` не проверяются.
//!
//! Функция [`ltl_warnings`] — публичный API наравне с
//! [`unused_variable_warnings`](crate::unused_variable_warnings); `lamc` их пока
//! не печатает (известный дефект в бэклоге), но они доступны через API и тесты.

use crate::diagnostics::Diagnostic;
use crate::semantic::formula::Formula;
use crate::semantic::{FunctionDefinitionNode, ModelNode, StatementNode};
use crate::verification::ltl::Ltl;
use std::cell::RefCell;
use std::collections::BTreeSet;
use std::rc::Rc;

/// Собирает предупреждения по LTL-формулам модели и её вложенных моделей.
pub fn ltl_warnings(model: Rc<RefCell<ModelNode>>) -> Vec<Diagnostic> {
    let mut out = Vec::new();
    collect_model(&model, &mut out);
    out
}

fn collect_model(model: &Rc<RefCell<ModelNode>>, out: &mut Vec<Diagnostic>) {
    let borrowed = model.borrow();
    // Известные имена модели: переменные/порты/константы, состояния, условия.
    let known: BTreeSet<String> = borrowed
        .variables
        .keys()
        .chain(borrowed.states.keys())
        .chain(borrowed.conditions.keys())
        .cloned()
        .collect();
    let loc = borrowed.loc;

    // Формулы уровня модели.
    for f in &borrowed.formulas {
        check_formula(f, &known, loc, out);
    }
    // Формулы уровня состояний: прямые (`: [LTL]` в теле состояния) и внутри
    // именованных блоков состояния (`always`/`enter`/`exit`).
    for state in borrowed.states.values() {
        for f in state.formulas() {
            check_formula(f, &known, state.loc(), out);
        }
        for block in state.named_blocks() {
            if let Some(stmt) = block.statement() {
                walk_statement(stmt, &known, state.loc(), out);
            }
        }
    }
    // Формулы в телах именованных блоков (`enter`/`exit`/`always`).
    for block in &borrowed.named_blocks {
        if let Some(stmt) = block.statement() {
            walk_statement(stmt, &known, loc, out);
        }
    }
    // Формулы в телах функций.
    for func in borrowed.functions.values() {
        if let FunctionDefinitionNode::Local { body, .. } = func {
            walk_statement(body, &known, loc, out);
        }
    }

    let nested: Vec<Rc<RefCell<ModelNode>>> = borrowed.models.values().map(Rc::clone).collect();
    drop(borrowed);
    for nested_model in nested {
        collect_model(&nested_model, out);
    }
}

/// Обходит оператор в поисках встроенных формул (`: [LTL] …;` в блоке кода).
fn walk_statement(
    stmt: &StatementNode,
    known: &BTreeSet<String>,
    loc: crate::diagnostics::Location,
    out: &mut Vec<Diagnostic>,
) {
    match stmt {
        StatementNode::InlineFormula(formulas) => {
            for f in formulas {
                check_formula(f, known, loc, out);
            }
        }
        StatementNode::Block(stmts) => {
            for s in stmts {
                walk_statement(s, known, loc, out);
            }
        }
        StatementNode::If { then_, else_, .. } => {
            walk_statement(then_, known, loc, out);
            if let Some(e) = else_ {
                walk_statement(e, known, loc, out);
            }
        }
        StatementNode::Loop { body, .. } => walk_statement(body, known, loc, out),
        StatementNode::For { init, body, .. } => {
            if let Some(i) = init {
                walk_statement(i, known, loc, out);
            }
            walk_statement(body, known, loc, out);
        }
        StatementNode::Match { arms, .. } => {
            for arm in arms {
                walk_statement(&arm.body, known, loc, out);
            }
        }
        // Прочие операторы формул не содержат.
        _ => {}
    }
}

/// Проверяет формулу: LTL → SE-055 + проверка атомов (SE-056).
fn check_formula(
    formula: &Formula,
    known: &BTreeSet<String>,
    loc: crate::diagnostics::Location,
    out: &mut Vec<Diagnostic>,
) {
    match formula {
        Formula::LTL(ltl) => {
            out.push(
                Diagnostic::warning(
                    loc,
                    "LTL-формула разобрана и сохранена, но верификация LTL не реализована: \
                     ни одна цель генерации её не проверяет"
                        .to_string(),
                )
                .with_code("SE-055"),
            );
            let mut atoms = BTreeSet::new();
            collect_atoms(ltl, &mut atoms);
            for atom in atoms {
                if !known.contains(&atom) {
                    out.push(
                        Diagnostic::warning(
                            loc,
                            format!(
                                "атом '{}' LTL-формулы не соответствует ни одной переменной, \
                                 состоянию или именованному условию модели",
                                atom
                            ),
                        )
                        .with_code("SE-056"),
                    );
                }
            }
        }
        Formula::Formulas(inner) => {
            for f in inner {
                check_formula(f, known, loc, out);
            }
        }
        // Guard/None атомов LTL не несут.
        Formula::Guard(_, _) | Formula::None => {}
    }
}

/// Собирает имена атомов LTL-формулы (без `true`/`false`).
fn collect_atoms(ltl: &Ltl, out: &mut BTreeSet<String>) {
    match ltl {
        Ltl::Atom(name) => {
            out.insert(name.clone());
        }
        Ltl::True | Ltl::False => {}
        Ltl::Not(a) | Ltl::Next(a) | Ltl::Finally(a) | Ltl::Globally(a) => collect_atoms(a, out),
        Ltl::And(l, r)
        | Ltl::Or(l, r)
        | Ltl::Implies(l, r)
        | Ltl::Until(l, r)
        | Ltl::Release(l, r) => {
            collect_atoms(l, out);
            collect_atoms(r, out);
        }
    }
}
