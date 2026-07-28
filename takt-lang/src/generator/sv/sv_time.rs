//! Механизм времени цели `sv` (синтезируемый SystemVerilog, фича 0134).
//!
//! Два профиля (решение заказчика, анализ 0134-07):
//! - **«часы»** — служебный ВХОД `time_ms` (внешний источник, как `clk`/`en`);
//!   метка входа `<lvl>_takt_entry` латчит `time_ms`, условие — разностью.
//! - **«такты»** — счётчик тактов `<lvl>_takt_dwell` (как цель `c`), условие `>= D`.
//!
//! ⚠️ **Капкан цели:** `always_comb` вычисляет `_next` счётчика/метки из
//! **РЕГИСТРОВ** `state`/`prev_state` (не из `state_next` — иначе комбинационная
//! петля `UNOPTFLAT`), а условие выдержки читает **`_next`** (оно уже учитывает
//! текущий такт; чтение регистра сдвинуло бы выдержку на такт молча). Роль
//! `prev_state` — та же, что `takt_prev_state` в `c`: разорвать зависимость
//! детекции входа от `state_next`.
//!
//! ⚠️ `#`-задержки и `$time` не эмитируются НИКОГДА (сторож A7 — греп по выводу).

use crate::diagnostics::Diagnostic;
use crate::generator::indent::Printer;
use crate::generator::sv::sv_fsm::Reg;
use crate::generator::sv::sv_map::SvMap;
use crate::semantic::ModelNode;
use crate::semantic::duration::{TimeProfile, counter_bits, units_or_diagnostic};
use crate::semantic::minimap::Name;
use crate::semantic::time_ast::{
    model_tree_uses_duration_after, model_uses_duration_after, model_uses_tick_after,
};

/// Профиль модели — «часы»?
pub(crate) fn is_clock(map: &SvMap) -> bool {
    matches!(map.time_profile(), TimeProfile::Clock)
}

/// Нужен ли счётчик тактов у уровня: тактовая выдержка `after Nt` (любой профиль)
/// либо длительностная `after Nms` в профиле «такты».
pub(crate) fn needs_dwell(map: &SvMap, model: &ModelNode) -> bool {
    model_uses_tick_after(model) || (!is_clock(map) && model_uses_duration_after(model))
}

/// Нужна ли метка времени у уровня: профиль «часы» + `after Nms`.
pub(crate) fn needs_entry(map: &SvMap, model: &ModelNode) -> bool {
    is_clock(map) && model_uses_duration_after(model)
}

/// Нужен ли служебный вход `time_ms` модулю: профиль «часы» + длительностная
/// выдержка где-либо в дереве (вход один на модуль после уплощения).
pub(crate) fn needs_time_port(map: &SvMap, root: &ModelNode) -> bool {
    is_clock(map) && model_tree_uses_duration_after(root)
}

/// Разрядность метки/счётчика по максимуму `after` **дерева** модели (R8).
///
/// Один источник для объявления регистра, входа `time_ms` и сравнения: разойдись
/// они, поле оказалось бы уже сравнения — и выдержка молча переполнилась бы.
pub(crate) fn time_bits(map: &SvMap) -> Result<u8, Diagnostic> {
    let max = match map.root_model_node() {
        Some(model) => max_units_in_tree(map, &model.borrow())?,
        None => 0,
    };
    Ok(counter_bits(max).unwrap_or(64))
}

/// Максимум единиц профиля по выдержкам `after` этой модели и вложенных.
fn max_units_in_tree(map: &SvMap, model: &ModelNode) -> Result<u64, Diagnostic> {
    let mut max = 0u64;
    for state in model.states.values() {
        for reference in state.references() {
            if let crate::semantic::ConditionNode::After(nanos) = reference.cond {
                max = max.max(units_or_diagnostic(
                    nanos,
                    map.time_profile(),
                    crate::diagnostics::Location::Codegen,
                    "выдержка 'after'",
                )?);
            }
        }
    }
    for nested in model.models.values() {
        max = max.max(max_units_in_tree(map, &nested.borrow())?);
    }
    Ok(max)
}

/// Имя регистра счётчика тактов уровня.
pub(crate) fn dwell_reg(model: &Name) -> String {
    format!("{}_takt_dwell", model.unique_lowercase_snakecase())
}

/// Имя регистра метки времени входа уровня (профиль «часы»).
pub(crate) fn entry_reg(model: &Name) -> String {
    format!("{}_takt_entry", model.unique_lowercase_snakecase())
}

/// Имя регистра «состояние предыдущего такта» уровня.
pub(crate) fn prev_state_reg(model: &Name) -> String {
    format!("{}_takt_prev_state", model.unique_lowercase_snakecase())
}

/// Служебный вход времени модуля.
pub(crate) const TIME_MS_PORT: &str = "time_ms";

