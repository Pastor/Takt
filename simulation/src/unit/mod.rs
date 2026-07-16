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

#[derive(Clone, Default)]
pub enum Unit {
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

impl Context for Unit {
    fn get_value(&self, name: &str) -> Option<Value> {
        match self {
            Unit::None => None,
            // 0032: единственный источник истины — контекст модели. Собственной
            // карты значений у узла больше нет; читаем оттуда же, куда пишем.
            Unit::Node { context, .. } => context
                .as_ref()
                .and_then(|ctx| ctx.borrow().get_value(name)),
            Unit::Parallel { units, .. } => {
                units.iter().find_map(|unit| unit.borrow().get_value(name))
            }
            Unit::Sequential { units, index, .. } => {
                units.get(*index).and_then(|u| u.borrow().get_value(name))
            }
        }
    }

    fn set_value(&mut self, name: &str, value: Value) {
        match self {
            Unit::None => {}
            // 0032: запись идёт в контекст модели тем же путём, что присваивание
            // в теле блока. Shared-переменные уходят по цепочке `parent` в общий
            // родительский контекст — прежний широковещательный путь через
            // собственную карту узла упразднён.
            Unit::Node { context, .. } => {
                if let Some(ctx) = context {
                    ctx.borrow_mut().set_value(name, value);
                }
            }
            Unit::Parallel { units, .. } => {
                // Запись в параллельную композицию адресуется всем ветвям; каждая
                // маршрутизирует shared-имя в ОБЩИЙ родительский контекст, поэтому
                // повторная запись идемпотентна (одно значение, один источник).
                for unit in units.iter() {
                    unit.borrow_mut().set_value(name, value.clone());
                }
            }
            Unit::Sequential { units, index, .. } => {
                if let Some(u) = units.get(*index) {
                    u.borrow_mut().set_value(name, value);
                }
            }
        }
    }

