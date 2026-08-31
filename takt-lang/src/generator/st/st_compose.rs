//! Композиция моделей в цели Structured Text: `M1 | M2` и `M1 + M2`.
//!
//! Вынесено из `st_model.rs` (фича 0191): тот модуль сам объявляет два
//! предмета — «часть 1: простые модели» и «часть 2: композиция», — и по этой
//! границе разделён, когда упёрся в предел размера. Граница модулей = граница
//! ответственности, а не «отрезали столько, сколько мешало».
//!
//! ## Что здесь
//!
//! - экземпляры под-`FUNCTION_BLOCK` и их инициализаторы из аргументов
//!   инстанцирования (фича 0185);
//! - **параллельная** композиция (`M1 | M2`): вызовы под-FB подряд, завершение —
//!   конъюнкция их `is_done`;
//! - **последовательная** (`M1 + M2`): собственный счётчик шагов и вложенный
//!   `CASE` — форма снята зондом цели `c`, где у конкатенации свой `enum` шагов.

use crate::diagnostics::Diagnostic;
use crate::generator::indent::Printer;
use crate::generator::st::st_map::StMap;
use crate::generator::st::st_model::{BodyOutput, StateTable};
use crate::semantic::minimap::{Name, StateExtend};
use crate::semantic::type_node::TypeNode;
use crate::semantic::{ModelNode, StateNode};

/// Экземпляр под-`FUNCTION_BLOCK` внутри родительского FB.
///
/// Аналог поля `StackerCommandReceiver command_receiver0;` в структуре цели `c`
/// (Ф6). Числовой суффикс обязателен: одна и та же модель может входить в
/// композицию **несколько раз** (`elevator.takt:198` включает `Engine` пять раз).
#[derive(Debug)]
pub(crate) struct Instance {
    /// Имя переменной-экземпляра.
    pub name: String,
    /// Имя типа — `FUNCTION_BLOCK` под-модели.
    pub fb_type: String,
    /// Готовый инициализатор экземпляра из аргументов инстанцирования
    /// (фича 0185, режим `assign`) — `(step := 5)` либо `None`.
    ///
    /// В ST настройка задаётся **инициализатором экземпляра** —
    /// `tuner0 : Tuner := (step := 5);`. Это ближе всего к цели `c`, где
    /// присваивание идёт один раз в `_init`: присваивать в теле означало бы
    /// перетирать значение каждый скан, ломая параметр, который модель меняет
    /// сама. Печатается в [`emit_group`] — единственном месте, где рядом и имя
    /// модели, и её аргументы.
    pub init: Option<String>,
}

/// Инициализатор экземпляра FB из аргументов инстанцирования (фича 0185).
///
/// Форма `(step := 5, dwell := 2500)` — инициализатор структуры IEC; проба
/// MatIEC подтвердила, что `iec2c` её принимает. Значение печатается по **типу
/// параметра** целевой модели (`literal_init`, урок 0066: литерал в ST
/// типозависим — `bit` требует `TRUE`/`FALSE`, `duration` — миллисекунды).
fn instance_initializer(
    map: &StMap,
    model_name: &Name,
    args: &[crate::semantic::extend::ParameterArgument],
) -> Result<Option<String>, Diagnostic> {
    if args.is_empty() {
        return Ok(None);
    }
    let target = map.raw_model_at(model_name.clone())?;
    let target = target.borrow();
    let mut parts = Vec::with_capacity(args.len());
    for arg in args {
        let ty = target
            .variables
            .get(&arg.name)
            .map(|v| v.ty().clone())
            .ok_or_else(|| {
                Diagnostic::error(
                    arg.loc,
                    format!(
                        "Параметр '{}' модели '{}' не найден при печати инициализатора",
                        arg.name,
                        model_name.local()
                    ),
                )
                .with_code("ST-017")
            })?;
        // ⚠️ Агрегат МАССИВА здесь непечатаем (фича 0343): в объявлении
        // переменной форма `:= [9, 8, 7, 6]` законна и `iec2c` её принимает, а
        // в инициализаторе ЭКЗЕМПЛЯРА FB тот же массив даёт «Initialization
        // element identifier … is set to value of incompatible datatype».
        // Отказ `ST-017` честнее невалидного файла.
        if matches!(ty, crate::semantic::type_node::TypeNode::Array(_, _)) {
            return Err(Diagnostic::error(
                arg.loc,
                format!(
                    "Значение параметра '{}' — агрегат массива: инициализатор \
                     экземпляра FUNCTION_BLOCK такой формы в IEC 61131-3 не \
                     принимает (проверено iec2c). Передайте значения \
                     присваиванием в теле либо объявите массив у владельца",
                    arg.name
                ),
            )
            .with_code("ST-017"));
        }
        let value = crate::generator::st::st_decl::literal_init(&arg.value, &ty, None).ok_or_else(
            || {
                Diagnostic::error(
                    arg.loc,
                    format!(
                        "Значение параметра '{}' не печатается инициализатором ST",
                        arg.name
                    ),
                )
                .with_code("ST-017")
            },
        )?;
        parts.push(format!("{} := {}", arg.name, value));
    }
    Ok(Some(format!("({})", parts.join(", "))))
}

