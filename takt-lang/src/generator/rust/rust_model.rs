//! Автомат: `enum` состояний, `struct` модели, такт (задачи 0050-05, 0050-06).
//!
//! ## Контракт такта (ADR 0033) — воспроизводится, а не изобретается
//!
//! **Вход в стартовое состояние не расходует такт.** Цель `c` добивается этого
//! диспетчеризацией `INIT` через `if (model->state == {PREFIX}_INIT) { … }`
//! **до** `switch` и **без `break`** — тело стартового состояния исполняется в
//! том же такте. В Rust провала из `if` в `match` нет, но он и не нужен: `match`
//! читает **свежезаписанное** `self.state`, что даёт ровно тот же порядок.
//!
//! Контракт объявлен обязательным для будущих бэкендов (`CLAUDE.md`): «вход не
//! стоит такта; не воспроизводить чужие `INIT`-такты».
//!
//! ## Где Rust расходится с C — и почему это правильно
//!
//! | C | Rust | Причина |
//! |---|---|---|
//! | `if (c1) {…break;} if (c2) {…break;}` | `if c1 {…} else if c2 {…}` | `break` в C = «такт окончен», то есть второй `if` при сработавшем первом недостижим. `else if` выражает то же, но **без** недостижимого кода, который валит `-D warnings` (проба П5) |
//! | безусловный переход + следующие `ref` за ним | эмиссия рёбер прекращается | там же: код за безусловным переходом недостижим |
//! | `_TICK`, `_END` у составных состояний | не эмитятся | в C они мертвы и молча (поле пишется, но не читается); в Rust `dead_code` это ловит. Решение R9, вариант (а) |
//! | под-модель получает указатель `main` | корневые переменные — параметры `&mut` | `self.cabin.tick(&mut self)` заимствовал бы `self` дважды. Заимствования непересекающихся **полей** законны, поэтому `self.cabin.tick(&mut self.hal, &mut self.command)` собирается |

use crate::diagnostics::{Diagnostic, Location};
use crate::generator::indent::Printer;
use crate::generator::rust::rust_decl::{PortSet, default_value, model_fields};
use crate::generator::rust::rust_expr::{Scope, coerce_to};
use crate::generator::rust::rust_map::RustMap;
use crate::generator::rust::rust_name::{check_name_collisions, rust_type_name, rust_value_name};
use crate::generator::rust::rust_tick::emit_tick;
use crate::generator::rust::rust_time;
use crate::generator::rust::rust_type::rust_type;
use crate::semantic::minimap::{Element, Name, StateExtend};
use crate::semantic::{ModelNode, VariableNode};
use std::collections::{BTreeMap, BTreeSet};

/// Экземпляр под-модели, лежащий полем в `struct` родителя.
pub(crate) struct Instance {
    /// Имя поля (`main_cabin0`).
    pub(crate) field: String,
    /// Имя типа под-модели (`ElevatorMiniCabin`).
    pub(crate) ty: String,
    /// Уникальное имя модели в карте — для поиска её общих переменных.
    pub(crate) unique: String,
}

/// Таблица состояний модели: имя варианта `enum` для каждого состояния.
pub(crate) struct StateTable {
    /// Имя типа перечисления состояний (`ElevatorMiniCabinState`).
    pub(crate) enum_name: String,
    /// Достижимые состояния: уникальное имя → имя варианта.
    variants: Vec<(Name, String)>,
    /// Нужно ли эмитить собственный вариант `End`.
    ///
    /// Если у автора есть состояние `End`, оно даёт вариант `End` само —
    /// второй был бы дубликатом (та же развилка, что `end_already_generated` в
    /// `c_header.rs`).
    pub(crate) emit_end: bool,
}

impl StateTable {
    /// Строит таблицу по достижимым состояниям модели.
    pub(crate) fn build(map: &RustMap, name: &Name, states: &[Name]) -> Result<Self, Diagnostic> {
        let mut variants = Vec::new();
        for state in states {
            if map.state_at(state.clone()).is_none() {
                // Недостижимое состояние варианта не получает: неконструируемый
                // вариант валит `-D warnings` (dead_code). Решение R9, вариант (а).
                continue;
            }
            variants.push((
                state.clone(),
                rust_type_name(state.local(), Location::Codegen)?,
            ));
        }
        let named: Vec<(String, String)> = variants
            .iter()
            .map(|(n, v)| (n.local().to_string(), v.clone()))
            .collect();
        check_name_collisions(&named, "состояния модели", Location::Codegen)?;

        let emit_end = !variants.iter().any(|(_, v)| v == "End");
        Ok(Self {
            enum_name: format!("{}State", name.unique_camelcase()),
            variants,
            emit_end,
        })
    }

