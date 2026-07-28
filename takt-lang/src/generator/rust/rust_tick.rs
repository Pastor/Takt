//! Эмиссия такта автомата в цель Rust: `_tick` и переходы.
//!
//! Вынесено из `rust_model.rs` (фича 0088 — лимит размера модуля, ADR 0088):
//! чистое перемещение, вывод Rust байт-в-байт неизменен. Здесь — тело `tick`
//! (диспетчеризация состояний, контракт входа 0033), guard-формулы и все виды
//! переходов (простые, составные `= Модель`/`|`/`+`).

use crate::diagnostics::{Diagnostic, Location};
use crate::generator::indent::Printer;
use crate::generator::rust::rust_blocks::{emit_model_named_blocks, emit_named_blocks};
use crate::generator::rust::rust_decl::PortSet;
use crate::generator::rust::rust_expr::{Scope, condition_as_bool, unwrap_outer};
use crate::generator::rust::rust_map::RustMap;
use crate::generator::rust::rust_model::{
    ConcatStep, Instance, StateTable, needs_hal, seq_enum_name, seq_field_name, submodel_name,
};
use crate::generator::rust::rust_shared::{shared_type_name, shared_variables};
use crate::generator::rust::rust_stmt::StmtOutput;
use crate::semantic::minimap::{Element, Name, StateExtend};
use crate::semantic::type_node::TypeNode;
use crate::semantic::{Formula, ModelNode, StateNode};
use std::collections::BTreeSet;