/// Печатает ветвь состояния-композиции: вызовы под-FB и завершение по `is_done`.
///
/// Форма изоморфна цели `c` (Ф6, `stacker.c:414-439`): под-модели вызываются
/// **последовательно в одном такте** родителя, в порядке объявления, а
/// композиция завершается по **конъюнкции** их `is_done`. Настоящей
/// конкурентности нет — чередование детерминировано, что и нужно скан-циклу ПЛК.
#[allow(clippy::too_many_arguments)]
pub(crate) fn emit_composition(
    p: &mut Printer,
    map: &StMap,
    state_name: &Name,
    extend: &StateExtend,
    next: &Name,
    table: &StateTable,
    out: &mut BodyOutput,
    state: &StateNode,
    model: &ModelNode,
) -> Result<(), Diagnostic> {
    // Последовательная композиция идёт своим путём: ей нужен счётчик шагов.
    if let StateExtend::Concatenation(steps) = extend {
        return emit_concatenation(p, map, steps, state_name, next, table, out, state, model);
    }

    // Переменные корня под-FB видит через `VAR_IN_OUT`: в ST указателей нет, а
    // `main->lift_request` цели `c` выразить нечем (О1-в, проба П7).
    let ctx = ChainCtx {
        state_name,
        prefix: "",
    };
    let done = emit_branch(p, map, extend, &mut Vec::new(), &ctx, out)?;

    // Табличная форма (фича 0440): выход из состояния печатает таблица, а здесь
    // остаётся только защёлка готовности — она берётся ПОСЛЕ тика ветвей, ровно
    // там, где форма `CASE` печатает `IF <готовность> THEN`.
    if map.fsm_table() {
        crate::generator::st::st_table::emit_ready_latch(p, state_name, &done, out);
        return Ok(());
    }

    p.ident(&format!("IF {} THEN", done)).nl();
    p.up();
    emit_state_exit(p, map, state_name, state, model, next, table, out)?;
    p.down();
    p.ident("END_IF;").nl();
    Ok(())
}

/// Печатает выход из состояния-композиции: собственные рёбра, затем `next`/`END`.
///
/// Порядок — эталона (фича 0181): `ref` в порядке объявления, `next` последним.
/// Прежде рёбра терялись, и вход `start Entry = A | B { ref Finish: cond; }`
/// давал другой автомат (фича 0303).
///
/// ⚠️ `END` подставляется только состоянию **без** рёбер: у эталона узел
/// завершается при пустом списке переходов, а при несработавших остаётся в
/// состоянии.
#[allow(clippy::too_many_arguments)]
fn emit_state_exit(
    p: &mut Printer,
    map: &StMap,
    state_name: &Name,
    state: &StateNode,
    model: &ModelNode,
    next: &Name,
    table: &StateTable,
    out: &mut BodyOutput,
) -> Result<(), Diagnostic> {
    let references = match state {
        StateNode::Simple { references, .. } | StateNode::Implement { references, .. } => {
            references.clone()
        }
        StateNode::Unresolved => Vec::new(),
    };
    if crate::generator::st::st_edges::emit_edges(
        p,
        map,
        (state_name, state),
        model,
        table,
        out,
        &references,
    )? {
        return Ok(());
    }
    if next.unique().is_empty() && !references.is_empty() {
        return Ok(());
    }
    let target = if next.unique().is_empty() {
        table.end
    } else {
        table.number_of_local(next.local()).unwrap_or(table.end)
    };
    // Блок `exit` состояния-композиции (фича 0430). Прежде он терялся: у
    // простого терминального состояния цель его печатает (`st_model`), а у
    // композиции — нет, и один вход давал `hits = 0` против `1` у целей `c` и
    // `rust`. Порядок тот же, что у обычного перехода: `exit` источника, затем
    // смена состояния.
    crate::generator::st::st_model::emit_block(p, state, "exit", model, &mut out.stmt)?;
    p.ident(&format!("state := {}; (* {} *)", target, next.local()))
        .nl();
    Ok(())
}

