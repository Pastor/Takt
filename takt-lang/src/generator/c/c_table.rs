//! Табличная форма автомата у цели `c` (фича 0435).
//!
//! # Зачем
//!
//! Умолчание цели — `switch` по состоянию, где переход **вкраплён в тело**:
//! условие ребра стоит внутри `case`, рядом с блоками `always`. Автоматный
//! подход учит обратному: автомат есть **отношение переходов**, то есть
//! данные, которые можно прочитать, напечатать и проверить отдельно от кода.
//! Флаг `--fsm=table` печатает ровно это — таблицу строк «откуда → страж →
//! действие → куда» и один диспетчер, который её просматривает.
//!
//! # Правило
//!
//! Порядок строк **совпадает** с порядком, в котором цель печатает переходы
//! формой `switch`: рёбра `ref` в порядке объявления, затем `next`, затем
//! подстановка `END` терминальному состоянию (правила 0213 и 0303). Диспетчер
//! берёт **первую** строку, у которой `from` равен текущему состоянию и страж
//! истинен, — поэтому поведение обеих форм тождественно. Сторож этому —
//! потактовая сверка (`conformance_fsm_table_tests`), а не факт компиляции:
//! таблица, переставившая две строки, собирается тем же `cc` без замечаний.
//!
//! ⚠️ **Тела состояний остаются в `switch`.** Таблицей выражается **переход**,
//! а не содержимое такта: блоки `always`/`every`, тик реализации состояния и
//! проверки формул печатает прежний печатник. Иначе завелось бы второе знание
//! о теле — класс 0084/0193/0195.
//!
//! ⚠️ **Композиция выражается таблицей вся** (фича 0438). Завершение
//! реализации — предикат: у реализации одной моделью это
//! `M_is_done(&model->x)`, у параллели — конъюнкция готовностей ветвей, у
//! цепочки — «цепочка на последнем шаге, и он завершён». Машина шагов при этом
//! остаётся в теле такта: она ведёт переходы **внутри** состояния, а таблица —
//! выход **наружу**. Отказ `CC-025`, которым фича 0435 называла границу,
//! выведен: входа, который таблицей не выражается, не осталось.

use std::cell::RefCell;
use std::rc::Rc;

use crate::diagnostics::{Diagnostic, Location};
use crate::generator::indent::Printer;
use crate::semantic::StateNode;
use crate::semantic::minimap::{Element, Name, StateExtend};

use super::c_blocks::generate_named_blocks;
use super::c_expr::generate_condition_expr;
use super::c_map::CMap;

/// Строка таблицы переходов до печати.
struct Row {
    /// Состояние-источник — константа перечисления.
    from: String,
    /// Предикат завершения реализации состояния (`M_is_done(&model->x)`).
    ///
    /// `None` у простого состояния: завершать в нём нечего.
    done: Option<String>,
    /// Условие ребра, напечатанное в C. `None` — безусловное ребро.
    cond: Option<String>,
    /// Состояние-источник: его блоки `exit` исполняются при переходе.
    exit_state: Rc<RefCell<StateNode>>,
    /// Состояние-приёмник: его блоки `enter` исполняются при переходе.
    /// `None` у перехода в `END` — у синтетического состояния блоков нет.
    enter_state: Option<Rc<RefCell<StateNode>>>,
    /// Состояние-приёмник — константа перечисления.
    to: String,
}

