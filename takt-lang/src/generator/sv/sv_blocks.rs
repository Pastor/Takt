//! Эмиссия именованных блоков (`enter`/`always`/`exit`) в цель SystemVerilog.
//!
//! Вынесено из `sv_fsm.rs` (фича 0083: лимит размера модуля). Два источника
//! блоков — состояние и **сама модель** (model-level `always`, фича 0083);
//! обе печати идут в `always_comb` над `_next`-сигналами.

use crate::diagnostics::Diagnostic;
use crate::generator::indent::Printer;
use crate::generator::sv::sv_fsm::Fsm;
use crate::generator::sv::sv_stmt::print_statement;
use crate::semantic::{ModelNode, StateNode};

/// Печатает именованные блоки состояния (`enter`/`always`/`exit`).
pub(crate) fn emit_named_blocks(
    p: &mut Printer,
    state: &StateNode,
    fsm: &Fsm,
    block: &str,
) -> Result<(), Diagnostic> {
    for b in state.get_named_blocks(block) {
        if let Some(stmt) = b.statement() {
            print_statement(p, stmt, &fsm.scope())?;
        }
    }
    Ok(())
}

/// Печатает именованные блоки **уровня модели** (фича 0083): `always` вне
/// состояния. Аналог [`emit_named_blocks`], но источник — сама модель.
pub(crate) fn emit_model_named_blocks(
    p: &mut Printer,
    model: &ModelNode,
    fsm: &Fsm,
    block: &str,
) -> Result<(), Diagnostic> {
    for b in model.get_named_blocks(block) {
        if let Some(stmt) = b.statement() {
            print_statement(p, stmt, &fsm.scope())?;
        }
    }
    Ok(())
}