#[allow(clippy::too_many_arguments)]
pub(crate) fn emit_tick(
    p: &mut Printer,
    map: &RustMap,
    name: &Name,
    model: &ModelNode,
    element: &Element,
    table: &StateTable,
    instances: &[(Name, Vec<Instance>)],
    concats: &[(Name, Vec<ConcatStep>)],
    start: &Name,
    states: &[Name],
    shared: &[(String, TypeNode)],
    is_root: bool,
    uses_hal: bool,
    ports: &PortSet,
    warnings: &mut Vec<Diagnostic>,
) -> Result<(), Diagnostic> {
    // Поле `hal` есть только у корня; под-модель получает HAL параметром.
    // Параметр даётся, только если он ДЕЙСТВИТЕЛЬНО нужен: неиспользуемый
    // параметр — такое же `-D warnings`, как неиспользуемое поле.
    let needs_hal_param = !is_root && uses_hal;
    // Под-модель получает общие переменные ОДНИМ параметром `&mut Shared`
    // (фича 0059), а не по одному. Корень владеет полем `self.shared` и параметра
    // не получает. `hal` — ПОСЛЕДНИМ (правило 5 ADR / инвариант 0050).
    let needs_shared_param = !is_root && !shared.is_empty();
    let mut params = String::new();
    if needs_shared_param {
        params.push_str(&format!(", shared: &mut {}", shared_type_name(map)));
    }
    if needs_hal_param {
        params.push_str(", hal: &mut H");
    }
    let generics = if needs_hal_param { "<H: Hal>" } else { "" };
    let vis = if is_root { "pub " } else { "" };
    let _ = ports;

    let hal_access = match (uses_hal, is_root) {
        (false, _) => "",
        (true, true) => "self.hal",
        (true, false) => "hal",
    };
    // Изменяемость локальных `var` считается по ВСЕМ телам модели заранее:
    // печать потоковая, и в точке `let` будущие присваивания ещё не видны.
    let mut assigned = BTreeSet::new();
    for state_name in states {
        if let Ok(raw) = map.raw_state_at(state_name.clone()) {
            for block in raw.borrow().named_blocks() {
                if let Some(stmt) = block.statement() {
                    crate::generator::rust::rust_stmt::collect_assigned(stmt, &mut assigned);
                }
            }
        }
    }
    // Фича 0083: присваивания из model-level `always` тоже участвуют в расчёте
    // мутабельности локальных `let` (печать потоковая — будущие записи не видны).
    for block in model.get_named_blocks("always") {
        if let Some(stmt) = block.statement() {
            crate::generator::rust::rust_stmt::collect_assigned(stmt, &mut assigned);
        }
    }
    let mut scope = Scope {
        model,
        shared: shared.iter().map(|(n, _)| n.clone()).collect(),
        // Композирующая модель (корень) владеет полем `self.shared`; под-модель
        // получает `Shared` параметром → `shared.x` (фича 0059).
        shared_via_self: is_root,
        locals: Vec::new(),
        assigned,
        hal: hal_access.to_string(),
        has_self: true,
        // У корня `hal` — поле `H`, у под-модели — параметр `&mut H`.
        hal_is_ref: !is_root,
        // Карта «под-модель → поле» для спецформы `S(Модель) = Состояние`.
        instances: instances
            .iter()
            .flat_map(|(_, list)| list.iter())
            .map(|i| (i.unique.clone(), i.field.clone()))
            .collect(),
        time_profile: map.time_profile(),
    };

    p.ident("/// Один такт автомата.").nl();
    if is_root {
        p.ident("///").nl();
        p.ident("/// Вход в стартовое состояние такта **не расходует** (контракт")
            .nl();
        p.ident("/// ADR 0033): его тело исполняется в этом же вызове.")
            .nl();
    }
    // Параметров такта теперь всегда ≤ 3 (`self` + `&mut Shared?` + `&mut H?`,
    // фича 0059): общие переменные свёрнуты в одну структуру, поэтому заглушки
    // `#[allow(clippy::too_many_arguments)]` больше нет — политика (а) ADR 0050
    // без исключений. Раньше `MovementController` (`stacker.takt`) разделял с
    // корнем ВОСЕМЬ переменных (девять — размер корня) и такт получал 10
    // параметров.
    p.ident(&format!(
        "{}fn tick{}(&mut self{}) {{",
        vis, generics, params
    ))
    .nl();
    p.up();

    let mut out = StmtOutput::default();

    // Guard-формулы модели — первыми, как в цели `c`.
    if map.guard_enable() {
        for formula in &model.formulas {
            emit_guard(p, formula, &scope)?;
        }
    }

    // ── Контракт 0033: вход в стартовое состояние ────────────────────────────
    // `if` ДО `match`, без выхода из такта. В C здесь нет `break`, и тело
    // стартового состояния исполняется в том же такте; здесь того же добивается
    // `match`, читающий свежезаписанное `self.state`.
    let start_raw = map.raw_state_at(start.clone())?;
    p.ident(&format!("if self.state == {}::Init {{", table.enum_name))
        .nl();
    p.up();
    emit_named_blocks(p, &start_raw.borrow(), "enter", &mut scope, &mut out)?;
    p.ident(&format!("self.state = {};", table.path_of(start)?))
        .nl();
    // Латч метки времени входа в стартовое состояние (фича 0134, профиль «часы»):
    // отсчёт выдержки — от входа «до такта 1», а не от нуля абсолютного времени.
    crate::generator::rust::rust_time::emit_first_entry_latch(p, map, model, hal_access);
    p.down();
    p.ident("}").nl();

    // Фича 0083: model-level `always` (вне состояния) — каждый такт до `match`,
    // безусловно по состоянию (эталон — шаг 2 `execution("always")` симулятора).
    emit_model_named_blocks(p, model, "always", &mut scope, &mut out)?;

    // ── Разбор состояний ─────────────────────────────────────────────────────
    p.ident("match self.state {").nl();
    p.up();
    for state_name in states {
        let Some(state) = map.state_at(state_name.clone()) else {
            continue;
        };
        let raw = map.raw_state_at(state_name.clone())?;
        let raw = &*raw.borrow();
        p.ident(&format!("{} => {{", table.path_of(state_name)?))
            .nl();
        p.up();

        if map.guard_enable() {
            for formula in raw.formulas() {
                emit_guard(p, formula, &scope)?;
            }
        }
        emit_named_blocks(p, raw, "always", &mut scope, &mut out)?;
        // Периодические блоки `every` (фича 0134-09) — после `always`, как в
        // симуляторе (`execute_every` следом за `execution("always")`).
        crate::generator::rust::rust_every::emit_state_body(
            p,
            map,
            model,
            state_name.local(),
            hal_access,
            &mut scope,
            &mut out,
        )?;

        match &state {
            Element::State { .. } => {
                let emitted = emit_transitions(p, raw, map, table, states, &mut scope, &mut out)?;
                // Терминальное состояние: переходов нет — уходим в End. Состояние,
                // само являющееся End, самоперехода не получает.
                if raw.is_terminated() && !emitted && table.variant_of(state_name)? != "End" {
                    emit_named_blocks(p, raw, "exit", &mut scope, &mut out)?;
                    p.ident(&format!("self.state = {};", table.end_path())).nl();
                }
            }
            Element::StateExtend { extend, next, .. } => {
                emit_extend(
                    p, map, table, state_name, raw, extend, next, instances, concats, name,
                    &mut scope, &mut out, is_root,
                )?;
            }
            Element::Model { .. } => {
                return Err(Diagnostic::error(
                    Location::Codegen,
                    "Модель в позиции состояния".to_string(),
                )
                .with_code("RS-012"));
            }
        }
        p.down();
        p.ident("}").nl();
    }
    if table.emit_end {
        p.ident(&format!("{} => {{}}", table.end_path())).nl();
    }
    p.ident(&format!("{}::Init => {{}}", table.enum_name)).nl();
    p.down();
    p.ident("}").nl();

    // Обновление счётчика/метки времени в КОНЦЕ такта (фича 0134): одним
    // сравнением с `takt_prev_state`, как `c_time::emit_state_time_update`.
    crate::generator::rust::rust_time::emit_tick_update(p, map, model, hal_access)?;

    p.down();
    p.ident("}").nl().nl();

    let _ = (name, element);
    warnings.append(&mut out.warnings);
    Ok(())
}

