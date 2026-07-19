pub(crate) mod builder;
pub(crate) mod statement;
pub(crate) mod viewport;

use crate::context::Context;
use crate::eval::value::Value;
use grammar::diagnostics::Diagnostic;
use std::cell::RefCell;
use std::cmp::PartialEq;
use std::collections::HashMap;
use std::rc::Rc;

/// Человекочитаемое описание диагностики для `TickResult::Failed`.
fn describe(diagnostic: &Diagnostic) -> String {
    match &diagnostic.code {
        Some(code) => format!("{} ({code})", diagnostic.message),
        None => diagnostic.message.clone(),
    }
}

/// Предикат перехода: именованное условие с функцией-проверкой.
///
/// `name` — отображается как метка ребра в SVG-графе.
/// Клонирование дёшево (`Rc` под капотом).
#[derive(Clone)]
// Тип `func` — замыкание-предикат за `Rc<dyn Fn>`; это и есть суть Predicate,
// вынос в псевдоним лишь спрятал бы её (сигнатура отличается от `Execution`
// возвращаемым `bool`, не `Flow`).
#[allow(clippy::type_complexity)]
pub(crate) struct Predicate {
    pub(crate) name: String,
    func: Rc<dyn Fn(&mut dyn Context) -> Result<bool, Diagnostic>>,
}

impl Predicate {
    pub(crate) fn new(
        name: impl Into<String>,
        f: impl Fn(&mut dyn Context) -> Result<bool, Diagnostic> + 'static,
    ) -> Self {
        Self {
            name: name.into(),
            func: Rc::new(f),
        }
    }

    pub(crate) fn evaluate(&self, ctx: &mut dyn Context) -> Result<bool, Diagnostic> {
        (self.func)(ctx)
    }
}

/// Поток управления после исполнения оператора.
///
/// До задачи 0025-02b-2 исполнитель ничего не возвращал, поэтому
/// `return`/`break`/`continue` были невыразимы и молча ронялись.
#[derive(Debug, Clone, PartialEq)]
pub(crate) enum Flow {
    /// Исполнение продолжается со следующего оператора.
    Normal,
    /// `break` — выйти из ближайшего цикла.
    Break,
    /// `continue` — перейти к следующей итерации ближайшего цикла.
    Continue,
    /// `return [значение]` — выйти из тела функции.
    Return(Option<Value>),
}

/// Исполнитель. `Err` — ошибка вычисления: она **обязана** дойти до
/// вызывающего, а не быть напечатанной и забытой (требование R5 фичи 0025).
pub(crate) type Execution = Rc<dyn Fn(&mut dyn Context) -> Result<Flow, Diagnostic>>;
type Executions = HashMap<String, Vec<Execution>>;

/// Проверяемое обязательство (инвариант/Guard-формула, фича 0044): предикат
/// условия и опциональное имя инварианта для диагностики SIM-025.
pub(crate) type Guard = (Predicate, Option<String>);

/// Набор инвариантов узла: формулы модели (проверяются каждый такт) и формулы
/// по состояниям (проверяются, пока автомат в этом состоянии). Точки проверки —
/// эталон порождённого C (ADR 0044): модель до `always`, состояние до `always`.
#[derive(Clone, Default)]
pub(crate) struct Guards {
    /// Инварианты уровня модели.
    pub(crate) model: Vec<Guard>,
    /// Инварианты по имени состояния.
    pub(crate) per_state: HashMap<String, Vec<Guard>>,
}

/// Результат шага симуляции.
///
/// `pub` (а не `pub(crate)`), поскольку возвращается публичным [`Unit::tick`] —
/// иначе `private_interfaces` (пункт бэклога, закрыт попутно задачей 0025-05).
#[derive(Eq, PartialEq, Clone, Debug)]
pub enum TickResult {
    Processing,
    Terminated,
    /// Ошибка вычисления: симуляция недостоверна, продолжать нельзя.
    ///
    /// Именно этот вариант делает ошибку **отличимой** от честно ложного
    /// условия (требование R5): раньше и то и другое давало `false`.
    Failed(String),
}

/// Исполняемый узел автомата — **непрозрачная** обёртка над приватной формой
/// [`UnitKind`] (фича 0036). Форма узла (варианты, поля) — деталь крейта:
/// наружу видны только методы-аксессоры (`tick`, `variable`, `current_state`,
/// …). Так внутренние типы (`Context`/`Flow`/`Predicate`/`Guards`) честно
/// остаются `pub(crate)`, а публичный API крейта не рассогласован
/// (`private_interfaces` держится линтом в `lib.rs`).
#[derive(Clone, Default)]
pub struct Unit(UnitKind);

