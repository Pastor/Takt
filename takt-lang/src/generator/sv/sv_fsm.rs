//! Автомат: состояния, переходы, сброс, композиция (задача 0045-05).
//!
//! ## Two-process, и это решение про семантику, а не про стиль
//!
//! `always_comb` считает следующее состояние и значения (**блокирующие**
//! присваивания), `always_ff` защёлкивает их по фронту (**неблокирующие**).
//! Разделение воспроизводит императивность тела состояния: `v := 1; w := v;`
//! даёт `w = 1`, как в симуляторе и в цели `c`. One-process дал бы `w` = старое
//! `v` — другую модель; three-process (Мур) отвергнут ADR, потому что **модель
//! Takt не Мур**: `enter`/`always`/`exit` суть действия с побочными эффектами,
//! привязанные к переходу.
//!
//! ## Сброс и контракт 0033: сдвиг = 0 достаётся конструктивно
//!
//! Синтетического `INIT`-состояния **нет вовсе** — его роль исполняет цепь
//! сброса: ветвь `if (!rst_n)` кладёт в регистры стартовые состояния всех
//! уровней **одним фронтом**. Поэтому первый же такт после снятия `rst_n`
//! исполняет тело стартового состояния, и сдвиг равен нулю **на любой глубине**
//! (в цели `c` уровни входят последовательно, отсюда сдвиг = глубине, и его
//! потребовалось *убирать* правкой — фича 0033).
//!
//! Отсюда же [`SV-008`]: `enter` стартового состояния обязан быть
//! **константным**. Ветвь сброса синтезируется в цепь сброса триггеров и
//! выражений не вычисляет — вычислять там нечем.
//!
//! ## Композиция: инлайн ВНУТРЬ ветви `case` родителя
//!
//! Формулировка ADR — «логика под-моделей инлайнится в общий `always_comb` в
//! порядке вызова `_tick`» — верна по **порядку**, но умалчивает о
//! **вложенности**. Эталон (`stacker.c:425-433`) зовёт `_tick` под-моделей
//! **внутри** `case STACKER_STACKER`, то есть они работают, только пока родитель
//! в этом состоянии. Вынос наружу изменил бы модель: под-модели продолжали бы
//! работать после выхода родителя.
//!
//! ## `is_done` под-модели читает `_next`, а не регистр
//!
//! В C `_is_done` вызывается **после** `_tick` и читает `model->state` — то есть
//! значение, которое тик только что записал (присваивание в C немедленно
//! видимо). В `always_comb` рабочая копия — `state_next`, поэтому эквивалент —
//! `(<sub>_state_next == <SUB>_END)`. Чтение регистра дало бы значение
//! **предыдущего** такта, то есть ровно тот сдвиг, который осуждает ADR 0033.

use crate::diagnostics::{Diagnostic, Location};
use crate::generator::indent::Printer;
use crate::generator::sv::sv_blocks::{emit_model_prelude, emit_named_blocks, emit_state_prelude};
use crate::generator::sv::sv_const;
use crate::generator::sv::sv_expr::{Scope, print_condition, sv_enum_variant_name};
use crate::generator::sv::sv_map::SvMap;
use crate::generator::sv::sv_module::{SvPorts, check_sv_name};
use crate::generator::sv::sv_names;
use crate::generator::sv::sv_stmt::{
    emit_hoisted_locals, has_early_return, hoist_locals, print_statement,
};
use crate::generator::sv::sv_time;
use crate::generator::sv::sv_type::{enum_width, sv_enum_type_name, sv_type};
use crate::semantic::minimap::{Element, Name, StateExtend};
use crate::semantic::type_node::TypeNode;
use crate::semantic::{ExpressionNode, FunctionDefinitionNode, ModelNode, StateNode, VariableNode};
use std::cell::RefCell;
use std::collections::{BTreeMap, BTreeSet};
use std::rc::Rc;

/// Блок модели: имя в карте и её узел.
pub(crate) type Block = (Name, Rc<RefCell<ModelNode>>);

/// Строит диагностику `SV-002` — конструкция не транслируется.
pub(crate) fn sv002(what: &str) -> Diagnostic {
    Diagnostic::error(
        Location::Codegen,
        format!(
            "{} не транслируется в SystemVerilog целью 'sv'. Молчаливо \
             пропустить конструкцию нельзя: порождённый модуль вёл бы себя \
             иначе, чем модель",
            what
        ),
    )
    .with_code("SV-002")
}

/// Регистровый сигнал модуля: объявление и сброс.
pub(crate) struct Reg {
    /// Имя сигнала (без `_next`).
    pub(crate) name: String,
    /// Часть объявления до имени (`logic [7:0]`, `stacker_state_e`).
    pub(crate) prefix: String,
    /// Распакованная размерность после имени (пусто либо `[0:3]`).
    pub(crate) suffix: String,
    /// Значение в ветви сброса.
    pub(crate) reset: String,
    /// Объявлять ли сам регистр: у выходного порта он уже объявлен в заголовке.
    pub(crate) declare_reg: bool,
}