/// Печатает ВЕТВЬ композиции и возвращает выражение её завершения.
///
/// Разбор рекурсивен, потому что рекурсивно само дерево композиции: ветвь —
/// модель, параллель ветвей либо **цепочка** `+`. Прежде разбор был плоским
/// (`collect_models` сводил дерево к списку моделей), и цепочка внутри
/// параллели попадала в него отказом `ST-011`: конструкцию, которую исполняют
/// эталон, `c` и `rust`, цель не переводила вовсе (фича 0427).
///
/// `path` — место ветви в дереве (носитель [`chain_site`]): по нему строится
/// имя счётчика вложенной цепочки, иначе две цепочки одного состояния делили
/// бы один счётчик.
/// Печатает ВЕТВЬ композиции и возвращает выражение её завершения.
///
/// Разбор рекурсивен, потому что рекурсивно само дерево композиции: ветвь —
/// модель, параллель ветвей либо **цепочка** `+`. Прежде разбор был плоским
/// (`collect_models` сводил дерево к списку моделей), и цепочка внутри
/// параллели попадала в него отказом `ST-011`: конструкцию, которую исполняют
/// эталон, `c` и `rust`, цель не переводила вовсе (фича 0427).
///
/// `path` — место ветви в дереве (носитель
/// [`chain_site`](crate::generator::chain_site)): по нему строится имя счётчика
/// вложенной цепочки, иначе две цепочки одного состояния делили бы один
/// счётчик.
///
/// `prefix` — приставка имени экземпляра под-FB. Внутри цепочки она
/// `<состояние>_` (форма снята зондом цели `c`), снаружи пуста: менять её у
/// параллели значило бы переписать вывод всего корпуса ради вложенного случая.
fn emit_branch(
    p: &mut Printer,
    map: &StMap,
    extend: &StateExtend,
    path: &mut Vec<usize>,
    ctx: &ChainCtx<'_>,
    out: &mut BodyOutput,
) -> Result<String, Diagnostic> {
    match extend {
        StateExtend::None => Ok(String::new()),
        StateExtend::Model(name, args) => emit_instance(p, map, name, args, ctx.prefix, out),
        StateExtend::Parallel(items) => {
            let mut done_terms = Vec::new();
            for (i, item) in items.iter().enumerate() {
                path.push(i);
                let done = emit_branch(p, map, item, path, ctx, out)?;
                path.pop();
                if !done.is_empty() {
                    done_terms.push(done);
                }
            }
            Ok(done_terms.join(" AND "))
        }
        // ВЛОЖЕННАЯ цепочка: своя машина шагов и своё завершение.
        //
        // ⚠️ Завершение — **терминальное значение счётчика**, а не «все шаги
        // готовы»: шаг, до которого очередь не дошла, свой `is_done` не
        // выставлял ни разу (урок 0426).
        StateExtend::Concatenation(items) => {
            let counter = emit_chain_case(p, map, items, path, ctx, out, &ChainExit::Done)?;
            Ok(format!("{} = {}", counter, items.len()))
        }
    }
}

/// Контекст печати ветвей одного состояния: имя состояния и приставка имён.
struct ChainCtx<'a> {
    state_name: &'a Name,
    prefix: &'a str,
}

/// Что делать по завершении последнего шага цепочки.
enum ChainExit<'a> {
    /// Цепочка ВЕРХНЕГО уровня: пройдя последний шаг, состояние уходит дальше.
    Parent {
        state: &'a StateNode,
        model: &'a ModelNode,
        next: &'a Name,
        table: &'a StateTable,
    },
    /// ВЛОЖЕННАЯ цепочка (фича 0427): выхода из состояния у неё нет — её
    /// завершение читает вмещающая композиция по терминальному значению
    /// счётчика.
    Done,
}