/// Внутренняя форма [`Unit`]. `pub(crate)`: имя доступно потребителям крейта
/// (`state_io`, `builder`, `viewport`), но наружу не реэкспортируется.
// `Node` — доминирующий вариант (реальные автоматы), `None`/композиты редки:
// боксировать `Node` ради выравнивания размера значило бы платить за общий
// случай. Осознанный компромисс (как было и у прежнего `enum Unit`).
#[allow(clippy::large_enum_variant)]
#[derive(Clone, Default)]
pub(crate) enum UnitKind {
    #[default]
    None,
    Node {
        context: Option<Rc<RefCell<dyn Context>>>,

        state_transitions: HashMap<String, Vec<(String, Predicate)>>,
        state_executions: HashMap<String, Executions>,
        state: Option<String>,

        executions: Executions,
        /// Инварианты модели и состояний (фича 0044), проверяются каждый такт.
        guards: Guards,
        /// Последний сработавший переход: (из, в, имя_предиката).
        last_transition: Option<(String, String, String)>,
        /// Исполнен ли `enter` **стартового** состояния (Д5).
        ///
        /// `enter` вызывался только в ветке перехода, поэтому начальная
        /// инициализация модели терялась. Флаг гарантирует «ровно один раз»:
        /// при возобновлении из сохранённого состояния (`state_io::restore`)
        /// он выставляется в `true` — модель уже находится в состоянии, входить
        /// в него повторно нельзя.
        entered_initial: bool,
    },
    Parallel {
        units: Vec<Rc<RefCell<Unit>>>,
        executions: Executions,
    },
    Sequential {
        units: Vec<Rc<RefCell<Unit>>>,
        index: usize,
        executions: Executions,
    },
}

impl Unit {
    /// Конструктор из внутренней формы — для потребителей крейта вне модуля
    /// `unit` (`state_io`), которым приватное поле недоступно напрямую.
    pub(crate) fn from_kind(kind: UnitKind) -> Self {
        Unit(kind)
    }

    /// Заимствование внутренней формы (чтение) — для тех же потребителей.
    pub(crate) fn kind(&self) -> &UnitKind {
        &self.0
    }

    /// Заимствование внутренней формы (запись).
    pub(crate) fn kind_mut(&mut self) -> &mut UnitKind {
        &mut self.0
    }
}

impl Context for Unit {
    fn get_value(&self, name: &str) -> Option<Value> {
        match &self.0 {
            UnitKind::None => None,
            // 0032: единственный источник истины — контекст модели. Собственной
            // карты значений у узла больше нет; читаем оттуда же, куда пишем.
            UnitKind::Node { context, .. } => context
                .as_ref()
                .and_then(|ctx| ctx.borrow().get_value(name)),
            UnitKind::Parallel { units, .. } => {
                units.iter().find_map(|unit| unit.borrow().get_value(name))
            }
            UnitKind::Sequential { units, index, .. } => {
                units.get(*index).and_then(|u| u.borrow().get_value(name))
            }
        }
    }

    fn set_value(&mut self, name: &str, value: Value) {
        match &mut self.0 {
            UnitKind::None => {}
            // 0032: запись идёт в контекст модели тем же путём, что присваивание
            // в теле блока. Shared-переменные уходят по цепочке `parent` в общий
            // родительский контекст — прежний широковещательный путь через
            // собственную карту узла упразднён.
            UnitKind::Node { context, .. } => {
                if let Some(ctx) = context {
                    ctx.borrow_mut().set_value(name, value);
                }
            }
            UnitKind::Parallel { units, .. } => {
                // Запись в параллельную композицию адресуется всем ветвям; каждая
                // маршрутизирует shared-имя в ОБЩИЙ родительский контекст, поэтому
                // повторная запись идемпотентна (одно значение, один источник).
                for unit in units.iter() {
                    unit.borrow_mut().set_value(name, value.clone());
                }
            }
            UnitKind::Sequential { units, index, .. } => {
                if let Some(u) = units.get(*index) {
                    u.borrow_mut().set_value(name, value);
                }
            }
        }
    }