/// Сигналы и отображения модуля, собранные по всем уровням.
pub(crate) struct Fsm {
    /// Регистры: состояния уровней, переменные, выходные порты.
    regs: Vec<Reg>,
    /// Уровни с механизмом времени (фича 0134): перекрытие `_next` в `always_comb`.
    time_levels: Vec<sv_time::TimeLevel>,
    /// Имена регистровых сигналов (ключи для `Scope::registered`).
    registered: BTreeSet<String>,
    /// Имя регистра состояния по уникальному имени модели.
    pub(crate) state_reg: BTreeMap<String, String>,
    /// Варианты перечислений модели — для восстановления варианта по значению.
    enums: BTreeMap<String, Vec<(String, i128)>>,
    /// Цепочки `+`: несущее состояние и число шагов (для эмиссии enum шага,
    /// задача 0057-01). Порядок — обхода `build`, значит детерминирован (0048).
    step_enums: Vec<(Name, usize)>,
    /// Предупреждения генератора, собранные при печати выражений (фича 0064).
    ///
    /// Ячейка, а не поле-вектор: печать выражения идёт через `&Scope`
    /// (неизменяемая ссылка), а `SV-009` (переменный делитель) обнаруживается
    /// именно там — в единственной точке трансляции всех выражений. Отсюда
    /// интерьерная мутабельность: `print_expression` дописывает сюда через
    /// `scope.warnings`, а `generate_program` отдаёт их **вызывающему** —
    /// печатает CLI (фича 0168). ⚠️ Прежде здесь был `report` с `eprintln!` из
    /// библиотеки, воспроизведённый по образцу `rust`/`st` вместе с их дефектом:
    /// `--quiet` такой вывод не глушил, а формат расходился с общим.
    pub(crate) warnings: std::cell::RefCell<Vec<Diagnostic>>,
}

/// Собирает аргументы инстанцирования по всем реализациям карты (фича 0185).
///
/// Ключ — уникальное имя модели. ⚠️ Уплощение цели `sv` даёт **один** набор
/// регистров на модель: два экземпляра одной модели делят их. Поэтому два
/// РАЗНЫХ набора аргументов у одной модели невыразимы — громкий отказ `SV-016`
/// вместо молчаливой победы последнего. Одинаковые наборы законны.
fn collect_instantiation_args(
    map: &SvMap,
    blocks: &[Block],
) -> Result<BTreeMap<String, Vec<crate::semantic::extend::ParameterArgument>>, Diagnostic> {
    let mut by_model: BTreeMap<String, Vec<crate::semantic::extend::ParameterArgument>> =
        BTreeMap::new();
    let mut walk = |extend: &StateExtend| -> Result<(), Diagnostic> {
        collect_from_extend(extend, &mut by_model)
    };
    for (name, _) in blocks {
        let Some(Element::Model { states, .. }) = map.model_element_of(name) else {
            continue;
        };
        for state_name in &states {
            if let Some(Element::StateExtend { extend, .. }) = map.state_at(state_name.clone()) {
                walk(&extend)?;
            }
        }
    }
    Ok(by_model)
}

/// Рекурсивный обход реализации для [`collect_instantiation_args`].
fn collect_from_extend(
    extend: &StateExtend,
    by_model: &mut BTreeMap<String, Vec<crate::semantic::extend::ParameterArgument>>,
) -> Result<(), Diagnostic> {
    match extend {
        StateExtend::None => Ok(()),
        StateExtend::Model(name, args) => {
            match by_model.get(name.unique()) {
                None => {
                    by_model.insert(name.unique().to_string(), args.clone());
                }
                Some(existing) if existing == args => {}
                Some(_) => {
                    let loc = args
                        .first()
                        .map(|a| a.loc)
                        .unwrap_or(crate::diagnostics::Location::Codegen);
                    return Err(Diagnostic::error(
                        loc,
                        format!(
                            "Модель '{}' инстанцирована с РАЗНЫМИ наборами аргументов: \
                             цель sv уплощает композицию, и экземпляры одной модели \
                             делят одни регистры — разные настройки невыразимы. \
                             Дайте копиям разные имена моделей либо используйте \
                             другую цель",
                            name.local()
                        ),
                    )
                    .with_code("SV-016"));
                }
            }
            Ok(())
        }
        StateExtend::Parallel(items) | StateExtend::Concatenation(items) => {
            for item in items {
                collect_from_extend(item, by_model)?;
            }
            Ok(())
        }
    }
}

/// Имя регистра состояния модели.
///
/// У корня — просто `state` (форма ADR); у под-модели — с префиксом её
/// уникального имени, иначе при уплощении все уровни делили бы один регистр.
fn state_reg_name(name: &Name, root: &Name) -> String {
    if name.unique() == root.unique() {
        "state".to_string()
    } else {
        format!("{}_state", name.unique_lowercase_snakecase())
    }
}

/// Имя типа-перечисления состояний модели.
fn state_enum_name(name: &Name) -> String {
    format!("{}_state_e", name.unique_lowercase_snakecase())
}

