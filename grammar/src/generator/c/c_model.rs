//! Генерация автоматов состояний: init, tick, is_done, reset функции модели.
//!
//! Содержит логику генерации C-функций для всех моделей:
//! [`generate_model_functions`], [`generate_function_prototypes`]
//! и вспомогательные функции для работы с параллельными и последовательными состояниями.

use super::c_expr::{
    generate_code_block, generate_condition_expr, generate_expr, generate_formula_check,
};
use crate::diagnostics::{Diagnostic, Location};
use crate::generator::c;
use crate::generator::c::c_map::CMap;
use crate::generator::indent::Printer;
use crate::semantic::minimap::{Element, Name, StateExtend};
use crate::semantic::type_node::TypeNode;
use crate::semantic::{ExpressionNode, StateNode, VariableNode};

/// Генерирует именованные блоки состояния (enter/exit/always).
fn generate_named_blocks(
    printer: &mut Printer,
    state: &StateNode,
    map: &CMap,
    owner: &Element,
    block_name: &str,
) -> Result<(), Diagnostic> {
    let blocks = state.get_named_blocks(block_name);
    for block in blocks {
        let Some(stmt) = block.statement() else {
            continue;
        };
        generate_code_block(printer, map, owner, vec![], stmt, true)?;
    }
    Ok(())
}

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

fn generate_model_init(
    printer: &mut Printer,
    model: &Element,
    map: &CMap,
) -> Result<(), Diagnostic> {
    let Element::Model {
        start: _,
        states: _,
        name,
    } = model
    else {
        return Err(Diagnostic::error(
            Location::Codegen,
            "Элемент не является моделью".to_string(),
        )
        .with_code("CC-006"));
    };
    let raw = map.raw_model_at(name.clone())?;
    let raw = &*raw.borrow();
    printer
        .ident("model->state = ")
        .print(&name.unique_uppercase_snakecase())
        .print("_INIT;")
        .nl();
    for var in raw.variables.values() {
        let VariableNode::Simple {
            name: var_name,
            ty,
            expr,
            ..
        } = var
        else {
            continue;
        };
        // Пропускаем неиспользуемые переменные — они не попадают в struct
        if !map.usage().variables.contains(var_name) {
            continue;
        }
        if let ExpressionNode::None = expr {
            continue;
        }
        // 0029-05: массив в C **не присваивается** — ни скаляром, ни агрегатом.
        // Общая ветка ниже печатала `model->x = …` для любого типа, что на
        // настоящем массиве даёт «array type is not assignable».
        if is_real_array(ty) {
            generate_array_init(printer, map, model, &var.name(), ty, expr)?;
            continue;
        }
        printer.ident(&format!("model->{} = ", var.name()));
        generate_expr(printer, map, model, vec![], expr, 0, true)?;
        printer.print(";").nl();
    }
    Ok(())
}

/// Настоящий ли это массив C (`elem name[N]`), а не бит-вектор.
///
/// `[bit;N]` при N ∈ {8,16,32,64} — **скаляр** `uint{N}_t` (доминирующая идиома
/// корпуса), и присваивание ему законно. Прочие массивы — агрегаты.
fn is_real_array(ty: &TypeNode) -> bool {
    matches!(ty, TypeNode::Array(_, elem) if !matches!(**elem, TypeNode::Bit))
}

