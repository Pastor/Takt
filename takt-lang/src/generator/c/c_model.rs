//! Генерация автоматов состояний: init, tick, is_done, reset функции модели.
//!
//! Содержит логику генерации C-функций для всех моделей:
//! [`generate_model_functions`], [`generate_function_prototypes`]
//! и вспомогательные функции для работы с параллельными и последовательными состояниями.

use super::c_expr::{generate_condition_expr, generate_formula_check};
use crate::diagnostics::{Diagnostic, Location};
use crate::generator::c;
use crate::generator::c::c_map::CMap;
use crate::generator::indent::Printer;
use crate::semantic::StateNode;
use crate::semantic::minimap::{Element, Name, StateExtend};

/// Генерирует именованные блоки состояния (enter/exit/always).
use super::c_blocks::{generate_model_named_blocks, generate_named_blocks};
use super::c_model_init::{generate_concat_item_init, generate_model_init};

/// Генерирует прототипы функций для всех используемых моделей.
pub(super) fn generate_function_prototypes(
    printer: &mut Printer,
    map: &CMap,
) -> Result<(), Diagnostic> {
    let root_name = map.root_name();
    let sorted_models = c::topological_sort_models(map, map.using_models());

    if !sorted_models.is_empty() {
        for element in &sorted_models {
            let Element::Model { name, .. } = element else {
                continue;
            };
            let s = name.unique_camelcase();
            printer
                .print(&format!("/// Model functions '{}'", name))
                .nl();
            printer
                .print(&format!(
                    "static void {0}_init({0} *model, {1} *main);",
                    s,
                    root_name.unique_camelcase()
                ))
                .nl();
            printer
                .print(&format!(
                    "static void {0}_tick({0} *model, {1} *main);",
                    s,
                    root_name.unique_camelcase()
                ))
                .nl();
            printer
                .print(&format!(
                    "static bool {0}_is_done(const {0} *model, {1} *main);",
                    s,
                    root_name.unique_camelcase()
                ))
                .nl();
        }
        printer.nl();
    }
    Ok(())
}

/// Генерирует вызовы `_tick` для элементов параллельного блока.
///
/// Возвращает список C-выражений `{Name}_is_done(...)` для итоговой проверки
/// готовности всех веток. Вложенные параллели также тикаются рекурсивно.
fn generate_parallel_items_tick(
    printer: &mut Printer,
    parent_access: &str,
    parent_unique_upper: &str,
    items: &[StateExtend],
    call_append: &str,
) -> Vec<String> {
    let mut done_exprs = Vec::new();
    for (idx, item) in items.iter().enumerate() {
        match item {
            StateExtend::Model(name) => {
                let field = format!(
                    "{}.{}{}",
                    parent_access,
                    name.local_lowercase_snakecase(),
                    idx
                );
                printer
                    .ident(&format!(
                        "{}_tick(&{}{});",
                        name.unique_camelcase(),
                        field,
                        call_append,
                    ))
                    .nl();
                done_exprs.push(format!(
                    "{}_is_done(&{}{})",
                    name.unique_camelcase(),
                    field,
                    call_append,
                ));
            }
            StateExtend::Parallel(inner) => {
                let nested_access = format!("{}.parallel{}", parent_access, idx);
                let nested_upper = format!("{}_PARALLEL{}", parent_unique_upper, idx);
                let inner_done = generate_parallel_items_tick(
                    printer,
                    &nested_access,
                    &nested_upper,
                    inner,
                    call_append,
                );
                if !inner_done.is_empty() {
                    done_exprs.push(format!("({})", inner_done.join(" && ")));
                }
            }
            _ => {}
        }
    }
    done_exprs
}