/// Уровень (модель) с механизмом времени (фича 0134): имена регистров и профиль.
pub(crate) struct TimeLevel {
    /// Модель-уровень (для префикса имён регистров).
    model: Name,
    /// Имя регистра состояния этого уровня.
    state_reg: String,
    /// Нужен ли счётчик тактов (`<lvl>_takt_dwell`).
    dwell: bool,
    /// Нужна ли метка времени входа (`<lvl>_takt_entry`, профиль «часы»).
    entry: bool,
    /// Разрядность метки/счётчика.
    bits: u8,
}

/// Заводит регистры времени уровня (фича 0134): счётчик/метка + метка предыдущего
/// состояния (детекция входа). Получают объявление, `_next`, сброс и защёлкивание
/// генериком `Reg` — как `<state>_step` (0057). Имена уровня передаются готовыми
/// (`enum_name`/`end_var`/`state_reg`), чтобы не тянуть в `sv_time` их построители.
#[allow(clippy::too_many_arguments)]
pub(crate) fn push_time_regs(
    regs: &mut Vec<Reg>,
    registered: &mut std::collections::BTreeSet<String>,
    time_levels: &mut Vec<TimeLevel>,
    map: &SvMap,
    name: &Name,
    model: &ModelNode,
    enum_name: &str,
    end_var: &str,
    state_reg: &str,
) -> Result<(), Diagnostic> {
    let dwell = needs_dwell(map, model);
    let entry = needs_entry(map, model);
    if !dwell && !entry {
        return Ok(());
    }
    let bits = time_bits(map)?;
    let word = format!("logic [{}:0]", bits.saturating_sub(1));
    let mut push = |sig: String, prefix: String, reset: String| {
        registered.insert(sig.clone());
        regs.push(Reg {
            name: sig,
            prefix,
            suffix: String::new(),
            reset,
            declare_reg: true,
        });
    };
    if dwell {
        push(dwell_reg(name), word.clone(), "'0".to_string());
    }
    if entry {
        push(entry_reg(name), word, "'0".to_string());
    }
    // `prev_state` сбрасывается в END-сентинел (не в стартовое): sv без INIT, и на
    // ПЕРВОМ такте `state(start) != prev(END)` даёт вход.
    push(
        prev_state_reg(name),
        enum_name.to_string(),
        end_var.to_string(),
    );
    time_levels.push(TimeLevel {
        model: name.clone(),
        state_reg: state_reg.to_string(),
        dwell,
        entry,
        bits,
    });
    Ok(())
}

/// Перекрывает умолчание `_next` регистров времени явной комбинационной формулой
/// (фича 0134). Вход в состояние — `state != prev_state` (оба РЕГИСТРЫ): счётчик
/// сбрасывается в 1 и растёт, метка латчит `time_ms`. `prev_state_next = state`.
pub(crate) fn emit_time_updates(p: &mut Printer, levels: &[TimeLevel]) -> Result<(), Diagnostic> {
    for lvl in levels {
        let state = &lvl.state_reg;
        let prev = prev_state_reg(&lvl.model);
        let entered = format!("{} != {}", state, prev);
        p.ident(&format!("{}_next = {};", prev, state)).nl();
        if lvl.dwell {
            let dwell = dwell_reg(&lvl.model);
            p.ident(&format!(
                "{dwell}_next = ({entered}) ? {b}'d1 : {dwell} + {b}'d1;",
                b = lvl.bits
            ))
            .nl();
        }
        if lvl.entry {
            let entry = entry_reg(&lvl.model);
            p.ident(&format!(
                "{entry}_next = ({entered}) ? {TIME_MS_PORT} : {entry};"
            ))
            .nl();
        }
    }
    if !levels.is_empty() {
        p.nl();
    }
    Ok(())
}

/// Строит guard выдержки `after` уровня, читая `_next` счётчика/метки (фича 0134).
///
/// `Some(guard)` для `After`/`AfterTicks`; `None` для прочих условий (их печатает
/// общий `print_condition`). Читается именно `_next`: оно уже учло текущий такт —
/// чтение регистра сдвинуло бы выдержку молча.
pub(crate) fn after_guard(
    levels: &[TimeLevel],
    map: &SvMap,
    model: &Name,
    cond: &crate::semantic::ConditionNode,
) -> Option<Result<String, Diagnostic>> {
    use crate::semantic::ConditionNode;
    let level = levels.iter().find(|l| l.model.unique() == model.unique())?;
    match cond {
        ConditionNode::After(nanos) => {
            let units = match units_or_diagnostic(
                *nanos,
                map.time_profile(),
                crate::diagnostics::Location::Codegen,
                "выдержка 'after'",
            ) {
                Ok(u) => u,
                Err(e) => return Some(Err(e)),
            };
            if is_clock(map) {
                Some(Ok(format!(
                    "({TIME_MS_PORT} - {entry}_next) >= {units}",
                    entry = entry_reg(&level.model)
                )))
            } else {
                Some(Ok(format!(
                    "{dwell}_next >= {units}",
                    dwell = dwell_reg(&level.model)
                )))
            }
        }
        ConditionNode::AfterTicks(ticks) => Some(Ok(format!(
            "{dwell}_next >= {ticks}",
            dwell = dwell_reg(&level.model)
        ))),
        _ => None,
    }
}