    /// Имя варианта для состояния.
    pub(crate) fn variant_of(&self, state: &Name) -> Result<String, Diagnostic> {
        self.variants
            .iter()
            .find(|(n, _)| n.unique() == state.unique())
            .map(|(_, v)| v.clone())
            .ok_or_else(|| {
                Diagnostic::error(
                    Location::Codegen,
                    format!("Состояние '{}' недостижимо и варианта не имеет", state),
                )
                .with_code("RS-013")
            })
    }

    /// Полный путь к варианту (`ElevatorMiniCabinState::Idle`).
    pub(crate) fn path_of(&self, state: &Name) -> Result<String, Diagnostic> {
        Ok(format!("{}::{}", self.enum_name, self.variant_of(state)?))
    }

    /// Путь к терминальному варианту.
    pub(crate) fn end_path(&self) -> String {
        format!("{}::End", self.enum_name)
    }
}

/// Печатает перечисление состояний модели.
///
/// Перечисление **приватно** — в отличие от пользовательских перечислений. Это
/// не мелочь: проба 2026-07-16 показала, что `pub enum` с неконструируемым
/// вариантом `-D warnings` **проходит**, а приватный — нет. То есть публичность
/// здесь была бы вариантом (б) решения R9 («заглушить линт») в маскировке.
/// Состояния придумывает генератор, поэтому сторож `dead_code` над ними должен
/// остаться живым: неконструируемый вариант = дефект эмиссии.
pub(crate) fn emit_state_enum(p: &mut Printer, table: &StateTable) -> Result<(), Diagnostic> {
    p.ident("#[derive(Debug, Clone, Copy, PartialEq, Eq)]").nl();
    p.ident(&format!("enum {} {{", table.enum_name)).nl();
    p.up();
    p.ident("/// Модель создана, но стартовое состояние ещё не занято.")
        .nl();
    p.ident("Init,").nl();
    for (_, variant) in &table.variants {
        p.ident(&format!("{},", variant)).nl();
    }
    if table.emit_end {
        p.ident("/// Автомат завершён (`is_done`).").nl();
        p.ident("End,").nl();
    }
    p.down();
    p.ident("}").nl().nl();
    Ok(())
}

/// Печатает перечисления шагов последовательных композиций модели.
///
/// Приватны — как и перечисление состояний, и по той же причине: их придумывает
/// генератор, поэтому `dead_code` над ними обязан остаться сторожем.
///
/// Варианты `Init`/`End`, которые цель `c` эмитит у составного состояния
/// (`{STATE}_INIT`, `{STATE}_END`), здесь **не эмитятся**: в C они мертвы —
/// `_init` сразу ставит вариант ПЕРВОГО шага, а `End` не пишется никогда. В C
/// это молча, в Rust `dead_code` поймал бы (решение R9, вариант (а)).
fn emit_seq_enums(
    p: &mut Printer,
    model: &Name,
    concats: &[(Name, Vec<ConcatStep>)],
) -> Result<(), Diagnostic> {
    for (state, steps) in concats {
        p.ident(&format!(
            "/// Шаг последовательной композиции состояния '{}'.",
            state.local()
        ))
        .nl();
        p.ident("#[derive(Debug, Clone, Copy, PartialEq, Eq)]").nl();
        p.ident(&format!("enum {} {{", seq_enum_name(model, state)?))
            .nl();
        p.up();
        for step in steps {
            p.ident(&format!("{},", step.variant)).nl();
        }
        p.down();
        p.ident("}").nl().nl();
    }
    Ok(())
}