/// Имя варианта терминального состояния модели.
pub(crate) fn end_variant(name: &Name) -> String {
    format!("{}_END", name.unique_uppercase_snakecase())
}

/// Имя регистра шага цепочки `+`, несомой состоянием `state` (задача 0057-01).
///
/// Ключ — уникальное имя несущего состояния: две цепочки `+` на разных
/// состояниях (в т.ч. в разных уровнях после уплощения) получают разные
/// регистры автоматически, как и регистры состояний уровней.
pub(crate) fn step_reg_name(state: &Name) -> String {
    format!("{}_step", state.unique_lowercase_snakecase())
}

/// Имя типа-перечисления шага цепочки `+`.
pub(crate) fn step_enum_name(state: &Name) -> String {
    format!("{}_step_e", state.unique_lowercase_snakecase())
}

/// Имя варианта `STEP_i` шага цепочки `+`.
pub(crate) fn step_variant(state: &Name, i: usize) -> String {
    format!("{}_STEP_{}", state.unique_uppercase_snakecase(), i)
}

/// Имя сигнала переменной модели.
///
/// Префикс обязателен: модуль один на корневую модель, поэтому переменные всех
/// под-моделей попадают в одно пространство имён. У цели `c` они лежат по своим
/// структурам (`model->stacker.command_receiver0.counter`), и две под-модели
/// вправе иметь переменную `counter`; здесь они слиплись бы **молча**.
fn var_signal_name(model: &Name, var: &str) -> String {
    format!("{}_{}", model.unique_lowercase_snakecase(), var)
}

/// Значение сигнала в ветви сброса по инициализирующему выражению.
///
/// Единая точка для **переменной модели** и для **начального значения выходного
/// порта** (фича 0187, задача 04): и то и другое ложится в одну и ту же цепь
/// сброса, поэтому правила печати литерала обязаны совпадать. Разъехавшись, они
/// дали бы порту и переменной разное значение при одном и том же тексте
/// инициализатора — расхождение, которое ни verilator, ни yosys не заметят.
///
/// `what` — родительный падеж описания места (`переменной 'x'`, `порта 'led'`):
/// подставляется в диагностику `SV-002`.
///
/// # Ошибки
/// [`SV-002`](sv002) — выражение не является константой: ветвь сброса
/// синтезируется в цепь сброса триггеров и выражений не вычисляет.
fn reset_value(
    expr: &ExpressionNode,
    ty: &TypeNode,
    enums: &BTreeMap<String, Vec<(String, i128)>>,
    what: &str,
    loc: Location,
) -> Result<String, Diagnostic> {
    Ok(match expr {
        // Значение перечисления приходит ЧИСЛОМ (`command := Up` — это
        // `Number(2)`), а перечисления SV строго типизированы: без
        // восстановления варианта ветвь сброса дала бы `%Error-ENUMVALUE`. Та
        // же ловушка описана для цели `rust`.
        ExpressionNode::Number(n) => sv_const::enum_literal(ty, *n, enums)
            // Широкое значение сброса — размерной формой по ширине регистра
            // (фича 0157): голое десятичное больше `i32::MAX` даёт
            // `WIDTHEXPAND`.
            .or_else(|| super::sv_type::sized_literal(*n, ty))
            .unwrap_or_else(|| n.to_string()),
        ExpressionNode::Bool(b) => if *b { "1'b1" } else { "1'b0" }.to_string(),
        // Литерал длительности (фича 0183) — константа в **миллисекундах**: тип
        // `duration` в целях есть беззнаковый вектор миллисекунд, поэтому и
        // значение сброса такое же.
        ExpressionNode::Duration(nanos) => {
            crate::semantic::duration::value_millis(*nanos, loc, &format!("инициализатор {what}"))?
                .to_string()
        }
        // Умолчание без инициализатора: регистр обязан иметь значение сброса —
        // «неинициализированного» триггера не бывает.
        ExpressionNode::None => "'0".to_string(),
        _ => {
            return Err(sv002(&format!(
                "инициализатор {}: ветвь сброса синтезируется в цепь сброса \
                 триггеров и выражений не вычисляет — допустима только \
                 константа",
                what
            )));
        }
    })
}

/// Варианты перечисления состояний модели: сами состояния плюс `END`.
///
/// `INIT` не добавляется — его в цели `sv` не существует (см. шапку модуля).
/// Если состояние с именем `End` объявлено автором, второй `END` не заводится:
/// `unique_uppercase_snakecase` даёт для него ровно `<MODEL>_END`.
fn state_variants(model: &Name, states: &[Name]) -> Vec<String> {
    let mut variants: Vec<String> = states
        .iter()
        .map(|s| s.unique_uppercase_snakecase())
        .collect();
    let end = end_variant(model);
    if !variants.contains(&end) {
        variants.push(end);
    }
    variants
}