/// Печатает guard-формулу как `assert!`.
///
/// Проба 2026-07-16: `assert!` живёт в `core`, профиль `no_std` её не теряет.
/// Имя инварианта попадает в сообщение — в цели `c` генератор его игнорирует,
/// здесь оно бесплатно и полезно.
fn emit_guard(p: &mut Printer, formula: &Formula, scope: &Scope) -> Result<(), Diagnostic> {
    match formula {
        Formula::Guard(cond, label) => {
            let text = condition_as_bool(cond, scope)?;
            let text = unwrap_outer(&text);
            match label {
                Some(label) => {
                    p.ident(&format!(
                        "assert!({}, \"нарушен инвариант '{}'\");",
                        text, label
                    ))
                    .nl();
                }
                None => {
                    p.ident(&format!("assert!({});", text)).nl();
                }
            }
            Ok(())
        }
        Formula::Formulas(items) => {
            for item in items {
                emit_guard(p, item, scope)?;
            }
            Ok(())
        }
        // LTL описывает бесконечные прогоны — предмет `taktc verify`, а не
        // прошивки. Цель `c` поступает так же.
        Formula::LTL(_) | Formula::None => Ok(()),
    }
}

/// Печатает переходы состояния. Возвращает `true`, если эмитирован безусловный.
///
/// Цепочка `if`/`else if` вместо C-шного «`if (c) {…break;}` подряд»: `break` в
/// C означает «такт окончен», то есть следующие `if` при сработавшем первом
/// недостижимы. Семантика та же, но недостижимого кода нет — а он валит гейт
/// (`unreachable_statement`, проба П5).
fn emit_transitions(
    p: &mut Printer,
    raw: &StateNode,
    map: &RustMap,
    table: &StateTable,
    states: &[Name],
    scope: &mut Scope,
    out: &mut StmtOutput,
) -> Result<bool, Diagnostic> {
    use crate::semantic::ConditionNode;
    // СОСЕДНИЕ рёбра в одно и то же состояние сливаются в одно условие через
    // `||`. Это не косметика: тела у них совпадают дословно (тот же `exit`, тот
    // же `enter`, то же присваивание), и clippy справедливо считает такую
    // цепочку подозрительной (`if_same_then_else`). Слияние семантику
    // сохраняет — при одинаковых телах «первое сработавшее» неотличимо от
    // дизъюнкции.
    //
    // Сливаются только СОСЕДНИЕ: между рёбрами в разные состояния порядок
    // значим (первое сработавшее выигрывает), и переставлять их нельзя.
    // Реальный случай — `LiftOperating` в `stacker.takt`: захват и укладка
    // ведут в одно `LiftDone` по разным условиям.
    let mut edges: Vec<(Name, Vec<&crate::semantic::ReferenceNode<StateNode>>)> = Vec::new();
    for reference in raw.references() {
        let Some(target) = states.iter().find(|n| n.local() == reference.name).cloned() else {
            continue;
        };
        if map.state_at(target.clone()).is_none() {
            continue;
        }
        match edges.last_mut() {
            Some((last, group)) if last.unique() == target.unique() => group.push(reference),
            _ => edges.push((target, vec![reference])),
        }
    }

    let mut first = true;
    for (target, group) in edges {
        let reference = group[0];
        let unconditional = group
            .iter()
            .any(|r| matches!(r.cond, ConditionNode::None | ConditionNode::Unresolved(_)));
        if unconditional {
            // Безусловный переход. Всё, что за ним, недостижимо — в C это молча,
            // в Rust валит `-D warnings`. Поэтому эмиссия рёбер прекращается.
            if first {
                emit_named_blocks(p, raw, "exit", scope, out)?;
                emit_enter_of(p, map, &target, scope, out)?;
                p.ident(&format!("self.state = {};", table.path_of(&target)?))
                    .nl();
            } else {
                p.ident("} else {").nl();
                p.up();
                emit_named_blocks(p, raw, "exit", scope, out)?;
                emit_enter_of(p, map, &target, scope, out)?;
                p.ident(&format!("self.state = {};", table.path_of(&target)?))
                    .nl();
                p.down();
                p.ident("}").nl();
            }
            return Ok(true);
        }
        // Условия группы объединяются через `||` — тела у них совпадают.
        let mut parts = Vec::new();
        for r in &group {
            parts.push(condition_as_bool(&r.cond, scope).map_err(|di| {
                Diagnostic::error_with_note(
                    r.location,
                    format!(
                        "условный переход в состояние '{}' не переводится в Rust: {}",
                        target.local(),
                        di.message
                    ),
                    di.loc,
                    match &di.code {
                        Some(code) => format!("причина [{}]: {}", code, di.message),
                        None => format!("причина: {}", di.message),
                    },
                )
                .with_code("RS-020")
            })?);
        }
        let cond = if parts.len() == 1 {
            parts.remove(0)
        } else {
            format!("({})", parts.join(" || "))
        };
        let _unused = |di: Diagnostic| -> Diagnostic {
            Diagnostic::error_with_note(
                reference.location,
                format!(
                    "условный переход в состояние '{}' не переводится в Rust: {}",
                    target.local(),
                    di.message
                ),
                di.loc,
                match &di.code {
                    Some(code) => format!("причина [{}]: {}", code, di.message),
                    None => format!("причина: {}", di.message),
                },
            )
            .with_code("RS-020")
        };
        let head = if first { "if" } else { "} else if" };
        first = false;
        p.ident(&format!("{} {} {{", head, unwrap_outer(&cond)))
            .nl();
        p.up();
        emit_named_blocks(p, raw, "exit", scope, out)?;
        emit_enter_of(p, map, &target, scope, out)?;
        p.ident(&format!("self.state = {};", table.path_of(&target)?))
            .nl();
        p.down();
    }
    if !first {
        p.ident("}").nl();
    }
    Ok(false)
}