/// Печатает таблицу переходов модели, стражи, действия и диспетчер.
///
/// Зовётся **перед** `_init` (фича 0435): всё напечатанное здесь статично и
/// обязано стоять выше `_tick`, который зовёт диспетчер.
pub(super) fn emit_transition_table(
    printer: &mut Printer,
    model: &Element,
    map: &CMap,
    wants_root: bool,
) -> Result<(), Diagnostic> {
    let collected = rows(model, map)?;
    let struct_name = model.name().unique_camelcase();
    let table_name = format!("{}_TRANSITIONS", model.name().unique_uppercase_snakecase());
    // Указатель на корень печатается по той же нужде, что у `_tick` (фича
    // 0419): страж и действие — вынесенные наружу куски его тела.
    let params = signature(&struct_name, map, wants_root);
    let args = if wants_root { "model, main" } else { "model" };

    printer
        .print(&format!(
            "/// Таблица переходов модели {} (форма --fsm=table)",
            model.name()
        ))
        .nl();
    printer
        .print(&format!("typedef bool (*{struct_name}_Guard)({params});"))
        .nl();
    printer
        .print(&format!("typedef void (*{struct_name}_Action)({params});"))
        .nl();
    printer.print("typedef struct {").nl();
    printer.up();
    printer.ident("int from;").nl();
    printer.ident(&format!("{struct_name}_Guard guard;")).nl();
    printer.ident(&format!("{struct_name}_Action action;")).nl();
    printer.ident("int to;").nl();
    printer.down();
    printer
        .print(&format!("}} {struct_name}_Transition;"))
        .nl()
        .nl();

    let mut cells = Vec::new();
    for (index, row) in collected.iter().enumerate() {
        let guard = emit_guard(printer, row, &struct_name, &params, index)?;
        let action = emit_action(printer, row, map, model, &struct_name, &params, index)?;
        cells.push((row.from.clone(), guard, action, row.to.clone()));
    }

    if cells.is_empty() {
        // Переходов у модели нет: массив нулевой длины в C недопустим, поэтому
        // таблицы не печатается вовсе, а диспетчер пуст. Форма такта при этом
        // одна на все модели — вызов из `_tick` остаётся.
        printer
            .print(&format!("static void {struct_name}_dispatch({params}) {{"))
            .nl();
        printer.up().ident("(void)model;").nl();
        if wants_root {
            printer.ident("(void)main;").nl();
        }
        printer.down().print("}").nl().nl();
        return Ok(());
    }

    printer
        .print(&format!(
            "static const {struct_name}_Transition {table_name}[] = {{"
        ))
        .nl();
    printer.up();
    for (from, guard, action, to) in &cells {
        printer
            .ident(&format!(
                "{{ {from}, {}, {}, {to} }},",
                guard.as_deref().unwrap_or("0"),
                action.as_deref().unwrap_or("0")
            ))
            .nl();
    }
    printer.down();
    printer.print("};").nl().nl();

    printer
        .print("/// Диспетчер: первая строка с совпавшим состоянием и истинным стражем")
        .nl();
    printer
        .print(&format!("static void {struct_name}_dispatch({params}) {{"))
        .nl();
    printer.up();
    printer
        .ident(&format!(
            "const unsigned count = sizeof({table_name}) / sizeof({table_name}[0]);"
        ))
        .nl();
    printer.ident("unsigned index = 0;").nl();
    printer.ident("for (; index < count; index++) {").nl();
    printer.up();
    printer
        .ident(&format!(
            "const {struct_name}_Transition *row = &{table_name}[index];"
        ))
        .nl();
    printer
        .ident("if (row->from != (int)model->state) {")
        .nl()
        .up()
        .ident("continue;")
        .nl()
        .down()
        .ident("}")
        .nl();
    printer
        .ident(&format!("if (row->guard != 0 && !row->guard({args})) {{"))
        .nl()
        .up()
        .ident("continue;")
        .nl()
        .down()
        .ident("}")
        .nl();
    printer
        .ident("if (row->action != 0) {")
        .nl()
        .up()
        .ident(&format!("row->action({args});"))
        .nl()
        .down()
        .ident("}")
        .nl();
    printer.ident("model->state = row->to;").nl();
    printer.ident("return;").nl();
    printer.down();
    printer.ident("}").nl();
    printer.down();
    printer.print("}").nl().nl();
    Ok(())
}

/// Печатает вызов диспетчера в теле `_tick` (после `switch` по состоянию).
pub(super) fn emit_dispatch_call(printer: &mut Printer, model: &Element, wants_root: bool) {
    let args = if wants_root { "model, main" } else { "model" };
    printer
        .ident(&format!(
            "{}_dispatch({args});",
            model.name().unique_camelcase()
        ))
        .nl();
}