    fn dump(&self) -> HashMap<String, Value> {
        match &self.0 {
            // Снимок узла — состояние его модели (и родителей) из контекста.
            UnitKind::Node { context, .. } => context
                .as_ref()
                .map(|ctx| ctx.borrow().dump())
                .unwrap_or_default(),
            // Композиты снимаются рекурсивно по детям (см. `state_io::snapshot`),
            // собственного состояния у них нет.
            _ => HashMap::new(),
        }
    }
}

impl Unit {
    pub fn tick(&mut self) -> TickResult {
        if let Err(diagnostic) = self.enter_initial_state() {
            return TickResult::Failed(describe(&diagnostic));
        }
        // 0044: инварианты (Guard-формулы) проверяются ДО `always` — как в
        // порождённом C (`assert()` до `switch`/`always`). Нарушение
        // останавливает прогон (`Failed`), совпадая с `assert()` → `abort()`;
        // ошибка вычисления самого условия ≠ нарушению (R15). Для композитов
        // проверяет каждый дочерний `Node` в своём `tick`.
        if matches!(self.0, UnitKind::Node { .. })
            && let Some(failed) = self.check_guards()
        {
            return failed;
        }
        if let Err(diagnostic) = self.execution("always") {
            return TickResult::Failed(describe(&diagnostic));
        }
        // Диспетчеризация по форме без удержания заимствования `self.0`: ветвь
        // вызывает методы, которым нужен `&mut self` (`match &self.0 { … =>
        // self.tick_node() }` дал бы конфликт заимствований).
        if matches!(self.0, UnitKind::None) {
            return TickResult::Terminated;
        }
        if matches!(self.0, UnitKind::Node { .. }) {
            return self.tick_node();
        }
        if matches!(self.0, UnitKind::Parallel { .. }) {
            return self.tick_parallel();
        }
        self.tick_sequential()
    }

    /// Проверяет инварианты модели и текущего состояния (фича 0044). Возвращает
    /// `Some(Failed)` при нарушении или ошибке вычисления, `None` если все
    /// обязательства выполнены. Различает нарушение (SIM-025) и ошибку самого
    /// условия (существующий `SIM-0xx`) — как переходы в `tick_node` (R15).
    fn check_guards(&mut self) -> Option<TickResult> {
        let guards: Vec<Guard> = if let UnitKind::Node { guards, state, .. } = &self.0 {
            let mut all = guards.model.clone();
            if let Some(s) = state
                && let Some(sg) = guards.per_state.get(s)
            {
                all.extend(sg.clone());
            }
            all
        } else {
            return None;
        };
        for (pred, name) in &guards {
            match pred.evaluate(self) {
                Ok(true) => {}
                Ok(false) => {
                    let named = name.as_ref().map(|n| format!(" '{n}'")).unwrap_or_default();
                    return Some(TickResult::Failed(format!(
                        "нарушен инвариант{named} (SIM-025)"
                    )));
                }
                Err(diagnostic) => return Some(TickResult::Failed(describe(&diagnostic))),
            }
        }
        None
    }