/// Генерирует переходы между состояниями для простого состояния [`Element::State`].
///
/// Для каждой ссылки (`ref`-перехода) формирует:
/// - безусловный переход (`ConditionNode::None`): `exit → enter → state → break`
/// - условный переход: `if (cond) { exit → enter → state → break }`
fn generate_state_transitions(
    printer: &mut Printer,
    raw_state: &StateNode,
    map: &CMap,
    model: &Element,
    _model_name: &Name,
    states: &[Name],
) -> Result<(), Diagnostic> {
    use crate::semantic::ConditionNode;
    for reference in raw_state.references() {
        let Some(target) = states.iter().find(|n| n.local() == reference.name).cloned() else {
            continue; // целевое состояние не найдено в достижимых состояниях
        };
        let has_cond = !matches!(
            reference.cond,
            ConditionNode::None | ConditionNode::Unresolved(_)
        );
        if has_cond {
            match generate_condition_expr(&reference.cond, map, model) {
                Ok(cond_str) => {
                    printer.ident(&format!("if ({}) {{", cond_str)).up().nl();
                    generate_named_blocks(printer, raw_state, map, model, "exit")?;
                    let target_rc = map.raw_state_at(target.clone())?;
                    let target_raw = &*target_rc.borrow();
                    generate_named_blocks(printer, target_raw, map, model, "enter")?;
                    printer
                        .ident(&format!(
                            "model->state = {};",
                            target.unique_uppercase_snakecase()
                        ))
                        .nl();
                    printer.ident("break;").nl();
                    printer.down().ident("}").nl();
                }
                // 0028-01: было — печать комментария-заглушки «условный переход
                // не поддерживается» в
                // порождаемый C и **проглатывание** `Err`. Последствия (проба
                // воспроизводила): переход не генерировался, `taktc` печатал
                // «Скомпилировано» и завершался с кодом 0, а порождённый C
                // собирался БЕЗ ЗАМЕЧАНИЙ — комментарий сборку не ломает. На
                // выходе — мёртвый автомат, обнаруживаемый на объекте, а не в CI.
                //
                // Ошибка, а не предупреждение (ADR 0028, Option B): при коде
                // возврата 0 главный риск сохраняется. Ужесточение не ломает ни
                // одной работающей сборки — оно ломает ровно те, что были
                // сломаны и без него, просто молча (корпус `examples/` эту ветку
                // не задевает — проверено прогоном).
                Err(di) => {
                    return Err(Diagnostic::error_with_note(
                        // R3: позиция `ref` в исходнике, а не Location::Codegen —
                        // иначе пользователю негде искать причину.
                        reference.location,
                        format!(
                            "условный переход в состояние '{}' не переводится в C: {}",
                            target.local(),
                            di.message
                        ),
                        di.loc,
                        // R2: исходная причина приложена ЗАМЕТКОЙ, а не схлопнута
                        // в строку — код исходной диагностики иначе теряется.
                        match &di.code {
                            Some(code) => format!("причина [{}]: {}", code, di.message),
                            None => format!("причина: {}", di.message),
                        },
                    )
                    .with_code("CC-018"));
                }
            }
        } else {
            // Безусловный переход: exit → enter → state → break
            generate_named_blocks(printer, raw_state, map, model, "exit")?;
            let target_rc = map.raw_state_at(target.clone())?;
            let target_raw = &*target_rc.borrow();
            generate_named_blocks(printer, target_raw, map, model, "enter")?;
            printer
                .ident(&format!(
                    "model->state = {};",
                    target.unique_uppercase_snakecase()
                ))
                .nl();
            printer.ident("break;").nl();
        }
    }
    Ok(())
}