/// Печатает цепочку `+` как `CASE` по собственному счётчику; возвращает его имя.
///
/// `exit` — что делать на терминальном значении счётчика: у цепочки **верхнего
/// уровня** это выход из состояния, у **вложенной** — ничего.
fn emit_chain_case(
    p: &mut Printer,
    map: &StMap,
    items: &[StateExtend],
    path: &mut Vec<usize>,
    ctx: &ChainCtx<'_>,
    out: &mut BodyOutput,
    exit: &ChainExit<'_>,
) -> Result<String, Diagnostic> {
    let counter = format!(
        "{}_step{}",
        ctx.state_name.local_lowercase_snakecase(),
        crate::generator::chain_site::suffix(path)
    );
    out.stmt
        .hoisted
        .push(crate::generator::st::st_stmt::Hoisted {
            name: counter.clone(),
            ty: TypeNode::Integer {
                bits: 8,
                signed: false,
            },
        });
    // Внутри цепочки имена экземпляров несут приставку состояния — форма снята
    // зондом цели `c` (`extend_complex.h`).
    let inner_prefix = format!("{}_", ctx.state_name.local_lowercase_snakecase());
    let inner = ChainCtx {
        state_name: ctx.state_name,
        prefix: &inner_prefix,
    };
    // Табличная форма (фича 0440): выход печатает таблица, а здесь готовность
    // сбрасывается перед машиной шагов и взводится ровно там, где форма `CASE`
    // исполняет выход, — в ветви последнего шага (фича 0443).
    let parent_chain = matches!(exit, ChainExit::Parent { .. });
    if map.fsm_table() && parent_chain {
        crate::generator::st::st_table::emit_ready_latch(p, ctx.state_name, "FALSE", out);
    }
    p.ident(&format!("CASE {} OF", counter)).nl();
    p.up();
    for (i, item) in items.iter().enumerate() {
        p.ident(&format!("{}:", i)).nl();
        p.up();
        path.push(i);
        let done = emit_branch(p, map, item, path, &inner, out)?;
        path.pop();
        p.ident(&format!("IF {} THEN", done)).nl();
        p.up();
        // ⚠️ ПОСЛЕДНИЙ шаг цепочки ВЕРХНЕГО уровня уводит состояние **в этом
        // же скане** (фича 0443). Прежде он лишь ставил счётчик на терминальное
        // значение, а выход исполняла отдельная ветвь `CASE` — то есть скан
        // спустя: цель `st` тратила лишний скан там, где эталон и цель `c`
        // уходят сразу. `iec2c` вывод принимал, а расхождение видела только
        // потактовая сверка (класс 0191, но в композиции).
        //
        // ⚠️ У ВЛОЖЕННОЙ цепочки поведение прежнее: её готовность вмещающая
        // композиция читает по терминальному значению счётчика (`counter = N`),
        // и читает его в том же скане — лишнего скана там не было.
        match (i + 1 == items.len(), exit) {
            (true, ChainExit::Parent { .. }) if map.fsm_table() => {
                crate::generator::st::st_table::emit_ready_latch(p, ctx.state_name, "TRUE", out);
            }
            (
                true,
                ChainExit::Parent {
                    state,
                    model,
                    next,
                    table,
                },
            ) => {
                emit_state_exit(p, map, ctx.state_name, state, model, next, table, out)?;
            }
            _ => {
                p.ident(&format!("{} := {};", counter, i + 1)).nl();
            }
        }
        p.down();
        p.ident("END_IF;").nl();
        p.down();
    }
    p.down();
    p.ident("END_CASE;").nl();
    Ok(counter)
}

/// Печатает экземпляр под-FB, его вызов и возвращает выражение `is_done`.
fn emit_instance(
    p: &mut Printer,
    map: &StMap,
    model_name: &Name,
    model_args: &[crate::semantic::extend::ParameterArgument],
    prefix: &str,
    out: &mut BodyOutput,
) -> Result<String, Diagnostic> {
    // Числовой суффикс — по образцу цели `c` (`start_a0`, `start_b1`): одна и
    // та же модель может входить в композицию несколько раз.
    let index = out.instances.len();
    let inst = format!(
        "{}{}{}",
        prefix,
        model_name.local_lowercase_snakecase(),
        index
    );
    out.instances.push(Instance {
        name: inst.clone(),
        fb_type: model_name.unique_camelcase(),
        init: instance_initializer(map, model_name, model_args)?,
    });
    let args: Vec<String> = map
        .shared_variables(model_name)
        .into_iter()
        .map(|(n, _)| format!("{} := {}", n, n))
        .collect();
    p.ident(&format!("{}({});", inst, args.join(", "))).nl();
    Ok(format!("{}.is_done", inst))
}

/// Печатает последовательную композицию (`M1 + M2`) как вложенный `CASE` по
/// собственному счётчику шагов.
///
/// Форма — из зонда цели `c` (`extend_complex.h`): у конкатенации там **свой**
/// `enum` шагов (`…_START_A0`, `…_START_B1`, `…_START_PARALLEL2`, `…_START_E3`),
/// отдельный от состояния модели. В ST это переменная-счётчик и вложенный `CASE`
/// (форма проверена пробой ✅).
///
/// Шаг завершается по `is_done` своей группы; параллельная группа внутри
/// конкатенации (`A + (C | D) + E`) — по конъюнкции, как обычная параллель, а
/// **цепочка** внутри шага — по своему счётчику (фича 0427).
#[allow(clippy::too_many_arguments)]
fn emit_concatenation(
    p: &mut Printer,
    map: &StMap,
    steps: &[StateExtend],
    state_name: &Name,
    next: &Name,
    table: &StateTable,
    out: &mut BodyOutput,
    state: &StateNode,
    model: &ModelNode,
) -> Result<(), Diagnostic> {
    let ctx = ChainCtx {
        state_name,
        prefix: "",
    };
    let exit = ChainExit::Parent {
        state,
        model,
        next,
        table,
    };
    emit_chain_case(p, map, steps, &mut Vec::new(), &ctx, out, &exit)?;
    Ok(())
}