    fn tick_node(&mut self) -> TickResult {
        // Шаг 1: клонируем имя текущего состояния
        let state_name: String = if let UnitKind::Node { state: Some(s), .. } = &self.0 {
            s.clone()
        } else {
            // state: None — узел не инициализирован или завершён
            return TickResult::Terminated;
        };

        // Шаг 2: клонируем список переходов (Rc-предикаты)
        let transitions: Vec<(String, Predicate)> = if let UnitKind::Node {
            state_transitions,
            ..
        } = &self.0
        {
            state_transitions
                .get(&state_name)
                .cloned()
                .unwrap_or_default()
        } else {
            unreachable!()
        };

        if transitions.is_empty() {
            return TickResult::Terminated;
        }

        // Шаг 3: ищем первый сработавший переход.
        //
        // R5: ошибка вычисления условия — **не** «условие ложно». Раньше
        // `create_predicate` сводил `Err` и невычислимый результат к `false`, и
        // отличить сломанную модель от честно неактивного перехода было нельзя.
        let mut fired = None;
        for (name, pred) in &transitions {
            match pred.evaluate(self) {
                Ok(true) => {
                    fired = Some((name.clone(), pred.name.clone()));
                    break;
                }
                Ok(false) => {}
                Err(diagnostic) => return TickResult::Failed(describe(&diagnostic)),
            }
        }

        if let UnitKind::Node {
            last_transition, ..
        } = &mut self.0
        {
            *last_transition = None;
        }

        if let Some((next, pred_name)) = fired {
            // Шаг 4: исполнители выхода из текущего состояния
            let exit_fns: Vec<Execution> = if let UnitKind::Node {
                state_executions, ..
            } = &self.0
            {
                state_executions
                    .get(&state_name)
                    .and_then(|m| m.get("exit"))
                    .cloned()
                    .unwrap_or_default()
            } else {
                unreachable!()
            };
            for f in &exit_fns {
                if let Err(diagnostic) = f(self) {
                    return TickResult::Failed(describe(&diagnostic));
                }
            }

            // Шаг 5: исполнители входа в следующее состояние
            let enter_fns: Vec<Execution> = if let UnitKind::Node {
                state_executions, ..
            } = &self.0
            {
                state_executions
                    .get(&next)
                    .and_then(|m| m.get("enter"))
                    .cloned()
                    .unwrap_or_default()
            } else {
                unreachable!()
            };
            for f in &enter_fns {
                if let Err(diagnostic) = f(self) {
                    return TickResult::Failed(describe(&diagnostic));
                }
            }

            // Шаг 6: переход в новое состояние + запись последнего перехода
            if let UnitKind::Node {
                state,
                last_transition,
                ..
            } = &mut self.0
            {
                last_transition.replace((state_name, next.clone(), pred_name));
                *state = Some(next);
            }
        }

        TickResult::Processing
    }

    /// Извлекает и сбрасывает последний сработавший переход: (из, в, имя_предиката).
    /// Для составных Unit (Parallel/Sequential) рекурсивно собирает из всех дочерних.
    pub fn take_last_transition(&mut self) -> Option<(String, String, String)> {
        self.take_last_transitions().into_iter().next()
    }

    /// Рекурсивно извлекает все сработавшие переходы из этого узла и его потомков.
    pub fn take_last_transitions(&mut self) -> Vec<(String, String, String)> {
        match &mut self.0 {
            UnitKind::Node {
                last_transition, ..
            } => last_transition.take().map(|t| vec![t]).unwrap_or_default(),
            UnitKind::Parallel { units, .. } | UnitKind::Sequential { units, .. } => units
                .iter()
                .flat_map(|u| u.borrow_mut().take_last_transitions())
                .collect(),
            UnitKind::None => vec![],
        }
    }

    /// Возвращает имена состояний, достижимых из активных за один переход.
    pub fn reachable_from_active(&self) -> Vec<String> {
        match &self.0 {
            UnitKind::Node {
                state,
                state_transitions,
                ..
            } => {
                let current = match state {
                    Some(s) => s,
                    None => return vec![],
                };
                state_transitions
                    .get(current)
                    .map(|ts| ts.iter().map(|(to, _)| to.clone()).collect())
                    .unwrap_or_default()
            }
            UnitKind::Parallel { units, .. } | UnitKind::Sequential { units, .. } => units
                .iter()
                .flat_map(|u| u.borrow().reachable_from_active())
                .collect(),
            UnitKind::None => vec![],
        }
    }

    fn tick_parallel(&mut self) -> TickResult {
        // Тикаем ВСЕ дочерние и собираем результаты — нельзя прерываться раньше
        let results: Vec<TickResult> = if let UnitKind::Parallel { units, .. } = &self.0 {
            units.iter().map(|u| u.borrow_mut().tick()).collect()
        } else {
            unreachable!()
        };
        // Ошибка любого из параллельных детей делает шаг недостоверным (R5).
        if let Some(failed) = results
            .iter()
            .find(|r| matches!(r, TickResult::Failed(_)))
            .cloned()
        {
            return failed;
        }
        if results.iter().all(|r| *r == TickResult::Terminated) {
            TickResult::Terminated
        } else {
            TickResult::Processing
        }
    }

    fn tick_sequential(&mut self) -> TickResult {
        let (index, len) = if let UnitKind::Sequential { units, index, .. } = &self.0 {
            (*index, units.len())
        } else {
            unreachable!()
        };
        if index >= len {
            return TickResult::Terminated;
        }
        let child_result = if let UnitKind::Sequential { units, index, .. } = &self.0 {
            units[*index].borrow_mut().tick()
        } else {
            unreachable!()
        };
        match child_result {
            TickResult::Processing => TickResult::Processing,
            // Ошибка ребёнка — ошибка всей последовательности (R5).
            failed @ TickResult::Failed(_) => failed,
            TickResult::Terminated => {
                if let UnitKind::Sequential { index, .. } = &mut self.0 {
                    *index += 1;
                }
                TickResult::Processing
            }
        }
    }