    fn dump(&self) -> HashMap<String, Value> {
        match self {
            // Снимок узла — состояние его модели (и родителей) из контекста.
            Unit::Node { context, .. } => context
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
        if let Unit::Node { .. } = self
            && let Some(failed) = self.check_guards()
        {
            return failed;
        }
        if let Err(diagnostic) = self.execution("always") {
            return TickResult::Failed(describe(&diagnostic));
        }
        match self {
            Unit::None => TickResult::Terminated,
            Unit::Node { .. } => self.tick_node(),
            Unit::Parallel { .. } => self.tick_parallel(),
            Unit::Sequential { .. } => self.tick_sequential(),
        }
    }

    /// Проверяет инварианты модели и текущего состояния (фича 0044). Возвращает
    /// `Some(Failed)` при нарушении или ошибке вычисления, `None` если все
    /// обязательства выполнены. Различает нарушение (SIM-025) и ошибку самого
    /// условия (существующий `SIM-0xx`) — как переходы в `tick_node` (R15).
    fn check_guards(&mut self) -> Option<TickResult> {
        let guards: Vec<Guard> = if let Unit::Node { guards, state, .. } = self {
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
        let state_name: String = if let Unit::Node { state: Some(s), .. } = self {
            s.clone()
        } else {
            // state: None — узел не инициализирован или завершён
            return TickResult::Terminated;
        };

        // Шаг 2: клонируем список переходов (Rc-предикаты)
        let transitions: Vec<(String, Predicate)> = if let Unit::Node {
            state_transitions, ..
        } = self
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

        if let Unit::Node {
            last_transition, ..
        } = self
        {
            *last_transition = None;
        }

        if let Some((next, pred_name)) = fired {
            // Шаг 4: исполнители выхода из текущего состояния
            let exit_fns: Vec<Execution> = if let Unit::Node {
                state_executions, ..
            } = self
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
            let enter_fns: Vec<Execution> = if let Unit::Node {
                state_executions, ..
            } = self
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
            if let Unit::Node {
                state,
                last_transition,
                ..
            } = self
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
        match self {
            Unit::Node {
                last_transition, ..
            } => last_transition.take().map(|t| vec![t]).unwrap_or_default(),
            Unit::Parallel { units, .. } | Unit::Sequential { units, .. } => units
                .iter()
                .flat_map(|u| u.borrow_mut().take_last_transitions())
                .collect(),
            Unit::None => vec![],
        }
    }

    /// Возвращает имена состояний, достижимых из активных за один переход.
    pub fn reachable_from_active(&self) -> Vec<String> {
        match self {
            Unit::Node {
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
            Unit::Parallel { units, .. } | Unit::Sequential { units, .. } => units
                .iter()
                .flat_map(|u| u.borrow().reachable_from_active())
                .collect(),
            Unit::None => vec![],
        }
    }

    fn tick_parallel(&mut self) -> TickResult {
        // Тикаем ВСЕ дочерние и собираем результаты — нельзя прерываться раньше
        let results: Vec<TickResult> = if let Unit::Parallel { units, .. } = self {
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
        let (index, len) = if let Unit::Sequential { units, index, .. } = self {
            (*index, units.len())
        } else {
            unreachable!()
        };
        if index >= len {
            return TickResult::Terminated;
        }
        let child_result = if let Unit::Sequential { units, index, .. } = self {
            units[*index].borrow_mut().tick()
        } else {
            unreachable!()
        };
        match child_result {
            TickResult::Processing => TickResult::Processing,
            // Ошибка ребёнка — ошибка всей последовательности (R5).
            failed @ TickResult::Failed(_) => failed,
            TickResult::Terminated => {
                if let Unit::Sequential { index, .. } = self {
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
        let state_name = match self {
            Unit::Node {
                entered_initial: true,
                ..
            } => return Ok(()),
            Unit::Node {
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
            Unit::Parallel { .. } | Unit::Sequential { .. } | Unit::None => return Ok(()),
        };
        let enter_fns: Vec<Execution> = match self {
            Unit::Node {
                state_executions, ..
            } => state_executions
                .get(&state_name)
                .and_then(|m| m.get("enter"))
                .cloned()
                .unwrap_or_default(),
            Unit::Parallel { .. } | Unit::Sequential { .. } | Unit::None => vec![],
        };
        for f in &enter_fns {
            f(self)?;
        }
        Ok(())
    }

    pub fn execution(&mut self, name: &str) -> Result<(), Diagnostic> {
        // Шаг 1: клонируем Rc-ссылки на функции уровня unit, не удерживая заимствование self
        let unit_fns: Vec<Execution> = match self {
            Unit::Node { executions, .. } => executions.get(name).cloned().unwrap_or_default(),
            Unit::Parallel { executions, .. } | Unit::Sequential { executions, .. } => {
                executions.get(name).cloned().unwrap_or_default()
            }
            Unit::None => vec![],
        };
        // Шаг 2: вызываем — self свободен от заимствования
        for f in &unit_fns {
            f(self)?;
        }
        // Шаг 3: для Node — функции уровня текущего состояния
        let state_fns: Vec<Execution> = match self {
            Unit::Node {
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
        match self {
            Unit::Parallel { units, .. } => {
                let units = units.clone();
                for u in units.iter() {
                    u.borrow_mut().execution(name)?;
                }
            }
            Unit::Sequential { units, index, .. } => {
                if *index < units.len() {
                    units[*index].clone().borrow_mut().execution(name)?;
                }
            }
            Unit::Node { .. } | Unit::None => {}
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
        match self {
            Unit::Node { state, .. } => state.as_deref(),
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
        match self {
            Unit::None => true,
            Unit::Node {
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
                    .map_or(true, |t| t.is_empty())
            }
            Unit::Parallel { units, .. } | Unit::Sequential { units, .. } => {
                units.iter().all(|u| u.borrow().is_terminal())
            }
        }
    }

    pub fn union(&self, other: &Self) -> Self {
        match self.clone() {
            Unit::None => other.clone(),
            Unit::Node { .. } => self.union_parallel(other),
            Unit::Parallel {
                mut units,
                mut executions,
                ..
            } => {
                if let Unit::Parallel {
                    units: other_units,
                    executions: other_executions,
                    ..
                } = other
                {
                    units.append(&mut other_units.clone());
                    for (k, v) in other_executions.clone() {
                        executions.entry(k).or_default().extend(v);
                    }
                    Unit::Parallel { units, executions }
                } else {
                    units.push(Rc::new(RefCell::new(other.clone())));
                    Unit::Parallel { units, executions }
                }
            }
            Unit::Sequential { .. } => self.union_parallel(other),
        }
    }

    fn union_parallel(&self, other: &Unit) -> Unit {
        if let Unit::Parallel {
            units: other_units,
            executions,
            ..
        } = other
        {
            let mut units = other_units.clone();
            units.insert(0, Rc::new(RefCell::new(self.clone())));
            Unit::Parallel {
                units,
                executions: executions.clone(),
            }
        } else {
            Unit::Parallel {
                units: vec![
                    Rc::new(RefCell::new(self.clone())),
                    Rc::new(RefCell::new(other.clone())),
                ],
                executions: HashMap::new(),
            }
        }
    }

    pub fn add(&self, other: &Self) -> Self {
        match self.clone() {
            Unit::None => other.clone(),
            Unit::Node { .. } => {
                let (mut units, executions) = if let Unit::Sequential {
                    units, executions, ..
                } = other.clone()
                {
                    (units, executions)
                } else {
                    (vec![Rc::new(RefCell::new(other.clone()))], HashMap::new())
                };
                units.insert(0, Rc::new(RefCell::new(self.clone())));
                Unit::Sequential {
                    units,
                    index: 0,
                    executions,
                }
            }
            Unit::Parallel { .. } => {
                let units = vec![
                    Rc::new(RefCell::new(self.clone())),
                    Rc::new(RefCell::new(other.clone())),
                ];
                Unit::Sequential {
                    units,
                    index: 0,
                    executions: HashMap::new(),
                }
            }
            Unit::Sequential {
                mut units,
                mut executions,
                ..
            } => {
                if let Unit::Sequential {
                    units: mut other_units,
                    executions: other_executions,
                    ..
                } = other.clone()
                {
                    units.append(&mut other_units);
                    other_executions.into_iter().for_each(|(k, v)| {
                        executions.entry(k).or_default().extend(v);
                    });
                } else {
                    units.push(Rc::new(RefCell::new(other.clone())));
                }
                Unit::Sequential {
                    units,
                    index: 0,
                    executions,
                }
            }
        }
    }
}

fn collect_active_states(unit: &Unit, out: &mut Vec<String>) {
    match unit {
        Unit::None => {}
        Unit::Node { state, .. } => {
            if let Some(s) = state {
                out.push(s.clone());
            }
        }
        Unit::Parallel { units, .. } => {
            for u in units {
                collect_active_states(&u.borrow(), out);
            }
        }
        Unit::Sequential { units, index, .. } => {
            if let Some(u) = units.get(*index) {
                collect_active_states(&u.borrow(), out);
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::cell::Cell;
    use std::collections::HashMap;

    struct MockCtx(HashMap<String, Value>);

    impl Context for MockCtx {
        fn get_value(&self, name: &str) -> Option<Value> {
            self.0.get(name).cloned()
        }

        fn set_value(&mut self, name: &str, value: Value) {
            self.0.insert(name.to_string(), value);
        }
    }

    fn ctx_with(key: &str, val: Value) -> Rc<RefCell<dyn Context>> {
        let mut m = HashMap::new();
        m.insert(key.to_string(), val);
        Rc::new(RefCell::new(MockCtx(m)))
    }

    fn node() -> Unit {
        Unit::Node {
            entered_initial: false,
            context: None,
            executions: HashMap::new(),
            state: None,
            state_transitions: HashMap::new(),
            state_executions: HashMap::new(),
            guards: Default::default(),
            last_transition: None,
        }
    }

    fn node_with(key: &str, val: Value) -> Unit {
        Unit::Node {
            entered_initial: false,
            context: Some(ctx_with(key, val)),
            executions: HashMap::new(),
            state: None,
            state_transitions: HashMap::new(),
            state_executions: HashMap::new(),
            guards: Default::default(),
            last_transition: None,
        }
    }

    fn parallel(units: Vec<Unit>) -> Unit {
        Unit::Parallel {
            units: units
                .into_iter()
                .map(|u| Rc::new(RefCell::new(u)))
                .collect(),
            executions: HashMap::new(),
        }
    }

    fn sequential(units: Vec<Unit>) -> Unit {
        Unit::Sequential {
            units: units
                .into_iter()
                .map(|u| Rc::new(RefCell::new(u)))
                .collect(),
            index: 0,
            executions: HashMap::new(),
        }
    }

    fn len(u: &Unit) -> usize {
        match u {
            Unit::Parallel { units, .. } | Unit::Sequential { units, .. } => units.len(),
            _ => 0,
        }
    }

    // ── union ────────────────────────────────────────────────────────────────

    // unit::union: нейтральный элемент — None | X == X.
    // Создаём None.union(node) и проверяем, что результат — сам Node, без обёртки.
    #[test]
    fn test_union_none_with_node() {
        let result = Unit::None.union(&node());
        assert!(matches!(result, Unit::Node { .. }));
    }

    // unit::union: два Node объединяются в Parallel с двумя дочерними.
    // Проверяем вариант результата и количество дочерних.
    #[test]
    fn test_union_node_with_node() {
        let result = node().union(&node());
        assert!(matches!(result, Unit::Parallel { .. }));
        assert_eq!(len(&result), 2);
    }

    // unit::union: Node | Parallel([a, b]) — self вставляется в начало существующего Parallel.
    // Ожидаем Parallel из трёх элементов: [Node, a, b].
    #[test]
    fn test_union_node_with_parallel() {
        let result = node().union(&parallel(vec![node(), node()]));
        assert!(matches!(result, Unit::Parallel { .. }));
        assert_eq!(len(&result), 3);
    }

    // unit::union: Parallel([a]) | Parallel([b, c]) — списки units объединяются в один Parallel.
    // Ожидаем три элемента: [a, b, c].
    #[test]
    fn test_union_parallel_with_parallel() {
        let result = parallel(vec![node()]).union(&parallel(vec![node(), node()]));
        assert!(matches!(result, Unit::Parallel { .. }));
        assert_eq!(len(&result), 3);
    }

    // unit::union: Parallel([a]) | Node — Node добавляется в конец существующего Parallel.
    // Ожидаем два элемента.
    #[test]
    fn test_union_parallel_with_node() {
        let result = parallel(vec![node()]).union(&node());
        assert!(matches!(result, Unit::Parallel { .. }));
        assert_eq!(len(&result), 2);
    }

    // unit::union: Sequential | Node — Sequential оборачивается в новый Parallel.
    // Ожидаем Parallel из двух элементов: [Sequential, Node].
    #[test]
    fn test_union_sequential_with_node() {
        let result = sequential(vec![node()]).union(&node());
        assert!(matches!(result, Unit::Parallel { .. }));
        assert_eq!(len(&result), 2);
    }

    // unit::union: Sequential | Parallel([a, b]) — Sequential вставляется в начало Parallel.
    // Ожидаем Parallel из трёх элементов: [Sequential, a, b].
    #[test]
    fn test_union_sequential_with_parallel() {
        let result = sequential(vec![node()]).union(&parallel(vec![node(), node()]));
        assert!(matches!(result, Unit::Parallel { .. }));
        assert_eq!(len(&result), 3);
    }

    // ── add ──────────────────────────────────────────────────────────────────

    // unit::add: нейтральный элемент — None + X == X.
    // Создаём None.add(node) и проверяем, что результат — сам Node.
    #[test]
    fn test_add_none_with_node() {
        let result = Unit::None.add(&node());
        assert!(matches!(result, Unit::Node { .. }));
    }

    // unit::add: два Node формируют Sequential из двух элементов.
    // Проверяем вариант результата и количество дочерних.
    #[test]
    fn test_add_node_with_node() {
        let result = node().add(&node());
        assert!(matches!(result, Unit::Sequential { .. }));
        assert_eq!(len(&result), 2);
    }

    // unit::add: Node + Sequential([a, b]) — выравнивание: Node вставляется в начало,
    // результат Sequential([Node, a, b]) из трёх элементов, без вложенности.
    #[test]
    fn test_add_node_with_sequential() {
        let result = node().add(&sequential(vec![node(), node()]));
        assert!(matches!(result, Unit::Sequential { .. }));
        assert_eq!(len(&result), 3);
    }

    // unit::add: Sequential([a]) + Sequential([b, c]) — конкатенация списков units.
    // Результат Sequential([a, b, c]) из трёх элементов.
    #[test]
    fn test_add_sequential_with_sequential() {
        let result = sequential(vec![node()]).add(&sequential(vec![node(), node()]));
        assert!(matches!(result, Unit::Sequential { .. }));
        assert_eq!(len(&result), 3);
    }

    // unit::add: Parallel + Node — Parallel не выравнивается, оборачивается целиком.
    // Результат Sequential([Parallel, Node]) из двух элементов.
    #[test]
    fn test_add_parallel_with_node() {
        let result = parallel(vec![node(), node()]).add(&node());
        assert!(matches!(result, Unit::Sequential { .. }));
        assert_eq!(len(&result), 2);
    }

    // ── Context ──────────────────────────────────────────────────────────────

    // get_value: Unit::None не содержит переменных — всегда возвращает None.
    #[test]
    fn test_context_none_returns_none() {
        assert!(Unit::None.get_value("x").is_none());
    }

    // get_value: Node без внешнего контекста и без variables — возвращает None.
    #[test]
    fn test_context_node_without_ctx_returns_none() {
        assert!(node().get_value("x").is_none());
    }

    // get_value: Node с внешним контекстом, содержащим переменную "x".
    // Проверяем, что значение корректно делегируется из context.
    #[test]
    fn test_context_node_with_ctx_returns_value() {
        let u = node_with("x", Value::Number(42));
        assert!(matches!(u.get_value("x"), Some(Value::Number(42))));
    }

    // get_value: контрпример — запрашиваем ключ "y", в контексте есть только "x".
    // Проверяем, что чужой ключ не возвращается.
    #[test]
    fn test_context_node_wrong_key_returns_none() {
        let u = node_with("x", Value::Number(1));
        assert!(u.get_value("y").is_none());
    }

    // get_value: Parallel ищет переменную во всех дочерних узлах.
    // Переменная "a" есть только у первого ребёнка — проверяем, что она находится.
    #[test]
    fn test_context_parallel_finds_in_children() {
        let u = parallel(vec![node_with("a", Value::Boolean(true)), node()]);
        assert!(matches!(u.get_value("a"), Some(Value::Boolean(true))));
    }

    // get_value: контрпример для Parallel — запрашиваем ключ "b", которого нет ни у кого.
    #[test]
    fn test_context_parallel_missing_returns_none() {
        let u = parallel(vec![node_with("a", Value::Number(1))]);
        assert!(u.get_value("b").is_none());
    }

    // get_value: Sequential предоставляет контекст только активного (index=0) дочернего узла.
    // Переменная "b" из узла index=1 недоступна, пока index не продвинут.
    #[test]
    fn test_context_sequential_reads_active_index() {
        let u = sequential(vec![
            node_with("a", Value::Number(10)),
            node_with("b", Value::Number(20)),
        ]);
        assert!(matches!(u.get_value("a"), Some(Value::Number(10))));
        assert!(u.get_value("b").is_none());
    }

    // ── is_terminal ──────────────────────────────────────────────────────────

    // is_terminal: Unit::None считается терминальным — это нейтральный пустой элемент.
    #[test]
    fn test_is_terminal_none() {
        assert!(Unit::None.is_terminal());
    }

    // is_terminal: Node с state: None и без переходов считается терминальным
    // (нет активного состояния + нет возможных переходов → выполнение завершено).
    #[test]
    fn test_is_terminal_node() {
        assert!(node().is_terminal());
    }

    // is_terminal: Parallel терминален только если все дочерние терминальны.
    // Оба ребёнка (node() и None) терминальны — ожидаем true.
    #[test]
    fn test_is_terminal_parallel_all_terminal() {
        assert!(parallel(vec![node(), Unit::None]).is_terminal());
    }

    // is_terminal: Sequential терминален только если все дочерние терминальны.
    // Оба node() терминальны — ожидаем true.
    #[test]
    fn test_is_terminal_sequential_all_terminal() {
        assert!(sequential(vec![node(), node()]).is_terminal());
    }

    // ── tick: вспомогательные конструкторы ───────────────────────────────────

    /// Узел в именованном терминальном состоянии (нет исходящих переходов).
    // ── R5: ошибка вычисления отличима от ложного условия (задача 0025-05) ───

    #[test]
    fn r5_eval_error_is_distinguishable_from_false_condition() {
        // Ядро требования R5. Раньше `create_predicate` сводил Err и
        // невычислимый результат к `false`, поэтому сломанная модель выглядела
        // как модель с неактивным переходом.
        let mut st = HashMap::new();
        let failing = Predicate::new("сломанное", |_| {
            Err(grammar::diagnostics::Diagnostic::error(
                grammar::diagnostics::Location::Builtin,
                "деление на ноль".to_string(),
            )
            .with_code("SIM-001"))
        });
        st.insert("A".to_string(), vec![("B".to_string(), failing)]);
        st.insert("B".to_string(), vec![]);
        let mut u = Unit::Node {
            entered_initial: true,
            context: None,
            state_transitions: st,
            state_executions: HashMap::new(),
            state: Some("A".to_string()),
            executions: HashMap::new(),
            guards: Default::default(),
            last_transition: None,
        };
        match u.tick() {
            TickResult::Failed(details) => {
                assert!(details.contains("деление на ноль"), "детали: {details}");
                assert!(
                    details.contains("SIM-001"),
                    "код обязан быть в деталях: {details}"
                );
            }
            other => panic!("ошибка вычисления обязана давать Failed, получено {other:?}"),
        }
    }

    #[test]
    fn r5_false_condition_is_not_an_error() {
        // Контрпример: честно ложное условие — это Processing, а не Failed.
        let mut st = HashMap::new();
        st.insert(
            "A".to_string(),
            vec![("B".to_string(), Predicate::new("ложное", |_| Ok(false)))],
        );
        st.insert("B".to_string(), vec![]);
        let mut u = Unit::Node {
            entered_initial: true,
            context: None,
            state_transitions: st,
            state_executions: HashMap::new(),
            state: Some("A".to_string()),
            executions: HashMap::new(),
            guards: Default::default(),
            last_transition: None,
        };
        assert_eq!(u.tick(), TickResult::Processing);
    }

    // ── Д5: enter стартового состояния (задача 0025-04) ──────────────────────

    /// Строит узел с `enter`-исполнителем, считающим вызовы.
    fn node_with_enter(counter: Rc<Cell<u32>>) -> Unit {
        let mut st = HashMap::new();
        st.insert("A".to_string(), vec![]);
        let mut execs: HashMap<String, Executions> = HashMap::new();
        let f: Execution = Rc::new(move |_ctx: &mut dyn Context| {
            counter.set(counter.get() + 1);
            Ok(Flow::Normal)
        });
        let mut m: Executions = HashMap::new();
        m.insert("enter".to_string(), vec![f]);
        execs.insert("A".to_string(), m);
        Unit::Node {
            entered_initial: false,
            context: None,
            state_transitions: st,
            state_executions: execs,
            state: Some("A".to_string()),
            executions: HashMap::new(),
            guards: Default::default(),
            last_transition: None,
        }
    }

    #[test]
    fn d5_enter_of_start_state_runs_on_first_tick() {
        // Д5: раньше `enter` вызывался только в ветке перехода, поэтому
        // начальная инициализация модели терялась.
        let counter = Rc::new(Cell::new(0));
        let mut u = node_with_enter(counter.clone());
        u.tick();
        assert_eq!(
            counter.get(),
            1,
            "enter стартового состояния обязан исполниться"
        );
    }

    #[test]
    fn d5_enter_of_start_state_runs_exactly_once() {
        let counter = Rc::new(Cell::new(0));
        let mut u = node_with_enter(counter.clone());
        u.tick();
        u.tick();
        u.tick();
        assert_eq!(counter.get(), 1, "enter обязан исполниться ровно один раз");
    }

    #[test]
    fn d5_restore_does_not_rerun_enter() {
        // Возобновление: модель уже в состоянии — входить повторно нельзя,
        // иначе enter затрёт загруженные значения.
        let counter = Rc::new(Cell::new(0));
        let mut u = node_with_enter(counter.clone());
        let snap = crate::state_io::snapshot(&u);
        crate::state_io::restore(&mut u, &snap);
        u.tick();
        assert_eq!(
            counter.get(),
            0,
            "после restore enter не должен исполняться"
        );
    }

    fn node_terminal(name: &str) -> Unit {
        let mut st = HashMap::new();
        st.insert(name.to_string(), vec![]);
        Unit::Node {
            entered_initial: false,
            context: None,
            executions: HashMap::new(),
            state: Some(name.to_string()),
            state_transitions: st,
            state_executions: HashMap::new(),
            guards: Default::default(),
            last_transition: None,
        }
    }

    /// Узел в состоянии `from` с одним переходом в `to`; предикат всегда `cond`.
    fn node_with_transition(from: &str, to: &str, cond: bool) -> Unit {
        let pred = Predicate::new("cond", move |_| Ok(cond));
        let mut st = HashMap::new();
        st.insert(from.to_string(), vec![(to.to_string(), pred)]);
        st.insert(to.to_string(), vec![]);
        Unit::Node {
            entered_initial: false,
            context: None,
            executions: HashMap::new(),
            state: Some(from.to_string()),
            state_transitions: st,
            state_executions: HashMap::new(),
            guards: Default::default(),
            last_transition: None,
        }
    }

    fn current_state(u: &Unit) -> Option<&str> {
        match u {
            Unit::Node { state, .. } => state.as_deref(),
            _ => None,
        }
    }

    // ── tick: Unit::None ─────────────────────────────────────────────────────

    // tick: Unit::None всегда возвращает Terminated — пустой узел не имеет работы.
    #[test]
    fn test_tick_none_is_terminated() {
        assert_eq!(Unit::None.tick(), TickResult::Terminated);
    }

    // ── tick: Unit::Node ─────────────────────────────────────────────────────

    // tick/Node: узел без активного состояния (state: None) считается завершённым.
    // Вызываем tick() и ожидаем Terminated — нет состояния, нечего выполнять.
    #[test]
    fn test_tick_node_no_state_is_terminated() {
        assert_eq!(node().tick(), TickResult::Terminated);
    }

    // tick/Node: узел в терминальном состоянии (переходов нет) возвращает Terminated.
    // Создаём узел в состоянии "S" без исходящих переходов и вызываем tick().
    #[test]
    fn test_tick_node_terminal_state_is_terminated() {
        let mut u = node_terminal("S");
        assert_eq!(u.tick(), TickResult::Terminated);
    }

    // tick/Node: при срабатывании перехода возвращается Processing (работа продолжается).
    // Узел A→B с предикатом true: после tick() ещё не завершён — в B ещё не были.
    #[test]
    fn test_tick_node_active_transition_returns_processing() {
        let mut u = node_with_transition("A", "B", true);
        assert_eq!(u.tick(), TickResult::Processing);
    }

    // tick/Node: срабатывание перехода обновляет поле state.
    // После tick() состояние должно смениться с "A" на "B".
    #[test]
    fn test_tick_node_active_transition_updates_state() {
        let mut u = node_with_transition("A", "B", true);
        u.tick();
        assert_eq!(current_state(&u), Some("B"));
    }

    // tick/Node: если предикат не выполнен, состояние не меняется.
    // Контрпример: узел A→B с предикатом false — после tick() остаётся в "A".
    #[test]
    fn test_tick_node_inactive_predicate_stays_in_same_state() {
        let mut u = node_with_transition("A", "B", false);
        u.tick();
        assert_eq!(current_state(&u), Some("A"));
    }

    // tick/Node: если предикат не выполнен, tick() возвращает Processing.
    // Узел остаётся в активном состоянии с незавершёнными переходами.
    #[test]
    fn test_tick_node_inactive_predicate_returns_processing() {
        let mut u = node_with_transition("A", "B", false);
        assert_eq!(u.tick(), TickResult::Processing);
    }

    // tick/Node: детерминизм — при нескольких срабатывающих переходах берётся только первый.
    // Из состояния "A" есть переходы A→B и A→C, оба с предикатом true.
    // После tick() состояние должно быть "B" (первый по порядку), а не "C".
    #[test]
    fn test_tick_node_only_first_matching_transition_taken() {
        let pred = Predicate::new("always", |_| Ok(true));
        let mut st = HashMap::new();
        st.insert(
            "A".to_string(),
            vec![
                ("B".to_string(), pred.clone()),
                ("C".to_string(), pred.clone()),
            ],
        );
        st.insert("B".to_string(), vec![]);
        st.insert("C".to_string(), vec![]);
        let mut u = Unit::Node {
            entered_initial: false,
            context: None,
            executions: HashMap::new(),
            state: Some("A".to_string()),
            state_transitions: st,
            state_executions: HashMap::new(),
            guards: Default::default(),
            last_transition: None,
        };
        u.tick();
        assert_eq!(current_state(&u), Some("B"));
    }

    // tick/Node: полный цикл — переход в терминальное состояние и обнаружение завершения.
    // tick 1: A→B (Processing); tick 2: B терминальный (Terminated).
    #[test]
    fn test_tick_node_reaches_terminal_after_transition() {
        let mut u = node_with_transition("A", "B", true);
        u.tick();
        assert_eq!(u.tick(), TickResult::Terminated);
    }

    // ── tick: Unit::Parallel ─────────────────────────────────────────────────

    // tick/Parallel: все дочерние терминальны — Parallel возвращает Terminated.
    // node() (state: None) и Unit::None оба терминальны.
    #[test]
    fn test_tick_parallel_all_terminated() {
        let mut u = parallel(vec![node(), Unit::None]);
        assert_eq!(u.tick(), TickResult::Terminated);
    }

    // tick/Parallel: хотя бы один дочерний не завершён — Parallel возвращает Processing.
    // Unit::None терминален, но второй узел ещё в процессе перехода.
    #[test]
    fn test_tick_parallel_one_active_returns_processing() {
        let mut u = parallel(vec![Unit::None, node_with_transition("A", "B", true)]);
        assert_eq!(u.tick(), TickResult::Processing);
    }

    // tick/Parallel: все дочерние тикаются за один вызов, без раннего прерывания.
    // Оба узла должны совершить переход за один tick(), даже если первый уже «готов».
    // Проверяем состояния обоих дочерних после одного tick().
    #[test]
    fn test_tick_parallel_all_units_are_ticked() {
        let mut u = parallel(vec![
            node_with_transition("A", "B", true),
            node_with_transition("X", "Y", true),
        ]);
        u.tick();
        let Unit::Parallel { units, .. } = &u else {
            panic!("ожидался Parallel")
        };
        assert_eq!(current_state(&units[0].borrow()), Some("B"));
        assert_eq!(current_state(&units[1].borrow()), Some("Y"));
    }

    // ── tick: Unit::Sequential ───────────────────────────────────────────────

    // tick/Sequential: пустой список дочерних — сразу Terminated.
    #[test]
    fn test_tick_sequential_empty_is_terminated() {
        let mut u = sequential(vec![]);
        assert_eq!(u.tick(), TickResult::Terminated);
    }

    // tick/Sequential: последовательное продвижение через дочерние.
    // node() с state: None завершается немедленно, поэтому каждый tick продвигает index.
    // При двух дочерних нужно три тика: два для продвижения index и один для Terminated.
    #[test]
    fn test_tick_sequential_advances_through_children() {
        let mut u = sequential(vec![node(), node()]);
        assert_eq!(u.tick(), TickResult::Processing); // child[0] завершился → index 0→1
        assert_eq!(u.tick(), TickResult::Processing); // child[1] завершился → index 1→2
        assert_eq!(u.tick(), TickResult::Terminated); // index ≥ len
    }

    // tick/Sequential: ожидание активного дочернего перед продвижением к следующему.
    // Первый ребёнок проходит A→B (два тика), затем завершается; второй завершается сразу.
    // Итого четыре тика до Terminated Sequential.
    #[test]
    fn test_tick_sequential_waits_for_child() {
        let mut u = sequential(vec![node_with_transition("A", "B", true), node()]);
        assert_eq!(u.tick(), TickResult::Processing); // child[0]: A→B, Processing
        assert_eq!(u.tick(), TickResult::Processing); // child[0]: B терминальный → index 0→1
        assert_eq!(u.tick(), TickResult::Processing); // child[1]: Terminated → index 1→2
        assert_eq!(u.tick(), TickResult::Terminated); // index ≥ len
    }
}
