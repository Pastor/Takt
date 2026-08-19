//! Ребро после безусловного недостижимо: `SE-116` (фича 0273).
//!
//! # Что и почему
//!
//! Переходы состояния проверяются **по порядку записи**, и первое безусловное
//! ребро завершает цепочку: всё, что записано после него, не сработает никогда.
//! Это правило исполняют все четыре цели и эталон (фича
//! [0213](../../../../docs/features/0213-c-redundant-break.md) сравняла с ними
//! цель `c`), но автору о нём не говорил **никто**.
//!
//! Замер 2026-08-19 на записи
//!
//! ```takt
//! start Run {
//!     always { n := n + 1; }
//!     ref Done;
//!     ref Late: n = 1;   // не сработает никогда
//! }
//! ```
//!
//! `taktc compile` завершался успешно и молча; эталон уходил в `Done` на первом
//! такте. Ни семантика, ни цели, ни проверка недетерминизма ничего не сообщали.
//!
//! ⚠️ **Проверка недетерминизма (Ce14, `SE-037`) класс не покрывает:** она ищет
//! **несколько** безусловных рёбер, а здесь безусловное — одно, и лишними
//! оказываются условные, стоящие за ним.
//!
//! ⚠️ **Что считать безусловным, решает не эта проверка**, а общий предикат
//! `ConditionNode::is_unconditional` (фича 0291): правило одно на всех
//! потребителей.
//!
//! # Почему предупреждение, а не ошибка
//!
//! Запись законна и её смысл определён: недостижимое ребро просто ничего не
//! делает. Отказ ломал бы работающие входы (в фикстурах такие есть), а
//! умолчание оставляет автора в неведении — отсюда предупреждение, как у Ce14.

use crate::diagnostics::Diagnostic;
use crate::semantic::{ModelNode, StateNode};
use std::cell::RefCell;
use std::rc::Rc;

/// Собирает предупреждения о недостижимых рёбрах модели и всех вложенных.
pub fn check_unreachable_edges(model: Rc<RefCell<ModelNode>>) -> Vec<Diagnostic> {
    let mut warnings = Vec::new();
    check_model(&model, &mut warnings);
    warnings
}

/// Обход одной модели: состояния, затем вложенные модели.
fn check_model(model: &Rc<RefCell<ModelNode>>, warnings: &mut Vec<Diagnostic>) {
    let borrowed = model.borrow();
    for state in borrowed.states.values() {
        let references = match state {
            StateNode::Simple { references, .. } | StateNode::Implement { references, .. } => {
                references
            }
            StateNode::Unresolved => continue,
        };
        // Первое безусловное ребро завершает цепочку. Всё, что за ним, —
        // недостижимо; высказываемся о КАЖДОМ (правило 0151: одна диагностика
        // на элемент, все элементы высказываются).
        let Some(cut) = references.iter().position(|r| r.cond.is_unconditional()) else {
            continue;
        };
        for dead in references.iter().skip(cut + 1) {
            warnings.push(
                Diagnostic::warning(
                    dead.location,
                    format!(
                        "переход в '{}' недостижим: он записан после безусловного перехода \
                         в '{}', а тот завершает выбор. Поставьте это ребро выше \
                         безусловного либо снабдите безусловное условием",
                        dead.name, references[cut].name
                    ),
                )
                .with_code("SE-116"),
            );
        }
    }
    for nested in borrowed.models.values() {
        check_model(nested, warnings);
    }
}