/// Нужен ли модели указатель на корень в `_tick` (фича 0419).
///
/// Носитель ОДИН на две точки: сигнатуру стража с действием и вызов
/// диспетчера. Разъедься они — вывод не собрался бы («too few arguments»).
pub(super) fn wants_root(model: &Element, map: &CMap) -> bool {
    if model.name().eq(&map.root_name()) {
        return false;
    }
    map.raw_model_at(model.name().clone()).is_ok_and(|rc| {
        super::c_needs::model_fn_needs_root(
            &rc,
            super::c_needs::ModelFn::Tick,
            super::c_time::is_clock_profile(map),
        )
    })
}

/// Сигнатура стража, действия и диспетчера модели.
fn signature(struct_name: &str, map: &CMap, wants_root: bool) -> String {
    if wants_root {
        format!(
            "{struct_name} *model, {} *main",
            map.root_name().unique_camelcase()
        )
    } else {
        format!("{struct_name} *model")
    }
}

/// Собирает строки таблицы переходов модели.
///
/// Порядок — тот же, что у печати `switch` (`c_model::generate_model_tick`):
/// состояния в порядке `states`, внутри состояния — `ref` в порядке
/// объявления, затем `next`/`END`.
fn rows(model: &Element, map: &CMap) -> Result<Vec<Row>, Diagnostic> {
    let Element::Model {
        states,
        name: model_name,
        ..
    } = model
    else {
        return Err(Diagnostic::error(
            Location::Codegen,
            "Элемент не является моделью".to_string(),
        )
        .with_code("CC-006"));
    };
    let is_main = model.name().eq(&map.root_name());
    let mut collected = Vec::new();
    for state_name in states.iter() {
        let raw_rc = map.raw_state_at(state_name.clone())?;
        let Some(element) = map.state_at(state_name.clone()) else {
            continue; // недостижимое состояние — как и в форме `switch`
        };
        let from = state_name.unique_uppercase_snakecase();
        // Предикат завершения есть только у состояния с реализацией.
        let (done, next) = match &element {
            Element::State { .. } => (None, None),
            Element::StateExtend { extend, next, .. } => {
                match done_predicate(extend, state_name, map, is_main)? {
                    Done::Predicate(text) => (Some(text), Some(next.clone())),
                    // Реализация, у которой завершения нет вовсе: в форме
                    // `switch` цель тоже не печатает перехода — состояние
                    // остаётся в себе. Строк у него нет, и это не отказ.
                    Done::NoTransition => continue,
                }
            }
            Element::Model { .. } => {
                unreachable!("CMap::state_at отдаёт только State/StateExtend (фильтр is_state)")
            }
        };
        let closed =
            push_reference_rows(&mut collected, &raw_rc, &from, &done, states, map, model)?;
        if closed {
            // Всё, что за безусловным ребром, недостижимо (фича 0213).
            continue;
        }
        let has_references = !raw_rc.borrow().references().is_empty();
        match next {
            Some(next) if !next.local().is_empty() => {
                let target = map.raw_state_at(next.clone())?;
                collected.push(Row {
                    from: from.clone(),
                    done: done.clone(),
                    cond: None,
                    exit_state: Rc::clone(&raw_rc),
                    enter_state: Some(target),
                    to: next.unique_uppercase_snakecase(),
                });
            }
            // `END` подставляется только состоянию БЕЗ переходов (правило
            // 0303): состояние с рёбрами, ни одно из которых не сработало,
            // остаётся на месте.
            Some(_) if has_references => {}
            Some(_) => collected.push(end_row(&from, &done, &raw_rc, model_name)),
            None => {
                let terminated = raw_rc.borrow().is_terminated();
                if terminated && !state_name.local().to_uppercase().eq("END") {
                    collected.push(end_row(&from, &done, &raw_rc, model_name));
                }
            }
        }
    }
    Ok(collected)
}

