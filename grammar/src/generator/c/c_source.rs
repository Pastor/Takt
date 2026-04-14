//! Генерация исходного C-файла (`.c`) из семантического дерева BuT.
//!
//! Содержит все функции генерации `.c`-исходника:
//! [`generate_source`], вспомогательные `resolve_variable_c_expr`,
//! `generate_function_call`, `generate_stmt_expression`,
//! `generate_code_block` и утилиты именования.
//!
//! ## Состояние реализации
//!
//! Генерация `.c`-файлов поддерживает:
//! - Все унарные и бинарные операторы (включая `pow()` для `**`).
//! - Чтение/запись портов через указатели `read_bit`/`write_bit`/`read_float`/`write_float`.
//! - Присваивание, вызовы локальных и внешних функций.
//! - Встроенные функции: `min`, `max`, `abs`, `clamp` (раскрываются как тернарные выражения).
//! - Условные операторы (`if`/`else`), циклы (`loop`, `for`), объявления переменных.

use super::{get_c_type, get_typed_variable};
use crate::diagnostics::{Diagnostic, Location};
use crate::generator::c;
use crate::generator::c::c_map::CMap;
use crate::generator::indent::Printer;
use crate::semantic::extend::Extend;
use crate::semantic::minimap::{Element, Name, StateExtend};
use crate::semantic::naming::normalize_lowercase_snakecase;
use crate::semantic::type_node::TypeNode;
use crate::semantic::{
    ConditionDefinitionNode, ExpressionNode, FunctionDefinitionNode, ModelNode, StateNode,
    StatementNode, VariableNode,
};
use std::cell::RefCell;
use std::rc::Rc;

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
        generate_code_block(printer, map, owner, vec![], stmt)?;
    }
    Ok(())
}

fn generate_function_prototypes(printer: &mut Printer, map: &CMap) -> Result<(), Diagnostic> {
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
                    "static void {0}_init({0} *model, const {1} *main);",
                    s,
                    root_name.unique_camelcase()
                ))
                .nl();
            printer
                .print(&format!(
                    "static void {0}_tick({0} *model, const {1} *main);",
                    s,
                    root_name.unique_camelcase()
                ))
                .nl();
            printer
                .print(&format!(
                    "static bool {0}_is_done(const {0} *model, const {1} *main);",
                    s,
                    root_name.unique_camelcase()
                ))
                .nl();
        }
        printer.nl();
    }
    Ok(())
}

/// Генерирует содержимое `.c`-файла для модели.
pub(super) fn generate_source(filename: &str, map: &CMap) -> Result<String, Diagnostic> {
    let mut source = String::new();
    let mut printer = Printer::new(4, &mut source);
    printer
        .print(format!("#include \"{}.h\"", filename).as_str())
        .nl();
    printer.print("#include <assert.h>").nl();
    printer.print("#include <math.h>").nl();
    generate_constants_and_ports_and_enums(&mut printer, map)?;
    generate_function_prototypes(&mut printer, map)?;
    generate_functions(&mut printer, map)?;
    for model in map.using_models() {
        generate_model_functions(&mut printer, &model, map)?;
    }
    generate_model_functions(&mut printer, &map.model(), map)?;
    Ok(source)
}

