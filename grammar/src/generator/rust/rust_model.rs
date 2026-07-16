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
use crate::generator::rust::rust_expr::{Scope, coerce_to, condition_as_bool, unwrap_outer};
use crate::generator::rust::rust_map::RustMap;
use crate::generator::rust::rust_name::{check_name_collisions, rust_type_name, rust_value_name};
use crate::generator::rust::rust_stmt::{StmtOutput, print_statement};
use crate::generator::rust::rust_type::rust_type;
use crate::semantic::minimap::{Element, Name, StateExtend};
use crate::semantic::type_node::TypeNode;
use crate::semantic::{Formula, ModelNode, StateNode, VariableNode};
use std::collections::BTreeSet;

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
    emit_end: bool,
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
fn seq_enum_name(model: &Name, state: &Name) -> Result<String, Diagnostic> {
    Ok(format!(
        "{}{}Seq",
        model.unique_camelcase(),
        rust_type_name(state.local(), Location::Codegen)?
    ))
}

/// Имя поля-счётчика шага (`start_seq`).
fn seq_field_name(state: &Name) -> Result<String, Diagnostic> {
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

/// Возвращает переменные **корня**, которыми пользуется под-модель.
///
/// Один источник истины для параметров `tick` и для аргументов вызова: разойдись
/// они, порождённый код не собрался бы (либо, хуже, связал бы не те переменные).
///
/// В цели `c` этой задачи нет — там под-модель получает указатель `main` и
/// пишет `main->command`. В Rust так нельзя: `self.cabin.tick(&mut self)`
/// заимствует `self` дважды. Ровно ту же задачу — и по той же причине
/// («указателей нет») — цель `st` решает через `VAR_IN_OUT`.
///
/// Список считается по **фактическому** использованию, а не «все переменные
/// корня»: лишний параметр — это лишний обязательный аргумент у каждого вызова.
pub(crate) fn shared_variables(map: &RustMap, sub: &Name) -> Vec<(String, TypeNode)> {
    let Some(root) = map.root_model_node() else {
        return Vec::new();
    };
    let Ok(sub_model) = map.raw_model_at(sub.clone()) else {
        return Vec::new();
    };
    let usage = crate::semantic::unused::compute_usage(std::rc::Rc::clone(&sub_model));
    let own: Vec<String> = sub_model.borrow().variables.keys().cloned().collect();
    let root_ref = root.borrow();

    // Переменные корня, нужные функциям, которые под-модель ВЫЗЫВАЕТ.
    //
    // Без этого шага под-модели нечего было бы передать: `travel_time` объявлена
    // в корне и читает его переменные, а `compute_usage(sub)` обходит только
    // СВОИ тела функций — тело корневой функции в него не попадает. Цель `c`
    // такой задачи не знает: там под-модель просто отдаёт указатель `main`.
    let mut from_calls: BTreeSet<String> = BTreeSet::new();
    for fname in &usage.functions {
        let Ok(needs) = crate::generator::rust::rust_needs::needs_of_call(
            fname,
            &sub_model.borrow(),
            &mut BTreeSet::new(),
        ) else {
            continue;
        };
        from_calls.extend(needs.vars.into_keys());
    }

    let mut shared = Vec::new();
    for (name, var) in &root_ref.variables {
        // Константы не разделяются: они неизменны и живут на уровне модуля.
        let VariableNode::Simple { ty, .. } = var else {
            continue;
        };
        let used = usage.variables.contains(name) || from_calls.contains(name);
        if used && !own.contains(name) {
            shared.push((name.clone(), ty.clone()));
        }
    }
    shared
}

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
    let shared = if is_root {
        Vec::new()
    } else {
        shared_variables(map, name)
    };
    // HAL нужен модели, только если она к нему обращается — сама либо через
    // под-модель. Класть `hal: H` в модель, которая его не читает, нельзя:
    // `field 'hal' is never read` валит гейт (проба 2026-07-16).
    let uses_hal = !ports.is_empty() && needs_hal(map, name, is_root, &mut BTreeSet::new());
    let struct_name = name.unique_camelcase();

    emit_state_enum(p, &table)?;
    emit_seq_enums(p, name, &concats)?;

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
        p.ident(&format!(
            "{}: {},",
            rust_value_name(vname, *loc)?,
            rust_type(ty, &format!("переменная '{}'", vname))?
        ))
        .nl();
    }
    p.ident(&format!("state: {},", table.enum_name)).nl();
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
fn needs_hal(map: &RustMap, name: &Name, is_root: bool, seen: &mut BTreeSet<String>) -> bool {
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
        // (так устроен `comprehensive.lam`) — проверка только собственной
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
        if has_ports || needs_call {
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
fn submodel_name(map: &RustMap, unique: &str) -> Option<Name> {
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
        locals: Vec::new(),
        assigned: BTreeSet::new(),
        hal: String::new(),
        has_self: false,
        hal_is_ref: false,
        instances: Vec::new(),
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
        p.ident(&format!("{}: {},", rust_value_name(vname, *loc)?, value))
            .nl();
    }
    p.ident(&format!("state: {}::Init,", table.enum_name)).nl();
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
        locals: Vec::new(),
        assigned: BTreeSet::new(),
        hal: String::new(),
        has_self: false,
        hal_is_ref: false,
        instances: Vec::new(),
    };
    let vis = if is_root { "pub " } else { "" };
    p.ident("/// Возвращает модель в начальное состояние.").nl();
    p.ident("///").nl();
    p.ident("/// Блоки `enter` здесь не исполняются: по контракту ADR 0033 вход")
        .nl();
    p.ident("/// в стартовое состояние — это поведение, и оно живёт в `tick`.")
        .nl();
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
        p.ident(&format!(
            "self.{} = {};",
            rust_value_name(vname, *loc)?,
            value
        ))
        .nl();
    }
    p.ident(&format!("self.state = {}::Init;", table.enum_name))
        .nl();
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