/// Строка «в терминальное состояние модели»: `exit` источника, затем `END`.
fn end_row(
    from: &str,
    done: &Option<String>,
    raw_state: &Rc<RefCell<StateNode>>,
    model_name: &Name,
) -> Row {
    Row {
        from: from.to_string(),
        done: done.clone(),
        cond: None,
        exit_state: Rc::clone(raw_state),
        enter_state: None,
        to: format!("{}_END", model_name.unique_uppercase_snakecase()),
    }
}

/// Строки по рёбрам `ref` состояния.
///
/// Возвращает `true`, если цепочка закрыта безусловным ребром простого
/// состояния: дальше строк у него не будет (правило 0213).
fn push_reference_rows(
    out: &mut Vec<Row>,
    raw_state: &Rc<RefCell<StateNode>>,
    from: &str,
    done: &Option<String>,
    states: &[Name],
    map: &CMap,
    model: &Element,
) -> Result<bool, Diagnostic> {
    let references = raw_state.borrow().references().to_vec();
    for reference in references {
        let Some(target) = states.iter().find(|n| n.local() == reference.name).cloned() else {
            continue; // целевое состояние недостижимо — как и в форме `switch`
        };
        let target_raw = map.raw_state_at(target.clone())?;
        let unconditional = reference.cond.is_unconditional();
        let cond = if unconditional {
            None
        } else {
            // Отказ печатника доезжает до автора обёрткой `CC-018` — тем же
            // способом, что в форме `switch` (фича 0236): позиция ребра,
            // причина заметкой.
            match generate_condition_expr(&reference.cond, map, model) {
                Ok(text) => Some(text),
                Err(di) => {
                    return Err(Diagnostic::error_with_note(
                        reference.location,
                        format!(
                            "условный переход в состояние '{}' не переводится в C: {}",
                            target.local(),
                            di.message
                        ),
                        di.loc,
                        match &di.code {
                            Some(code) => format!("причина [{}]: {}", code, di.message),
                            None => format!("причина: {}", di.message),
                        },
                    )
                    .with_code("CC-018"));
                }
            }
        };
        out.push(Row {
            from: from.to_string(),
            done: done.clone(),
            cond,
            exit_state: Rc::clone(raw_state),
            enter_state: Some(target_raw),
            to: target.unique_uppercase_snakecase(),
        });
        // ⚠️ У состояния С РЕАЛИЗАЦИЕЙ безусловное ребро цепочку не закрывает:
        // его строка всё равно сторожится предикатом завершения, и следующая
        // строка (`next`/`END`) остаётся достижимой ровно так же, как в форме
        // `switch` — там `generate_state_transitions` печатает их внутри
        // `if (is_done)`.
        if unconditional && done.is_none() {
            return Ok(true);
        }
    }
    Ok(false)
}

/// Ответ о завершении реализации состояния.
enum Done {
    /// Предикат завершения — выражение C для стража строки.
    Predicate(String),
    /// Перехода у состояния нет вовсе: строк не будет, и это не отказ —
    /// форма `switch` на таком входе тоже не печатает перехода.
    NoTransition,
}