/// Инициализирует настоящий массив в `_init` — поэлементно.
///
/// # Почему не присваиванием
///
/// В C массив **не является** изменяемым lvalue: `model->data = …` отвергается
/// (`array type 'uint8_t[4]' is not assignable`) и для скаляра, и для агрегата,
/// и даже для составного литерала. Единственная форма — поэлементная запись
/// (либо `memcpy`, но он тянет `<string.h>` ради инициализации).
///
/// Дефект вскрыт правкой Д1 (0029-01) и **ею не создан**: прежде `[u8;4]`
/// объявлялось как `uint4_t data` — скаляр несуществующего типа, которому
/// присваивание синтаксически «сходилось». Стоило массиву стать настоящим —
/// вылез инициализатор.
///
/// # Скалярный инициализатор массива не поддерживается намеренно
///
/// `var data: [u8;4] := 0;` — что это значит, язык **не определяет**: обнулить
/// весь массив? записать 0 в первый элемент? Три ответа расходятся уже сегодня
/// (кандидат «Семантика `[bit;N]` расходится втрое» в `FEATURES.md`): цель `st`
/// инициализатор молча отбрасывает, полагаясь на обнуление по правилам IEC;
/// симулятор кладёт в переменную **скаляр** `Number(0)`, после чего чтение
/// `data[0]` даёт `SIM-010` («переменная не является массивом» — проба). Выбрать
/// один ответ здесь — значит решить вопрос семантики языка, на что фича 0029 не
/// уполномочена (см. её карточку). Поэтому — `CC-017`, а не догадка.
fn generate_array_init(
    printer: &mut Printer,
    map: &CMap,
    model: &Element,
    field: &str,
    ty: &TypeNode,
    expr: &ExpressionNode,
) -> Result<(), Diagnostic> {
    let TypeNode::Array(size, _) = ty else {
        return Err(Diagnostic::error(
            Location::Codegen,
            format!("переменная '{}': ожидался массив", field),
        )
        .with_code("CC-017"));
    };
    // Агрегат записывается двумя формами: `:= {0, 0}` (`Initializer`) и
    // `:= [0, 0]` (`Array`). Для C они неразличимы — обе дают поэлементную
    // запись; принимать только одну значило бы отвергать корректный исходник.
    let (ExpressionNode::Initializer(elems) | ExpressionNode::Array(elems)) = expr else {
        return Err(Diagnostic::error(
            Location::Codegen,
            format!(
                "переменная '{}': скалярный инициализатор массива не выразим в C — \
                 массив в C не присваивается; используйте агрегат вида ':= {{0, 0, …}}'",
                field
            ),
        )
        .with_code("CC-017"));
    };
    if elems.len() != usize::from(*size) {
        return Err(Diagnostic::error(
            Location::Codegen,
            format!(
                "переменная '{}': инициализатор из {} элементов не соответствует массиву [{}]",
                field,
                elems.len(),
                size
            ),
        )
        .with_code("CC-017"));
    }
    for (i, elem) in elems.iter().enumerate() {
        printer.ident(&format!("model->{}[{}] = ", field, i));
        generate_expr(printer, map, model, vec![], elem, 0, true)?;
        printer.print(";").nl();
    }
    Ok(())
}

/// Генерирует вызовы `_init` для элементов параллельного блока (рекурсивно).
///
/// * `parent_access` — путь к полю-структуре параллели (например, `"model->start"`).
/// * `parent_unique_upper` — уникальный префикс enum в UPPER_SNAKE_CASE
///   (например, `"EXTEND_COMPLEX_C_START"`), используется для формирования имён
///   enum-вариантов вложенных параллелей.
fn generate_parallel_items_init(
    printer: &mut Printer,
    parent_access: &str,
    parent_unique_upper: &str,
    items: &[StateExtend],
    append: &str,
) {
    for (idx, item) in items.iter().enumerate() {
        match item {
            StateExtend::Model(name) => {
                printer
                    .ident(&format!(
                        "{}_init(&{}.{}{}{});",
                        name.unique_camelcase(),
                        parent_access,
                        name.local_lowercase_snakecase(),
                        idx,
                        append,
                    ))
                    .nl();
            }
            StateExtend::Parallel(inner) => {
                let nested_access = format!("{}.parallel{}", parent_access, idx);
                let nested_upper = format!("{}_PARALLEL{}", parent_unique_upper, idx);
                generate_parallel_items_init(printer, &nested_access, &nested_upper, inner, append);
                printer
                    .ident(&format!("{}.state = {}_INIT;", nested_access, nested_upper))
                    .nl();
            }
            _ => {}
        }
    }
}

