//! Композиция состояний в цели SystemVerilog: параллельная `|` (ADR 0045,
//! Option A′) и последовательная `+` (ADR 0057, Option A).
//!
//! ## Общий принцип: инлайн ВНУТРЬ ветви `case` родителя
//!
//! Логика под-модели инлайнится в `always_comb` **внутри** `case`-ветви несущего
//! состояния (эталон — `stacker.c`: `_tick` под-моделей зовётся из `case`
//! родителя). Вынос наружу изменил бы модель: под-модели продолжали бы работать
//! после выхода родителя.
//!
//! ## Готовность шага читает `_next`, а не регистр
//!
//! В C `_is_done` вызывается после `_tick` и видит только что записанное
//! значение. В `always_comb` рабочая копия — `state_next`, поэтому эквивалент —
//! `(<sub>_state_next == <SUB>_END)`. Чтение регистра дало бы значение
//! **предыдущего** такта — ровно тот сдвиг, который осуждает ADR 0033. Здесь жил
//! дефект SV-`|` (см. CLAUDE.md).
//!
//! ## Последовательная `+`: регистр шага (ADR 0057)
//!
//! Каждой цепочке `+` дан независимый регистр `<state>_step` (сброс `STEP_0`,
//! задача 0057-01). В `always_comb` внутри ветви несущего состояния — вложенный
//! `unique case (<step>)`, где активен ровно **один** шаг. При завершении шага
//! `<step>_next` продвигается к следующему; на последнем шаге переход выполняет
//! **родительское** состояние. Тайминг совпадает с C `break`: следующий шаг
//! тикается на такте **после** завершения предыдущего (регистр защёлкивает
//! `<step>_next`).

use crate::diagnostics::Diagnostic;
use crate::generator::indent::Printer;
use crate::generator::sv::sv_blocks::emit_named_blocks;
use crate::generator::sv::sv_fsm::{
    Fsm, emit_model_body, end_variant, step_reg_name, step_variant, sv002,
};
use crate::generator::sv::sv_map::SvMap;
use crate::semantic::StateNode;
use crate::semantic::minimap::{Name, StateExtend};

/// Печатает состояние-реализацию (`S = A | B` или `S = A + B`).
#[allow(clippy::too_many_arguments)]
pub(crate) fn emit_extend(
    p: &mut Printer,
    map: &SvMap,
    fsm: &Fsm,
    state_name: &Name,
    state: &StateNode,
    model: &Name,
    extend: &StateExtend,
    next: &Name,
    states: &[Name],
) -> Result<(), Diagnostic> {
    match extend {
        StateExtend::None => Ok(()),
        // Последовательная композиция: шаг-регистр + инлайн активного шага.
        StateExtend::Concatenation(items) => {
            emit_concatenation(p, map, fsm, state_name, state, model, items, next, states)
        }
        // Параллельная композиция (и вырожденный случай одной модели): под-модели
        // работают одновременно, родитель уходит дальше, когда завершились ВСЕ.
        StateExtend::Parallel(_) | StateExtend::Model(_, _) => {
            let done_exprs = inline_composed(p, map, fsm, extend)?;
            if done_exprs.is_empty() {
                return Ok(());
            }
            p.ident(&format!("if ({}) begin", done_exprs.join(" && ")))
                .nl();
            p.up();
            emit_parent_transition(p, map, fsm, state, model, next, states)?;
            p.down();
            p.ident("end").nl();
            Ok(())
        }
    }
}

/// Печатает цепочку `+`: `unique case (<step>)`, по одному активному шагу.
#[allow(clippy::too_many_arguments)]
fn emit_concatenation(
    p: &mut Printer,
    map: &SvMap,
    fsm: &Fsm,
    state_name: &Name,
    state: &StateNode,
    model: &Name,
    items: &[StateExtend],
    next: &Name,
    states: &[Name],
) -> Result<(), Diagnostic> {
    let step = step_reg_name(state_name);
    // `unique case`: варианты `STEP_0..STEP_{N-1}` покрыты все — ветвь по шагу на
    // каждый элемент, значит CASEINCOMPLETE не возникает.
    p.ident(&format!("unique case ({})", step)).nl();
    p.up();
    for (i, item) in items.iter().enumerate() {
        p.ident(&format!("{}: begin", step_variant(state_name, i)))
            .nl();
        p.up();
        let done_exprs = inline_composed(p, map, fsm, item)?;
        // Пустой шаг (`None`) не продвигается — но в цепочке `+` его не бывает.
        let cond = if done_exprs.is_empty() {
            "1'b0".to_string()
        } else {
            done_exprs.join(" && ")
        };
        p.ident(&format!("if ({}) begin", cond)).nl();
        p.up();
        if i + 1 == items.len() {
            // Последний шаг завершён — переход РОДИТЕЛЬСКОГО состояния.
            emit_parent_transition(p, map, fsm, state, model, next, states)?;
        } else {
            // Иначе — продвижение шага; следующий тикается на такте после
            // (регистр защёлкивает `_next`), как `break` в C.
            p.ident(&format!(
                "{}_next = {};",
                step,
                step_variant(state_name, i + 1)
            ))
            .nl();
        }
        p.down();
        p.ident("end").nl();
        p.down();
        p.ident("end").nl();
    }
    p.down();
    p.ident("endcase").nl();
    Ok(())
}