/// Предикат «реализация состояния завершена» для строки таблицы.
///
/// Реализация одной моделью даёт `M_is_done(&model->x)`; параллель — конъюнкцию
/// готовностей своих ветвей, цепочка — «цепочка на последнем шаге, и он
/// завершён» (фича 0438). Все три берутся **у тех же носителей**, что печатают
/// тик: `c_compose::generate_parallel_items_tick` и `c_chain` — своё знание о
/// раскладке полей разошлось бы с ними при первой же правке (класс
/// 0084/0193/0195).
fn done_predicate(
    extend: &StateExtend,
    state_name: &Name,
    map: &CMap,
    is_main: bool,
) -> Result<Done, Diagnostic> {
    match extend {
        StateExtend::Model(name, _) => Ok(Done::Predicate(format!(
            "{}_is_done(&model->{})",
            name.unique_camelcase(),
            state_name.local_lowercase_snakecase()
        ))),
        StateExtend::Parallel(steps) => {
            // Текст тиков здесь не нужен — нужен только список готовностей,
            // который носитель возвращает попутно. Печать уходит в буфер и
            // выбрасывается: тела состояний печатает `c_model`, и второй их
            // копии в выводе быть не должно.
            let mut sink = String::new();
            let mut scratch = Printer::new(4, &mut sink);
            let access = format!("model->{}", state_name.local_lowercase_snakecase());
            let upper = state_name.unique_uppercase_snakecase();
            let done = super::c_compose::generate_parallel_items_tick(
                &mut scratch,
                map,
                &access,
                &upper,
                steps,
                is_main,
            );
            if done.is_empty() {
                return Ok(Done::NoTransition);
            }
            Ok(Done::Predicate(done.join(" && ")))
        }
        // Цепочка `A + B` (фича 0438): машина шагов остаётся в теле такта —
        // она ведёт переходы ВНУТРИ состояния, — а наружу состояние уходит при
        // одном условии: «цепочка на последнем шаге, и он завершён». Это же
        // условие печатает форма `switch`, и берётся оно у общего носителя
        // имён `c_chain`.
        StateExtend::Concatenation(items) => Ok(super::c_chain::chain_done(
            map,
            (
                &state_name.local_lowercase_snakecase(),
                &state_name.unique_uppercase_snakecase(),
            ),
            items,
            is_main,
        )
        .map_or(Done::NoTransition, Done::Predicate)),
        StateExtend::None => Ok(Done::NoTransition),
    }
}

/// Печатает функцию-страж строки; `None` — строка безусловна.
fn emit_guard(
    printer: &mut Printer,
    row: &Row,
    struct_name: &str,
    params: &str,
    index: usize,
) -> Result<Option<String>, Diagnostic> {
    let text = match (&row.done, &row.cond) {
        (None, None) => return Ok(None),
        (Some(done), None) => done.clone(),
        (None, Some(cond)) => cond.clone(),
        (Some(done), Some(cond)) => format!("{done} && ({cond})"),
    };
    let name = format!("{struct_name}_guard_{index}");
    printer
        .print(&format!("static bool {name}({params}) {{"))
        .nl();
    printer.up();
    emit_unused(printer, &text, params);
    printer.ident(&format!("return {text};")).nl();
    printer.down();
    printer.print("}").nl().nl();
    Ok(Some(name))
}

/// Печатает функцию-действие строки (`exit` источника + `enter` приёмника);
/// `None` — блоков нет, и печатать нечего.
fn emit_action(
    printer: &mut Printer,
    row: &Row,
    map: &CMap,
    model: &Element,
    struct_name: &str,
    params: &str,
    index: usize,
) -> Result<Option<String>, Diagnostic> {
    // Тело печатается в буфер: по нему решается, нужна ли функция вообще и
    // нужна ли заглушка неиспользуемого параметра (приём фичи 0260).
    let mut body = String::new();
    {
        let mut buffered = printer.fork(&mut body);
        buffered.up();
        generate_named_blocks(&mut buffered, &row.exit_state.borrow(), map, model, "exit")?;
        if let Some(enter) = &row.enter_state {
            generate_named_blocks(&mut buffered, &enter.borrow(), map, model, "enter")?;
        }
        buffered.down();
    }
    if body.trim().is_empty() {
        return Ok(None);
    }
    let name = format!("{struct_name}_action_{index}");
    printer
        .print(&format!("static void {name}({params}) {{"))
        .nl();
    printer.up();
    emit_unused(printer, &body, params);
    printer.down();
    printer.print(&body);
    printer.print("}").nl().nl();
    Ok(Some(name))
}

/// Печатает заглушки неиспользуемых параметров.
///
/// Признак — упоминание имени в **напечатанном** тексте тела: тот же приём,
/// что у заглушки параметра функции (фича 0337). Ошибка в безопасную сторону:
/// лишняя заглушка законна, пропущенная даёт предупреждение `cc`.
fn emit_unused(printer: &mut Printer, body: &str, params: &str) {
    if super::c_params::is_unused(body, "model") {
        printer.ident(&super::c_params::unused_guard("model")).nl();
    }
    if params.contains("*main") && super::c_params::is_unused(body, "main") {
        printer.ident(&super::c_params::unused_guard("main")).nl();
    }
}