/// Печатает сигнатуру `tick` и его тело.
#[allow(clippy::too_many_arguments)]
fn emit_tick(
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
    let mut params = String::new();
    if needs_hal_param {
        params.push_str(", hal: &mut H");
    }
    for (vname, ty) in shared {
        params.push_str(&format!(
            ", {}: &mut {}",
            rust_value_name(vname, Location::Codegen)?,
            rust_type(ty, &format!("общая переменная '{}'", vname))?
        ));
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
    let mut scope = Scope {
        model,
        shared: shared.iter().map(|(n, _)| n.clone()).collect(),
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
    };

    p.ident("/// Один такт автомата.").nl();
    if is_root {
        p.ident("///").nl();
        p.ident("/// Вход в стартовое состояние такта **не расходует** (контракт")
            .nl();
        p.ident("/// ADR 0033): его тело исполняется в этом же вызове.")
            .nl();
    }
    // Число параметров такта диктует МОДЕЛЬ: столько переменных корня читает
    // под-модель (плюс HAL). Это данные, а не стиль, и порог clippy (7) к ним
    // отношения не имеет — в `stacker.lam` `MovementController` разделяет с
    // корнем девять переменных. Единственное «лечение» — свернуть их в общую
    // структуру; это кандидат в расширения, а не дефект эмиссии.
    //
    // Глушение точечное: конкретный линт на конкретном методе. Ни `dead_code`,
    // ни прочие сторожа R9 оно не задевает.
    // Порог clippy — 7, и `self` в них ВХОДИТ (проверено: 7 параметров плюс
    // `self` дают «8/7»).
    if 1 + shared.len() + usize::from(needs_hal_param) > 7 {
        p.ident("#[allow(clippy::too_many_arguments)]").nl();
    }
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
    p.down();
    p.ident("}").nl();

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
        // LTL описывает бесконечные прогоны — предмет `lamc verify`, а не
        // прошивки. Цель `c` поступает так же.
        Formula::LTL(_) | Formula::None => Ok(()),
    }
}

/// Печатает именованные блоки (`enter`/`exit`/`always`).
fn emit_named_blocks(
    p: &mut Printer,
    state: &StateNode,
    kind: &str,
    scope: &mut Scope,
    out: &mut StmtOutput,
) -> Result<(), Diagnostic> {
    for block in state.get_named_blocks(kind) {
        if let Some(stmt) = block.statement() {
            print_statement(stmt, scope, p, out)?;
        }
    }
    Ok(())
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
    // Реальный случай — `LiftOperating` в `stacker.lam`: захват и укладка
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
    // HAL передаётся, только если он нужен ВЫЗЫВАЕМОЙ модели — тем же
    // предикатом, каким `emit_tick` решает, объявлять ли параметр.
    if !scope.hal.is_empty() && needs_hal(map, &sub_name, false, &mut BTreeSet::new()) {
        args.push(scope.hal_argument("вызов такта под-модели")?);
    }
    for (vname, _) in shared_variables(map, &sub_name) {
        let ident = rust_value_name(&vname, Location::Codegen)?;
        if is_root {
            args.push(format!("&mut self.{}", ident));
        } else {
            // В под-модели общая переменная — уже `&mut`; передаём перезаимствованием.
            args.push(ident);
        }
    }
    Ok(args.join(", "))
}
