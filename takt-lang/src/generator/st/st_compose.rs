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

use crate::diagnostics::{Diagnostic, Location};
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
/// Модель композиции вместе с аргументами её инстанцирования (фича 0185).
pub(crate) type ModelRef = (Name, Vec<crate::semantic::extend::ParameterArgument>);

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

    let mut group = Vec::new();
    collect_models(extend, &mut group)?;
    // Переменные корня под-FB видит через `VAR_IN_OUT`: в ST указателей нет, а
    // `main->lift_request` цели `c` выразить нечем (О1-в, проба П7).
    let done_terms = [emit_group(p, map, &group, "", out)?];

    p.ident(&format!("IF {} THEN", done_terms.join(" AND ")))
        .nl();
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
        state_name,
        state,
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
    p.ident(&format!("state := {}; (* {} *)", target, next.local()))
        .nl();
    Ok(())
}

/// Печатает шаг: вызовы моделей группы и условие её завершения.
///
/// Возвращает выражение «группа завершена» (конъюнкция `is_done`).
fn emit_group(
    p: &mut Printer,
    map: &StMap,
    group: &[ModelRef],
    prefix: &str,
    out: &mut BodyOutput,
) -> Result<String, Diagnostic> {
    let mut done_terms = Vec::new();
    for (model_name, model_args) in group {
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
        done_terms.push(format!("{}.is_done", inst));
    }
    Ok(done_terms.join(" AND "))
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
/// конкатенации (`A + (C | D) + E`) — по конъюнкции, как обычная параллель.
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
    let counter = format!("{}_step", state_name.local_lowercase_snakecase());
    out.stmt
        .hoisted
        .push(crate::generator::st::st_stmt::Hoisted {
            name: counter.clone(),
            ty: TypeNode::Integer {
                bits: 8,
                signed: false,
            },
        });
    let prefix = format!("{}_", state_name.local_lowercase_snakecase());

    p.ident(&format!("CASE {} OF", counter)).nl();
    p.up();
    for (i, step) in steps.iter().enumerate() {
        let mut group = Vec::new();
        collect_models(step, &mut group)?;
        p.ident(&format!("{}:", i)).nl();
        p.up();
        let done = emit_group(p, map, &group, &prefix, out)?;
        p.ident(&format!("IF {} THEN", done)).nl();
        p.up();
        p.ident(&format!("{} := {};", counter, i + 1)).nl();
        p.down();
        p.ident("END_IF;").nl();
        p.down();
    }
    // Последний шаг: конкатенация пройдена — рёбра состояния, затем `next`.
    p.ident(&format!("{}: (* конкатенация завершена *)", steps.len()))
        .nl();
    p.up();
    emit_state_exit(p, map, state_name, state, model, next, table, out)?;
    p.down();
    p.down();
    p.ident("END_CASE;").nl();
    Ok(())
}

/// Собирает модели композиции в порядке объявления.
///
/// # Ошибки
/// `ST-011` — `Concatenation` (`M1 + M2`): последовательная композиция требует
/// собственного счётчика шагов (в цели `c` — вложенный `enum state`) и
/// реализуется частью 3.
fn collect_models(extend: &StateExtend, out: &mut Vec<ModelRef>) -> Result<(), Diagnostic> {
    match extend {
        StateExtend::None => Ok(()),
        // Вместе с именем несём аргументы инстанцирования (фича 0185): они
        // свойство места, а не модели, и потерять их здесь значило бы молча
        // выбросить настройку экземпляра.
        StateExtend::Model(name, args) => {
            out.push((name.clone(), args.clone()));
            Ok(())
        }
        StateExtend::Parallel(steps) => {
            for step in steps {
                collect_models(step, out)?;
            }
            Ok(())
        }
        // Конкатенацию печатает `emit_concatenation` — у неё свой счётчик шагов.
        // Сюда она попадает только вложенной в параллель (`(A + B) | C`), а такой
        // вложенности нужен ещё один уровень счётчика: пока — громкий отказ, а не
        // печать шагов как параллельных (это молча изменило бы семантику).
        StateExtend::Concatenation(_) => Err(Diagnostic::error(
            crate::generator::site::at(Location::Codegen),
            "Конкатенация внутри параллельной композиции (`(A + B) | C`) требует \
             вложенного счётчика шагов. Напечатать её шаги как параллельные значило \
             бы молча изменить семантику модели"
                .to_string(),
        )
        .with_code("ST-011")),
    }
}