impl Fsm {
    /// Собирает сигналы модуля со всех уровней.
    ///
    /// # Ошибки
    /// Диагностики отображения типов (`SV-002`…`SV-004`) и имён
    /// (`SV-007`/`SV-012`).
    pub(crate) fn build(
        map: &SvMap,
        blocks: &[Block],
        root: &Name,
        ports: &SvPorts,
        mmio: Option<&crate::generator::sv::sv_mmio::Mmio>,
    ) -> Result<Self, Diagnostic> {
        let mut fsm = Fsm {
            regs: Vec::new(),
            time_levels: Vec::new(),
            registered: BTreeSet::new(),
            state_reg: BTreeMap::new(),
            enums: BTreeMap::new(),
            step_enums: Vec::new(),
            warnings: std::cell::RefCell::new(Vec::new()),
        };

        // Аргументы инстанцирования (фича 0185): значение сброса регистра
        // параметра берётся из места инстанцирования, а не из объявления.
        let instantiation_args = collect_instantiation_args(map, blocks)?;

        // Перечисления собираются со всех уровней: `command := Up` в АСД —
        // это `Number(2)`, и без списка вариантов имя `COMMAND_UP` не построить.
        for (_, model_rc) in blocks {
            for def in model_rc.borrow().enums.values() {
                fsm.enums
                    .entry(def.name.clone())
                    .or_insert_with(|| def.variants.clone());
            }
        }

        for (name, model_rc) in blocks {
            let Some(Element::Model { start, states, .. }) = map.model_element_of(name) else {
                continue;
            };
            // Регистр состояния уровня. Сбрасывается в СТАРТОВОЕ состояние —
            // никакого INIT: все уровни сбрасываются одним фронтом, отсюда
            // сдвиг = 0 на любой глубине.
            let reg = state_reg_name(name, root);
            fsm.state_reg.insert(name.unique().to_string(), reg.clone());
            fsm.registered.insert(reg.clone());
            fsm.regs.push(Reg {
                name: reg,
                prefix: state_enum_name(name),
                suffix: String::new(),
                reset: start.unique_uppercase_snakecase(),
                declare_reg: true,
            });

            // Переменные модели: свой регистр на каждую, с префиксом уровня.
            let model = model_rc.borrow();
            for var in model.variables.values() {
                let VariableNode::Simple {
                    name: var_name,
                    ty,
                    expr,
                    loc,
                    ..
                } = var
                else {
                    continue;
                };
                if !map.usage().variables.contains(var_name) {
                    continue;
                }
                check_sv_name(var_name, *loc)?;
                let signal = var_signal_name(name, var_name);
                let decl = sv_type(ty, &format!("переменная '{}'", var_name))?;
                // Параметр, заданный при инстанцировании (фича 0185): значение
                // места инстанцирования перекрывает инициализатор объявления —
                // тот же приоритет, что у цели `c` (присваивание после `_init`).
                let arg_expr = instantiation_args
                    .get(name.unique())
                    .and_then(|args| args.iter().find(|a| a.name == *var_name))
                    .map(|a| a.value.clone());
                let expr = arg_expr.as_ref().unwrap_or(expr);
                let reset = reset_value(
                    expr,
                    ty,
                    &fsm.enums,
                    &format!("переменной '{var_name}'"),
                    *loc,
                )?;
                fsm.registered.insert(signal.clone());
                fsm.regs.push(Reg {
                    name: signal,
                    prefix: decl.prefix,
                    suffix: decl.suffix,
                    reset,
                    declare_reg: true,
                });
            }
            drop(model);

            // Регистр шага на каждую цепочку `+` (задача 0057-01). Служебное
            // состояние вне модели: в `is_done` и в выходные порты не попадает —
            // это отдельный `Reg`, а не порт.
            for state_name in &states {
                let Some(Element::StateExtend {
                    extend: StateExtend::Concatenation(items),
                    ..
                }) = map.state_at(state_name.clone())
                else {
                    continue;
                };
                let signal = step_reg_name(state_name);
                fsm.registered.insert(signal.clone());
                fsm.regs.push(Reg {
                    name: signal,
                    prefix: step_enum_name(state_name),
                    suffix: String::new(),
                    reset: step_variant(state_name, 0),
                    declare_reg: true,
                });
                fsm.step_enums.push((state_name.clone(), items.len()));
            }

            // Регистры механизма времени уровня (фича 0134). Логика — в `sv_time`.
            sv_time::push_time_regs(
                &mut fsm.regs,
                &mut fsm.registered,
                &mut fsm.time_levels,
                map,
                name,
                &model_rc.borrow(),
                &state_enum_name(name),
                &end_variant(name),
                &state_reg_name(name, root),
            )?;
        }

        // Выходные порты: регистр уже объявлен в заголовке модуля, нужна только
        // комбинационная пара. Значение сброса — начальное значение порта
        // (`:=`, фича 0187) либо ноль: вывод кристалла обязан иметь
        // определённое значение с первого же фронта. Ветвь сброса — то самое
        // «до первого такта» контракта R5: у цели `sv` иного места нет, там же
        // живут стартовые состояния (ADR 0033).
        for port in &ports.outputs {
            fsm.registered.insert(port.name.clone());
            fsm.regs.push(Reg {
                name: port.name.clone(),
                prefix: port.ty.prefix.clone(),
                suffix: port.ty.suffix.clone(),
                reset: reset_value(
                    &port.init,
                    &port.ty_node,
                    &fsm.enums,
                    &format!("порта '{}'", port.name),
                    port.loc,
                )?,
                declare_reg: false,
            });
        }

        // Адресованные `out`-порты цели `sv-mmio` (фича 0062): в отличие от портов
        // модуля, объявляются ВНУТРИ (`declare_reg: true`) — это биты регистрового
        // файла, а не выводы кристалла. Запись `_next` автоматом и защёлкивание —
        // те же, что у выходного порта; шина их только читает (мультиплексор
        // `reg_rdata` в `sv_mmio`). `in`-биты регистрами автомата не становятся:
        // их пишет шина, и обслуживает их отдельный `always_ff` в `sv_mmio`.
        if let Some(m) = mmio {
            for port in m.outputs() {
                let ty = crate::generator::sv::sv_mmio::port_sv_type(port)?;
                let name = crate::generator::sv::sv_mmio::port_signal_name(port).to_string();
                fsm.registered.insert(name.clone());
                let reset = reset_value(
                    &port.init,
                    &port.ty,
                    &fsm.enums,
                    &format!("порта '{}'", port.name),
                    port.loc,
                )?;
                fsm.regs.push(Reg {
                    name,
                    prefix: ty.prefix,
                    suffix: ty.suffix,
                    reset,
                    declare_reg: true,
                });
            }
        }
        Ok(fsm)
    }