    /// Д5: исполняет `enter` стартового состояния — ровно один раз, до первого
    /// `always` и до проверки переходов.
    ///
    /// Для `Parallel`/`Sequential` вызывать не нужно: их дети получают вызов
    /// через собственный [`Unit::tick`].
    fn enter_initial_state(&mut self) -> Result<(), Diagnostic> {
        let state_name = match &mut self.0 {
            UnitKind::Node {
                entered_initial: true,
                ..
            } => return Ok(()),
            UnitKind::Node {
                entered_initial,
                state,
                ..
            } => {
                *entered_initial = true;
                match state {
                    Some(name) => name.clone(),
                    None => return Ok(()),
                }
            }
            UnitKind::Parallel { .. } | UnitKind::Sequential { .. } | UnitKind::None => {
                return Ok(());
            }
        };
        let enter_fns: Vec<Execution> = match &self.0 {
            UnitKind::Node {
                state_executions, ..
            } => state_executions
                .get(&state_name)
                .and_then(|m| m.get("enter"))
                .cloned()
                .unwrap_or_default(),
            UnitKind::Parallel { .. } | UnitKind::Sequential { .. } | UnitKind::None => vec![],
        };
        for f in &enter_fns {
            f(self)?;
        }
        Ok(())
    }

    pub fn execution(&mut self, name: &str) -> Result<(), Diagnostic> {
        // Шаг 1: клонируем Rc-ссылки на функции уровня unit, не удерживая заимствование self
        let unit_fns: Vec<Execution> = match &self.0 {
            UnitKind::Node { executions, .. } => executions.get(name).cloned().unwrap_or_default(),
            UnitKind::Parallel { executions, .. } | UnitKind::Sequential { executions, .. } => {
                executions.get(name).cloned().unwrap_or_default()
            }
            UnitKind::None => vec![],
        };
        // Шаг 2: вызываем — self свободен от заимствования
        for f in &unit_fns {
            f(self)?;
        }
        // Шаг 3: для Node — функции уровня текущего состояния
        let state_fns: Vec<Execution> = match &self.0 {
            UnitKind::Node {
                state: Some(s),
                state_executions,
                ..
            } => state_executions
                .get(s.as_str())
                .and_then(|m| m.get(name))
                .cloned()
                .unwrap_or_default(),
            _ => vec![],
        };
        for f in &state_fns {
            f(self)?;
        }
        // Шаг 4: рекурсия в дочерние — ошибка ребёнка поднимается наверх (R5).
        match &self.0 {
            UnitKind::Parallel { units, .. } => {
                let units = units.clone();
                for u in units.iter() {
                    u.borrow_mut().execution(name)?;
                }
            }
            UnitKind::Sequential { units, index, .. } => {
                if *index < units.len() {
                    units[*index].clone().borrow_mut().execution(name)?;
                }
            }
            UnitKind::Node { .. } | UnitKind::None => {}
        }
        Ok(())
    }

    /// Читает значение переменной или порта — **публичная точка наблюдения**.
    ///
    /// Нужна, чтобы тесты и внешние инструменты могли сверять **вычисленные
    /// значения**, а не только факт перехода. Отсутствие такого слоя и позволило
    /// восьми дефектам фичи 0025 прожить при зелёных тестах: проверялись
    /// переходы, а значения — нет.
    ///
    /// Читает по той же цепочке, что и вычислитель ([`Context::get_value`]):
    /// сначала собственные переменные юнита, затем контекст модели.
    pub fn variable(&self, name: &str) -> Option<Value> {
        self.get_value(name)
    }

    /// Возвращает имя текущего активного состояния (только для Unit::Node).
    pub fn current_state(&self) -> Option<&str> {
        match &self.0 {
            UnitKind::Node { state, .. } => state.as_deref(),
            _ => None,
        }
    }

    /// Рекурсивно собирает имена всех активных состояний по дереву Unit.
    ///
    /// Для Sequential возвращает состояние текущего активного дочернего Unit.
    /// Для Parallel — состояния всех дочерних Units.
    pub fn active_states(&self) -> Vec<String> {
        let mut out = Vec::new();
        collect_active_states(self, &mut out);
        out
    }

