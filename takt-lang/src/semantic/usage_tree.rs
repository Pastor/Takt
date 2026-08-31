//! Использование имён с учётом ДЕТЕЙ ПО ВЫЗОВУ (фичи 0450, 0452).
//!
//! Признаки целей спрашивают не «что упоминает эта модель», а «что упоминает
//! она вместе с теми, кого тикает»: вложенными моделями и теми, которыми
//! реализованы её состояния (`= M`, `A | B`, `A + B`). Вторых в
//! [`ModelNode::models`](crate::semantic::ModelNode) нет, и пока каждая цель
//! считала сама, три из них печатали вывод, отвергаемый их же инструментами
//! при нулевом коде возврата `taktc`.
//!
//! ⚠️ Модуль отделён от [`unused`](super::unused) по размеру (правило
//! `docs/CODE.md`), а не по смыслу: правило «что считать упоминанием» остаётся
//! одно — обход тот же, меняется лишь объём дерева.

use std::cell::RefCell;
use std::collections::HashSet;
use std::rc::Rc;

use crate::semantic::ModelNode;
use crate::semantic::unused::{UsageSet, collect_model_usage, compute_usage};

/// Имена, которые модель и её дети по вызову **читают** (фича 0452).
///
/// Отличие от [`usage_with_implementations`] — режим `reads_only`: цель
/// присваивания местом чтения не считается (правило 0387). Нужен там, где
/// сторона порта печатается по факту: у двунаправленного порта цели `sv`
/// сигнал `_i` есть **вход модуля**, и без чтения `verilator` под `-Wall`
/// отвечает `UNUSEDSIGNAL`.
pub fn reads_with_implementations(model: &Rc<RefCell<ModelNode>>) -> UsageSet {
    let mut set = UsageSet {
        reads_only: true,
        ..UsageSet::default()
    };
    collect_model_usage(Rc::clone(model), &mut set);
    let mut queue = crate::semantic::extend::implementation_children(&model.borrow());
    let mut seen: HashSet<*const RefCell<ModelNode>> = HashSet::new();
    while let Some(child) = queue.pop() {
        if !seen.insert(Rc::as_ptr(&child)) {
            continue;
        }
        collect_model_usage(Rc::clone(&child), &mut set);
        queue.extend(crate::semantic::extend::implementation_children(
            &child.borrow(),
        ));
    }
    set
}

/// Использование имён моделью **и её детьми по вызову** (фича 0450).
///
/// Дети по вызову — это не только вложенные модели (их обходит
/// [`compute_usage`]), но и те, которыми **реализованы состояния** (`= M`,
/// `A | B`, `A + B`): их тик зовёт эта же модель и передаёт им своё
/// окружение.
///
/// ⚠️ Носитель общий у двух целей. `rust` спрашивает его о параметре
/// `&mut Shared`, `st` — о секции `VAR_IN_OUT`; до фичи 0450 каждая считала
/// сама через `compute_usage`, реализаций не видела, и обе печатали вывод,
/// который отвергают их инструменты при НУЛЕВОМ коде возврата `taktc`:
/// `rustc` — «cannot find value `shared` in this scope», `iec2c` —
/// «Ambiguous enumerate value or Variable not declared in this scope».
pub fn usage_with_implementations(model: &Rc<RefCell<ModelNode>>) -> UsageSet {
    let mut set = compute_usage(Rc::clone(model));
    let mut queue = crate::semantic::extend::implementation_children(&model.borrow());
    let mut seen: HashSet<*const RefCell<ModelNode>> = HashSet::new();
    while let Some(child) = queue.pop() {
        if !seen.insert(Rc::as_ptr(&child)) {
            continue;
        }
        let child_usage = compute_usage(Rc::clone(&child));
        set.variables.extend(child_usage.variables);
        set.constants.extend(child_usage.constants);
        set.ports.extend(child_usage.ports);
        set.functions.extend(child_usage.functions);
        queue.extend(crate::semantic::extend::implementation_children(
            &child.borrow(),
        ));
    }
    set
}