    /// Контекст печати выражений с собранными отображениями.
    pub(crate) fn scope(&self) -> Scope<'_> {
        Scope {
            registered: &self.registered,
            function: None,
            enums: &self.enums,
            warnings: &self.warnings,
        }
    }
}

/// Печатает константы модели как `localparam`.
///
/// `localparam`, а не `parameter`: значение задано моделью и переопределению
/// извне не подлежит — `parameter` объявил бы его настройкой модуля, которой
/// автор не давал.
/// Печатает пользовательские перечисления модели.
pub(crate) fn emit_enums(p: &mut Printer, blocks: &[Block]) -> Result<(), Diagnostic> {
    let mut seen: BTreeSet<String> = BTreeSet::new();
    for (_, model_rc) in blocks {
        let model = model_rc.borrow();
        for def in model.enums.values() {
            if !seen.insert(def.name.clone()) {
                continue;
            }
            // Ширина — по ДИАПАЗОНУ ЗНАЧЕНИЙ (задача 0045-03). Формула ADR (по
            // числу вариантов) на `Idle = 670` дала бы `logic [0:0]` и
            // `%Error-ENUMITEMWIDTH`.
            let (width, signed) =
                enum_width(&def.variants, &format!("перечисление '{}'", def.name))?;
            let sign = if signed { "signed " } else { "" };
            p.ident(&format!("typedef enum logic {}[{}:0] {{", sign, width - 1))
                .nl();
            p.up();
            for (i, (variant, value)) in def.variants.iter().enumerate() {
                let comma = if i + 1 == def.variants.len() { "" } else { "," };
                p.ident(&format!(
                    "{} = {}'d{}{}",
                    sv_enum_variant_name(&def.name, variant),
                    width,
                    value,
                    comma
                ))
                .nl();
            }
            p.down();
            p.ident(&format!("}} {};", sv_enum_type_name(&def.name)))
                .nl()
                .nl();
        }
    }
    Ok(())
}

/// Печатает функции модели как `function automatic`.
///
/// **`automatic` обязателен, а не украшение.** У статической функции SV
/// переменные разделяются между вызовами, поэтому два вызова в одном
/// `always_comb` дали бы **гонку** — то есть тихо неверную схему. `function`
/// без `automatic` — скрытый дефект.
///
/// Состояние модели параметрами **не передаётся** — в отличие от цели `rust`
/// (`rust_needs::FnNeeds`): в уплощённом модуле сигналы видны функции напрямую.
pub(crate) fn emit_functions(
    p: &mut Printer,
    map: &SvMap,
    fsm: &Fsm,
    blocks: &[Block],
) -> Result<(), Diagnostic> {
    let mut seen: BTreeSet<String> = BTreeSet::new();
    for (_, model_rc) in blocks {
        let model = model_rc.borrow();
        for func in model.functions.values() {
            let FunctionDefinitionNode::Local {
                name,
                params,
                ret,
                body,
                loc,
                ..
            } = func
            else {
                // `External` отвергается в месте вызова (`SV-005`): функция,
                // которую никто не зовёт, вывод не ломает и запрета не требует.
                continue;
            };
            if !map.usage().functions.contains(name) || !seen.insert(name.clone()) {
                continue;
            }
            check_sv_name(name, *loc)?;
            let ret_ty = sv_type(ret, &format!("возвращаемый тип функции '{}'", name))?;
            let mut sig: Vec<String> = Vec::new();
            for (param, ty) in params {
                check_sv_name(param, *loc)?;
                let decl = sv_type(ty, &format!("параметр '{}' функции '{}'", param, name))?;
                sig.push(format!("input {}", decl.declare(param)));
            }
            p.ident(&format!(
                "function automatic {} {}({});",
                ret_ty.prefix,
                name,
                sig.join(", ")
            ))
            .nl();
            p.up();
            // Объявления — до операторов: этого требует SystemVerilog, а Takt
            // разрешает объявить переменную посреди тела.
            let mut locals = Vec::new();
            hoist_locals(body, &mut locals);
            emit_hoisted_locals(p, &locals)?;
            // Возврат печатается присваиванием имени функции и исполнения не
            // прерывает, поэтому досрочный возврат сменил бы смысл молча.
            if has_early_return(body) {
                return Err(sv002(&format!(
                    "досрочный возврат из функции '{}': возврат в цели 'sv' \
                     печатается присваиванием имени функции и исполнение не \
                     прерывает, поэтому допустим только последним оператором \
                     тела. Ключевое слово 'return' эту задачу решило бы, но его \
                     не принимает синтезатор yosys. Перепишите функцию так, \
                     чтобы возврат был один и стоял в конце",
                    name
                )));
            }
            let scope = Scope {
                registered: &fsm.registered,
                function: Some(name),
                enums: &fsm.enums,
                warnings: &fsm.warnings,
            };
            print_statement(p, body, &scope)?;
            p.down();
            p.ident("endfunction").nl().nl();
        }
    }
    Ok(())
}