/// Генерирует вызов `_init` для одного элемента конкатенации и возвращает
/// соответствующий вариант enum `{state_local}_state`.
///
/// * `state_local` — локальное имя состояния в lowercase_snake_case (например, `"start"`).
/// * `state_unique_upper` — уникальный префикс enum в UPPER_SNAKE_CASE
///   (например, `"EXTEND_COMPLEX_START"`).
fn generate_concat_item_init(
    printer: &mut Printer,
    state_local: &str,
    state_unique_upper: &str,
    item: &StateExtend,
    idx: usize,
    append: &str,
) -> Result<String, Diagnostic> {
    match item {
        StateExtend::Model(name) => {
            printer
                .ident(&format!(
                    "{}_init(&model->{}_{}{}{});",
                    name.unique_camelcase(),
                    state_local,
                    name.local_lowercase_snakecase(),
                    idx,
                    append,
                ))
                .nl();
            Ok(format!(
                "{}_{}{}",
                state_unique_upper,
                name.local_lowercase_snakecase().to_uppercase(),
                idx,
            ))
        }
        StateExtend::Parallel(inner) => {
            let access = format!("model->{}_parallel{}", state_local, idx);
            let nested_upper = format!("{}_PARALLEL{}", state_unique_upper, idx);
            generate_parallel_items_init(printer, &access, &nested_upper, inner, append);
            printer
                .ident(&format!("{}.state = {}_INIT;", access, nested_upper))
                .nl();
            Ok(format!("{}_PARALLEL{}", state_unique_upper, idx))
        }
        _ => Err(Diagnostic::error(
            Location::Codegen,
            "Неподдерживаемый тип элемента конкатенации".to_string(),
        )
        .with_code("CC-007")),
    }
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
                Err(di) => {
                    // Условие не поддерживается — оставляем комментарий
                    printer
                        .ident(&format!(
                            "//TODO: условный переход в {} не поддерживается. Причина: {}",
                            target.local(),
                            di.message
                        ))
                        .nl();
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

    printer.ident("switch (model->state) {").up().nl();
    printer
        .ident("case ")
        .print(&*name.unique_uppercase_snakecase())
        .print("_INIT: {")
        .up()
        .nl();
    let raw_state = map.raw_state_at(start.clone())?;
    let raw_state = &*raw_state.borrow();
    let append = if !is_main { ", main" } else { ", model" };
    let call_append = if !is_main { ", main" } else { ", model" };
    match map.state_at(start.clone()) {
        Some(Element::State { name, .. }) => {
            // Простое стартовое состояние: enter → state
            generate_named_blocks(printer, raw_state, map, model, "enter")?;
            printer
                .ident(&format!(
                    "model->state = {};",
                    name.unique_uppercase_snakecase()
                ))
                .nl();
        }
        Some(Element::StateExtend {
            name: state_name,
            extend,
            next: _,
        }) => {
            if let StateExtend::Model(name) = extend {
                // _init → enter → state
                printer
                    .ident(&format!(
                        "{}_init(&model->{}",
                        name.unique_camelcase(),
                        state_name.local().to_lowercase()
                    ))
                    .print(append)
                    .print(");")
                    .nl();
                generate_named_blocks(printer, raw_state, map, model, "enter")?;
                printer
                    .ident(&format!(
                        "model->state = {};",
                        state_name.unique_uppercase_snakecase()
                    ))
                    .nl();
            } else if let StateExtend::Parallel(steps) = extend {
                // parallel_init → enter → state
                let local = state_name.local_lowercase_snakecase();
                let unique_upper = state_name.unique_uppercase_snakecase();
                let access = format!("model->{}", local);
                generate_parallel_items_init(printer, &access, &unique_upper, &steps, append);
                printer
                    .ident(&format!("model->{}.state = {}_INIT;", local, unique_upper))
                    .nl();
                generate_named_blocks(printer, raw_state, map, model, "enter")?;
                printer
                    .ident(&format!(
                        "model->state = {};",
                        state_name.unique_uppercase_snakecase()
                    ))
                    .nl();
            } else if let StateExtend::Concatenation(steps) = extend {
                // concat_init → enter → state
                let local = state_name.local_lowercase_snakecase();
                let unique_upper = state_name.unique_uppercase_snakecase();
                if let Some(first) = steps.first() {
                    let variant = generate_concat_item_init(
                        printer,
                        &local,
                        &unique_upper,
                        first,
                        0,
                        append,
                    )?;
                    printer
                        .ident(&format!("model->{}_state = {};", local, variant))
                        .nl();
                }
                generate_named_blocks(printer, raw_state, map, model, "enter")?;
                printer
                    .ident(&format!(
                        "model->state = {};",
                        state_name.unique_uppercase_snakecase()
                    ))
                    .nl();
            }
        }
        _ => {
            return Err(Diagnostic::error(
                Location::Codegen,
                "Начальное состояние модели не определено".to_string(),
            )
            .with_code("CC-008"));
        }
    }
    printer.ident("break;").nl();
    printer.down().ident("}").nl();
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
        if let Element::State { .. } = state {
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
        } else if let Element::StateExtend { extend, next, .. } = state {
            if let StateExtend::Model(name) = extend {
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
            } else if let StateExtend::Parallel(steps) = extend {
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
                    generate_extend_transition(printer, raw_state, map, model, &model_name, &next)?;
                    printer.down().ident("}").nl();
                }
            } else if let StateExtend::Concatenation(steps) = extend {
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
        } else {
            //TODO: Реализовать
            printer.ident("//FIXME: Пока не реализовано").nl();
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
    printer.down().ident("}").nl();
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
        .ident(format!("{}_init(model", &struct_name).as_str())
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