/// Печатает блоки `enter` целевого состояния перехода.
fn emit_enter_of(
    p: &mut Printer,
    map: &RustMap,
    target: &Name,
    scope: &mut Scope,
    out: &mut StmtOutput,
) -> Result<(), Diagnostic> {
    let raw = map.raw_state_at(target.clone())?;
    emit_named_blocks(p, &raw.borrow(), "enter", scope, out)
}

/// Печатает такт составного состояния (`= Модель`, `A | B`, `A + B`).
#[allow(clippy::too_many_arguments)]
fn emit_extend(
    p: &mut Printer,
    map: &RustMap,
    table: &StateTable,
    state_name: &Name,
    raw: &StateNode,
    extend: &StateExtend,
    next: &Name,
    instances: &[(Name, Vec<Instance>)],
    concats: &[(Name, Vec<ConcatStep>)],
    model_name: &Name,
    scope: &mut Scope,
    out: &mut StmtOutput,
    is_root: bool,
) -> Result<(), Diagnostic> {
    let Some((_, list)) = instances
        .iter()
        .find(|(n, _)| n.unique() == state_name.unique())
    else {
        return Ok(());
    };
    match extend {
        StateExtend::None => Ok(()),
        // Параллельная композиция: тикают ВСЕ ветви каждый такт, в порядке
        // объявления — как в цели `c` (ветвь, уже завершённая, тикает в свой
        // `End` вхолостую). Порядок обязан совпадать с C, иначе потактовая
        // сверка разъедется.
        StateExtend::Model(_) | StateExtend::Parallel(_) => {
            let mut done = Vec::new();
            for instance in list {
                let args = call_args(map, instance, scope, is_root)?;
                p.ident(&format!("self.{}.tick({});", instance.field, args))
                    .nl();
                done.push(format!("self.{}.is_done()", instance.field));
            }
            if done.is_empty() {
                return Ok(());
            }
            p.ident(&format!("if {} {{", done.join(" && "))).nl();
            p.up();
            emit_extend_transition(p, map, table, raw, next, scope, out)?;
            p.down();
            p.ident("}").nl();
            Ok(())
        }
        // Последовательная композиция: шаги идут ПО ОЧЕРЕДИ, внутри шага
        // (параллельная группа) — одновременно.
        StateExtend::Concatenation(_) => {
            let Some((_, steps)) = concats
                .iter()
                .find(|(n, _)| n.unique() == state_name.unique())
            else {
                return Ok(());
            };
            let field = seq_field_name(state_name)?;
            let seq = seq_enum_name(model_name, state_name)?;
            for (idx, step) in steps.iter().enumerate() {
                let head = if idx == 0 { "if" } else { "} else if" };
                p.ident(&format!(
                    "{} self.{} == {}::{} {{",
                    head, field, seq, step.variant
                ))
                .nl();
                p.up();
                // Ветви шага тикают все и в порядке объявления — как в C.
                let mut done = Vec::new();
                for instance in &step.instances {
                    let args = call_args(map, instance, scope, is_root)?;
                    p.ident(&format!("self.{}.tick({});", instance.field, args))
                        .nl();
                    done.push(format!("self.{}.is_done()", instance.field));
                }
                p.ident(&format!("if {} {{", done.join(" && "))).nl();
                p.up();
                match steps.get(idx + 1) {
                    // Передача хода следующему шагу СТОИТ ТАКТА — как в C, где
                    // за установкой варианта стоит `break`. Здесь такт кончается
                    // сам: цепочка `else if` уже вошла в эту ветвь, и следующий
                    // шаг в этом же такте не тикнет.
                    Some(next_step) => {
                        for instance in &next_step.instances {
                            p.ident(&format!("self.{}.init();", instance.field)).nl();
                        }
                        p.ident(&format!("self.{} = {}::{};", field, seq, next_step.variant))
                            .nl();
                    }
                    // Последний шаг завершён — уходим из составного состояния.
                    None => emit_extend_transition(p, map, table, raw, next, scope, out)?,
                }
                p.down();
                p.ident("}").nl();
                p.down();
            }
            p.ident("}").nl();
            Ok(())
        }
    }
}