/// Печатает перечисления состояний всех уровней.
pub(crate) fn emit_state_enums(
    p: &mut Printer,
    map: &SvMap,
    blocks: &[Block],
) -> Result<(), Diagnostic> {
    for (name, _) in blocks {
        let Some(Element::Model { states, .. }) = map.model_element_of(name) else {
            continue;
        };
        // Алфавит имени состояния (фича 0200): перечислитель печатается здесь,
        // и без этой проверки не-ASCII имя доехало бы до `verilator`. ⚠️ Дыру
        // нашёл тест по видам объявлений, а не чтение: у переменных, портов и
        // функций проверка была, у состояний — нет.
        sv_names::check_state_names(map, name)?;
        let variants = state_variants(name, &states);
        // Ширина — по диапазону значений (задача 0045-03). Значения назначает
        // генератор (0..n-1), поэтому формула вырождается в ⌈log₂(n)⌉ — то есть
        // совпадает с формулой ADR именно здесь, где та была верна.
        let numbered: Vec<(String, i128)> = variants
            .iter()
            .enumerate()
            .map(|(i, v)| (v.clone(), i as i128))
            .collect();
        let (width, _) = enum_width(&numbered, &format!("состояния модели '{}'", name))?;
        p.ident(&format!(
            "// Состояния модели '{}'. Синтетического INIT нет: стартовое",
            name
        ))
        .nl();
        p.ident("// состояние живёт в ветви сброса (контракт ADR 0033).")
            .nl();
        p.ident(&format!("typedef enum logic [{}:0] {{", width - 1))
            .nl();
        p.up();
        for (i, (variant, value)) in numbered.iter().enumerate() {
            let comma = if i + 1 == numbered.len() { "" } else { "," };
            p.ident(&format!("{} = {}'d{}{}", variant, width, value, comma))
                .nl();
        }
        p.down();
        p.ident(&format!("}} {};", state_enum_name(name))).nl().nl();
    }
    Ok(())
}

/// Печатает перечисления шага для цепочек `+` (задача 0057-01).
///
/// Значения назначает генератор (0..n-1), поэтому ширина — ⌈log₂(n)⌉, как у
/// перечислений состояний. Порядок — обхода `Fsm::build` (детерминизм 0048).
pub(crate) fn emit_step_enums(p: &mut Printer, fsm: &Fsm) -> Result<(), Diagnostic> {
    for (state, count) in &fsm.step_enums {
        let numbered: Vec<(String, i128)> = (0..*count)
            .map(|i| (step_variant(state, i), i as i128))
            .collect();
        let (width, _) = enum_width(&numbered, &format!("шаг цепочки '{}'", state))?;
        p.ident(&format!(
            "// Шаг последовательной композиции '{}' (`+`).",
            state
        ))
        .nl();
        p.ident(&format!("typedef enum logic [{}:0] {{", width - 1))
            .nl();
        p.up();
        for (i, (variant, value)) in numbered.iter().enumerate() {
            let comma = if i + 1 == numbered.len() { "" } else { "," };
            p.ident(&format!("{} = {}'d{}{}", variant, width, value, comma))
                .nl();
        }
        p.down();
        p.ident(&format!("}} {};", step_enum_name(state))).nl().nl();
    }
    Ok(())
}

/// Печатает объявления регистров и их комбинационных пар.
pub(crate) fn emit_signals(p: &mut Printer, fsm: &Fsm) {
    for reg in &fsm.regs {
        let decl = |name: &str| -> String {
            if reg.suffix.is_empty() {
                format!("{} {};", reg.prefix, name)
            } else {
                format!("{} {} {};", reg.prefix, name, reg.suffix)
            }
        };
        if reg.declare_reg {
            p.ident(&decl(&reg.name)).nl();
        }
        p.ident(&decl(&format!("{}_next", reg.name))).nl();
    }
    p.nl();
}

