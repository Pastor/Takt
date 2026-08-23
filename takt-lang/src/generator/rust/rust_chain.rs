//! Последовательные композиции цели `rust` — шаги, их имена и сбор.
//!
//! Выделено из `rust_model` фичей 0426 по границе ответственности: модель
//! отвечает за структуру и её методы, а этот модуль — за один вопрос, «какие
//! цепочки есть в состоянии и как называются их счётчики». Цепочек стало
//! несколько на состояние (вложенные в параллель), и знание о них лучше
//! держать вместе.

use crate::diagnostics::{Diagnostic, Location};
use crate::generator::rust::rust_map::RustMap;
use crate::generator::rust::rust_model::{Instance, collect_instances};
use crate::generator::rust::rust_name::{rust_type_name, rust_value_name};
use crate::semantic::minimap::{Element, Name, StateExtend};

/// Один шаг последовательной композиции (`A + B + (C | D) + E`).
///
/// Шаг — это либо одна под-модель, либо параллельная группа: внутри шага всё
/// тикает **одновременно**, а сами шаги идут **по очереди**.
pub(crate) struct ConcatStep {
    /// Имя варианта перечисления шага (`A0`, `Group2`).
    pub(crate) variant: String,
    /// Экземпляры, принадлежащие шагу (у параллельной группы — все её ветви).
    pub(crate) instances: Vec<Instance>,
}

/// Разбирает последовательную композицию на шаги.
///
/// Префиксы полей строятся **той же** формулой, что и в [`collect_instances`]:
/// поля `struct` и цепочка такта обязаны смотреть на одни и те же имена, иначе
/// порождённый код не соберётся.
///
/// # Ошибки
/// [`RS-021`] на вложенной последовательной композиции внутри шага. Цель `c`
/// такой случай **молча пропускает** (`_ => {}` в `generate_concat_tick`), то
/// есть шаг просто не тикает и автомат встаёт. Повторять это нельзя: тихо
/// вставший автомат — ровно тот дефект, ради отсутствия которого заведена цель.
pub(crate) fn concat_steps(
    steps: &[StateExtend],
    prefix: &str,
    state: &Name,
) -> Result<Vec<ConcatStep>, Diagnostic> {
    let mut out = Vec::new();
    for (idx, step) in steps.iter().enumerate() {
        let (variant, sub) = match step {
            StateExtend::Model(name, _) => (
                format!(
                    "{}{}",
                    rust_type_name(name.local(), Location::Codegen)?,
                    idx
                ),
                format!("{}_{}{}", prefix, name.local_lowercase_snakecase(), idx),
            ),
            StateExtend::Parallel(_) => {
                (format!("Group{}", idx), format!("{}_group{}", prefix, idx))
            }
            StateExtend::Concatenation(_) => {
                return Err(Diagnostic::error(
                    Location::Codegen,
                    format!(
                        "Состояние '{}': последовательная композиция вложена в шаг \
                         другой последовательной композиции — это не транслируется \
                         в Rust. Разнесите шаги по отдельным состояниям",
                        state.local()
                    ),
                )
                .with_code("RS-021"));
            }
            StateExtend::None => continue,
        };
        let mut instances = Vec::new();
        collect_instances(step, &sub, &mut instances)?;
        if instances.is_empty() {
            continue;
        }
        out.push(ConcatStep { variant, instances });
    }
    Ok(out)
}

/// Имя перечисления шага последовательной композиции (`RootStartSeq`).
pub(crate) fn seq_enum_name(
    model: &Name,
    state: &Name,
    suffix: Option<&str>,
) -> Result<String, Diagnostic> {
    // Суффикс отличает ВЛОЖЕННУЮ цепочку от цепочки состояния (фича 0426):
    // внутри одной параллели их может быть несколько, и у каждой свой счётчик.
    let extra = match suffix {
        Some(tag) => rust_type_name(tag, Location::Codegen)?,
        None => String::new(),
    };
    Ok(format!(
        "{}{}{}Seq",
        model.unique_camelcase(),
        rust_type_name(state.local(), Location::Codegen)?,
        extra
    ))
}

/// Имя поля-счётчика шага (`start_seq`).
pub(crate) fn seq_field_name(state: &Name, suffix: Option<&str>) -> Result<String, Diagnostic> {
    let base = match suffix {
        Some(tag) => format!("{}_{}_seq", state.local_lowercase_snakecase(), tag),
        None => format!("{}_seq", state.local_lowercase_snakecase()),
    };
    rust_value_name(&base, Location::Codegen)
}

/// Последовательная композиция модели: состояние, суффикс поля и шаги.
///
/// ⚠️ Суффикс отличает ВЛОЖЕННУЮ цепочку (фича 0426): `A + B` внутри `| C`
/// имеет собственный счётчик шагов, и цепочек в одном состоянии может быть
/// несколько. `None` — цепочка самого состояния.
pub(crate) struct Chain {
    pub(crate) state: Name,
    pub(crate) suffix: Option<String>,
    pub(crate) steps: Vec<ConcatStep>,
}

/// Все последовательные композиции модели — включая вложенные в параллель.
pub(crate) fn model_concats(map: &RustMap, states: &[Name]) -> Result<Vec<Chain>, Diagnostic> {
    let mut out = Vec::new();
    for state in states {
        let Some(Element::StateExtend { extend, .. }) = map.state_at(state.clone()) else {
            continue;
        };
        match &extend {
            StateExtend::Concatenation(steps) => {
                let parsed = concat_steps(steps, &state.local_lowercase_snakecase(), state)?;
                if !parsed.is_empty() {
                    out.push(Chain {
                        state: state.clone(),
                        suffix: None,
                        steps: parsed,
                    });
                }
            }
            // Вложенные цепочки параллели (фича 0426). Прежде их не собирал
            // никто, и печать такта тикала все ветви РАЗОМ: `A + B` внутри
            // `| C` превращалась в параллель трёх — валидный Rust, другой
            // автомат, инструменты вывод принимали.
            StateExtend::Parallel(items) => {
                for (idx, item) in items.iter().enumerate() {
                    let StateExtend::Concatenation(steps) = item else {
                        continue;
                    };
                    let tag = format!("group{idx}");
                    let prefix = format!("{}_{}", state.local_lowercase_snakecase(), tag);
                    let parsed = concat_steps(steps, &prefix, state)?;
                    if !parsed.is_empty() {
                        out.push(Chain {
                            state: state.clone(),
                            suffix: Some(tag),
                            steps: parsed,
                        });
                    }
                }
            }
            _ => {}
        }
    }
    Ok(out)
}