    pub fn is_terminal(&self) -> bool {
        match &self.0 {
            UnitKind::None => true,
            UnitKind::Node {
                state: current_state,
                state_transitions,
                ..
            } => {
                let Some(state_name) = current_state else {
                    // Нет активного состояния — терминально если нет возможных переходов
                    return state_transitions.is_empty();
                };
                // FIXME: состояния с вложенными моделями (без явных переходов, но с continuation)
                //        будут ошибочно считаться терминальными.
                state_transitions
                    .get(state_name)
                    .is_none_or(|t| t.is_empty())
            }
            UnitKind::Parallel { units, .. } | UnitKind::Sequential { units, .. } => {
                units.iter().all(|u| u.borrow().is_terminal())
            }
        }
    }

    pub fn union(&self, other: &Self) -> Self {
        match self.clone().0 {
            UnitKind::None => other.clone(),
            UnitKind::Node { .. } => self.union_parallel(other),
            UnitKind::Parallel {
                mut units,
                mut executions,
            } => {
                if let UnitKind::Parallel {
                    units: other_units,
                    executions: other_executions,
                    ..
                } = &other.0
                {
                    units.append(&mut other_units.clone());
                    for (k, v) in other_executions.clone() {
                        executions.entry(k).or_default().extend(v);
                    }
                    Unit(UnitKind::Parallel { units, executions })
                } else {
                    units.push(Rc::new(RefCell::new(other.clone())));
                    Unit(UnitKind::Parallel { units, executions })
                }
            }
            UnitKind::Sequential { .. } => self.union_parallel(other),
        }
    }

    fn union_parallel(&self, other: &Unit) -> Unit {
        if let UnitKind::Parallel {
            units: other_units,
            executions,
            ..
        } = &other.0
        {
            let mut units = other_units.clone();
            units.insert(0, Rc::new(RefCell::new(self.clone())));
            Unit(UnitKind::Parallel {
                units,
                executions: executions.clone(),
            })
        } else {
            Unit(UnitKind::Parallel {
                units: vec![
                    Rc::new(RefCell::new(self.clone())),
                    Rc::new(RefCell::new(other.clone())),
                ],
                executions: HashMap::new(),
            })
        }
    }

    pub fn add(&self, other: &Self) -> Self {
        match self.clone().0 {
            UnitKind::None => other.clone(),
            UnitKind::Node { .. } => {
                let (mut units, executions) =
                    if let UnitKind::Sequential {
                        units, executions, ..
                    } = other.clone().0
                    {
                        (units, executions)
                    } else {
                        (vec![Rc::new(RefCell::new(other.clone()))], HashMap::new())
                    };
                units.insert(0, Rc::new(RefCell::new(self.clone())));
                Unit(UnitKind::Sequential {
                    units,
                    index: 0,
                    executions,
                })
            }
            UnitKind::Parallel { .. } => {
                let units = vec![
                    Rc::new(RefCell::new(self.clone())),
                    Rc::new(RefCell::new(other.clone())),
                ];
                Unit(UnitKind::Sequential {
                    units,
                    index: 0,
                    executions: HashMap::new(),
                })
            }
            UnitKind::Sequential {
                mut units,
                mut executions,
                ..
            } => {
                if let UnitKind::Sequential {
                    units: mut other_units,
                    executions: other_executions,
                    ..
                } = other.clone().0
                {
                    units.append(&mut other_units);
                    other_executions.into_iter().for_each(|(k, v)| {
                        executions.entry(k).or_default().extend(v);
                    });
                } else {
                    units.push(Rc::new(RefCell::new(other.clone())));
                }
                Unit(UnitKind::Sequential {
                    units,
                    index: 0,
                    executions,
                })
            }
        }
    }
}

fn collect_active_states(unit: &Unit, out: &mut Vec<String>) {
    match &unit.0 {
        UnitKind::None => {}
        UnitKind::Node { state, .. } => {
            if let Some(s) = state {
                out.push(s.clone());
            }
        }
        UnitKind::Parallel { units, .. } => {
            for u in units {
                collect_active_states(&u.borrow(), out);
            }
        }
        UnitKind::Sequential { units, index, .. } => {
            if let Some(u) = units.get(*index) {
                collect_active_states(&u.borrow(), out);
            }
        }
    }
}

#[cfg(test)]
mod tests;