/// Генерирует переход из расширенного состояния (Parallel / Concatenation).
///
/// При пустом `next` выполняет `model->state = {MODEL}_END; break;`.
/// Иначе — устанавливает целевое состояние и генерирует блоки `exit` / `enter`.
/// Закрывающую `}` добавляет вызывающий код.
fn generate_extend_transition(
    printer: &mut Printer,
    raw_state: &StateNode,
    map: &CMap,
    model: &Element,
    model_name: &Name,
    next: &Name,
) -> Result<(), Diagnostic> {
    if next.local().is_empty() {
        // Переход в терминальное состояние: exit текущего → state = END → break
        generate_named_blocks(printer, raw_state, map, model, "exit")?;
        printer
            .ident(&format!(
                "model->state = {}_END;",
                model_name.unique_uppercase_snakecase()
            ))
            .nl();
        printer.ident("break;").nl();
    } else {
        // Переход в следующее состояние: exit текущего → enter следующего → state → break
        generate_named_blocks(printer, raw_state, map, model, "exit")?;
        let next_raw = map.raw_state_at(next.clone())?;
        let next_raw = &*next_raw.borrow();
        generate_named_blocks(printer, next_raw, map, model, "enter")?;
        printer
            .ident(&format!(
                "model->state = {};",
                next.unique_uppercase_snakecase()
            ))
            .nl();
        printer.ident("break;").nl();
    }
    Ok(())
}

/// Генерирует tick-логику для состояния с конкатенационной компоновкой.
///
/// Формирует цепочку `if / else if` по полю `{state_local}_state`:
/// каждый вариант тикает активный элемент и при его завершении инициализирует
/// следующий или выполняет переход (через [`generate_extend_transition`]).
#[allow(clippy::too_many_arguments)]
fn generate_concat_tick(
    printer: &mut Printer,
    state_local: &str,
    state_unique_upper: &str,
    items: &[StateExtend],
    call_append: &str,
    append: &str,
    raw_state: &StateNode,
    map: &CMap,
    model: &Element,
    model_name: &Name,
    next: &Name,
) -> Result<(), Diagnostic> {
    let state_field = format!("model->{}_state", state_local);
    for (idx, item) in items.iter().enumerate() {
        // Вычисляем имя варианта enum для текущего элемента
        let current_variant = match item {
            StateExtend::Model(name) => format!(
                "{}_{}{}",
                state_unique_upper,
                name.local_lowercase_snakecase().to_uppercase(),
                idx,
            ),
            StateExtend::Parallel(_) | StateExtend::Concatenation(_) => {
                format!("{}_PARALLEL{}", state_unique_upper, idx)
            }
            _ => continue,
        };

        // Открываем if / else if
        if idx == 0 {
            printer
                .ident(&format!("if ({} == {}) {{", state_field, current_variant))
                .up()
                .nl();
        } else {
            printer
                .down()
                .ident(&format!(
                    "}} else if ({} == {}) {{",
                    state_field, current_variant
                ))
                .up()
                .nl();
        }

        let is_last = idx + 1 >= items.len();

        match item {
            StateExtend::Model(name) => {
                let field = format!(
                    "model->{}_{}{}",
                    state_local,
                    name.local_lowercase_snakecase(),
                    idx
                );
                // Тик текущего элемента
                printer
                    .ident(&format!(
                        "{}_tick(&{}{});",
                        name.unique_camelcase(),
                        field,
                        call_append
                    ))
                    .nl();
                // Проверяем завершение
                printer
                    .ident(&format!(
                        "if ({}_is_done(&{}{})) {{",
                        name.unique_camelcase(),
                        field,
                        call_append,
                    ))
                    .up()
                    .nl();
                if is_last {
                    generate_extend_transition(printer, raw_state, map, model, model_name, next)?;
                } else {
                    let next_variant = generate_concat_item_init(
                        printer,
                        state_local,
                        state_unique_upper,
                        &items[idx + 1],
                        idx + 1,
                        append,
                    )?;
                    printer
                        .ident(&format!("{} = {};", state_field, next_variant))
                        .nl();
                    printer.ident("break;").nl();
                }
                printer.down().ident("}").nl();
            }
            StateExtend::Parallel(inner) => {
                let parallel_access = format!("model->{}_parallel{}", state_local, idx);
                let nested_upper = format!("{}_PARALLEL{}", state_unique_upper, idx);
                let done_exprs = generate_parallel_items_tick(
                    printer,
                    &parallel_access,
                    &nested_upper,
                    inner,
                    call_append,
                );
                if !done_exprs.is_empty() {
                    printer
                        .ident(&format!("if ({}) {{", done_exprs.join(" && ")))
                        .up()
                        .nl();
                    if is_last {
                        generate_extend_transition(
                            printer, raw_state, map, model, model_name, next,
                        )?;
                    } else {
                        let next_variant = generate_concat_item_init(
                            printer,
                            state_local,
                            state_unique_upper,
                            &items[idx + 1],
                            idx + 1,
                            append,
                        )?;
                        printer
                            .ident(&format!("{} = {};", state_field, next_variant))
                            .nl();
                        printer.ident("break;").nl();
                    }
                    printer.down().ident("}").nl();
                }
            }
            _ => {}
        }
    }
    // Закрываем последний if / else if блок
    if !items.is_empty() {
        printer.down().ident("}").nl();
    }
    Ok(())
}

