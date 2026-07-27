//! Проход по АСД для конструкций времени (фича 0134): частота и `every`.
//!
//! Отдельная стадия, а не ветка в `tree.rs`, по двум причинам. Первая
//! формальная: `tree.rs` пришпилен реестром размеров и расти не имеет права.
//! Вторая содержательная: частота — **свойство единицы компиляции**, а не
//! элемента модели, и собирать её обходом всего дерева честнее, чем
//! накапливать в разборе элементов.
//!
//! ## Что делает
//!
//! 1. Собирает объявления `clock` (включая вложенные модели). Все они обязаны
//!    называть **одну** частоту: разные значения — ошибка автора (`SE-067`), а
//!    не «побеждает последнее». Итог кладётся в
//!    [`ModelNode::clock_hz`](super::ModelNode::clock_hz) корня.
//! 2. Отвергает `every 100ms { … }` (`SE-066`) — до его реализации.
//!
//! ⚠️ **Второй пункт закрывает молчаливую потерю.** Проба при разработке
//! 0134-03: модель с `every` компилировалась **успешно**, тело блока пропадало
//! бесследно, и вдобавок компилятор сообщал «переменная 'n' нигде не
//! используется» — то есть говорил автору неправду о его собственном коде.
//! Отказ обязан быть громким, пока конструкция не исполняется.
//!
//! Профиль выбирается позже и **не здесь**:
//! [`duration::resolve_profile`](super::duration::resolve_profile) сводит
//! объявление модели с флагом `--tick-hz` (флаг переопределяет).

use crate::diagnostics::Diagnostic;
use crate::parser::ast::{Model, ModelElement, StateElement};
use crate::semantic::ModelNode;
use std::cell::RefCell;
use std::rc::Rc;

/// Обходит АСД: собирает частоту тактирования и отвергает нереализованное.
///
/// # Ошибки
///
/// - `SE-067` — две несовпадающие частоты в одной единице компиляции;
/// - `SE-066` — `every` (пока не исполняется ни одной целью).
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
            // `every` пока не исполняется: отказ вместо тихой потери тела.
            ModelElement::State(state) => {
                for element in &state.elements {
                    if let StateElement::Every(def) = element {
                        return Err(Diagnostic::error(
                            def.loc,
                            format!(
                                "периодическое действие 'every {}' пока не поддерживается",
                                def.text
                            ),
                        )
                        .with_code("SE-066"));
                    }
                }
            }
            _ => {}
        }
    }
    Ok(())
}