/// Собирает экземпляры под-моделей состояния в плоский список полей.
///
/// Цель `c` строит вложенные анонимные структуры (`main.cabin0`), потому что ей
/// нужно место под enum составного состояния. Здесь поля плоские: enum
/// последовательной композиции живёт отдельным полем, а параллельной — не нужен
/// вовсе (в C он мёртв: пишется в `_init` и не читается никогда).
pub(crate) fn collect_instances(
    extend: &StateExtend,
    prefix: &str,
    out: &mut Vec<Instance>,
) -> Result<(), Diagnostic> {
    match extend {
        StateExtend::None => Ok(()),
        StateExtend::Model(name) => {
            out.push(Instance {
                field: rust_value_name(prefix, Location::Codegen)?,
                ty: name.unique_camelcase(),
                unique: name.unique().to_string(),
            });
            Ok(())
        }
        StateExtend::Parallel(steps) | StateExtend::Concatenation(steps) => {
            for (idx, step) in steps.iter().enumerate() {
                let sub = match step {
                    StateExtend::Model(name) => {
                        format!("{}_{}{}", prefix, name.local_lowercase_snakecase(), idx)
                    }
                    _ => format!("{}_group{}", prefix, idx),
                };
                collect_instances(step, &sub, out)?;
            }
            Ok(())
        }
    }
}

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
            StateExtend::Model(name) => (
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
pub(crate) fn seq_enum_name(model: &Name, state: &Name) -> Result<String, Diagnostic> {
    Ok(format!(
        "{}{}Seq",
        model.unique_camelcase(),
        rust_type_name(state.local(), Location::Codegen)?
    ))
}

/// Имя поля-счётчика шага (`start_seq`).
pub(crate) fn seq_field_name(state: &Name) -> Result<String, Diagnostic> {
    rust_value_name(
        &format!("{}_seq", state.local_lowercase_snakecase()),
        Location::Codegen,
    )
}

/// Все последовательные композиции модели: состояние → его шаги.
pub(crate) fn model_concats(
    map: &RustMap,
    states: &[Name],
) -> Result<Vec<(Name, Vec<ConcatStep>)>, Diagnostic> {
    let mut out = Vec::new();
    for state in states {
        let Some(Element::StateExtend { extend, .. }) = map.state_at(state.clone()) else {
            continue;
        };
        let StateExtend::Concatenation(steps) = &extend else {
            continue;
        };
        let parsed = concat_steps(steps, &state.local_lowercase_snakecase(), state)?;
        if !parsed.is_empty() {
            out.push((state.clone(), parsed));
        }
    }
    Ok(out)
}

/// Все экземпляры под-моделей модели — по одному на элемент композиции.
pub(crate) fn model_instances(
    map: &RustMap,
    states: &[Name],
) -> Result<Vec<(Name, Vec<Instance>)>, Diagnostic> {
    let mut out = Vec::new();
    for state in states {
        let Some(Element::StateExtend { extend, .. }) = map.state_at(state.clone()) else {
            continue;
        };
        let mut instances = Vec::new();
        collect_instances(&extend, &state.local_lowercase_snakecase(), &mut instances)?;
        if !instances.is_empty() {
            out.push((state.clone(), instances));
        }
    }
    Ok(out)
}

use crate::generator::rust::rust_shared::{
    emit_shared_new_block, emit_shared_struct, shared_type_name, shared_union, shared_variables,
    union_names as shared_union_names,
};

/// Печатает `struct` модели и её `impl`.
#[allow(clippy::too_many_arguments)]
pub(crate) fn emit_model(
    p: &mut Printer,
    map: &RustMap,
    name: &Name,
    model: &ModelNode,
    is_root: bool,
    ports: &PortSet,
    warnings: &mut Vec<Diagnostic>,
) -> Result<(), Diagnostic> {
    let element = if is_root {
        map.model()
    } else {
        map.element_of(name).ok_or_else(|| {
            Diagnostic::error(
                Location::Codegen,
                format!("Модель '{}' отсутствует в снимке карты", name),
            )
            .with_code("RS-012")
        })?
    };
    let Element::Model { states, start, .. } = &element else {
        return Err(Diagnostic::error(
            Location::Codegen,
            format!("Элемент '{}' не является моделью", name),
        )
        .with_code("RS-012"));
    };

    let table = StateTable::build(map, name, states)?;
    let instances = model_instances(map, states)?;
    let concats = model_concats(map, states)?;
    // Корень владеет структурой `Shared` (объединение нужд под-моделей); его
    // scope.shared — весь союз (для доступа `self.shared.x`). Под-модель получает
    // `&mut Shared` и разделяет лишь свою часть (фича 0059).
    let shared = if is_root {
        shared_union(map)
    } else {
        shared_variables(map, name)
    };
    let union_names = shared_union_names(map, is_root);
    // HAL нужен модели, только если она к нему обращается — сама либо через
    // под-модель. Класть `hal: H` в модель, которая его не читает, нельзя:
    // `field 'hal' is never read` валит гейт (проба 2026-07-16).
    let uses_hal = !ports.is_empty() && needs_hal(map, name, is_root, &mut BTreeSet::new());
    let struct_name = name.unique_camelcase();

    emit_state_enum(p, &table)?;
    emit_seq_enums(p, name, &concats)?;

    // ── struct Shared (фича 0059) ──────────────────────────────────────────────
    // Эмиссия — в `rust_shared` (приватная структура, правило 3 ADR); у корня и
    // только если под-моделям есть что разделять (правило 1).
    if is_root {
        emit_shared_struct(p, map, name.local(), &shared)?;
    }

    // ── struct ───────────────────────────────────────────────────────────────
    // Объявление параметра и его подстановка — РАЗНЫЕ строки: граница пишется
    // один раз (`impl<H: Hal>`), а в позиции типа стоит голое имя
    // (`ElevatorMini<H>`). Повторить границу в аргументах — ошибка E0229.
    let generics = if is_root && uses_hal { "<H: Hal>" } else { "" };
    let type_args = if is_root && uses_hal { "<H>" } else { "" };
    p.ident(&format!("/// Модель '{}'.", name.local())).nl();
    p.ident(&format!("pub struct {}{} {{", struct_name, generics))
        .nl();
    let _ = &type_args;
    p.up();
    for (_, var) in model_fields(model, map) {
        let VariableNode::Simple {
            name: vname,
            ty,
            loc,
            ..
        } = var
        else {
            continue;
        };
        // Общая переменная уезжает в поле `shared` — прямым полем не остаётся.
        if union_names.contains(vname) {
            continue;
        }
        p.ident(&format!(
            "{}: {},",
            rust_value_name(vname, *loc)?,
            rust_type(ty, &format!("переменная '{}'", vname))?
        ))
        .nl();
    }
    if is_root && !shared.is_empty() {
        p.ident("/// Общие с под-моделями переменные (фича 0059).")
            .nl();
        p.ident(&format!("shared: {},", shared_type_name(map))).nl();
    }
    p.ident(&format!("state: {},", table.enum_name)).nl();
    // Поля механизма времени (фича 0134): счётчик тактов / метка `now_ms` /
    // предыдущее состояние — только при использовании `after` (иначе `-D warnings`
    // упадёт на неиспользуемом поле). Логика в `rust_time`.
    rust_time::emit_struct_fields(p, map, model, &table.enum_name)?;
    for (state, steps) in &concats {
        p.ident(&format!(
            "/// Текущий шаг последовательной композиции состояния '{}'.",
            state.local()
        ))
        .nl();
        p.ident(&format!(
            "{}: {},",
            seq_field_name(state)?,
            seq_enum_name(name, state)?
        ))
        .nl();
        let _ = steps;
    }
    for (_, list) in &instances {
        for instance in list {
            p.ident(&format!("{}: {},", instance.field, instance.ty))
                .nl();
        }
    }
    if is_root && uses_hal {
        p.ident("/// Аппаратный слой. Заменяет `void *userdata` цели `c`.")
            .nl();
        p.ident("hal: H,").nl();
    }
    p.down();
    p.ident("}").nl().nl();

    // ── impl ─────────────────────────────────────────────────────────────────
    p.ident(&format!("impl{} {}{} {{", generics, struct_name, type_args))
        .nl();
    p.up();
    emit_new(
        p, map, model, &table, &instances, &concats, name, is_root, uses_hal,
    )?;
    emit_init(p, map, model, &table, &instances, &concats, name, is_root)?;
    emit_tick(
        p, map, name, model, &element, &table, &instances, &concats, start, states, &shared,
        is_root, uses_hal, ports, warnings,
    )?;
    emit_reset(p, is_root)?;
    emit_is_done(p, &table, is_root)?;
    p.down();
    p.ident("}").nl().nl();
    Ok(())
}

/// Нужен ли модели доступ к HAL — **транзитивно**.
///
/// Транзитивность здесь обязательна, а не желательна. Модель может не иметь ни
/// одного порта и всё же нуждаться в `hal`: если её под-модель к железу
/// обращается, родитель обязан HAL **пронести**. Ровно так устроен
/// `elevator_mini`, где все порты объявлены в `Cabin`/`Motor`, а корень —
/// только композиция.
///
/// Обратное так же важно: дать `hal` модели, которая его не трогает, нельзя —
/// у корня это `field 'hal' is never read`, у под-модели `unused variable: hal`.
/// И то и другое валит гейт. То есть точность этого предиката — условие
/// прохождения `-D warnings`, а не аккуратность (решение R9).
pub(crate) fn needs_hal(
    map: &RustMap,
    name: &Name,
    is_root: bool,
    seen: &mut BTreeSet<String>,
) -> bool {
    if !seen.insert(name.unique().to_string()) {
        // Цикл невозможен (модель не содержит саму себя), но защита дешевле
        // доказательства.
        return false;
    }
    // Собственные порты и вызовы, требующие HAL, — прямая нужда.
    if let Ok(model_rc) = map.raw_model_at(name.clone()) {
        let model = model_rc.borrow();
        let usage_of_model = crate::semantic::unused::compute_usage(std::rc::Rc::clone(&model_rc));
        // Порт, которого модель КАСАЕТСЯ, а не который она ОБЪЯВЛЯЕТ. Разница
        // не теоретическая: `stacker` пишет `cmd_fork`, объявленный в корне, из
        // под-модели. Проверка по объявлениям давала бы «HAL не нужен», и запись
        // печаталась бы в пустоту — `RS-022` на ровном месте.
        let has_ports = !usage_of_model.ports.is_empty();
        // Вызовы считаются по фактическому использованию ИМЕННО этой модели, а не
        // по всему файлу: иначе `debug` в одной модели потянул бы `hal` во все.
        //
        // Важно: имя ищется через `search_func`, который поднимается по цепочке
        // родителей. `extern fn` объявлен в корне, а вызывает его под-модель
        // (так устроен `comprehensive.takt`) — проверка только собственной
        // таблицы функций дала бы «HAL не нужен», и вызов напечатался бы в
        // пустоту (`.log_temp(x)` без получателя).
        let usage = &usage_of_model;
        // Нужда вызываемых функций — тоже нужда модели: если `travel_time`
        // читает порт, то вызывающая её модель обязана иметь `hal`, чтобы было
        // что передать. Считается тем же предикатом, что и сигнатура функции.
        let needs_call = usage.functions.iter().any(|fname| {
            crate::generator::rust::rust_needs::needs_of_call(fname, &model, &mut BTreeSet::new())
                .map(|needs| needs.hal)
                .unwrap_or(false)
        });
        // Выдержка `after Nms` в профиле «часы» зовёт `now_ms` — метод HAL (0134).
        let needs_time = rust_time::needs_entry_ms(map, &model);
        if has_ports || needs_call || needs_time {
            return true;
        }
    }
    // Нужда под-моделей — тоже нужда: HAL придётся пронести через себя.
    let element = if is_root {
        map.model()
    } else {
        match map.element_of(name) {
            Some(element) => element,
            None => return false,
        }
    };
    let Element::Model { states, .. } = &element else {
        return false;
    };
    let Ok(instances) = model_instances(map, states) else {
        return false;
    };
    for (_, list) in instances {
        for instance in list {
            let Some(sub) = submodel_name(map, &instance.unique) else {
                continue;
            };
            if needs_hal(map, &sub, false, seen) {
                return true;
            }
        }
    }
    false
}

/// Ищет имя под-модели в карте по её уникальному имени.
pub(crate) fn submodel_name(map: &RustMap, unique: &str) -> Option<Name> {
    map.using_models()
        .into_iter()
        .find_map(|element| match element {
            Element::Model { name, .. } if name.unique() == unique => Some(name),
            _ => None,
        })
}

/// Печатает конструктор.
#[allow(clippy::too_many_arguments)]
fn emit_new(
    p: &mut Printer,
    map: &RustMap,
    model: &ModelNode,
    table: &StateTable,
    instances: &[(Name, Vec<Instance>)],
    concats: &[(Name, Vec<ConcatStep>)],
    model_name: &Name,
    is_root: bool,
    uses_hal: bool,
) -> Result<(), Diagnostic> {
    let scope = Scope {
        model,
        shared: Vec::new(),
        shared_via_self: false,
        locals: Vec::new(),
        assigned: BTreeSet::new(),
        hal: String::new(),
        has_self: false,
        hal_is_ref: false,
        instances: Vec::new(),
        time_profile: map.time_profile(),
    };
    let args = if is_root && uses_hal { "hal: H" } else { "" };
    let vis = if is_root { "pub " } else { "" };
    if is_root && uses_hal {
        p.ident("/// Создаёт модель поверх аппаратного слоя `hal`.")
            .nl();
        p.ident("///").nl();
        p.ident("/// В отличие от цели `c`, забыть проставить доступ к железу")
            .nl();
        p.ident("/// невозможно: без `hal` модель не конструируется.")
            .nl();
    } else {
        p.ident("/// Создаёт модель в начальном состоянии.").nl();
    }
    // Общие переменные корня инициализируются внутри блока `shared { … }`
    // (фича 0059). Собираем их значения, прямые поля печатаем сразу.
    let union = if is_root {
        shared_union(map)
    } else {
        Vec::new()
    };
    let union_names: BTreeSet<String> = union.iter().map(|(n, _)| n.clone()).collect();
    let mut shared_inits: BTreeMap<String, String> = BTreeMap::new();
    p.ident(&format!("{}fn new({}) -> Self {{", vis, args)).nl();
    p.up();
    p.ident("Self {").nl();
    p.up();
    for (_, var) in model_fields(model, map) {
        let VariableNode::Simple {
            name: vname,
            ty,
            expr,
            loc,
            ..
        } = var
        else {
            continue;
        };
        let value = match expr {
            crate::semantic::ExpressionNode::None => default_value(ty, model)?,
            other => coerce_to(other, ty, &scope)?,
        };
        if union_names.contains(vname) {
            shared_inits.insert(vname.clone(), value);
        } else {
            p.ident(&format!("{}: {},", rust_value_name(vname, *loc)?, value))
                .nl();
        }
    }
    if is_root {
        emit_shared_new_block(p, map, &union, &shared_inits)?;
    }
    p.ident(&format!("state: {}::Init,", table.enum_name)).nl();
    // Начальные значения полей времени (фича 0134): метку латчим не здесь, а в
    // INIT-диспетчере такта (в конструкторе HAL под-модели недоступен).
    rust_time::emit_new_fields(p, map, model, &table.enum_name)?;
    // Счётчик шага стартует с ПЕРВОГО шага, а не с «Init»: так же поступает
    // `_init` цели `c` (её варианты `{STATE}_INIT`/`_END` не пишутся никогда).
    for (state, steps) in concats {
        let first = steps.first().ok_or_else(|| {
            Diagnostic::error(
                Location::Codegen,
                format!("Состояние '{}': композиция без шагов", state.local()),
            )
            .with_code("RS-021")
        })?;
        p.ident(&format!(
            "{}: {}::{},",
            seq_field_name(state)?,
            seq_enum_name(model_name, state)?,
            first.variant
        ))
        .nl();
    }
    for (_, list) in instances {
        for instance in list {
            p.ident(&format!("{}: {}::new(),", instance.field, instance.ty))
                .nl();
        }
    }
    if is_root && uses_hal {
        p.ident("hal,").nl();
    }
    p.down();
    p.ident("}").nl();
    p.down();
    p.ident("}").nl().nl();
    Ok(())
}

/// Печатает `init` — приведение памяти в начальное состояние.
///
/// Блоков `enter` здесь нет **намеренно**: по ADR 0033 (R6) в `_init` живёт
/// только память, а поведение входа — в такте. Иначе вход в стартовое состояние
/// стоил бы такта, и трасса разошлась бы с симулятором.
#[allow(clippy::too_many_arguments)]
fn emit_init(
    p: &mut Printer,
    map: &RustMap,
    model: &ModelNode,
    table: &StateTable,
    instances: &[(Name, Vec<Instance>)],
    concats: &[(Name, Vec<ConcatStep>)],
    model_name: &Name,
    is_root: bool,
) -> Result<(), Diagnostic> {
    let scope = Scope {
        model,
        shared: Vec::new(),
        shared_via_self: false,
        locals: Vec::new(),
        assigned: BTreeSet::new(),
        hal: String::new(),
        has_self: false,
        hal_is_ref: false,
        instances: Vec::new(),
        time_profile: map.time_profile(),
    };
    let vis = if is_root { "pub " } else { "" };
    p.ident("/// Возвращает модель в начальное состояние.").nl();
    p.ident("///").nl();
    p.ident("/// Блоки `enter` здесь не исполняются: по контракту ADR 0033 вход")
        .nl();
    p.ident("/// в стартовое состояние — это поведение, и оно живёт в `tick`.")
        .nl();
    let union_names = shared_union_names(map, is_root);
    p.ident(&format!("{}fn init(&mut self) {{", vis)).nl();
    p.up();
    for (_, var) in model_fields(model, map) {
        let VariableNode::Simple {
            name: vname,
            ty,
            expr,
            loc,
            ..
        } = var
        else {
            continue;
        };
        let value = match expr {
            crate::semantic::ExpressionNode::None => default_value(ty, model)?,
            other => coerce_to(other, ty, &scope)?,
        };
        // Общая переменная живёт в `self.shared` (фича 0059).
        let target = if union_names.contains(vname) {
            format!("self.shared.{}", rust_value_name(vname, *loc)?)
        } else {
            format!("self.{}", rust_value_name(vname, *loc)?)
        };
        p.ident(&format!("{} = {};", target, value)).nl();
    }
    p.ident(&format!("self.state = {}::Init;", table.enum_name))
        .nl();
    // Сброс полей времени (фича 0134): в 0 / `Init`. Метку латчит INIT-диспетчер
    // такта — `init(&mut self)` под-модели HAL не имеет.
    rust_time::emit_init(p, map, model, &table.enum_name);
    for (state, steps) in concats {
        let first = steps.first().ok_or_else(|| {
            Diagnostic::error(
                Location::Codegen,
                format!("Состояние '{}': композиция без шагов", state.local()),
            )
            .with_code("RS-021")
        })?;
        p.ident(&format!(
            "self.{} = {}::{};",
            seq_field_name(state)?,
            seq_enum_name(model_name, state)?,
            first.variant
        ))
        .nl();
    }
    // Инициализация вложенных — здесь (0033, R6): чтение поля до первого `tick`
    // не должно давать мусор.
    //
    // Инициализируются ВСЕ экземпляры, включая шаги композиции, которые ещё не
    // наступили. Цель `c` в `_init` трогает только первый шаг, но расхождения
    // нет: шаг не тикает до своей очереди, а при передаче хода его `init`
    // вызывается заново — как и в C.
    for (_, list) in instances {
        for instance in list {
            p.ident(&format!("self.{}.init();", instance.field)).nl();
        }
    }
    p.down();
    p.ident("}").nl().nl();
    Ok(())
}

/// Печатает `reset` — паритет с целью `c`, где `_reset` вызывает `_init`.
///
/// Эмитится **только корню**. Цель `c` заводит `_reset` каждой модели, но
/// сбрасывать под-модель по отдельности никто не может и не должен: сброс
/// корня и так доходит до вложенных через `init`. В C такой `_reset` — просто
/// мёртвая функция, и это молча; в Rust `dead_code` ловит её и валит гейт
/// («method `reset` is never used»).
///
/// Ещё один случай решения R9, вариант (а): не эмитить то, чего не бывает.
/// Заглушить линт было бы дешевле на одну строку — и на этом закончился бы
/// сторож, который эту находку и принёс.
fn emit_reset(p: &mut Printer, is_root: bool) -> Result<(), Diagnostic> {
    if !is_root {
        return Ok(());
    }
    p.ident("/// Сбрасывает модель. Паритет с `_reset` цели `c`.")
        .nl();
    p.ident("///").nl();
    p.ident("/// Сброс доходит до вложенных моделей через `init`.")
        .nl();
    p.ident("pub fn reset(&mut self) {").nl();
    p.up();
    p.ident("self.init();").nl();
    p.down();
    p.ident("}").nl().nl();
    Ok(())
}

/// Печатает `is_done`.
///
/// Обращение `self.state == …::End` — единственное место, где вариант `End`
/// упоминается у модели без терминальных состояний. Этого достаточно: проба
/// 2026-07-16 показала, что упоминание варианта в сравнении считается его
/// конструированием, поэтому `dead_code` на `End` не срабатывает никогда, и
/// специально «оживлять» его не требуется.
fn emit_is_done(p: &mut Printer, table: &StateTable, is_root: bool) -> Result<(), Diagnostic> {
    let vis = if is_root { "pub " } else { "" };
    p.ident("/// Завершён ли автомат модели.").nl();
    p.ident(&format!("{}fn is_done(&self) -> bool {{", vis))
        .nl();
    p.up();
    p.ident(&format!("self.state == {}", table.end_path())).nl();
    p.down();
    p.ident("}").nl().nl();
    Ok(())
}
