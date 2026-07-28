//! Механизм времени цели `c` (фича 0134, задача 0134-04).
//!
//! Здесь — то, что цель `c` знает о времени: разрядность счётчика выдержки и
//! правила его обновления. Арифметика длительности **не здесь**: пересчёт
//! «наносекунды → единицы профиля» живёт в общем слое
//! [`semantic::duration`](crate::semantic::duration) — иначе цели дали бы разное
//! число тактов для одного текста (правило 7 ADR 0134).
//!
//! Вынесено отдельным модулем не по вкусу: `c_header.rs` вместе с этим кодом
//! давал 1004 строки при лимите 1000, и гейт размера отказал.

use crate::diagnostics::{Diagnostic, Location};
use crate::generator::c::c_map::CMap;

/// Максимальная выдержка модели в единицах профиля — для выбора разрядности.
///
/// Разрядность счётчика выбирает **компилятор** (T7 отчёта `DIFF.md`): счётчик,
/// объявленный узко, молча зациклил бы выдержку. Пересчёт идёт через общий слой
/// `semantic::duration`, а не своей арифметикой.
pub(super) fn counter_ticks(
    map: &CMap,
    model: &crate::semantic::ModelNode,
) -> Result<u64, Diagnostic> {
    let profile = map.time_profile();
    let mut max = 0u64;
    for state in model.states.values() {
        for reference in state.references() {
            let nanos = match &reference.cond {
                crate::semantic::ConditionNode::After(nanos) => Some(*nanos),
                _ => None,
            };
            if let Some(nanos) = nanos {
                let units = crate::semantic::duration::units_or_diagnostic(
                    nanos,
                    profile,
                    Location::Codegen,
                    "выдержка 'after'",
                )?;
                max = max.max(units);
            }
        }
    }
    Ok(max)
}

/// Отпечаток контракта частоты для заголовка C (фича 0134-05), готовый блок.
///
/// `Some(текст)` — модель объявила `clock`, значит контракт частоты подтверждён
/// (кодоген сюда доходит только после `resolve_profile`, вернувшего `Ticks`), и
/// его нужно закрепить статическим утверждением. `#ifndef`-дефолт держит гейт
/// `cc -c` зелёным (частота по умолчанию совпадает); интегратор, задав
/// `TAKT_TICK_HZ` иным значением, ловит несовпадение при сборке прошивки — там,
/// где частоту задаёт уже система тактирования, а не компилятор. `None` — частоту
/// задал лишь `--tick-hz` без объявления в модели: закреплять нечего.
pub(super) fn clock_contract_block(map: &CMap) -> Option<String> {
    let hertz = map.root_model_node().and_then(|m| m.borrow().clock_hz)?;
    Some(format!(
        "/* Контракт частоты Takt (clock): объявленная моделью частота. */\n\
         #define TAKT_REQUIRED_CLOCK_HZ {hertz}u\n\
         #ifndef TAKT_TICK_HZ\n\
         #define TAKT_TICK_HZ TAKT_REQUIRED_CLOCK_HZ\n\
         #endif\n\
         _Static_assert(TAKT_TICK_HZ == TAKT_REQUIRED_CLOCK_HZ,\n    \
         \"частота тактирования не совпадает с объявленной моделью Takt\");\n\n"
    ))
}