/// Печатает переход по завершении реализации состояния.
fn emit_extend_transition(
    p: &mut Printer,
    map: &RustMap,
    table: &StateTable,
    raw: &StateNode,
    next: &Name,
    scope: &mut Scope,
    out: &mut StmtOutput,
) -> Result<(), Diagnostic> {
    emit_named_blocks(p, raw, "exit", scope, out)?;
    if next.local().is_empty() {
        p.ident(&format!("self.state = {};", table.end_path())).nl();
        return Ok(());
    }
    emit_enter_of(p, map, next, scope, out)?;
    p.ident(&format!("self.state = {};", table.path_of(next)?))
        .nl();
    Ok(())
}

/// Строит список аргументов вызова `tick` под-модели.
///
/// Аргументы обязаны совпадать с параметрами, которые печатает [`emit_tick`] для
/// **той же** модели: и то и другое считается по одним предикатам
/// ([`needs_hal`], [`shared_variables`]). Разойдись они — порождённый код не
/// собрался бы, а в худшем случае связал бы не те переменные.
fn call_args(
    map: &RustMap,
    instance: &Instance,
    scope: &Scope,
    is_root: bool,
) -> Result<String, Diagnostic> {
    let sub_name = submodel_name(map, &instance.unique).ok_or_else(|| {
        Diagnostic::error(
            Location::Codegen,
            format!("Под-модель '{}' не найдена в карте", instance.unique),
        )
        .with_code("RS-012")
    })?;

    let mut args = Vec::new();
    // Общие переменные — ОДНИМ аргументом `&mut Shared` (фича 0059), в порядке
    // сигнатуры (shared, затем hal). У корня — `&mut self.shared`; у под-модели —
    // ретрансляция полученного параметра `shared` (метод-вызов перезаимствует
    // его автоматически). Передаётся, только если вызываемой модели он нужен.
    if !shared_variables(map, &sub_name).is_empty() {
        if is_root {
            args.push("&mut self.shared".to_string());
        } else {
            args.push("shared".to_string());
        }
    }
    // HAL — ПОСЛЕДНИМ (правило 5 ADR / инвариант 0050), тем же предикатом, каким
    // `emit_tick` решает, объявлять ли параметр.
    if !scope.hal.is_empty() && needs_hal(map, &sub_name, false, &mut BTreeSet::new()) {
        args.push(scope.hal_argument("вызов такта под-модели")?);
    }
    Ok(args.join(", "))
}