fn generate_model_tick(
    printer: &mut Printer,
    model: &Element,
    map: &CMap,
) -> Result<(), Diagnostic> {
    let is_main = model.name().eq(&map.root_name());
    let model_name = model.name();
    let Element::Model {
        start,
        states,
        name,
    } = model
    else {
        return Err(Diagnostic::error(
            Location::Codegen,
            "Элемент не является моделью".to_string(),
        )
        .with_code("CC-006"));
    };

    // Проверки Guard-формул модели
    if map.guard_enable() {
        let raw_model = map.raw_model_at(model_name.clone())?;
        for formula in &raw_model.borrow().formulas {
            generate_formula_check(printer, map, model, formula)?;
        }
    }

    // 0033 (Option B): вход в стартовое состояние НЕ расходует такт. Работа
    // INIT (инициализация вложенных, блоки `enter`, установка стартового
    // состояния) диспетчеризуется в `if` ДО `switch` и НЕ завершается `break`,
    // поэтому тело стартового состояния исполняется в ЭТОМ ЖЕ такте — как в
    // симуляторе (`enter_initial_state`, 0025-04). Блоки `enter` остаются внутри
    // `_tick` (могут писать в порты), а не выносятся в `_init` вне цикла
    // сканирования. Правка рекурсивна по уровням: вложенный `_tick` делает то же,
    // поэтому сдвиг обнуляется на любой глубине.
    let raw_state = map.raw_state_at(start.clone())?;
    let raw_state = &*raw_state.borrow();
    let append = if !is_main { ", main" } else { ", model" };
    let call_append = if !is_main { ", main" } else { ", model" };
    printer
        .ident(&format!(
            "if (model->state == {}_INIT) {{",
            name.unique_uppercase_snakecase()
        ))
        .up()
        .nl();
    // Инициализация вложенных вынесена в `_init` (0033, R6). Здесь — только
    // ПОВЕДЕНИЕ входа: блоки `enter` (могут писать в порты, поэтому внутри такта)
    // и установка стартового состояния. Без `break` — тело исполняется в этом же
    // такте.
    let start_variant_upper = match map.state_at(start.clone()) {
        Some(Element::State { name, .. }) => name.unique_uppercase_snakecase(),
        Some(Element::StateExtend { name, .. }) => name.unique_uppercase_snakecase(),
        _ => {
            return Err(Diagnostic::error(
                Location::Codegen,
                "Начальное состояние модели не определено".to_string(),
            )
            .with_code("CC-008"));
        }
    };
    generate_named_blocks(printer, raw_state, map, model, "enter")?;
    printer
        .ident(&format!("model->state = {};", start_variant_upper))
        .nl();
    printer.down().ident("}").nl();

    // Фича 0083: именованные блоки **уровня модели** (`always` вне состояния)
    // исполняются КАЖДЫЙ такт до диспетчеризации состояния, безусловно по
    // состоянию — как шаг 2 `execution("always")` симулятора (эталон). Прежде
    // блок молча терялся: `generate_model_tick` эмитил только state-level
    // (`always` на композите работал, т.к. он — тело синтетического состояния).
    let raw_model = map.raw_model_at(model_name.clone())?;
    generate_model_named_blocks(printer, &raw_model.borrow(), map, model, "always")?;

    // Тело стартового состояния исполняется в том же такте (без `break` выше).
    printer.ident("switch (model->state) {").up().nl();
    let mut end_already_defined = false;
    for state_name in states.iter() {
        let raw_state = map.raw_state_at(state_name.clone())?;
        let raw_state = &*raw_state.borrow();
        let Some(state) = map.state_at(state_name.clone()) else {
            continue; // недостижимое состояние — пропускаем генерацию case
        };
        printer
            .ident("case ")
            .print(&state_name.unique_uppercase_snakecase())
            .print(": {")
            .up()
            .nl();

        // Проверка Guard-формул состояния
        if map.guard_enable() {
            for formula in raw_state.formulas() {
                generate_formula_check(printer, map, model, formula)?;
            }
        }

        generate_named_blocks(printer, raw_state, map, model, "always")?;
        // Периодические блоки `every` (фича 0134-09) — после `always`, как в
        // симуляторе (шаг `execute_every` идёт следом за `execution("always")`).
        crate::generator::c::c_every::emit_state_body(
            printer,
            map,
            model,
            &raw_model.borrow(),
            state_name.local(),
        )?;
        // 0028-02: было — цепочка `if let / else if let / else`, где хвостовой
        // `else` печатал в порождаемый C комментарий-заглушку «пока не
        // реализовано». Ветка
        // НЕДОСТИЖИМА (доказательство — в ветке `Element::Model` ниже), то есть
        // это мёртвый код, маскировавшийся под незавершённую работу.
        //
        // Исчерпывающий `match` без ветки `_` заменяет дисциплину на проверку
        // компилятором: новый вариант `Element` теперь ВАЛИТ СБОРКУ, а не
        // выпадает в молчаливый no-op. Цепочка `if let` этого не давала — для
        // неё компилятор обязан допустить `else`.
        match state {
            Element::State { .. } => {
                generate_state_transitions(printer, raw_state, map, model, &model_name, states)?;
                // Терминальное состояние (нет переходов) — явно переходим в END
                // Исключение: если состояние уже является END (не добавляем самопереход)
                if raw_state.is_terminated() && !state_name.local().to_uppercase().eq("END") {
                    generate_named_blocks(printer, raw_state, map, model, "exit")?;
                    printer
                        .ident(&format!(
                            "model->state = {}_END;",
                            model_name.unique_uppercase_snakecase()
                        ))
                        .nl();
                }
            }
            Element::StateExtend { extend, next, .. } => {
                // 0028-02: цепочка по `StateExtend` тоже стала исчерпывающим
                // `match` — у неё не было ветки `None`, и такое состояние молча
                // не порождало НИЧЕГО, даже комментария — тише заглушки №1.
                match extend {
                    StateExtend::Model(name) => {
                        printer
                            .ident(&format!(
                                "{}_tick(&model->{}",
                                name.unique_camelcase(),
                                state_name.local().to_lowercase()
                            ))
                            .print(call_append)
                            .print(");")
                            .nl();
                        printer
                            .ident(&format!(
                                "if ({}_is_done(&model->{}",
                                name.unique_camelcase(),
                                state_name.local().to_lowercase()
                            ))
                            .print(call_append)
                            .print(")) {")
                            .up()
                            .nl();
                        if next.local().is_empty() {
                            // exit текущего → state = END → break
                            generate_named_blocks(printer, raw_state, map, model, "exit")?;
                            printer
                                .ident(&format!(
                                    "model->state = {}_END;",
                                    model_name.unique_uppercase_snakecase()
                                ))
                                .nl();
                            printer.ident("break;").nl();
                        } else {
                            // exit текущего → enter следующего → state → break
                            generate_named_blocks(printer, raw_state, map, model, "exit")?;
                            let next_state = map.raw_state_at(next.clone())?;
                            let next_state = &*next_state.borrow();
                            generate_named_blocks(printer, next_state, map, model, "enter")?;
                            printer
                                .ident(&format!(
                                    "model->state = {};",
                                    next.unique_uppercase_snakecase()
                                ))
                                .nl();
                            printer.ident("break;").nl();
                        }
                        printer.down().ident("}").nl();
                    }
                    StateExtend::Parallel(steps) => {
                        let local = state_name.local_lowercase_snakecase();
                        let unique_upper = state_name.unique_uppercase_snakecase();
                        let access = format!("model->{}", local);
                        let done_exprs = generate_parallel_items_tick(
                            printer,
                            &access,
                            &unique_upper,
                            &steps,
                            call_append,
                        );
                        if !done_exprs.is_empty() {
                            printer
                                .ident(&format!("if ({}) {{", done_exprs.join(" && ")))
                                .up()
                                .nl();
                            generate_extend_transition(
                                printer,
                                raw_state,
                                map,
                                model,
                                &model_name,
                                &next,
                            )?;
                            printer.down().ident("}").nl();
                        }
                    }
                    StateExtend::Concatenation(steps) => {
                        let local = state_name.local_lowercase_snakecase();
                        let unique_upper = state_name.unique_uppercase_snakecase();
                        generate_concat_tick(
                            printer,
                            &local,
                            &unique_upper,
                            &steps,
                            call_append,
                            append,
                            raw_state,
                            map,
                            model,
                            &model_name,
                            &next,
                        )?;
                    }
                    // 0028-02: ветки для этого варианта НЕ БЫЛО — состояние с
                    // неразрешённой реализацией молча не порождало ничего.
                    // Достижимость не установлена (проба через `Extend::Unresolved`
                    // отклоняется семантикой раньше кодогенерации), поэтому
                    // поведение сохранено прежним — пустая генерация, — но теперь
                    // оно ЯВНОЕ и обосновано здесь, а не подразумевается отсутствием
                    // ветки. Задача 0028-02 объём не расширяет: доказать достижимость
                    // и выбрать между `Err` и пустой генерацией — предмет отдельной
                    // работы (кандидат в бэклоге).
                    StateExtend::None => {}
                }
            }
            // 0028-02: НЕДОСТИЖИМО по контракту `CMap::state_at` (`c_map.rs:125`):
            // он отдаёт элемент, только если `element.is_state()`, а тот —
            // `matches!(self, Element::State { .. } | Element::StateExtend { .. })`
            // (`minimap.rs:137`). То есть `Element::Model` сюда не попадает
            // никогда; прежде здесь печатался комментарий-заглушка.
            //
            // `unreachable!`, а не `Err`: инвариант обеспечен ТИПОМ-ФИЛЬТРОМ, а не
            // входными данными пользователя — диагностика предлагала бы ему
            // исправить то, чего он не писал.
            Element::Model { .. } => unreachable!(
                "CMap::state_at отдаёт только State/StateExtend (фильтр is_state); \
                 Element::Model сюда не попадает"
            ),
        }
        printer.ident("break;").nl();
        printer.down().ident("}").nl();
        if !end_already_defined {
            end_already_defined = state_name.local().to_uppercase().eq("END");
        }
    }
    if !end_already_defined {
        printer
            .ident("case ")
            .print(&name.unique_uppercase_snakecase())
            .print("_END: {")
            .up()
            .nl();
        printer.ident("break;").nl();
        printer.down().ident("}").nl();
    }
    // Вариант `_INIT` до `switch` не доходит (снят диспетчером выше, 0033), но
    // остаётся значением перечисления — `default` гасит -Wswitch, не пряча при
    // этом реальный пропуск состояния (все достижимые состояния имеют `case`).
    printer.ident("default: break;").nl();
    printer.down().ident("}").nl();
    // Счётчик выдержки (фича 0134) обновляется в КОНЦЕ такта — так значение,
    // видимое условиям на такте M, равно числу тактов, прошедших с входа в
    // состояние. Смена состояния в этом такте означает вход, поэтому счётчик
    // становится 1 (на следующем такте с входа пройдёт ровно один такт), иначе
    // растёт. Обновление стоит здесь ОДИН раз, а не рядом с каждым из десяти
    // присваиваний `model->state` — второй экземпляр этой логики неминуемо
    // разъехался бы с первым.
    let raw_model = map.raw_model_at(model_name.clone())?;
    // HAL-указатель: `model` у корня, `main` у под-модели (как порты).
    let hal_ptr = if model_name.eq(&map.root_name()) {
        "model"
    } else {
        "main"
    };
    crate::generator::c::c_time::emit_state_time_update(
        printer,
        map,
        &raw_model.borrow(),
        hal_ptr,
    )?;
    Ok(())
}

