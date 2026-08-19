//! Печать исходящих рёбер состояния для цели `st` (фича 0303).
//!
//! Вынесено из `st_model.rs` по двум причинам. Формальная: файл упирается в
//! лимит размера модуля. Содержательная: **рёбра печатаются из двух мест** —
//! у обычного состояния и внутри ветви завершения состояния-композиции. Прежде
//! второго места не было вовсе: композиция уходила по `next`/`END`, а
//! собственные `ref`-рёбра состояния терялись — вход
//! `start Entry = A | B { ref Finish: cond; }` давал другой автомат, чем
//! эталон, и молча.
//!
//! Порядок проверки задан эталоном (фича 0181): сначала `ref` в порядке
//! объявления, затем `next`.

use crate::diagnostics::{Diagnostic, Location};
use crate::generator::indent::Printer;
use crate::generator::st::st_map::StMap;
use crate::generator::st::st_time;
use crate::semantic::minimap::Name;
use crate::semantic::{ConditionNode, ModelNode, ReferenceNode, StateNode};

use crate::generator::st::st_compose::Instance;
use crate::generator::st::st_expr::print_condition;
use crate::generator::st::st_model::{BodyOutput, StateTable, emit_transition, unknown_state};

/// Печатает цепочку `IF … ELSIF …` по рёбрам состояния.
///
/// Возвращает `true`, если напечатано **безусловное** ребро: цепочка на нём
/// заканчивается, и вызывающий не должен печатать переход по `next`/`END`.
#[allow(clippy::too_many_arguments)]
pub(crate) fn emit_edges(
    p: &mut Printer,
    map: &StMap,
    name: &Name,
    state: &StateNode,
    model: &ModelNode,
    table: &StateTable,
    out: &mut BodyOutput,
    references: &[ReferenceNode<StateNode>],
) -> Result<bool, Diagnostic> {
    // Выдержка `after` в профиле «часы» (фича 0134): штатный `TON`. Экземпляр на
    // каждое длительностное ребро, вызывается КАЖДЫЙ скан в состоянии — ДО цепочки
    // IF, иначе `.Q` не обновится. `IN := TRUE` взводит таймер; сброс `IN := FALSE`
    // при ЛЮБОМ выходе (иначе выдержка «прилипнет» и следующий вход сработает сразу).
    let clock = st_time::is_clock(map);
    let mut edge_timer: Vec<Option<String>> = vec![None; references.len()];
    if clock {
        for (i, reference) in references.iter().enumerate() {
            if let ConditionNode::After(nanos) = reference.cond {
                let timer = st_time::timer_name(name, i);
                p.ident(&format!(
                    "{}(IN := TRUE, PT := {});",
                    timer,
                    crate::semantic::duration::time_literal(nanos)
                ))
                .nl();
                out.instances.push(Instance {
                    name: timer.clone(),
                    fb_type: st_time::TON_TYPE.to_string(),
                    init: None,
                });
                edge_timer[i] = Some(timer);
            }
        }
    }
    let state_timers: Vec<String> = edge_timer.iter().flatten().cloned().collect();
    // Сброс всех таймеров состояния перед выходом — на любом ребре (перевзвод).
    let reset_timers = |p: &mut Printer| {
        for timer in &state_timers {
            p.ident(&format!("{}(IN := FALSE);", timer)).nl();
        }
    };

    // Порядок `ref` = порядок проверки, первый сработавший выигрывает (Ф5):
    // цепочка `if … break;` цели `c` — это `IF … ELSIF …` в ST.
    let mut printed_if = false;
    for (i, reference) in references.iter().enumerate() {
        let target = table.number_of_local(&reference.name).ok_or_else(|| {
            unknown_state(&format!(
                "переход ведёт в состояние '{}', которого нет в модели",
                reference.name
            ))
        })?;
        // Безусловный переход (`ref T;` без условия) приходит как
        // `ConditionNode::None`: проверять нечего — переход печатается как есть,
        // и цепочка на нём заканчивается.
        if reference.cond.is_unconditional() {
            if printed_if {
                p.ident("ELSE").nl();
                p.up();
                reset_timers(p);
                emit_transition(p, state, &reference.name, target, model, &mut out.stmt)?;
                p.down();
                p.ident("END_IF;").nl();
            } else {
                reset_timers(p);
                emit_transition(p, state, &reference.name, target, model, &mut out.stmt)?;
            }
            return Ok(true);
        }
        // Guard выдержки — по профилю (фича 0134): «часы» → `dwell.Q`, «такты» →
        // счётчик `takt_dwell >= D`. Прочие условия — обычный печатник.
        let guard = match &reference.cond {
            ConditionNode::After(nanos) => {
                if clock {
                    format!("{}.Q", edge_timer[i].as_deref().unwrap_or(""))
                } else {
                    let units = crate::semantic::duration::units_or_diagnostic(
                        *nanos,
                        map.time_profile(),
                        Location::Codegen,
                        "выдержка 'after'",
                    )?;
                    format!("{} >= {}", st_time::DWELL_FIELD, units)
                }
            }
            ConditionNode::AfterTicks(ticks) => format!("{} >= {}", st_time::DWELL_FIELD, ticks),
            // Вычисляемая выдержка (фича 0183): справа — выражение в
            // миллисекундах. В профиле «часы» таймер `TON` с переменным `PT`
            // потребовал бы иной обвязки, поэтому пока поддержан профиль «такты»:
            // отказ громкий, а не молча иная выдержка.
            ConditionNode::AfterExpr(inner) => {
                if clock {
                    return Err(Diagnostic::error(
                        Location::Codegen,
                        concat!(
                            "вычисляемая выдержка 'after' в профиле «часы» целью 'st' ",
                            "пока не поддерживается: переменный `PT` таймера требует ",
                            "своей обвязки. Передайте --tick-hz (профиль «такты») ",
                            "либо оставьте выдержку константной"
                        )
                        .to_string(),
                    )
                    .with_code("ST-016"));
                }
                let expr = print_condition(inner, model)?;
                match crate::semantic::duration::ticks_per_milli(
                    map.time_profile(),
                    Location::Codegen,
                )? {
                    Some(1) => format!("{} >= {expr}", st_time::DWELL_FIELD),
                    Some(multiplier) => {
                        format!("{} >= ({expr}) * {multiplier}", st_time::DWELL_FIELD)
                    }
                    // `None` невозможен: ветвь `clock` отсечена выше.
                    None => unreachable!("профиль «часы» отсечён выше"),
                }
            }
            other => print_condition(other, model)?,
        };
        p.ident(&format!(
            "{} {} THEN",
            if printed_if { "ELSIF" } else { "IF" },
            guard
        ))
        .nl();
        p.up();
        reset_timers(p);
        emit_transition(p, state, &reference.name, target, model, &mut out.stmt)?;
        p.down();
        printed_if = true;
    }
    if printed_if {
        p.ident("END_IF;").nl();
    }
    Ok(false)
}