/// Печатает `always_comb`: умолчания и тела уровней.
pub(crate) fn emit_comb(
    p: &mut Printer,
    map: &SvMap,
    fsm: &Fsm,
    root: &Name,
) -> Result<(), Diagnostic> {
    p.ident("// Комбинационная часть: БЛОКИРУЮЩИЕ присваивания, поэтому порядок")
        .nl();
    p.ident("// операторов и видимость записей внутри такта — в точности как в C.")
        .nl();
    p.ident("always_comb begin").nl();
    p.up();
    // Умолчания обязательны: неполное присваивание даёт защёлку, а
    // `verilator -Wall` — LATCH. Это условие гейта, а не стиль.
    p.ident("// Умолчание «остаться как есть». Без него неполное присваивание")
        .nl();
    p.ident("// даёт защёлку (verilator: LATCH).").nl();
    for reg in &fsm.regs {
        p.ident(&format!("{}_next = {};", reg.name, reg.name)).nl();
    }
    p.nl();
    // Перекрытие умолчаний регистров времени (фича 0134). Логика — в `sv_time`.
    sv_time::emit_time_updates(p, &fsm.time_levels)?;
    emit_model_body(p, map, fsm, root)?;
    p.down();
    p.ident("end").nl().nl();
    Ok(())
}

/// Печатает тело одного уровня: `unique case` по его состояниям.
pub(crate) fn emit_model_body(
    p: &mut Printer,
    map: &SvMap,
    fsm: &Fsm,
    model: &Name,
) -> Result<(), Diagnostic> {
    let Some(Element::Model { states, .. }) = map.model_element_of(model) else {
        return Err(sv002(&format!(
            "модель '{}' отсутствует в снимке карты",
            model
        )));
    };
    let reg = fsm
        .state_reg
        .get(model.unique())
        .ok_or_else(|| sv002(&format!("регистр состояния модели '{}'", model)))?;

    // Фича 0083: model-level `always` (вне состояния) — каждый такт до `unique
    // case`, безусловно по состоянию (эталон — шаг 2 `execution("always")`
    // симулятора). В `always_comb` работает над `_next` (умолчания уже заданы).
    let raw_model = map.raw_model_at(model.clone())?;
    emit_model_prelude(p, map, &raw_model.borrow(), fsm)?;

    p.ident(&format!("unique case ({})", reg)).nl();
    p.up();
    for state_name in &states {
        let Some(element) = map.state_at(state_name.clone()) else {
            continue; // недостижимое состояние — ветви не получает
        };
        let raw = map.raw_state_at(state_name.clone())?;
        let raw = &*raw.borrow();
        p.ident(&format!(
            "{}: begin",
            state_name.unique_uppercase_snakecase()
        ))
        .nl();
        p.up();
        emit_state_prelude(p, map, raw, fsm)?;
        // Периодические блоки `every` (фича 0134-09) — после `always`, как в
        // симуляторе. Гейт читает `_next` метки/счётчика (учёт текущего такта).
        {
            let rm = raw_model.borrow();
            for e in sv_time::model_every(&rm) {
                if e.state != state_name.local() {
                    continue;
                }
                let body = e.body;
                sv_time::emit_every_gate(
                    p,
                    &fsm.time_levels,
                    map,
                    model,
                    e.idx,
                    e.period_nanos,
                    |p| super::sv_stmt::print_statement(p, body, &fsm.scope()),
                )?;
            }
        }
        match element {
            Element::State { .. } => {
                emit_transitions(p, map, fsm, raw, model, &states)?;
            }
            Element::StateExtend { extend, next, .. } => {
                super::sv_compose::emit_extend(
                    p, map, fsm, state_name, raw, model, &extend, &next,
                )?;
            }
            Element::Model { .. } => {
                return Err(sv002("модель в позиции состояния"));
            }
        }
        p.down();
        p.ident("end").nl();
    }
    // Терминальная ветвь: `unique case` требует покрытия всех значений, иначе
    // Verilator даёт CASEINCOMPLETE.
    p.ident(&format!("{}: begin end", end_variant(model))).nl();
    p.down();
    p.ident("endcase").nl();
    Ok(())
}