fn generate_model_functions(
    mut printer: &mut Printer,
    model: &Element,
    map: &CMap,
) -> Result<(), Diagnostic> {
    let is_main = model.name().eq(&map.root_name());
    let Element::Model {
        name,
        states,
        start,
    } = model
    else {
        return Err(Diagnostic::error(
            Location::Codegen,
            format!("Model {} not defined", model.name().unique_camelcase()),
        ));
    };
    let mut append = String::new();
    let mut call_append = String::new();
    if !is_main {
        append.push_str(&format!(
            ", const {} *main",
            map.root_name().unique_camelcase()
        ));
        call_append.push_str(&", main".to_string());
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
    let mut cond = String::new();
    for state_name in states.iter() {
        let state = map.raw_state_at(state_name.clone())?;
        let state = &*state.borrow();
        if !state.is_terminated() {
            continue;
        }
        if !cond.is_empty() {
            cond.push_str(" || ");
        }
        cond.push_str("model->state == ");
        cond.push_str(&state_name.unique_uppercase_snakecase());
    }
    if cond.is_empty() {
        cond.push_str("false");
    }
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

fn generate_model_init(
    printer: &mut Printer,
    model: &Element,
    map: &CMap,
) -> Result<(), Diagnostic> {
    let Element::Model {
        start,
        states,
        name,
    } = model
    else {
        return Err(Diagnostic::error(
            Location::Codegen,
            "Элемент не является моделью".to_string(),
        ));
    };
    let raw = map.raw_model_at(name.clone())?;
    let raw = &*raw.borrow();
    printer
        .ident("model->state = ")
        .print(&name.unique_uppercase_snakecase())
        .print("_INIT;")
        .nl();
    for var in raw.variables.values() {
        let VariableNode::Simple { name, ty, expr, .. } = var else {
            continue;
        };
        if let ExpressionNode::None = expr {
            continue;
        }
        printer.ident(&format!("model->{} = ", var.name()));
        generate_expr(printer, map, model, vec![], expr, 0)?;
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
        )),
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
        printer
            .ident(&format!(
                "model->state = {}_END;",
                model_name.unique_uppercase_snakecase()
            ))
            .nl();
        printer.ident("break;").nl();
    } else {
        printer
            .ident(&format!(
                "model->state = {};",
                next.unique_uppercase_snakecase()
            ))
            .nl();
        generate_named_blocks(printer, raw_state, map, model, "exit")?;
        let next_raw = map.raw_state_at(next.clone())?;
        let next_raw = &*next_raw.borrow();
        generate_named_blocks(printer, next_raw, map, model, "enter")?;
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
    let is_main = model.name().eq(&map.root_name());
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
        ));
    };
    printer.ident("switch (model->state) {").up().nl();
    printer
        .ident("case ")
        .print(&*name.unique_uppercase_snakecase())
        .print("_INIT: {")
        .up()
        .nl();
    let raw_state = map.raw_state_at(start.clone())?;
    let raw_state = &*raw_state.borrow();
    generate_named_blocks(printer, raw_state, map, model, "enter")?;
    let append = if !is_main { ", main" } else { "" };
    let call_append = if !is_main { ", main" } else { ", model" };
    match map.state_at(start.clone()) {
        Some(Element::State { name, .. }) => {
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
            next,
        }) => {
            if let StateExtend::Model(name) = extend {
                printer
                    .ident(&format!(
                        "{}_init(&model->{}",
                        name.unique_camelcase(),
                        state_name.local().to_lowercase()
                    ))
                    .print(append)
                    .print(");")
                    .nl();
                printer
                    .ident(&format!(
                        "model->state = {};",
                        state_name.unique_uppercase_snakecase()
                    ))
                    .nl();
            } else if let StateExtend::Parallel(steps) = extend {
                let local = state_name.local_lowercase_snakecase();
                let unique_upper = state_name.unique_uppercase_snakecase();
                let access = format!("model->{}", local);
                generate_parallel_items_init(printer, &access, &unique_upper, &steps, append);
                printer
                    .ident(&format!("model->{}.state = {}_INIT;", local, unique_upper))
                    .nl();
                printer
                    .ident(&format!(
                        "model->state = {};",
                        state_name.unique_uppercase_snakecase()
                    ))
                    .nl();
            } else if let StateExtend::Concatenation(steps) = extend {
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
            ));
        }
    }
    printer.ident("break;").nl();
    printer.down().ident("}").nl();
    let mut end_already_defined = false;
    for state_name in states.iter() {
        let raw_state = map.raw_state_at(state_name.clone())?;
        let raw_state = &*raw_state.borrow();
        let Some(state) = map.state_at(state_name.clone()) else {
            return Err(Diagnostic::error(
                Location::Codegen,
                format!("Состояние '{}' не определено", state_name),
            ));
        };
        printer
            .ident("case ")
            .print(&state_name.unique_uppercase_snakecase())
            .print(": {")
            .up()
            .nl();
        generate_named_blocks(printer, raw_state, map, model, "always")?;
        if let Element::State { references, .. } = state {
            for reference in raw_state.references() {
                let cond = &reference.cond;
                //TODO: генерация условия перехода, если переходим то дополнительно генерируем код для блоков exit и enter следующего состояния
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
                    printer
                        .ident(&format!(
                            "model->state = {}_END;",
                            model_name.unique_uppercase_snakecase()
                        ))
                        .nl();
                    printer.ident("break;").nl();
                } else {
                    printer
                        .ident(&format!(
                            "model->state = {};",
                            next.unique_uppercase_snakecase()
                        ))
                        .nl();
                    generate_named_blocks(printer, raw_state, map, model, "exit")?;
                    let next_state = map.raw_state_at(next)?;
                    let next_state = &*next_state.borrow();
                    generate_named_blocks(printer, next_state, map, model, "enter")?;
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
        printer.ident("// FIXME: Пока не реализовано").nl();
        printer.ident("break;").nl();
        printer.down().ident("}").nl();
    }
    printer.down().ident("}").nl();
    Ok(())
}

fn generate_constants_and_ports_and_enums(
    printer: &mut Printer,
    map: &CMap,
) -> Result<(), Diagnostic> {
    let mut models = map.using_models();
    models.insert(
        0,
        Element::Model {
            name: map.root_name().clone(),
            states: map.states().clone(),
            start: map.start().clone(),
        },
    );
    for model in models {
        let model_name = model.name();
        let model = map.raw_model_at(model.name())?;
        let model = &*model.borrow();
        let variables = model.variables.clone().into_values();
        let mut lines = Vec::new();
        for var in variables {
            match var {
                VariableNode::Unresolved | VariableNode::Simple { .. } => {}
                VariableNode::Port { name, expr, .. } => {
                    let name = model_name.unique_uppercase_snakecase()
                        + "_"
                        + normalize_lowercase_snakecase(name.clone())
                            .to_uppercase()
                            .as_str();
                    let (address, _bit) = if let ExpressionNode::Address(address, bit) = expr {
                        (address, bit)
                    } else if let ExpressionNode::Number(address) = expr {
                        (address, 0)
                    } else {
                        return Err("Unresolved address".into());
                    };
                    lines.push(format!("#define PORT_{} 0x{:x}", name, address));
                }
                VariableNode::Const { name, ref expr, .. } => {
                    let name = model_name.unique_uppercase_snakecase()
                        + "_"
                        + normalize_lowercase_snakecase(name.clone())
                            .to_uppercase()
                            .as_str();
                    let value = const_expr_string(expr, &name)?;
                    lines.push(format!("#define CONST_{} {}", name, value));
                }
            }
        }
        if !lines.is_empty() {
            printer
                .print(format!("/// Константы и порты модели {}", model_name).as_str())
                .nl();
            lines.sort();
            printer.print(lines.join("\n").as_str()).nl();
        }

        let enums = model.enums.clone().into_values();
        let mut lines = Vec::new();
        for en in enums {
            let prefix = format!("#define ENUM_{}_", model_name.unique_uppercase_snakecase());
            for (name, value) in en.variants {
                lines.push(format!(
                    "{}{} {}",
                    prefix,
                    normalize_lowercase_snakecase(name.clone()).to_uppercase(),
                    value
                ));
            }
        }
        if !lines.is_empty() {
            printer
                .print(format!("/// Перечисления модели {}", model_name).as_str())
                .nl();
            lines.sort();
            printer.print(lines.join("\n").as_str()).nl();
        }
    }
    Ok(())
}

fn get_function_name(fun: &FunctionDefinitionNode) -> String {
    match fun {
        FunctionDefinitionNode::Local { upper, name, .. } => {
            let model_name = Name::from(upper.clone().unwrap().upgrade().unwrap());
            format!("{}_{}", model_name.unique_camelcase(), name)
        }
        FunctionDefinitionNode::External { name, .. } => name.clone(),
        _ => {
            unreachable!("Unresolved function definition");
        }
    }
}

/// Проверяет, является ли [`Extend`] прямой ссылкой на указанную модель.
///
/// Поддерживает `Extend::Model` и прозрачную обёртку `Extend::Parentless`.
fn extend_contains_model(extend: &Extend, target: &Rc<RefCell<ModelNode>>) -> bool {
    match extend {
        Extend::Model(m) => Rc::ptr_eq(m, target),
        Extend::Parentless(inner) => extend_contains_model(inner, target),
        _ => false,
    }
}

/// Возвращает имя поля в родительской C-структуре для вложенной модели.
///
/// Ищет в родительской модели состояние с `implements = Extend::Model(эта_модель)`
/// и возвращает имя этого состояния в snake_case (именно оно используется как поле
/// в сгенерированной C-структуре). Если не найдено — возвращает `None`.
fn field_name_in_parent(model_rc: &Rc<RefCell<ModelNode>>) -> Option<String> {
    let parent_rc = model_rc.borrow().upper.as_ref()?.upgrade()?;
    let parent = parent_rc.borrow();
    for (state_name, state_node) in &parent.states {
        if let StateNode::Implement { implements, .. } = state_node {
            if extend_contains_model(implements, model_rc) {
                return Some(normalize_lowercase_snakecase(state_name.clone()));
            }
        }
    }
    None
}

/// Преобразует [`VariableNode`] в C-выражение для чтения.
///
/// - `Simple` с `loc == Implicit` — локальная переменная (stack), доступ по имени.
/// - `Simple` — переменная модели, доступ `main->field` или `main->model.field`.
/// - `Const` — `CONST_{MODEL}_{NAME}`.
/// - `Port` — вызов `(*main->read_bit)(PORT_..., bit, main->userdata)`.
fn resolve_variable_c_expr(
    var: &VariableNode,
    params: &[(String, TypeNode)],
) -> Result<String, Diagnostic> {
    match var {
        VariableNode::Simple {
            name, upper, loc, ..
        } => {
            // Локальная переменная (объявлена через register_local_var) имеет loc == Implicit
            if matches!(loc, Location::Implicit) {
                return Ok(normalize_lowercase_snakecase(name.clone()));
            }
            // Параметр функции — тоже доступ по имени
            if params.iter().any(|(p, _)| p == name) {
                return Ok(normalize_lowercase_snakecase(name.clone()));
            }
            // Переменная уровня модели → main->field
            if let Some(model_rc) = upper.as_ref().and_then(|w| w.upgrade()) {
                // Извлекаем имя модели до вызова field_name_in_parent, чтобы избежать
                // двойного заимствования model_rc.
                let model_name_opt = model_rc.borrow().name.clone();
                if model_name_opt.is_some() {
                    // Вложенная модель: поле структуры называется по имени состояния-контейнера,
                    // а не по имени самой модели. Ищем это состояние в родителе.
                    let field = field_name_in_parent(&model_rc).unwrap_or_else(|| {
                        normalize_lowercase_snakecase(model_name_opt.unwrap_or_default())
                    });
                    Ok(format!(
                        "model->{}.{}",
                        field,
                        normalize_lowercase_snakecase(name.clone())
                    ))
                } else {
                    // Корневая модель — поле напрямую
                    Ok(format!(
                        "model->{}",
                        normalize_lowercase_snakecase(name.clone())
                    ))
                }
            } else {
                Ok(format!(
                    "model->{}",
                    normalize_lowercase_snakecase(name.clone())
                ))
            }
        }
        VariableNode::Const { name, upper, .. } => {
            // CONST_{MODEL_UPPERCASE}_{CONST_UPPERCASE}
            if let Some(model_rc) = upper.as_ref().and_then(|w| w.upgrade()) {
                let model_name = Name::from(model_rc);
                Ok(format!(
                    "CONST_{}_{}",
                    model_name.unique_uppercase_snakecase(),
                    normalize_lowercase_snakecase(name.clone()).to_uppercase()
                ))
            } else {
                Ok(format!(
                    "CONST_{}",
                    normalize_lowercase_snakecase(name.clone()).to_uppercase()
                ))
            }
        }
        VariableNode::Port {
            name,
            ty,
            expr,
            upper,
            ..
        } => {
            // Чтение порта через read_bit или read_float
            let model_name = if let Some(model_rc) = upper.as_ref().and_then(|w| w.upgrade()) {
                Name::from(model_rc)
            } else {
                return Err("Неразрешённый owner порта".into());
            };
            let port_name = format!(
                "PORT_{}_{}",
                model_name.unique_uppercase_snakecase(),
                normalize_lowercase_snakecase(name.clone()).to_uppercase()
            );
            let bit = if let ExpressionNode::Address(_, bit) = expr {
                *bit
            } else {
                0i64
            };
            match ty {
                TypeNode::Rational => Ok(format!(
                    "(*main->read_float)({}, main->userdata)",
                    port_name
                )),
                _ => Ok(format!(
                    "(*main->read_bit)({}, {}, main->userdata)",
                    port_name, bit
                )),
            }
        }
        VariableNode::Unresolved => Err("Неразрешённая переменная".into()),
    }
}

/// Генерирует список C-аргументов для вызова функции.
fn generate_args(
    map: &CMap,
    owner: &Element,
    params: &[(String, TypeNode)],
    args: &[ExpressionNode],
) -> Result<Vec<String>, Diagnostic> {
    let mut result = Vec::new();
    for arg in args {
        let mut s = String::new();
        let mut tmp = Printer::new(4, &mut s);
        generate_stmt_expression(&mut tmp, map, owner, params.to_vec(), arg)?;
        result.push(s);
    }
    Ok(result)
}

/// Генерирует C-вызов функции.
///
/// - `Local` → `{ModelCamelCase}_{name}(main, args...)`
/// - `External` → `{name}(args...)`
/// - `Builtin("min"|"max"|"abs"|"clamp")` → раскрывается как тернарное выражение
/// - `Builtin("debug"|"S")` → возвращает ошибку (не транслируется в C)
fn generate_function_call(
    printer: &mut Printer,
    map: &CMap,
    owner: &Element,
    params: Vec<(String, TypeNode)>,
    fun_def: &FunctionDefinitionNode,
    args: &[ExpressionNode],
) -> Result<(), Diagnostic> {
    match fun_def {
        FunctionDefinitionNode::Local { upper, name, .. } => {
            let model_rc =
                upper
                    .as_ref()
                    .and_then(|w| w.upgrade())
                    .ok_or_else(|| -> Diagnostic {
                        "Неразрешённый owner функции".into()
                    })?;
            let model_name = Name::from(model_rc);
            let func_name = format!("{}_{}", model_name.unique_camelcase(), name);
            let arg_strs = generate_args(map, owner, &params, args)?;
            let mut all_args = vec!["main".to_string()];
            all_args.extend(arg_strs);
            printer.print(&format!("{}({})", func_name, all_args.join(", ")));
        }
        FunctionDefinitionNode::External { name, .. } => {
            let arg_strs = generate_args(map, owner, &params, args)?;
            printer.print(&format!("{}({})", name, arg_strs.join(", ")));
        }
        FunctionDefinitionNode::Builtin(builtin_name, _, _) => match *builtin_name {
            "min" => {
                let arg_strs = generate_args(map, owner, &params, args)?;
                if arg_strs.len() >= 2 {
                    printer.print(&format!(
                        "((({a}) < ({b})) ? ({a}) : ({b}))",
                        a = arg_strs[0],
                        b = arg_strs[1]
                    ));
                }
            }
            "max" => {
                let arg_strs = generate_args(map, owner, &params, args)?;
                if arg_strs.len() >= 2 {
                    printer.print(&format!(
                        "((({a}) > ({b})) ? ({a}) : ({b}))",
                        a = arg_strs[0],
                        b = arg_strs[1]
                    ));
                }
            }
            "abs" => {
                let arg_strs = generate_args(map, owner, &params, args)?;
                if !arg_strs.is_empty() {
                    printer.print(&format!("((({x}) < 0) ? -({x}) : ({x}))", x = arg_strs[0]));
                }
            }
            "clamp" => {
                let arg_strs = generate_args(map, owner, &params, args)?;
                if arg_strs.len() >= 3 {
                    printer.print(&format!(
                        "((({x}) < ({lo})) ? ({lo}) : ((({x}) > ({hi})) ? ({hi}) : ({x})))",
                        x = arg_strs[0],
                        lo = arg_strs[1],
                        hi = arg_strs[2]
                    ));
                }
            }
            "debug" | "S" => {
                return Err(format!(
                    "Встроенная функция '{}' не поддерживается в C генераторе",
                    builtin_name
                )
                .as_str()
                .into());
            }
            other => {
                return Err(format!("Неизвестная встроенная функция '{}'", other)
                    .as_str()
                    .into());
            }
        },
        _ => {
            return Err("Неразрешённое определение функции".into());
        }
    }
    Ok(())
}

/// Возвращает C-приоритет выражения (больше = сильнее связывает).
///
/// Используется для минимизации лишних скобок: обёртка добавляется только
/// если приоритет дочернего узла ниже требуемого минимума от родителя.
fn expr_precedence(expr: &ExpressionNode) -> u8 {
    match expr {
        // Присваивание — наименьший приоритет
        ExpressionNode::Assign(..) => 1,
        // Тернарный оператор
        ExpressionNode::ConditionalOperator(..) => 2,
        // Логическое ИЛИ
        ExpressionNode::Or(..) => 3,
        // Логическое И
        ExpressionNode::And(..) => 4,
        // Побитовое ИЛИ
        ExpressionNode::BitwiseOr(..) => 5,
        // Побитовое исключающее ИЛИ
        ExpressionNode::BitwiseXor(..) => 6,
        // Побитовое И
        ExpressionNode::BitwiseAnd(..) => 7,
        // Равенство / неравенство
        ExpressionNode::Equal(..) | ExpressionNode::NotEqual(..) => 8,
        // Сравнение
        ExpressionNode::Less(..)
        | ExpressionNode::More(..)
        | ExpressionNode::LessEqual(..)
        | ExpressionNode::MoreEqual(..) => 9,
        // Битовые сдвиги
        ExpressionNode::ShiftLeft(..) | ExpressionNode::ShiftRight(..) => 10,
        // Аддитивные операторы
        ExpressionNode::Add(..) | ExpressionNode::Subtract(..) => 11,
        // Мультипликативные операторы
        ExpressionNode::Multiply(..) | ExpressionNode::Divide(..) | ExpressionNode::Modulo(..) => {
            12
        }
        // Унарные операторы и приведение типов
        ExpressionNode::Not(..)
        | ExpressionNode::BitwiseNot(..)
        | ExpressionNode::UnaryPlus(..)
        | ExpressionNode::Negate(..)
        | ExpressionNode::Cast(..) => 13,
        // Атомы: литералы, переменные, вызовы функций, скобки и т.п.
        _ => 15,
    }
}

/// Генерирует C-выражение из семантического узла с учётом приоритета операторов.
///
/// Скобки добавляются автоматически только там, где это необходимо для
/// сохранения семантики: если `expr_precedence(expr) < min_prec`.
///
/// Используйте `min_prec = 0` для выражений верхнего уровня.
fn generate_expr(
    printer: &mut Printer,
    map: &CMap,
    owner: &Element,
    params: Vec<(String, TypeNode)>,
    expr: &ExpressionNode,
    min_prec: u8,
) -> Result<(), Diagnostic> {
    let my_prec = expr_precedence(expr);
    let wrap = my_prec < min_prec;
    if wrap {
        printer.print("(");
    }
    match expr {
        ExpressionNode::None | ExpressionNode::Unresolved(_) => {
            return Err("Неразрешённое выражение".into());
        }

        // ── Литералы ──────────────────────────────────────────────────────────
        ExpressionNode::Number(n) => {
            printer.print(&n.to_string());
        }
        ExpressionNode::Bool(value) => {
            printer.print(if *value { "true" } else { "false" });
        }
        ExpressionNode::String(v) => {
            printer.print("\"").print(&v.join("")).print("\"");
        }
        ExpressionNode::Rational(s, neg) => {
            if *neg {
                printer.print("-");
            }
            printer.print(s);
        }

        // ── Унарные операторы ──────────────────────────────────────────────────
        // min_prec=14 для операнда: бинарные выражения (prec≤13) будут обёрнуты;
        // также исключает двусмысленные `--x` и `++x` (унарный + унарный).
        ExpressionNode::Not(e) => {
            printer.print("!");
            generate_expr(printer, map, owner, params, e, 14)?;
        }
        ExpressionNode::BitwiseNot(e) => {
            printer.print("~");
            generate_expr(printer, map, owner, params, e, 14)?;
        }
        ExpressionNode::UnaryPlus(e) => {
            printer.print("+");
            generate_expr(printer, map, owner, params, e, 14)?;
        }
        ExpressionNode::Negate(e) => {
            printer.print("-");
            generate_expr(printer, map, owner, params, e, 14)?;
        }

        // ── Степень → pow() ────────────────────────────────────────────────────
        ExpressionNode::Power(l, r) => {
            printer.print("pow((double)(");
            generate_expr(printer, map, owner, params.clone(), l, 0)?;
            printer.print("), (double)(");
            generate_expr(printer, map, owner, params, r, 0)?;
            printer.print("))");
        }

        // ── Бинарные арифметические ────────────────────────────────────────────
        // Левый операнд: допускается тот же приоритет (левоассоциативность).
        // Правый операнд: требует более высокого приоритета (wrap при равном).
        ExpressionNode::Multiply(l, r) => {
            generate_expr(printer, map, owner, params.clone(), l, 12)?;
            printer.print(" * ");
            generate_expr(printer, map, owner, params, r, 13)?;
        }
        ExpressionNode::Divide(l, r) => {
            generate_expr(printer, map, owner, params.clone(), l, 12)?;
            printer.print(" / ");
            generate_expr(printer, map, owner, params, r, 13)?;
        }
        ExpressionNode::Modulo(l, r) => {
            generate_expr(printer, map, owner, params.clone(), l, 12)?;
            printer.print(" % ");
            generate_expr(printer, map, owner, params, r, 13)?;
        }
        ExpressionNode::Add(l, r) => {
            generate_expr(printer, map, owner, params.clone(), l, 11)?;
            printer.print(" + ");
            generate_expr(printer, map, owner, params, r, 12)?;
        }
        ExpressionNode::Subtract(l, r) => {
            generate_expr(printer, map, owner, params.clone(), l, 11)?;
            printer.print(" - ");
            generate_expr(printer, map, owner, params, r, 12)?;
        }

        // ── Битовые сдвиги ────────────────────────────────────────────────────
        ExpressionNode::ShiftLeft(l, r) => {
            generate_expr(printer, map, owner, params.clone(), l, 10)?;
            printer.print(" << ");
            generate_expr(printer, map, owner, params, r, 11)?;
        }
        ExpressionNode::ShiftRight(l, r) => {
            generate_expr(printer, map, owner, params.clone(), l, 10)?;
            printer.print(" >> ");
            generate_expr(printer, map, owner, params, r, 11)?;
        }

        // ── Побитовые операторы ────────────────────────────────────────────────
        ExpressionNode::BitwiseAnd(l, r) => {
            generate_expr(printer, map, owner, params.clone(), l, 7)?;
            printer.print(" & ");
            generate_expr(printer, map, owner, params, r, 8)?;
        }
        ExpressionNode::BitwiseXor(l, r) => {
            generate_expr(printer, map, owner, params.clone(), l, 6)?;
            printer.print(" ^ ");
            generate_expr(printer, map, owner, params, r, 7)?;
        }
        ExpressionNode::BitwiseOr(l, r) => {
            generate_expr(printer, map, owner, params.clone(), l, 5)?;
            printer.print(" | ");
            generate_expr(printer, map, owner, params, r, 6)?;
        }

        // ── Сравнение ─────────────────────────────────────────────────────────
        ExpressionNode::Less(l, r) => {
            generate_expr(printer, map, owner, params.clone(), l, 9)?;
            printer.print(" < ");
            generate_expr(printer, map, owner, params, r, 10)?;
        }
        ExpressionNode::More(l, r) => {
            generate_expr(printer, map, owner, params.clone(), l, 9)?;
            printer.print(" > ");
            generate_expr(printer, map, owner, params, r, 10)?;
        }
        ExpressionNode::LessEqual(l, r) => {
            generate_expr(printer, map, owner, params.clone(), l, 9)?;
            printer.print(" <= ");
            generate_expr(printer, map, owner, params, r, 10)?;
        }
        ExpressionNode::MoreEqual(l, r) => {
            generate_expr(printer, map, owner, params.clone(), l, 9)?;
            printer.print(" >= ");
            generate_expr(printer, map, owner, params, r, 10)?;
        }
        ExpressionNode::Equal(l, r) => {
            generate_expr(printer, map, owner, params.clone(), l, 8)?;
            printer.print(" == ");
            generate_expr(printer, map, owner, params, r, 9)?;
        }
        ExpressionNode::NotEqual(l, r) => {
            generate_expr(printer, map, owner, params.clone(), l, 8)?;
            printer.print(" != ");
            generate_expr(printer, map, owner, params, r, 9)?;
        }

        // ── Логические ────────────────────────────────────────────────────────
        ExpressionNode::And(l, r) => {
            generate_expr(printer, map, owner, params.clone(), l, 4)?;
            printer.print(" && ");
            generate_expr(printer, map, owner, params, r, 5)?;
        }
        ExpressionNode::Or(l, r) => {
            generate_expr(printer, map, owner, params.clone(), l, 3)?;
            printer.print(" || ");
            generate_expr(printer, map, owner, params, r, 4)?;
        }

        // ── Специальные ───────────────────────────────────────────────────────
        // Явные скобки из исходного кода — всегда генерируем как есть.
        ExpressionNode::Parenthesis(e) => {
            printer.print("(");
            generate_expr(printer, map, owner, params, e, 0)?;
            printer.print(")");
        }

        // Тернарный оператор: условие обёртывается при prec ≤ ||, чтобы
        // присваивание или вложенный тернарный в условии был явно выделен.
        ExpressionNode::ConditionalOperator(cond, then_, else_) => {
            generate_expr(printer, map, owner, params.clone(), cond, 4)?;
            printer.print(" ? ");
            generate_expr(printer, map, owner, params.clone(), then_, 0)?;
            printer.print(" : ");
            generate_expr(printer, map, owner, params, else_, 0)?;
        }

        ExpressionNode::Assign(l, r) => {
            // Запись в порт → write_bit / write_float
            if let ExpressionNode::Variable(var_rc) = l.as_ref() {
                let var = var_rc.borrow();
                if let VariableNode::Port {
                    name,
                    ty,
                    expr: addr_expr,
                    upper,
                    ..
                } = &*var
                {
                    let model_name =
                        if let Some(model_rc) = upper.as_ref().and_then(|w| w.upgrade()) {
                            Name::from(model_rc)
                        } else {
                            return Err("Неразрешённый owner порта при записи".into());
                        };
                    let port_name = format!(
                        "PORT_{}_{}",
                        model_name.unique_uppercase_snakecase(),
                        normalize_lowercase_snakecase(name.clone()).to_uppercase()
                    );
                    let bit = if let ExpressionNode::Address(_, bit) = addr_expr {
                        *bit
                    } else {
                        0i64
                    };
                    let mut rhs_str = String::new();
                    {
                        let mut tmp = Printer::new(4, &mut rhs_str);
                        generate_expr(&mut tmp, map, owner, params, r, 0)?;
                    }
                    match ty {
                        TypeNode::Rational => {
                            printer.print(&format!(
                                "(*main->write_float)({}, {}, main->userdata)",
                                port_name, rhs_str
                            ));
                        }
                        _ => {
                            printer.print(&format!(
                                "(*main->write_bit)({}, {}, {}, main->userdata)",
                                port_name, bit, rhs_str
                            ));
                        }
                    }
                    return Ok(());
                }
            }
            // Обычное присваивание (право-ассоциативно: тот же prec не оборачивается)
            generate_expr(printer, map, owner, params.clone(), l, 1)?;
            printer.print(" = ");
            generate_expr(printer, map, owner, params, r, 1)?;
        }

        ExpressionNode::ArraySubscript(var_rc, idx) => {
            let var = var_rc.borrow();
            let var_expr = resolve_variable_c_expr(&*var, &params)?;
            printer.print(&format!("{}[{}]", var_expr, idx));
        }

        ExpressionNode::Variable(var_rc) => {
            let var = var_rc.borrow();
            printer.print(&resolve_variable_c_expr(&*var, &params)?);
        }

        ExpressionNode::Condition(cond_rc) => {
            let cond = cond_rc.borrow();
            let cond_str = condition_macro_name(&*cond);
            printer.print(&cond_str);
        }

        ExpressionNode::Function(fun_rc, args) => {
            let fun = fun_rc.borrow();
            generate_function_call(printer, map, owner, params, &*fun, args)?;
        }

        ExpressionNode::Initializer(elems) => {
            printer.print("{");
            for (i, elem) in elems.iter().enumerate() {
                if i > 0 {
                    printer.print(", ");
                }
                generate_expr(printer, map, owner, params.clone(), elem, 0)?;
            }
            printer.print("}");
        }

        ExpressionNode::Array(elems) => {
            printer.print("{");
            for (i, elem) in elems.iter().enumerate() {
                if i > 0 {
                    printer.print(", ");
                }
                generate_expr(printer, map, owner, params.clone(), elem, 0)?;
            }
            printer.print("}");
        }

        ExpressionNode::Cast(expr, typ) => {
            let model = map.raw_model_at(owner.name())?;
            let model = &*model.borrow();
            let type_c = get_c_type(typ, model).unwrap_or_else(|| "int".to_string());
            // Приводимое выражение оборачивается при prec < UNARY (13),
            // то есть при наличии бинарных операторов: (int)(a + b).
            printer.print("(").print(&type_c).print(")");
            generate_expr(printer, map, owner, params, expr, 13)?;
        }

        // ── Неподдерживаемые ──────────────────────────────────────────────────
        ExpressionNode::ArraySlice(_, _, _) => {
            return Err("ArraySlice не поддерживается в C генераторе".into());
        }
        ExpressionNode::BitAccess(_, _) => {
            return Err("BitAccess не поддерживается в C генераторе".into());
        }
        ExpressionNode::CodeBlock(_, _) => {
            return Err("CodeBlock не поддерживается как выражение в C генераторе".into());
        }
        ExpressionNode::NamedFunctionBox(_, _) => {
            return Err("NamedFunctionBox не поддерживается в C генераторе".into());
        }
        ExpressionNode::List(_) => {
            return Err("List не поддерживается в C генераторе".into());
        }
        ExpressionNode::Type(_) => {
            return Err("Type не поддерживается как выражение в C генераторе".into());
        }
        ExpressionNode::Address(_, _) => {
            return Err("Address не поддерживается как выражение в C генераторе".into());
        }
        ExpressionNode::Model(_) => {
            return Err("Model не поддерживается как выражение в C генераторе".into());
        }
    }
    if wrap {
        printer.print(")");
    }
    Ok(())
}

/// Генерирует C-выражение из семантического узла выражения.
///
/// Функция пишет в `printer` без начального отступа и без завершающего `;\n`.
/// Отступ и разделители добавляет вызывающий код.
/// Является обёрткой над [`generate_expr`] с `min_prec = 0`.
fn generate_stmt_expression(
    printer: &mut Printer,
    map: &CMap,
    owner: &Element,
    params: Vec<(String, TypeNode)>,
    expr: &ExpressionNode,
) -> Result<(), Diagnostic> {
    generate_expr(printer, map, owner, params, expr, 0)
}

/// Генерирует имя макроса условия вида `COND_{MODEL}_{NAME}`.
fn condition_macro_name(cond: &ConditionDefinitionNode) -> String {
    if let Some(model_rc) = cond.upper.as_ref().and_then(|w| w.upgrade()) {
        let model_name = Name::from(model_rc);
        format!(
            "COND_{}_{}",
            model_name.unique_uppercase_snakecase(),
            normalize_lowercase_snakecase(cond.name.clone()).to_uppercase()
        )
    } else {
        format!(
            "COND_{}",
            normalize_lowercase_snakecase(cond.name.clone()).to_uppercase()
        )
    }
}

/// Генерирует C-оператор из семантического узла.
///
/// Для `Block` рекурсивно генерирует все вложенные операторы.
/// Для `Expression` генерирует выражение с отступом и `;`.
/// Поддерживает `If`, `Loop`, `For`, `Variable`, `Return`, `Continue`, `Break`.
fn generate_code_block(
    printer: &mut Printer,
    map: &CMap,
    owner: &Element,
    params: Vec<(String, TypeNode)>,
    body: &StatementNode,
) -> Result<(), Diagnostic> {
    match body {
        StatementNode::None => {}
        StatementNode::Unresolved(_) => {}

        StatementNode::Block(block) => {
            for stmt in block {
                generate_code_block(printer, map, owner, params.clone(), stmt)?;
            }
        }

        StatementNode::Expression(expr) => {
            // Генерируем во временный буфер, чтобы безопасно пропустить
            // неподдерживаемые встроенные функции (debug, S) без порчи вывода
            let mut expr_buf = String::new();
            let result = {
                let mut tmp = Printer::new(4, &mut expr_buf);
                generate_stmt_expression(&mut tmp, map, owner, params, expr)
            };
            match result {
                Ok(()) if !expr_buf.is_empty() => {
                    printer.ident(&expr_buf).print(";").nl();
                }
                Ok(()) => {}
                Err(_) => {
                    // Пропускаем неподдерживаемые выражения (debug, S и т.п.)
                }
            }
        }

        StatementNode::If { cond, then_, else_ } => {
            // Печатаем первый if
            printer.ident("if (");
            generate_stmt_expression(printer, map, owner, params.clone(), cond)?;
            printer.print(") {").up().nl();
            generate_code_block(printer, map, owner, params.clone(), then_)?;

            // Обходим цепочку else/else-if: если else-ветка — одиночный if,
            // схлопываем в `} else if (...)`, чтобы не создавать лишней вложенности
            let mut current_else = else_.as_deref();
            loop {
                match current_else {
                    None => {
                        // Нет else — закрываем последний блок
                        printer.down().ident("}").nl();
                        break;
                    }
                    Some(StatementNode::If {
                        cond: ec,
                        then_: et,
                        else_: ee,
                    }) => {
                        // else-ветка — одиночный if: схлопываем в else if
                        printer.down().ident("} else if (");
                        generate_stmt_expression(printer, map, owner, params.clone(), ec)?;
                        printer.print(") {").up().nl();
                        generate_code_block(printer, map, owner, params.clone(), et)?;
                        current_else = ee.as_deref();
                    }
                    Some(else_stmt) => {
                        // else-ветка — произвольный блок
                        printer.down().ident("} else {").up().nl();
                        generate_code_block(printer, map, owner, params.clone(), else_stmt)?;
                        printer.down().ident("}").nl();
                        break;
                    }
                }
            }
        }

        StatementNode::Loop { cond, body } => {
            match cond {
                None => {
                    printer.ident("while (true) ");
                }
                Some(cond_expr) => {
                    printer.ident("while (");
                    generate_stmt_expression(printer, map, owner, params.clone(), cond_expr)?;
                    printer.print(") ");
                }
            }
            generate_code_block(printer, map, owner, params.clone(), body)?;
            printer.nl();
        }

        StatementNode::For {
            init,
            cond,
            step,
            body,
        } => {
            let has_var_init = matches!(
                init.as_ref().map(|b| b.as_ref()),
                Some(StatementNode::Variable(..))
            );

            if has_var_init {
                // Объявление переменной выносим перед `for` в обёртку `{}`
                printer.ident("{").nl();
                printer.up();
                if let Some(init_stmt) = init {
                    generate_code_block(printer, map, owner, params.clone(), init_stmt)?;
                }
                printer.ident("for (;");
                if let Some(cond_expr) = cond {
                    printer.print(" ");
                    generate_stmt_expression(printer, map, owner, params.clone(), cond_expr)?;
                }
                printer.print(";");
                if let Some(step_expr) = step {
                    printer.print(" ");
                    generate_stmt_expression(printer, map, owner, params.clone(), step_expr)?;
                }
                printer.print(") ");
                generate_code_block(printer, map, owner, params.clone(), body)?;
                printer.nl();
                printer.down();
                printer.ident("}").nl();
            } else {
                printer.ident("for (");
                if let Some(init_stmt) = init {
                    // Инициализация — только выражение (без отступа и точки с запятой)
                    if let StatementNode::Expression(expr) = init_stmt.as_ref() {
                        generate_stmt_expression(printer, map, owner, params.clone(), expr)?;
                    }
                }
                printer.print(";");
                if let Some(cond_expr) = cond {
                    printer.print(" ");
                    generate_stmt_expression(printer, map, owner, params.clone(), cond_expr)?;
                }
                printer.print(";");
                if let Some(step_expr) = step {
                    printer.print(" ");
                    generate_stmt_expression(printer, map, owner, params.clone(), step_expr)?;
                }
                printer.print(") ");
                generate_code_block(printer, map, owner, params.clone(), body)?;
                printer.nl();
            }
        }

        StatementNode::Variable(name, ty, init) => {
            let model = map.raw_model_at(owner.name())?;
            let model_ref = model.borrow();
            let snake_name = normalize_lowercase_snakecase(name.clone());
            let decl = get_typed_variable(ty, Some(snake_name.clone()), &*model_ref)
                .unwrap_or_else(|| format!("int {}", snake_name));
            printer.ident(&decl);
            if let Some(init_expr) = init {
                printer.print(" = ");
                generate_stmt_expression(printer, map, owner, params, init_expr)?;
            }
            printer.print(";").nl();
        }

        StatementNode::Return(ret) => {
            printer.ident("return");
            if let Some(expr) = ret {
                printer.print(" ");
                generate_stmt_expression(printer, map, owner, params, expr)?;
            }
            printer.print(";").nl();
        }

        StatementNode::Continue => {
            printer.ident("continue;").nl();
        }

        StatementNode::Break => {
            printer.ident("break;").nl();
        }
    }
    Ok(())
}

fn generate_functions(printer: &mut Printer, map: &CMap) -> Result<(), Diagnostic> {
    let mut models = map.using_models();
    models.insert(
        0,
        Element::Model {
            name: map.root_name().clone(),
            states: map.states().clone(),
            start: map.start().clone(),
        },
    );
    for model in models {
        let element = model.clone();
        let model = map.raw_model_at(model.name())?;
        let model = &*model.borrow();
        let mut external_funcs = Vec::new();
        let mut local_funcs = Vec::new();
        for ref fun in model.functions.clone().into_values() {
            match fun {
                FunctionDefinitionNode::Local {
                    params, body, ret, ..
                } => {
                    let mut definition = String::new();
                    let mut tiny_params = params
                        .iter()
                        .map(|(name, typ)| {
                            get_c_type(&typ, model)
                                .map(|c_type| format!("{} {}", c_type, name.clone()))
                                .unwrap()
                        })
                        .collect::<Vec<String>>();
                    tiny_params.insert(0, format!("{} *main", map.root_name().unique_camelcase()));
                    definition.push_str(
                        format!(
                            "static {} {}({}) {{\n",
                            get_c_type(&ret, model).unwrap().as_str(),
                            get_function_name(&fun),
                            tiny_params.join(", ")
                        )
                        .as_str(),
                    );
                    let mut code_block = String::new();
                    {
                        let mut tmp_printer = Printer::new(4, &mut code_block);
                        tmp_printer.up();
                        generate_code_block(&mut tmp_printer, map, &element, params.clone(), body)?;
                        tmp_printer.down();
                    }
                    definition.push_str(&code_block);
                    definition.push_str("}\n");
                    local_funcs.push(definition);
                }
                FunctionDefinitionNode::External { params, ret, .. } => {
                    let params = params
                        .iter()
                        .map(|(name, typ)| {
                            get_c_type(typ, model)
                                .map(|c_type| format!("{} {}", c_type, name.clone()))
                                .unwrap()
                        })
                        .collect::<Vec<String>>();
                    external_funcs.push(format!(
                        "extern {} {}({});",
                        get_c_type(&ret, model).unwrap().as_str(),
                        get_function_name(&fun),
                        params.join(", ").as_str()
                    ));
                }
                _ => {
                    return Err(format!("Unresolved function '{}'", fun.name())
                        .as_str()
                        .into());
                }
            }
        }

        if !external_funcs.is_empty() {
            printer.print("///Внешние функции").nl();
            external_funcs.sort();
            for func in external_funcs {
                printer.print(func.as_str()).nl();
            }
        }
        if !local_funcs.is_empty() {
            printer.print("///Функции моделей").nl();
            local_funcs.sort();
            for func in local_funcs {
                printer.print(func.as_str()).nl();
            }
        }
    }
    Ok(())
}

fn const_expr_string(expr: &ExpressionNode, name: &String) -> Result<String, Diagnostic> {
    Ok(if let ExpressionNode::Number(value) = expr {
        value.to_string()
    } else if let ExpressionNode::Bool(value) = expr {
        if *value {
            "true".to_string()
        } else {
            "false".to_string()
        }
    } else if let ExpressionNode::String(value) = expr {
        format!("\"{}\"", value.join(""))
    } else if let ExpressionNode::Rational(value, _) = expr {
        value.clone()
    } else if let ExpressionNode::Initializer(value) = expr {
        let mut parts = Vec::new();
        for v in value.iter() {
            parts.push(const_expr_string(v, name)?);
        }
        format!("{{{}}}", parts.join(", "))
    } else {
        return Err(format!("Unresolved constant '{}' value: {:?}", name, expr)
            .as_str()
            .into());
    })
}

#[cfg(test)]
mod tests {
    use crate::generator::c::c_map::CMap;
    use crate::generator::c::c_source::generate_source;
    use crate::semantic::ExpressionNode;
    use crate::{parse, semantic};

    use super::*;

    // ── Вспомогательная функция ────────────────────────────────────────────────

    /// Создаёт минимальный CMap для тестов генерации выражений.
    fn make_map_and_owner(src: &str) -> (CMap, Element) {
        let (ast, _) = parse(src, 0).expect("ошибка разбора");
        let model_rc = semantic::tree::construct_model(&ast, None, &[]).unwrap();
        model_rc.borrow_mut().name = Some("Main".to_string());
        let model = model_rc.borrow();
        let map = CMap::new(model.name(), &*model).unwrap();
        let owner = Element::Model {
            name: map.root_name().clone(),
            states: map.states().clone(),
            start: map.start().clone(),
        };
        (map, owner)
    }

    /// Генерирует C-строку из выражения.
    fn expr_to_str(map: &CMap, owner: &Element, expr: &ExpressionNode) -> String {
        let mut s = String::new();
        let mut printer = Printer::new(4, &mut s);
        generate_stmt_expression(&mut printer, map, owner, vec![], expr).unwrap();
        s
    }

    // ── Тесты литералов ────────────────────────────────────────────────────────

    #[test]
    fn test_expr_number() {
        let (map, owner) = make_map_and_owner("start Main { always { } }");
        let expr = ExpressionNode::Number(42);
        assert_eq!(expr_to_str(&map, &owner, &expr), "42");
    }

    #[test]
    fn test_expr_bool_true() {
        let (map, owner) = make_map_and_owner("start Main { always { } }");
        assert_eq!(
            expr_to_str(&map, &owner, &ExpressionNode::Bool(true)),
            "true"
        );
    }

    #[test]
    fn test_expr_bool_false() {
        let (map, owner) = make_map_and_owner("start Main { always { } }");
        assert_eq!(
            expr_to_str(&map, &owner, &ExpressionNode::Bool(false)),
            "false"
        );
    }

    #[test]
    fn test_expr_rational_positive() {
        let (map, owner) = make_map_and_owner("start Main { always { } }");
        let expr = ExpressionNode::Rational("3.14".to_string(), false);
        assert_eq!(expr_to_str(&map, &owner, &expr), "3.14");
    }

    #[test]
    fn test_expr_rational_negative() {
        let (map, owner) = make_map_and_owner("start Main { always { } }");
        let expr = ExpressionNode::Rational("3.14".to_string(), true);
        assert_eq!(expr_to_str(&map, &owner, &expr), "-3.14");
    }

    #[test]
    fn test_expr_string() {
        let (map, owner) = make_map_and_owner("start Main { always { } }");
        let expr = ExpressionNode::String(vec!["hello".to_string()]);
        assert_eq!(expr_to_str(&map, &owner, &expr), "\"hello\"");
    }

    // ── Тесты унарных операторов ───────────────────────────────────────────────

    #[test]
    fn test_expr_negate() {
        let (map, owner) = make_map_and_owner("start Main { always { } }");
        // Атом (число) — скобки не нужны
        let expr = ExpressionNode::Negate(Box::new(ExpressionNode::Number(42)));
        assert_eq!(expr_to_str(&map, &owner, &expr), "-42");
    }

    #[test]
    fn test_expr_negate_complex() {
        let (map, owner) = make_map_and_owner("start Main { always { } }");
        // Бинарное выражение внутри унарного — скобки нужны
        let inner = ExpressionNode::Add(
            Box::new(ExpressionNode::Number(1)),
            Box::new(ExpressionNode::Number(2)),
        );
        let expr = ExpressionNode::Negate(Box::new(inner));
        assert_eq!(expr_to_str(&map, &owner, &expr), "-(1 + 2)");
    }

    #[test]
    fn test_expr_negate_negate() {
        let (map, owner) = make_map_and_owner("start Main { always { } }");
        // Двойное отрицание — скобки нужны чтобы избежать `--x` (декремент в C)
        let inner = ExpressionNode::Negate(Box::new(ExpressionNode::Number(5)));
        let expr = ExpressionNode::Negate(Box::new(inner));
        assert_eq!(expr_to_str(&map, &owner, &expr), "-(-5)");
    }

    #[test]
    fn test_expr_not() {
        let (map, owner) = make_map_and_owner("start Main { always { } }");
        // Атом — без скобок
        let expr = ExpressionNode::Not(Box::new(ExpressionNode::Bool(true)));
        assert_eq!(expr_to_str(&map, &owner, &expr), "!true");
    }

    #[test]
    fn test_expr_not_complex() {
        let (map, owner) = make_map_and_owner("start Main { always { } }");
        // Логическое И внутри NOT — нужны скобки
        let inner = ExpressionNode::And(
            Box::new(ExpressionNode::Bool(true)),
            Box::new(ExpressionNode::Bool(false)),
        );
        let expr = ExpressionNode::Not(Box::new(inner));
        assert_eq!(expr_to_str(&map, &owner, &expr), "!(true && false)");
    }

    #[test]
    fn test_expr_bitwise_not() {
        let (map, owner) = make_map_and_owner("start Main { always { } }");
        // Атом — без скобок
        let expr = ExpressionNode::BitwiseNot(Box::new(ExpressionNode::Number(0xFF)));
        assert_eq!(expr_to_str(&map, &owner, &expr), "~255");
    }

    #[test]
    fn test_expr_parenthesis() {
        let (map, owner) = make_map_and_owner("start Main { always { } }");
        // Явные скобки из исходника — всегда генерируются
        let inner = Box::new(ExpressionNode::Number(42));
        let expr = ExpressionNode::Parenthesis(inner);
        assert_eq!(expr_to_str(&map, &owner, &expr), "(42)");
    }

    // ── Тесты бинарных операторов ──────────────────────────────────────────────

    #[test]
    fn test_expr_add() {
        let (map, owner) = make_map_and_owner("start Main { always { } }");
        // Атомы — без скобок
        let expr = ExpressionNode::Add(
            Box::new(ExpressionNode::Number(1)),
            Box::new(ExpressionNode::Number(2)),
        );
        assert_eq!(expr_to_str(&map, &owner, &expr), "1 + 2");
    }

    #[test]
    fn test_expr_subtract() {
        let (map, owner) = make_map_and_owner("start Main { always { } }");
        let expr = ExpressionNode::Subtract(
            Box::new(ExpressionNode::Number(5)),
            Box::new(ExpressionNode::Number(3)),
        );
        assert_eq!(expr_to_str(&map, &owner, &expr), "5 - 3");
    }

    #[test]
    fn test_expr_multiply() {
        let (map, owner) = make_map_and_owner("start Main { always { } }");
        let expr = ExpressionNode::Multiply(
            Box::new(ExpressionNode::Number(4)),
            Box::new(ExpressionNode::Number(5)),
        );
        assert_eq!(expr_to_str(&map, &owner, &expr), "4 * 5");
    }

    #[test]
    fn test_expr_divide() {
        let (map, owner) = make_map_and_owner("start Main { always { } }");
        let expr = ExpressionNode::Divide(
            Box::new(ExpressionNode::Number(10)),
            Box::new(ExpressionNode::Number(2)),
        );
        assert_eq!(expr_to_str(&map, &owner, &expr), "10 / 2");
    }

    #[test]
    fn test_expr_modulo() {
        let (map, owner) = make_map_and_owner("start Main { always { } }");
        let expr = ExpressionNode::Modulo(
            Box::new(ExpressionNode::Number(7)),
            Box::new(ExpressionNode::Number(3)),
        );
        assert_eq!(expr_to_str(&map, &owner, &expr), "7 % 3");
    }

    #[test]
    fn test_expr_shift_left() {
        let (map, owner) = make_map_and_owner("start Main { always { } }");
        let expr = ExpressionNode::ShiftLeft(
            Box::new(ExpressionNode::Number(1)),
            Box::new(ExpressionNode::Number(4)),
        );
        assert_eq!(expr_to_str(&map, &owner, &expr), "1 << 4");
    }

    #[test]
    fn test_expr_shift_right() {
        let (map, owner) = make_map_and_owner("start Main { always { } }");
        let expr = ExpressionNode::ShiftRight(
            Box::new(ExpressionNode::Number(16)),
            Box::new(ExpressionNode::Number(2)),
        );
        assert_eq!(expr_to_str(&map, &owner, &expr), "16 >> 2");
    }

    #[test]
    fn test_expr_bitwise_and() {
        let (map, owner) = make_map_and_owner("start Main { always { } }");
        let expr = ExpressionNode::BitwiseAnd(
            Box::new(ExpressionNode::Number(0xF0)),
            Box::new(ExpressionNode::Number(0xFF)),
        );
        assert_eq!(expr_to_str(&map, &owner, &expr), "240 & 255");
    }

    #[test]
    fn test_expr_bitwise_or() {
        let (map, owner) = make_map_and_owner("start Main { always { } }");
        let expr = ExpressionNode::BitwiseOr(
            Box::new(ExpressionNode::Number(0xF0)),
            Box::new(ExpressionNode::Number(0x0F)),
        );
        assert_eq!(expr_to_str(&map, &owner, &expr), "240 | 15");
    }

    #[test]
    fn test_expr_bitwise_xor() {
        let (map, owner) = make_map_and_owner("start Main { always { } }");
        let expr = ExpressionNode::BitwiseXor(
            Box::new(ExpressionNode::Number(0xAA)),
            Box::new(ExpressionNode::Number(0x55)),
        );
        assert_eq!(expr_to_str(&map, &owner, &expr), "170 ^ 85");
    }

    #[test]
    fn test_expr_less() {
        let (map, owner) = make_map_and_owner("start Main { always { } }");
        let expr = ExpressionNode::Less(
            Box::new(ExpressionNode::Number(1)),
            Box::new(ExpressionNode::Number(2)),
        );
        assert_eq!(expr_to_str(&map, &owner, &expr), "1 < 2");
    }

    #[test]
    fn test_expr_more() {
        let (map, owner) = make_map_and_owner("start Main { always { } }");
        let expr = ExpressionNode::More(
            Box::new(ExpressionNode::Number(3)),
            Box::new(ExpressionNode::Number(2)),
        );
        assert_eq!(expr_to_str(&map, &owner, &expr), "3 > 2");
    }

    #[test]
    fn test_expr_equal() {
        let (map, owner) = make_map_and_owner("start Main { always { } }");
        let expr = ExpressionNode::Equal(
            Box::new(ExpressionNode::Number(0)),
            Box::new(ExpressionNode::Number(0)),
        );
        assert_eq!(expr_to_str(&map, &owner, &expr), "0 == 0");
    }

    #[test]
    fn test_expr_not_equal() {
        let (map, owner) = make_map_and_owner("start Main { always { } }");
        let expr = ExpressionNode::NotEqual(
            Box::new(ExpressionNode::Number(1)),
            Box::new(ExpressionNode::Number(0)),
        );
        assert_eq!(expr_to_str(&map, &owner, &expr), "1 != 0");
    }

    #[test]
    fn test_expr_logical_and() {
        let (map, owner) = make_map_and_owner("start Main { always { } }");
        let expr = ExpressionNode::And(
            Box::new(ExpressionNode::Bool(true)),
            Box::new(ExpressionNode::Bool(false)),
        );
        assert_eq!(expr_to_str(&map, &owner, &expr), "true && false");
    }

    #[test]
    fn test_expr_logical_or() {
        let (map, owner) = make_map_and_owner("start Main { always { } }");
        let expr = ExpressionNode::Or(
            Box::new(ExpressionNode::Bool(false)),
            Box::new(ExpressionNode::Bool(true)),
        );
        assert_eq!(expr_to_str(&map, &owner, &expr), "false || true");
    }

    #[test]
    fn test_expr_conditional_operator() {
        let (map, owner) = make_map_and_owner("start Main { always { } }");
        // Атомы — без скобок
        let expr = ExpressionNode::ConditionalOperator(
            Box::new(ExpressionNode::Bool(true)),
            Box::new(ExpressionNode::Number(1)),
            Box::new(ExpressionNode::Number(0)),
        );
        assert_eq!(expr_to_str(&map, &owner, &expr), "true ? 1 : 0");
    }

    #[test]
    fn test_expr_cast() {
        let (map, owner) = make_map_and_owner("start Main { always { } }");
        // Атом — без скобок после типа
        let expr = ExpressionNode::Cast(Box::new(ExpressionNode::Number(42)), TypeNode::Bit);
        assert_eq!(expr_to_str(&map, &owner, &expr), "(int)42");
    }

    #[test]
    fn test_expr_cast_complex() {
        let (map, owner) = make_map_and_owner("start Main { always { } }");
        // Бинарное выражение — нужны скобки после типа
        let inner = ExpressionNode::Add(
            Box::new(ExpressionNode::Number(1)),
            Box::new(ExpressionNode::Number(2)),
        );
        let expr = ExpressionNode::Cast(Box::new(inner), TypeNode::Bit);
        assert_eq!(expr_to_str(&map, &owner, &expr), "(int)(1 + 2)");
    }

    // ── Тесты приоритета операторов ────────────────────────────────────────────

    #[test]
    fn test_expr_precedence_mul_wins_over_add_left() {
        let (map, owner) = make_map_and_owner("start Main { always { } }");
        // (a*b) + c → a * b + c (умножение на левой стороне сложения — без скобок)
        let mul = ExpressionNode::Multiply(
            Box::new(ExpressionNode::Number(2)),
            Box::new(ExpressionNode::Number(3)),
        );
        let expr = ExpressionNode::Add(Box::new(mul), Box::new(ExpressionNode::Number(4)));
        assert_eq!(expr_to_str(&map, &owner, &expr), "2 * 3 + 4");
    }

    #[test]
    fn test_expr_precedence_add_needs_parens_in_mul() {
        let (map, owner) = make_map_and_owner("start Main { always { } }");
        // (a+b) * c → (a + b) * c (сложение в левом операнде умножения — скобки)
        let add = ExpressionNode::Add(
            Box::new(ExpressionNode::Number(2)),
            Box::new(ExpressionNode::Number(3)),
        );
        let expr = ExpressionNode::Multiply(Box::new(add), Box::new(ExpressionNode::Number(4)));
        assert_eq!(expr_to_str(&map, &owner, &expr), "(2 + 3) * 4");
    }

    #[test]
    fn test_expr_precedence_sub_right_needs_parens() {
        let (map, owner) = make_map_and_owner("start Main { always { } }");
        // a - (b - c) → a - (b - c) (тот же приоритет на правой стороне вычитания)
        let sub_right = ExpressionNode::Subtract(
            Box::new(ExpressionNode::Number(3)),
            Box::new(ExpressionNode::Number(1)),
        );
        let expr =
            ExpressionNode::Subtract(Box::new(ExpressionNode::Number(5)), Box::new(sub_right));
        assert_eq!(expr_to_str(&map, &owner, &expr), "5 - (3 - 1)");
    }

    #[test]
    fn test_expr_precedence_or_inside_and() {
        let (map, owner) = make_map_and_owner("start Main { always { } }");
        // (a || b) && c → (a || b) && c (OR имеет меньший приоритет чем AND)
        let or_expr = ExpressionNode::Or(
            Box::new(ExpressionNode::Bool(true)),
            Box::new(ExpressionNode::Bool(false)),
        );
        let expr = ExpressionNode::And(Box::new(or_expr), Box::new(ExpressionNode::Bool(true)));
        assert_eq!(expr_to_str(&map, &owner, &expr), "(true || false) && true");
    }

    #[test]
    fn test_expr_precedence_and_no_parens_inside_or() {
        let (map, owner) = make_map_and_owner("start Main { always { } }");
        // (a && b) || c → a && b || c (AND имеет больший приоритет чем OR — без скобок)
        let and_expr = ExpressionNode::And(
            Box::new(ExpressionNode::Bool(true)),
            Box::new(ExpressionNode::Bool(false)),
        );
        let expr = ExpressionNode::Or(Box::new(and_expr), Box::new(ExpressionNode::Bool(true)));
        assert_eq!(expr_to_str(&map, &owner, &expr), "true && false || true");
    }

    #[test]
    fn test_expr_precedence_compare_in_not() {
        let (map, owner) = make_map_and_owner("start Main { always { } }");
        // !(a > b) → !(a > b)
        let cmp = ExpressionNode::More(
            Box::new(ExpressionNode::Number(5)),
            Box::new(ExpressionNode::Number(3)),
        );
        let expr = ExpressionNode::Not(Box::new(cmp));
        assert_eq!(expr_to_str(&map, &owner, &expr), "!(5 > 3)");
    }

    #[test]
    fn test_expr_initializer() {
        let (map, owner) = make_map_and_owner("start Main { always { } }");
        let expr = ExpressionNode::Initializer(vec![
            ExpressionNode::Number(1),
            ExpressionNode::Number(2),
            ExpressionNode::Number(3),
        ]);
        assert_eq!(expr_to_str(&map, &owner, &expr), "{1, 2, 3}");
    }

    // ── Интеграционные тесты generate_source ──────────────────────────────────

    #[test]
    fn test_generate_source_has_include_and_math() {
        let src = r#"start Main { always { } }"#;
        let (model_ast, _) = parse(src, 0).unwrap();
        let model_rc = semantic::tree::construct_model(&model_ast, None, &[]).unwrap();
        model_rc.borrow_mut().name = Some("Main".to_string());
        let model = model_rc.borrow();
        let map = CMap::new(model.name(), &*model).unwrap();
        let source = generate_source(map.get_filename(), &map).unwrap();
        assert!(
            source.contains("#include \""),
            "отсутствует #include header:\n{source}"
        );
        assert!(
            source.contains(".h\""),
            "отсутствует .h в include:\n{source}"
        );
        assert!(
            source.contains("#include <math.h>"),
            "отсутствует #include <math.h>:\n{source}"
        );
    }

    #[test]
    fn test_generate_source_with_const_and_port() {
        let src = r#"
type u8 = [bit;8];
const LIMIT: u8 = 100;
port SENSOR: u8 = 0x100000;
start Main { always { } }
        "#;
        let (model_ast, _) = parse(src, 0).unwrap();
        let model_rc = semantic::tree::construct_model(&model_ast, None, &[]).unwrap();
        model_rc.borrow_mut().name = Some("Main".to_string());
        let model = model_rc.borrow();
        let map = CMap::new(model.name(), &*model).unwrap();
        let source = generate_source(map.get_filename(), &map).unwrap();
        assert!(
            source.contains("CONST_MAIN_LIMIT"),
            "CONST_MAIN_LIMIT отсутствует:\n{source}"
        );
        assert!(
            source.contains("PORT_MAIN_SENSOR"),
            "PORT_MAIN_SENSOR отсутствует:\n{source}"
        );
    }

    #[test]
    fn test_generate_source_functions() {
        let src = r#"
extern fn log_val(x: bit);
fn double_it(x: bit) -> bit { return x; }
start Main { always { } }
        "#;
        let (model_ast, _) = parse(src, 0).unwrap();
        let model_rc = semantic::tree::construct_model(&model_ast, None, &[]).unwrap();
        model_rc.borrow_mut().name = Some("Main".to_string());
        let model = model_rc.borrow();
        let map = CMap::new(model.name(), &*model).unwrap();
        let source = generate_source(map.get_filename(), &map).unwrap();
        assert!(
            source.contains("extern void log_val"),
            "extern fn отсутствует:\n{source}"
        );
        assert!(
            source.contains("static int Main_double_it"),
            "local fn отсутствует:\n{source}"
        );
    }

    #[test]
    fn test_generate_if_no_double_parens() {
        // Проверяет, что условие `if` генерируется без двойных скобок: `if (cond)` а не `if ((cond))`.
        // В BuT условие `if` пишется без скобок (как в Rust): `if cond { ... }`.
        // Генератор добавляет ровно одну пару скобок для C.
        let src = r#"
type u8 = [bit;8];
fn check(value: u8) -> bit {
    if value > 100 {
        return 1;
    }
    return 0;
}
start Main { always { } }
        "#;
        let (model_ast, _) = parse(src, 0).unwrap();
        let model_rc = semantic::tree::construct_model(&model_ast, None, &[]).unwrap();
        model_rc.borrow_mut().name = Some("Main".to_string());
        let model = model_rc.borrow();
        let map = CMap::new(model.name(), &*model).unwrap();
        let source = generate_source(map.get_filename(), &map).unwrap();
        // Условие if должно иметь ровно одну пару скобок
        assert!(
            source.contains("if (value > 100)"),
            "ожидается `if (value > 100)`, получено:\n{source}"
        );
        // return должен быть без лишних скобок вокруг значения
        assert!(
            !source.contains("return (1)") && !source.contains("return (0)"),
            "return не должен оборачивать значение в скобки:\n{source}"
        );
    }

    #[test]
    /// Проверяет, что переменная вложенной модели в функции генерируется как
    /// `main->state_name.field`, а не `main->model_name.field`.
    ///
    /// Пример: модель `Controller` инстанциируется состоянием `Entry = Controller`.
    /// Поле в C-структуре называется `entry` (по имени состояния), поэтому
    /// функция `clamp_temp` должна обращаться к переменной как `main->entry.temperature`,
    /// а не `main->controller.temperature`.
    fn test_submodel_variable_uses_state_field_name() {
        let src = r#"
type u8 = [bit;8];
model Controller {
    var temperature: u8 = 0;
    fn clamp(value: u8) -> u8 {
        if value < temperature { return temperature; }
        return value;
    }
    start Idle { }
}
start Entry = Controller;
        "#;
        let (model_ast, _) = parse(src, 0).unwrap();
        let model_rc = semantic::tree::construct_model(&model_ast, None, &[]).unwrap();
        model_rc.borrow_mut().name = Some("Root".to_string());
        let model = model_rc.borrow();
        let map = CMap::new(model.name(), &*model).unwrap();
        let source = generate_source(map.get_filename(), &map).unwrap();
        // Поле должно называться по имени состояния (`entry`), а не модели (`controller`)
        assert!(
            source.contains("model->entry.temperature"),
            "ожидается `model->entry.temperature`, получено:\n{source}"
        );
        assert!(
            !source.contains("model->controller.temperature"),
            "не должно быть `model->controller.temperature`:\n{source}"
        );
    }

    #[test]
    fn test_generate_loop_no_double_parens() {
        // Проверяет, что условие `loop` (→ `while` в C) генерируется без двойных скобок.
        // В BuT: `loop cond { ... }` — без скобок вокруг условия.
        // Генератор добавляет ровно одну пару скобок для C: `while (cond)`.
        let src = r#"
type u8 = [bit;8];
fn check(n: u8) -> bit {
    loop n > 0 {
        return 0;
    }
    return 1;
}
start Main { always { } }
        "#;
        let (model_ast, _) = parse(src, 0).unwrap();
        let model_rc = semantic::tree::construct_model(&model_ast, None, &[]).unwrap();
        model_rc.borrow_mut().name = Some("Main".to_string());
        let model = model_rc.borrow();
        let map = CMap::new(model.name(), &*model).unwrap();
        let source = generate_source(map.get_filename(), &map).unwrap();
        assert!(
            source.contains("while (n > 0)"),
            "ожидается `while (n > 0)`, получено:\n{source}"
        );
    }

    // ── Тесты расширенных состояний: Parallel / Concatenation ─────────────────

    /// Вспомогательная функция: генерирует полный `.c`-исходник из BuT-строки.
    fn generate_source_str(src: &str) -> String {
        let (ast, _) = parse(src, 0).expect("ошибка разбора");
        let model_rc = semantic::tree::construct_model(&ast, None, &[]).unwrap();
        model_rc.borrow_mut().name = Some("Root".to_string());
        let model = model_rc.borrow();
        let map = CMap::new(model.name(), &*model).unwrap();
        generate_source(map.get_filename(), &map).unwrap()
    }

    /// INIT-блок для `S = A | B` должен инициализировать оба элемента параллели
    /// и выставить `model->s.state = ROOT_S_INIT`.
    #[test]
    fn test_init_parallel_generates_init_calls() {
        let src = "model A { start Start; } model B { start Start; } start S = A | B { next End; } state End;";
        let code = generate_source_str(src);
        // Оба элемента инициализируются в INIT-блоке
        assert!(
            code.contains("RootA_init(&model->s.a0)"),
            "ожидается RootA_init в INIT:\n{code}"
        );
        assert!(
            code.contains("RootB_init(&model->s.b1)"),
            "ожидается RootB_init в INIT:\n{code}"
        );
        // Состояние параллели выставляется в INIT
        assert!(
            code.contains("model->s.state = ROOT_S_INIT;"),
            "ожидается ROOT_S_INIT:\n{code}"
        );
        // Переход в состояние S
        assert!(
            code.contains("model->state = ROOT_S;"),
            "ожидается model->state = ROOT_S:\n{code}"
        );
    }

    /// INIT-блок для `S = A + B` должен инициализировать только первый элемент
    /// и установить `model->s_state = ROOT_S_A0`.
    /// Второй элемент должен инициализироваться только в TICK при завершении первого.
    #[test]
    fn test_init_concatenation_generates_first_init_only() {
        let src = "model A { start Start; } model B { start Start; } start S = A + B { next End; } state End;";
        let code = generate_source_str(src);
        // Первый элемент инициализируется в INIT-блоке
        assert!(
            code.contains("RootA_init(&model->s_a0)"),
            "ожидается RootA_init в INIT:\n{code}"
        );
        // Указатель конкатенации выставляется на первый элемент
        assert!(
            code.contains("model->s_state = ROOT_S_A0;"),
            "ожидается ROOT_S_A0:\n{code}"
        );
        // Второй элемент инициализируется только в TICK при завершении A
        assert!(
            code.contains("RootB_init(&model->s_b1)"),
            "ожидается RootB_init в TICK (при завершении A):\n{code}"
        );
        // В INIT-блоке B идёт ПОСЛЕ A (тик A и его is_done)
        let a0_init_pos = code.find("RootA_init(&model->s_a0)").unwrap();
        let b1_init_pos = code.find("RootB_init(&model->s_b1)").unwrap();
        assert!(
            a0_init_pos < b1_init_pos,
            "RootA_init должен быть раньше RootB_init в коде:\n{code}"
        );
    }

    /// TICK-блок для `S = A | B` должен тикать все элементы и проверять is_done.
    #[test]
    fn test_tick_parallel_generates_tick_and_done_check() {
        let src = "model A { start Start; } model B { start Start; } start S = A | B { next End; } state End;";
        let code = generate_source_str(src);
        // Тик обоих элементов
        assert!(
            code.contains("RootA_tick(&model->s.a0, model)"),
            "ожидается RootA_tick:\n{code}"
        );
        assert!(
            code.contains("RootB_tick(&model->s.b1, model)"),
            "ожидается RootB_tick:\n{code}"
        );
        // Проверка is_done обоих
        assert!(
            code.contains("RootA_is_done(&model->s.a0, model)"),
            "ожидается RootA_is_done:\n{code}"
        );
        assert!(
            code.contains("RootB_is_done(&model->s.b1, model)"),
            "ожидается RootB_is_done:\n{code}"
        );
        // Оба условия объединены через &&
        assert!(
            code.contains("&&"),
            "ожидается && для объединения is_done:\n{code}"
        );
    }

    /// TICK-блок для `S = A + B` должен генерировать if/else if по полю s_state.
    #[test]
    fn test_tick_concatenation_generates_state_chain() {
        let src = "model A { start Start; } model B { start Start; } start S = A + B { next End; } state End;";
        let code = generate_source_str(src);
        // Проверка по первому элементу
        assert!(
            code.contains("model->s_state == ROOT_S_A0"),
            "ожидается ROOT_S_A0 в условии:\n{code}"
        );
        // Тик A
        assert!(
            code.contains("RootA_tick(&model->s_a0, model)"),
            "ожидается RootA_tick:\n{code}"
        );
        // При завершении A инициализируется B
        assert!(
            code.contains("RootB_init(&model->s_b1)"),
            "ожидается RootB_init при переходе:\n{code}"
        );
        // Проверка по второму элементу
        assert!(
            code.contains("model->s_state == ROOT_S_B1"),
            "ожидается ROOT_S_B1 в условии:\n{code}"
        );
        // Тик B
        assert!(
            code.contains("RootB_tick(&model->s_b1, model)"),
            "ожидается RootB_tick:\n{code}"
        );
    }

    /// TICK-блок для `S = A + (B | C)` должен правильно обрабатывать
    /// вложенный параллельный блок внутри конкатенации.
    #[test]
    fn test_tick_concatenation_nested_parallel() {
        let src = "model A { start Start; } model B { start Start; } model C { start Start; }
start S = A + (B | C) { next End; }
state End;";
        let code = generate_source_str(src);
        // Тик A как первый элемент конкатенации
        assert!(
            code.contains("model->s_state == ROOT_S_A0"),
            "ожидается ROOT_S_A0:\n{code}"
        );
        // Параллельный блок как второй элемент конкатенации
        assert!(
            code.contains("ROOT_S_PARALLEL1"),
            "ожидается ROOT_S_PARALLEL1:\n{code}"
        );
        // Тик B внутри вложенной параллели
        assert!(
            code.contains("RootB_tick(&model->s_parallel1.b0, model)"),
            "ожидается RootB_tick в параллели:\n{code}"
        );
        assert!(
            code.contains("RootC_tick(&model->s_parallel1.c1, model)"),
            "ожидается RootC_tick в параллели:\n{code}"
        );
    }

    /// Генерация extend_complex.but не должна возвращать ошибку.
    #[test]
    fn test_extend_complex_generates_without_error() {
        let src = std::fs::read_to_string("tests/data/parser/valid/extend_complex.but")
            .expect("не удалось прочитать extend_complex.but");
        let (ast, _) = parse(&src, 0).expect("ошибка разбора extend_complex.but");
        let model_rc =
            semantic::tree::construct_model(&ast, None, &[]).expect("ошибка построения модели");
        model_rc.borrow_mut().name = Some("extend_complex".to_string());
        let model = model_rc.borrow();
        let map = CMap::new("extend_complex", &*model).expect("ошибка создания CMap");
        let result = generate_source(map.get_filename(), &map);
        assert!(
            result.is_ok(),
            "ожидается успешная генерация: {:?}",
            result.err()
        );
        let code = result.unwrap();
        // INIT для параллели: оба элемента C1, C2 инициализируются
        assert!(
            code.contains("ExtendComplexCC1_init"),
            "ожидается ExtendComplexCC1_init:\n{code}"
        );
        assert!(
            code.contains("ExtendComplexCC2_init"),
            "ожидается ExtendComplexCC2_init:\n{code}"
        );
        // INIT для конкатенации: только первый элемент A инициализируется
        assert!(
            code.contains("ExtendComplexA_init"),
            "ожидается ExtendComplexA_init:\n{code}"
        );
    }
}