/// Генерирует все C-функции для модели: init, tick, reset, is_done.
pub(super) fn generate_model_functions(
    mut printer: &mut Printer,
    model: &Element,
    map: &CMap,
) -> Result<(), Diagnostic> {
    let is_main = model.name().eq(&map.root_name());
    let Element::Model {
        name,
        states: _,
        start: _,
    } = model
    else {
        return Err(Diagnostic::error(
            Location::Codegen,
            format!("Model {} not defined", model.name().unique_camelcase()),
        )
        .with_code("CC-006"));
    };
    let mut append = String::new();
    let mut call_append = String::new();
    if !is_main {
        append.push_str(&format!(", {} *main", map.root_name().unique_camelcase()));
        call_append.push_str(", main");
    }
    let struct_name = name.unique_camelcase();
    printer
        .print(&format!(
            "/// Функция инициализации модели {}",
            model.name()
        ))
        .nl();
    printer
        .print("void ")
        .print(&struct_name)
        .print("_init(")
        .print(&struct_name)
        .print(" *model")
        .print(&append)
        .print(") {")
        .nl();
    //NOTICE: init
    printer.up();
    printer.ident("assert(0 != model);").nl();

    generate_model_init(&mut printer, model, map)?;
    printer.down();
    printer.print("}").nl().nl();
    printer
        .print(&format!("/// Функция обработки модели {}", model.name()))
        .nl();
    printer
        .print("void ")
        .print(&struct_name)
        .print("_tick(")
        .print(&struct_name)
        .print(" *model")
        .print(&append)
        .print(") {")
        .nl();
    //NOTICE: tick
    printer.up();
    printer.ident("assert(0 != model);").nl();
    if !is_main {
        printer.ident("assert(0 != main);").nl();
    }
    generate_model_tick(&mut printer, model, map)?;
    printer.down();
    printer.print("}").nl().nl();
    printer
        .print(&format!("/// Функция сброса модели {}", model.name()))
        .nl();
    printer
        .print("void ")
        .print(&struct_name)
        .print("_reset(")
        .print(&struct_name)
        .print(" *model")
        .print(&append)
        .print(") {")
        .nl();
    printer
        .up()
        .ident(format!("{}_init(model", struct_name).as_str())
        .print(&call_append)
        .print(");")
        .down()
        .nl();
    printer.print("}").nl().nl();
    printer
        .print(&format!(
            "/// Функция проверки терминального состояния модели {}",
            model.name()
        ))
        .nl();
    printer
        .print("bool ")
        .print(&struct_name)
        .print("_is_done(const ")
        .print(&struct_name)
        .print(" *model")
        .print(&append)
        .print(") {")
        .nl();
    // Единственное терминальное состояние модели — всегда END
    let cond = format!("model->state == {}_END", name.unique_uppercase_snakecase());
    printer
        .up()
        .ident("return ")
        .print(cond.as_str())
        .print(";")
        .down()
        .nl();
    printer.print("}").nl().nl();
    Ok(())
}
