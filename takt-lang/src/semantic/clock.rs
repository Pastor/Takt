//! Частота тактирования модели: `clock 1kHz;` (фича 0134, подзадача 0134-02).
//!
//! Отдельная стадия, а не ветка в `tree.rs`, по двум причинам. Первая
//! формальная: `tree.rs` пришпилен реестром размеров и расти не имеет права.
//! Вторая содержательная: частота — **свойство единицы компиляции**, а не
//! элемента модели, и собирать её обходом всего дерева честнее, чем
//! накапливать в разборе элементов.
//!
//! ## Что делает
//!
//! Обходит АСД (включая вложенные модели) и собирает все объявления `clock`.
//! Все они обязаны называть **одну** частоту: разные значения — ошибка автора
//! (`SE-067`), а не «побеждает последнее». Итог кладётся в
//! [`ModelNode::clock_hz`](super::ModelNode::clock_hz) корня.
//!
//! Профиль выбирается позже и **не здесь**:
//! [`duration::resolve_profile`](super::duration::resolve_profile) сводит
//! объявление модели с флагом `--tick-hz` (флаг переопределяет).

use crate::diagnostics::Diagnostic;
use crate::parser::ast::{Model, ModelElement};
use crate::semantic::ModelNode;
use std::cell::RefCell;
use std::rc::Rc;

/// Собирает частоту тактирования из АСД и кладёт её в корень дерева.
///
/// # Ошибки
///
/// `SE-067` — две несовпадающие частоты в одной единице компиляции.
pub(crate) fn collect_clock(ast: &Model, model: &Rc<RefCell<ModelNode>>) -> Result<(), Diagnostic> {
    let mut found: Option<u64> = None;
    walk(ast, &mut found)?;
    if let Some(hertz) = found {
        model.borrow_mut().clock_hz = Some(hertz);
    }
    Ok(())
}

/// Рекурсивный обход АСД: объявления `clock` текущей модели и вложенных.
fn walk(ast: &Model, found: &mut Option<u64>) -> Result<(), Diagnostic> {
    for element in &ast.elements {
        match element {
            ModelElement::Clock(def) => match *found {
                Some(previous) if previous != def.hertz => {
                    return Err(Diagnostic::error(
                        def.loc,
                        format!(
                            "частота тактирования объявлена дважды и по-разному: \
                             {previous} Гц и {} Гц — какая из них настоящая, \
                             знает только автор",
                            def.hertz
                        ),
                    )
                    .with_code("SE-067"));
                }
                _ => *found = Some(def.hertz),
            },
            ModelElement::Model(nested) => walk(nested, found)?,
            _ => {}
        }
    }
    Ok(())
}
