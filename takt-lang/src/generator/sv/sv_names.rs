//! Проверки имён цели `sv`, требующие доступа к семантической модели.
//!
//! # Зачем отдельный модуль
//!
//! `check_sv_name` живёт в `sv_module` и работает со строкой. Проверка имён
//! **состояний** (фича 0200) строкой не обходится: перечислитель печатается по
//! `Name`, у которого позиции нет, а диагностика обязана указывать на
//! объявление автора — значит нужен доступ к `ModelNode`. Отдельный модуль, а
//! не `sv_fsm`, потому что тот упирается в лимит размера
//! (`scripts/check-module-size.sh`).

use crate::diagnostics::Diagnostic;
use crate::generator::sv::sv_map::SvMap;
use crate::generator::sv::sv_module::check_sv_name;
use crate::semantic::minimap::Name;

/// Проверяет имена состояний модели на пригодность для SystemVerilog.
///
/// Позиция берётся из семантической модели: перечислитель печатается по
/// `Name`, у которого позиции нет, а диагностика обязана указывать на
/// объявление автора.
pub(crate) fn check_state_names(map: &SvMap, model: &Name) -> Result<(), Diagnostic> {
    let raw = map.raw_model_at(model.clone())?;
    let borrowed = raw.borrow();
    for state in borrowed.states.values() {
        let (name, loc) = match state {
            crate::semantic::StateNode::Simple { name, loc, .. }
            | crate::semantic::StateNode::Implement { name, loc, .. } => (name, *loc),
            crate::semantic::StateNode::Unresolved => continue,
        };
        check_sv_name(name, loc)?;
    }
    Ok(())
}