/// Печатает переходы состояния — цепочкой `if / else if`.
///
/// Цепочка, а не независимые `if`: в C каждый переход завершается `break`, то
/// есть **первый сработавший выигрывает**. Независимые `if` дали бы срабатывание
/// всех подходящих подряд, и последний затёр бы предыдущие — порядок рёбер
/// значим.
fn emit_transitions(
    p: &mut Printer,
    map: &SvMap,
    fsm: &Fsm,
    state: &StateNode,
    model: &Name,
    states: &[Name],
) -> Result<(), Diagnostic> {
    let mut printed = 0usize;
    for reference in state.references() {
        let Some(target) = states.iter().find(|n| n.local() == reference.name).cloned() else {
            continue; // цель вне достижимых состояний
        };
        // Решение «ребро безусловно» — у ОДНОГО носителя (фича 0291); см.
        // `ConditionNode::is_unconditional`. Прежде здесь стоял `Unresolved`,
        // и условное ребро становилось безусловным молча.
        let unconditional = reference.cond.is_unconditional();
        if unconditional {
            // Безусловное ребро: всё, что ниже, недостижимо, — и это верно, так
            // как в C оно тоже завершается `break`.
            if printed > 0 {
                p.ident("else begin").nl();
            } else {
                p.ident("begin").nl();
            }
        } else {
            // Выдержка `after` (фича 0134) — через `sv_time`; прочие — общий печатник.
            let scope = fsm.scope();
            let cond =
                match sv_time::after_guard(&fsm.time_levels, map, model, &reference.cond, &scope) {
                    Some(guard) => guard?,
                    None => print_condition(&reference.cond, &scope)?,
                };
            let keyword = if printed == 0 { "if" } else { "else if" };
            p.ident(&format!("{} ({}) begin", keyword, cond)).nl();
        }
        p.up();
        emit_named_blocks(p, state, fsm, "exit")?;
        let target_rc = map.raw_state_at(target.clone())?;
        emit_named_blocks(p, &target_rc.borrow(), fsm, "enter")?;
        let reg = fsm
            .state_reg
            .get(model.unique())
            .ok_or_else(|| sv002(&format!("регистр состояния модели '{}'", model)))?;
        p.ident(&format!(
            "{}_next = {};",
            reg,
            target.unique_uppercase_snakecase()
        ))
        .nl();
        p.down();
        p.ident("end").nl();
        printed += 1;
        if unconditional {
            break;
        }
    }
    // Терминальное состояние: переходов нет — уходим в END, как это делает C.
    if state.is_terminated() {
        emit_named_blocks(p, state, fsm, "exit")?;
        let reg = fsm
            .state_reg
            .get(model.unique())
            .ok_or_else(|| sv002(&format!("регистр состояния модели '{}'", model)))?;
        p.ident(&format!("{}_next = {};", reg, end_variant(model)))
            .nl();
    }
    Ok(())
}

/// Печатает `always_ff`: ветвь сброса и защёлкивание.
pub(crate) fn emit_ff(
    p: &mut Printer,
    map: &SvMap,
    fsm: &Fsm,
    blocks: &[Block],
) -> Result<(), Diagnostic> {
    // Блоки `enter` стартовых состояний исполняются цепью сброса — поэтому
    // обязаны быть константными (`SV-008`).
    let mut enter_resets: Vec<(String, String)> = Vec::new();
    for (name, _) in blocks {
        let Some(Element::Model { start, .. }) = map.model_element_of(name) else {
            continue;
        };
        let raw = map.raw_state_at(start.clone())?;
        let raw = &*raw.borrow();
        for b in raw.get_named_blocks("enter") {
            if let Some(stmt) = b.statement() {
                sv_const::constant_enter_assignments(
                    stmt,
                    start.local(),
                    raw.loc(),
                    &fsm.scope(),
                    &mut enter_resets,
                )?;
            }
        }
    }

    p.ident("// Регистровая часть: НЕБЛОКИРУЮЩИЕ присваивания. Ветвь сброса несёт")
        .nl();
    p.ident("// стартовые состояния ВСЕХ уровней — они сбрасываются одним фронтом,")
        .nl();
    p.ident("// поэтому сдвиг такта равен нулю на любой глубине (контракт 0033).")
        .nl();
    p.ident("always_ff @(posedge clk) begin").nl();
    p.up();
    p.ident("if (!rst_n) begin").nl();
    p.up();
    for reg in &fsm.regs {
        p.ident(&format!("{} <= {};", reg.name, reg.reset)).nl();
    }
    // Блоки `enter` стартовых состояний — после умолчаний: они их уточняют.
    for (signal, value) in &enter_resets {
        p.ident(&format!("{} <= {};", signal, value)).nl();
    }
    p.down();
    // `en` гейтит ТОЛЬКО защёлкивание: сброс выше проверяется первым и от `en`
    // не зависит (правило 3 ADR 0063) — иначе модуль нельзя сбросить, пока он
    // не разрешён. `en == 1` при неподключённом порте (умолчание) → шаг каждый
    // такт, как прежде.
    p.ident("end else if (en) begin").nl();
    p.up();
    for reg in &fsm.regs {
        p.ident(&format!("{} <= {}_next;", reg.name, reg.name)).nl();
    }
    p.down();
    p.ident("end").nl();
    p.down();
    p.ident("end").nl().nl();
    Ok(())
}

/// Печатает выход терминальности корневой модели.
pub(crate) fn emit_is_done(p: &mut Printer, root: &Name) {
    p.ident("// Терминальность модели наблюдаема снаружи — аналог _is_done() цели c.")
        .nl();
    p.ident(&format!(
        "assign is_done = (state == {});",
        end_variant(root)
    ))
    .nl();
}
