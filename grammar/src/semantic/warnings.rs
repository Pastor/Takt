//! Единая точка сбора предупреждений компилятора (фича 0081).
//!
//! До 0081 CLI (`lamc compile`) не звал ни `unused_variable_warnings` (`SE-036`),
//! ни `nondeterministic_transition_warnings` (`SE-037`/`SE-042`) — предупреждения
//! публичного API до пользователя не доезжали. Диагностика, которую никто не
//! печатает, равносильна её отсутствию.
//!
//! Новое предупреждение, добавленное в [`collect_model_warnings`], доезжает до
//! пользователя всеми целями `lamc`. Адрес-специфичные предупреждения
//! (`address_expr_warnings`, `address_map_overlay_warnings`) сюда **не** входят:
//! они зависят от цели (у адрес-потребляющих `c-hal`/`st-at` те же ситуации дают
//! ошибки), и собираются у вызывающего отдельно.

use crate::diagnostics::Diagnostic;
use crate::parser::ast;
use crate::semantic::ModelNode;
use std::cell::RefCell;
use std::rc::Rc;

/// Собирает **все** предупреждения над построенной моделью.
///
/// Порядок вызова — фиксированный (детерминизм вывода); входы смешаны намеренно:
/// большинство проверок берут семантическую модель, `stray_semicolon` и
/// `unknown_named_block` — АСД.
pub fn collect_model_warnings(ast: &ast::Model, model: &Rc<RefCell<ModelNode>>) -> Vec<Diagnostic> {
    let mut warnings = Vec::new();
    warnings.extend(crate::unused_variable_warnings(Rc::clone(model)));
    warnings.extend(crate::nondeterministic_transition_warnings(Rc::clone(
        model,
    )));
    warnings.extend(crate::unreachable_state_warnings(Rc::clone(model)));
    warnings.extend(crate::constant_condition_warnings(model));
    warnings.extend(crate::ltl_warnings(Rc::clone(model)));
    warnings.extend(crate::stray_semicolon_warnings(ast));
    warnings.extend(crate::unknown_named_block_warnings(ast));
    warnings
}