/// Инлайнит тело одного шага/ветви и возвращает done-выражения (на `_next`).
///
/// `Model` → инлайн такта под-модели + одно done-выражение; `Parallel` → инлайн
/// всех ветвей + конъюнкция done (ветвь `Parallel` может вкладываться). Прямая
/// вложенная `+` внутри шага — **явная диагностика** (R7): цель `c` такой случай
/// молча пропускает (`RS-021`), тишина здесь запрещена правилом 15.
fn inline_composed(
    p: &mut Printer,
    map: &SvMap,
    fsm: &Fsm,
    item: &StateExtend,
) -> Result<Vec<String>, Diagnostic> {
    match item {
        StateExtend::Model(sub, _) => {
            p.ident(&format!("// Под-модель '{}' — инлайн её такта.", sub))
                .nl();
            emit_model_body(p, map, fsm, sub)?;
            let sub_reg = fsm
                .state_reg
                .get(sub.unique())
                .ok_or_else(|| sv002(&format!("регистр состояния под-модели '{}'", sub)))?;
            // `_next`, а НЕ регистр: в C `_is_done` читает значение, только что
            // записанное тиком. Регистр дал бы значение предыдущего такта.
            Ok(vec![format!("({}_next == {})", sub_reg, end_variant(sub))])
        }
        StateExtend::Parallel(inner) => {
            let mut done_exprs = Vec::new();
            for it in inner {
                done_exprs.extend(inline_composed(p, map, fsm, it)?);
            }
            Ok(done_exprs)
        }
        StateExtend::Concatenation(_) => Err(sv002(
            "вложенная последовательная композиция (`+`) непосредственно внутри \
             шага композиции: цель 'sv' её не разворачивает. Оберните вложенную \
             цепочку в отдельную модель (`model M { start S = B + C; }`) и \
             используйте M как шаг — тогда она инлайнится штатным механизмом \
             уровней (регистр состояния под-модели). Это диагностика, а не \
             тишина: молчаливый пропуск дал бы автомат, стоящий на месте",
        )),
        StateExtend::None => Ok(Vec::new()),
    }
}

/// Печатает переход несущего (родительского) состояния после завершения
/// композиции: `exit` текущего → `enter` следующего → `state_next = NEXT` (или
/// `END`, если следующего нет). Общий хвост для `|` и последнего шага `+`.
fn emit_parent_transition(
    p: &mut Printer,
    map: &SvMap,
    fsm: &Fsm,
    state: &StateNode,
    model: &Name,
    next: &Name,
    states: &[Name],
) -> Result<(), Diagnostic> {
    // Собственные рёбра состояния-композиции (фича 0303) — ПЕРЕД `next`/`END`,
    // как у эталона: `ref` в порядке объявления, `next` последним (0181).
    if crate::generator::sv::sv_fsm::emit_transitions(p, map, fsm, state, model, states)? {
        return Ok(());
    }
    // `END` — только состоянию БЕЗ рёбер: у эталона узел завершается при пустом
    // списке переходов, а при несработавших остаётся в состоянии.
    if next.local().is_empty() && !state.references().is_empty() {
        return Ok(());
    }
    emit_named_blocks(p, state, fsm, "exit")?;
    let reg = fsm
        .state_reg
        .get(model.unique())
        .ok_or_else(|| sv002(&format!("регистр состояния модели '{}'", model)))?;
    if next.local().is_empty() {
        p.ident(&format!("{}_next = {};", reg, end_variant(model)))
            .nl();
    } else {
        let next_rc = map.raw_state_at(next.clone())?;
        emit_named_blocks(p, &next_rc.borrow(), fsm, "enter")?;
        p.ident(&format!(
            "{}_next = {};",
            reg,
            next.unique_uppercase_snakecase()
        ))
        .nl();
    }
    Ok(())
}
