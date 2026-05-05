use crate::context::Context;
use crate::predicate::create_predicate;

pub(crate) type Predicate = Rc<dyn Fn(&dyn Context) -> bool>;
pub(crate) type Execution = Box<dyn Fn(&mut dyn Context)>;

use crate::value::Value;
use grammar::diagnostics::{Diagnostic, Location};
use grammar::semantic::extend::Extend;
use grammar::semantic::{ConditionNode, ModelNode, ReferenceNode, StateNode, StateNodeKind};
use std::cell::RefCell;
use std::collections::HashMap;
use std::rc::Rc;

#[derive(Debug, PartialEq, Eq)]
enum Transition {
    Processing,
    Goto,
    Final,
}

/// Unit — исполняемая единица симуляции.
///
/// Варианты:
/// - [`Plain`](Unit::Plain) — конечный автомат; `states.is_empty()` означает
///   листовое состояние (tick возвращает Goto без делегирования);
/// - [`Parallel`](Unit::Parallel) — параллельная композиция;
/// - [`Sequence`](Unit::Sequence) — последовательная композиция.
///
/// При построении из семантической модели все вхождения одного ModelNode
/// превращаются в независимые копии, чтобы параллельные и повторные
/// использования не делили общее состояние симуляции.
pub(crate) enum Unit {
    Plain {
        upper: Option<Rc<RefCell<Unit>>>,
        /// Активные переходы для текущего состояния.
        transitions: Vec<(String, Predicate)>,
        /// Переходы каждого состояния: имя → [(цель, предикат)].
        /// При смене состояния `transitions` обновляется из этой таблицы.
        state_transitions: HashMap<String, Vec<(String, Predicate)>>,
        /// Безусловный next-переход для состояний с расширением: имя → цель.
        /// Срабатывает когда дочерний Unit вернул Final.
        state_nexts: HashMap<String, String>,
        states: HashMap<String, RefCell<Unit>>,
        current: String,

        always: Vec<Execution>,
        enters: Vec<Execution>,
        exits: Vec<Execution>,

        variables: HashMap<String, Value>,
    },
    /// Параллельная композиция: все дочерние Unit тикаются одновременно.
    Parallel {
        upper: Option<Rc<RefCell<Unit>>>,
        units: Vec<Rc<RefCell<Unit>>>,
        next: Option<String>,
    },
    /// Последовательная композиция: дочерние Unit выполняются по очереди.
    Sequence {
        upper: Option<Rc<RefCell<Unit>>>,
        units: Vec<Rc<RefCell<Unit>>>,
        index: usize,
        next: Option<String>,
    },
}

// ── Построение Unit из семантической модели ──────────────────────────────────

/// Строит [`Unit`] из корневой семантической модели.
///
/// Каждое вхождение модели в композиции (`+` / `|`) порождает независимую
/// копию, поэтому одна и та же модель может использоваться многократно
/// без пересечения состояний симуляции.
pub(crate) fn from_model(model: Rc<RefCell<ModelNode>>) -> Result<Unit, Diagnostic> {
    let borrowed = model.borrow();
    if borrowed.has_states() {
        from_model_plain(&borrowed)
    } else {
        from_extend(&borrowed.implements, None)
    }
}

fn from_model_plain(model: &ModelNode) -> Result<Unit, Diagnostic> {
    let start_name = model
        .states
        .iter()
        .find_map(|(name, state)| match state {
            StateNode::Simple {
                kind: StateNodeKind::Start,
                ..
            }
            | StateNode::Implement {
                kind: StateNodeKind::Start,
                ..
            } => Some(name.clone()),
            _ => None,
        })
        .ok_or_else(|| {
            Diagnostic::error(
                Location::Builtin,
                "Нет начального состояния в модели".to_string(),
            )
        })?;

    let mut states: HashMap<String, RefCell<Unit>> = HashMap::new();
    let mut state_transitions: HashMap<String, Vec<(String, Predicate)>> = HashMap::new();
    let mut state_nexts: HashMap<String, String> = HashMap::new();

    for (name, state_node) in &model.states {
        let unit = from_state_node(state_node)?;
        states.insert(name.clone(), RefCell::new(unit));

        let trans: Vec<(String, Predicate)> = state_node
            .references()
            .iter()
            .map(|r| {
                let pred: Predicate = if matches!(r.cond, ConditionNode::None) {
                    Rc::new(|_: &dyn Context| true)
                } else {
                    create_predicate(&r.cond)
                };
                (r.name.clone(), pred)
            })
            .collect();
        state_transitions.insert(name.clone(), trans);

        if let StateNode::Implement {
            next: Some(next_ref),
            ..
        } = state_node
        {
            state_nexts.insert(name.clone(), next_ref.name.clone());
        }
    }

    let transitions = make_transitions(&state_transitions, &start_name);

    Ok(Unit::Plain {
        upper: None,
        transitions,
        state_transitions,
        state_nexts,
        states,
        current: start_name,
        always: vec![],
        enters: vec![],
        exits: vec![],
        variables: HashMap::new(),
    })
}

fn from_state_node(state: &StateNode) -> Result<Unit, Diagnostic> {
    match state {
        StateNode::Unresolved => Err(Diagnostic::error(
            Location::Builtin,
            "Неразрешённое состояние".to_string(),
        )),
        // Простое состояние = листовой Plain (states пуст → tick вернёт Goto)
        StateNode::Simple { .. } => Ok(leaf_plain()),
        StateNode::Implement {
            implements, next, ..
        } => from_extend(implements, next.clone()),
    }
}

fn from_extend(
    extend: &Extend,
    next: Option<ReferenceNode<StateNode>>,
) -> Result<Unit, Diagnostic> {
    match extend {
        Extend::None | Extend::Unresolved(_) => Ok(leaf_plain()),
        Extend::Model(model) => {
            let borrowed = model.borrow();
            if borrowed.has_states() {
                from_model_plain(&borrowed)
            } else {
                from_extend(&borrowed.implements, None)
            }
        }
        Extend::Parentless(inner) => from_extend(inner, None),
        Extend::Concatenation(items) => {
            let units: Vec<Rc<RefCell<Unit>>> = items
                .iter()
                .map(|item| Ok(Rc::new(RefCell::new(from_extend(item, None)?))))
                .collect::<Result<_, Diagnostic>>()?;
            Ok(Unit::Sequence {
                upper: None,
                units,
                index: 0,
                next: next.map(|rf| rf.name.clone()),
            })
        }
        Extend::Parallel(items) => {
            let units: Vec<Rc<RefCell<Unit>>> = items
                .iter()
                .map(|item| Ok(Rc::new(RefCell::new(from_extend(item, None)?))))
                .collect::<Result<_, Diagnostic>>()?;
            Ok(Unit::Parallel {
                upper: None,
                units,
                next: next.map(|rf| rf.name.clone()),
            })
        }
    }
}

/// Листовой Plain: пустой states → tick() вернёт Goto.
fn leaf_plain() -> Unit {
    Unit::Plain {
        upper: None,
        transitions: vec![],
        state_transitions: HashMap::new(),
        state_nexts: HashMap::new(),
        states: HashMap::new(),
        current: String::new(),
        always: vec![],
        enters: vec![],
        exits: vec![],
        variables: HashMap::new(),
    }
}

/// Клонирует предикаты заданного состояния из таблицы переходов в активный список.
fn make_transitions(
    map: &HashMap<String, Vec<(String, Predicate)>>,
    state: &str,
) -> Vec<(String, Predicate)> {
    map.get(state)
        .map(|preds| {
            preds
                .iter()
                .map(|(target, pred)| (target.clone(), Rc::clone(pred)))
                .collect()
        })
        .unwrap_or_default()
}

// ── Реализации Context, enter, exit, tick ────────────────────────────────────

impl Context for Unit {
    fn get_value(&self, name: &str) -> Option<Value> {
        let result = match self {
            Unit::Plain { variables, .. } => variables.get(name).cloned(),
            Unit::Parallel { units, .. } | Unit::Sequence { units, .. } => {
                units.iter().flat_map(|u| u.borrow().get_value(name)).next()
            }
        };
        result.or_else(|| {
            let upper = match self {
                Unit::Plain { upper, .. }
                | Unit::Parallel { upper, .. }
                | Unit::Sequence { upper, .. } => upper.as_ref().map(|u| &**u),
            };
            upper.and_then(|u| u.borrow().get_value(name))
        })
    }
}

impl Unit {
    fn enter(&mut self) {
        let enters = match self {
            Unit::Plain { enters, .. } => std::mem::take(enters),
            Unit::Parallel { units, .. } => {
                units.iter().for_each(|u| u.borrow_mut().enter());
                return;
            }
            Unit::Sequence { units, index, .. } => {
                units.get(*index).unwrap().borrow_mut().enter();
                return;
            }
        };
        for f in &enters {
            f(self as &mut dyn Context);
        }
        match self {
            Unit::Plain { enters: e, .. } => *e = enters,
            Unit::Parallel { .. } | Unit::Sequence { .. } => unreachable!(),
        }
    }

    fn exit(&mut self) {
        let exits = match self {
            Unit::Plain { exits, .. } => std::mem::take(exits),
            Unit::Parallel { units, .. } => {
                units.iter().for_each(|u| u.borrow_mut().exit());
                return;
            }
            Unit::Sequence { units, index, .. } => {
                units.get(*index).unwrap().borrow_mut().exit();
                return;
            }
        };
        for f in &exits {
            f(self as &mut dyn Context);
        }
        match self {
            Unit::Plain { exits: e, .. } => *e = exits,
            Unit::Parallel { .. } | Unit::Sequence { .. } => unreachable!(),
        }
    }

    fn tick(&mut self) -> Result<Transition, Diagnostic> {
        if let Unit::Parallel { units, .. } = self {
            // Тикаем все ветви; собираем результаты чтобы не прерывать на первом несовпадении
            let results: Vec<Transition> = units
                .iter()
                .map(|u| u.borrow_mut().tick())
                .collect::<Result<_, _>>()?;
            if results
                .iter()
                .all(|r| matches!(r, Transition::Final | Transition::Goto))
            {
                return Ok(Transition::Final);
            }
            return Ok(Transition::Processing);
        } else if let Unit::Sequence { units, index, .. } = self {
            let result = units[*index].borrow_mut().tick()?;
            // Goto — элемент завершился (листовое состояние); Final — FSM исчерпал состояния
            if matches!(result, Transition::Goto | Transition::Final) {
                if *index + 1 >= units.len() {
                    return Ok(Transition::Final);
                }
                units[*index].borrow_mut().exit();
                *index += 1;
                units[*index].borrow_mut().enter();
                return Ok(Transition::Processing);
            }
            return Ok(Transition::Processing);
        }

        // Временно забираем always, вызываем и возвращаем обратно
        let always = match self {
            Unit::Plain { always, .. } => std::mem::take(always),
            Unit::Parallel { .. } | Unit::Sequence { .. } => unreachable!(),
        };
        for f in &always {
            f(self as &mut dyn Context);
        }
        match self {
            Unit::Plain { always: a, .. } => *a = always,
            Unit::Parallel { .. } | Unit::Sequence { .. } => unreachable!(),
        }

        // Тикаем текущее вложенное состояние
        let result = {
            let Unit::Plain {
                current, states, ..
            } = &mut *self
            else {
                unreachable!()
            };
            // Пустой states → листовое состояние: сигнализируем Goto
            if states.is_empty() {
                return Ok(Transition::Goto);
            }
            // Текущее состояние не найдено → конечное состояние (Final)
            let Some(state) = states.get(current.as_str()) else {
                return Ok(Transition::Final);
            };
            state.borrow_mut().tick()?
        };

        match result {
            Transition::Processing => return Ok(Transition::Processing),
            Transition::Final => {
                // Проверяем безусловный next-переход состояния с расширением
                let Unit::Plain {
                    current,
                    state_nexts,
                    states,
                    transitions,
                    state_transitions,
                    ..
                } = &mut *self
                else {
                    unreachable!()
                };
                if let Some(next_state) = state_nexts.get(current.as_str()).cloned() {
                    let cur = current.clone();
                    if let Some(state) = states.get(cur.as_str()) {
                        state.borrow_mut().exit();
                    }
                    *transitions = make_transitions(state_transitions, &next_state);
                    *current = next_state.clone();
                    let next_machine = states.get(next_state.as_str()).ok_or_else(|| {
                        Diagnostic::error(
                            Location::Builtin,
                            format!("Состояние '{}' не найдено", next_state),
                        )
                    })?;
                    next_machine.borrow_mut().enter();
                    return Ok(Transition::Processing);
                }
                return Ok(Transition::Final);
            }
            Transition::Goto => {}
        }

        // Временно забираем transitions; предикаты читают self через &dyn Context
        let transitions = match self {
            Unit::Plain { transitions, .. } => std::mem::take(transitions),
            Unit::Parallel { .. } | Unit::Sequence { .. } => unreachable!(),
        };
        let next_name = transitions
            .iter()
            .find(|(_, predicate)| predicate(&*self))
            .map(|(name, _)| name.clone());
        match self {
            Unit::Plain { transitions: t, .. } => *t = transitions,
            Unit::Parallel { .. } | Unit::Sequence { .. } => unreachable!(),
        }

        let Some(next) = next_name else {
            // Нет условных переходов — проверяем next-переход состояния с расширением
            let Unit::Plain {
                current,
                state_nexts,
                states,
                transitions,
                state_transitions,
                ..
            } = &mut *self
            else {
                unreachable!()
            };
            if let Some(next_state) = state_nexts.get(current.as_str()).cloned() {
                let cur = current.clone();
                if let Some(state) = states.get(cur.as_str()) {
                    state.borrow_mut().exit();
                }
                *transitions = make_transitions(state_transitions, &next_state);
                *current = next_state.clone();
                let next_machine = states.get(next_state.as_str()).ok_or_else(|| {
                    Diagnostic::error(
                        Location::Builtin,
                        format!("Состояние '{}' не найдено", next_state),
                    )
                })?;
                next_machine.borrow_mut().enter();
                return Ok(Transition::Processing);
            }
            return Ok(Transition::Goto);
        };

        // Переход: выходим из текущего, обновляем transitions и current, входим в новое
        let Unit::Plain {
            current,
            states,
            transitions,
            state_transitions,
            ..
        } = &mut *self
        else {
            unreachable!()
        };
        let cur = current.clone();
        if let Some(state) = states.get(cur.as_str()) {
            state.borrow_mut().exit();
        }
        *transitions = make_transitions(state_transitions, &next);
        *current = next.clone();
        let next_machine = states.get(next.as_str()).ok_or_else(|| {
            Diagnostic::error(Location::Builtin, "Состояние не найдено".to_string())
        })?;
        next_machine.borrow_mut().enter();

        Ok(Transition::Processing)
    }
}

// ── Тесты ────────────────────────────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use super::*;
    use std::cell::Cell;
    use std::collections::HashMap;
    use std::rc::Rc;

    fn always_true() -> Predicate {
        Rc::new(|_: &dyn Context| true)
    }

    fn always_false() -> Predicate {
        Rc::new(|_: &dyn Context| false)
    }

    /// Счётчик вызовов через Rc<Cell>, доступный из Fn-замыкания.
    fn call_counter() -> (Rc<Cell<u32>>, Execution) {
        let count = Rc::new(Cell::new(0u32));
        let c = count.clone();
        (count, Box::new(move |_| c.set(c.get() + 1)))
    }

    /// Листовой Plain: states пуст, tick() возвращает Goto.
    fn leaf() -> Unit {
        leaf_plain()
    }

    /// Создаёт Unit::Plain с одним дочерним листовым состоянием и заданными переходами.
    fn plain_with_leaf(name: &str, transitions: Vec<(String, Predicate)>) -> Unit {
        let mut states = HashMap::new();
        states.insert(name.to_string(), RefCell::new(leaf()));
        Unit::Plain {
            upper: None,
            transitions,
            state_transitions: HashMap::new(),
            state_nexts: HashMap::new(),
            states,
            current: name.to_string(),
            always: vec![],
            enters: vec![],
            exits: vec![],
            variables: HashMap::new(),
        }
    }

    // ── Листовой Plain (states пуст) ──────────────────────────────────────────

    #[test]
    fn test_leaf_tick_returns_goto() {
        let mut m = leaf();
        assert!(matches!(m.tick().unwrap(), Transition::Goto));
    }

    #[test]
    fn test_leaf_enter_calls_enters() {
        let (count, exec) = call_counter();
        let mut m = Unit::Plain {
            upper: None,
            transitions: vec![],
            state_transitions: HashMap::new(),
            state_nexts: HashMap::new(),
            states: HashMap::new(),
            current: String::new(),
            always: vec![],
            enters: vec![exec],
            exits: vec![],
            variables: HashMap::new(),
        };
        m.enter();
        assert_eq!(count.get(), 1);
    }

    #[test]
    fn test_leaf_exit_calls_exits() {
        let (count, exec) = call_counter();
        let mut m = Unit::Plain {
            upper: None,
            transitions: vec![],
            state_transitions: HashMap::new(),
            state_nexts: HashMap::new(),
            states: HashMap::new(),
            current: String::new(),
            always: vec![],
            enters: vec![],
            exits: vec![exec],
            variables: HashMap::new(),
        };
        m.exit();
        assert_eq!(count.get(), 1);
    }

    #[test]
    fn test_leaf_enter_exit_empty_do_not_panic() {
        let mut m = leaf();
        m.enter();
        m.exit();
    }

    #[test]
    fn test_leaf_always_called_on_tick() {
        // У листового Plain always вызывается при каждом tick (до проверки states.is_empty)
        let (count, exec) = call_counter();
        let mut m = Unit::Plain {
            upper: None,
            transitions: vec![],
            state_transitions: HashMap::new(),
            state_nexts: HashMap::new(),
            states: HashMap::new(),
            current: String::new(),
            always: vec![exec],
            enters: vec![],
            exits: vec![],
            variables: HashMap::new(),
        };
        m.tick().unwrap();
        m.tick().unwrap();
        assert_eq!(count.get(), 2);
    }

    // ── Unit::Plain: конечное состояние ──────────────────────────────────────

    #[test]
    fn test_plain_missing_current_state_returns_final() {
        // states непуст, но current не совпадает ни с одним ключом → Final
        let mut states = HashMap::new();
        states.insert("B".to_string(), RefCell::new(leaf()));
        let mut m = Unit::Plain {
            upper: None,
            transitions: vec![],
            state_transitions: HashMap::new(),
            state_nexts: HashMap::new(),
            states,
            current: "A".to_string(), // нет в states
            always: vec![],
            enters: vec![],
            exits: vec![],
            variables: HashMap::new(),
        };
        assert!(matches!(m.tick().unwrap(), Transition::Final));
    }

    // ── Unit::Plain: always выполняется каждый тик ────────────────────────────

    #[test]
    fn test_plain_always_called_each_tick() {
        let (count, exec) = call_counter();
        let mut states = HashMap::new();
        states.insert("S".to_string(), RefCell::new(leaf()));
        let mut m = Unit::Plain {
            upper: None,
            transitions: vec![],
            state_transitions: HashMap::new(),
            state_nexts: HashMap::new(),
            states,
            current: "S".to_string(),
            always: vec![exec],
            enters: vec![],
            exits: vec![],
            variables: HashMap::new(),
        };
        m.tick().unwrap();
        m.tick().unwrap();
        assert_eq!(count.get(), 2);
    }

    // ── Unit::Plain: дочерний leaf (Goto) ─────────────────────────────────────

    #[test]
    fn test_plain_child_goto_no_transitions_returns_goto() {
        let mut m = plain_with_leaf("S", vec![]);
        assert!(matches!(m.tick().unwrap(), Transition::Goto));
    }

    #[test]
    fn test_plain_child_goto_false_predicate_returns_goto() {
        let mut m = plain_with_leaf("S", vec![("T".to_string(), always_false())]);
        assert!(matches!(m.tick().unwrap(), Transition::Goto));
    }

    // ── Unit::Plain: срабатывание перехода ───────────────────────────────────

    #[test]
    fn test_plain_transition_fires_returns_processing() {
        let mut m = plain_with_leaf("S", vec![("T".to_string(), always_true())]);
        if let Unit::Plain { states, .. } = &mut m {
            states.insert("T".to_string(), RefCell::new(leaf()));
        }
        assert!(matches!(m.tick().unwrap(), Transition::Processing));
    }

    #[test]
    fn test_plain_transition_fires_updates_current() {
        let mut m = plain_with_leaf("S", vec![("T".to_string(), always_true())]);
        if let Unit::Plain { states, .. } = &mut m {
            states.insert("T".to_string(), RefCell::new(leaf()));
        }
        m.tick().unwrap();
        if let Unit::Plain { current, .. } = &m {
            assert_eq!(current, "T");
        } else {
            panic!("ожидался Unit::Plain");
        }
    }

    #[test]
    fn test_plain_transition_skips_false_takes_first_true() {
        let transitions = vec![
            ("T".to_string(), always_false()),
            ("U".to_string(), always_true()),
        ];
        let mut m = plain_with_leaf("S", transitions);
        if let Unit::Plain { states, .. } = &mut m {
            states.insert("T".to_string(), RefCell::new(leaf()));
            states.insert("U".to_string(), RefCell::new(leaf()));
        }
        m.tick().unwrap();
        if let Unit::Plain { current, .. } = &m {
            assert_eq!(current, "U");
        } else {
            panic!("ожидался Unit::Plain");
        }
    }

    #[test]
    fn test_plain_transition_target_missing_returns_error() {
        let mut m = plain_with_leaf("S", vec![("MISSING".to_string(), always_true())]);
        assert!(m.tick().is_err());
    }

    // ── exits/enters при переходе ─────────────────────────────────────────────

    #[test]
    fn test_plain_transition_calls_exit_then_enter() {
        let (exit_count, exit_exec) = call_counter();
        let (enter_count, enter_exec) = call_counter();

        let mut states: HashMap<String, RefCell<Unit>> = HashMap::new();
        states.insert(
            "S".to_string(),
            RefCell::new(Unit::Plain {
                upper: None,
                transitions: vec![],
                state_transitions: HashMap::new(),
                state_nexts: HashMap::new(),
                states: HashMap::new(),
                current: String::new(),
                always: vec![],
                enters: vec![],
                exits: vec![exit_exec],
                variables: HashMap::new(),
            }),
        );
        states.insert(
            "T".to_string(),
            RefCell::new(Unit::Plain {
                upper: None,
                transitions: vec![],
                state_transitions: HashMap::new(),
                state_nexts: HashMap::new(),
                states: HashMap::new(),
                current: String::new(),
                always: vec![],
                enters: vec![enter_exec],
                exits: vec![],
                variables: HashMap::new(),
            }),
        );

        let mut m = Unit::Plain {
            upper: None,
            transitions: vec![("T".to_string(), always_true())],
            state_transitions: HashMap::new(),
            state_nexts: HashMap::new(),
            states,
            current: "S".to_string(),
            always: vec![],
            enters: vec![],
            exits: vec![],
            variables: HashMap::new(),
        };

        m.tick().unwrap();
        assert_eq!(exit_count.get(), 1, "exit состояния S должен вызваться");
        assert_eq!(enter_count.get(), 1, "enter состояния T должен вызваться");
    }

    // ── Unit::Plain: дочерний Final не запускает переходы ────────────────────

    #[test]
    fn test_plain_child_final_propagates_without_checking_transitions() {
        // Внутренний Plain с непустым states, но current не найден → Final
        let mut inner_states = HashMap::new();
        inner_states.insert("X".to_string(), RefCell::new(leaf()));
        let inner = Unit::Plain {
            upper: None,
            transitions: vec![],
            state_transitions: HashMap::new(),
            state_nexts: HashMap::new(),
            states: inner_states,
            current: "MISSING".to_string(),
            always: vec![],
            enters: vec![],
            exits: vec![],
            variables: HashMap::new(),
        };
        let mut states = HashMap::new();
        states.insert("S".to_string(), RefCell::new(inner));
        let mut m = Unit::Plain {
            upper: None,
            transitions: vec![("T".to_string(), always_true())],
            state_transitions: HashMap::new(),
            state_nexts: HashMap::new(),
            states,
            current: "S".to_string(),
            always: vec![],
            enters: vec![],
            exits: vec![],
            variables: HashMap::new(),
        };
        assert!(matches!(m.tick().unwrap(), Transition::Final));
    }

    // ── Unit::Plain: Processing от дочернего ─────────────────────────────────

    #[test]
    fn test_plain_child_processing_propagates() {
        let mut inner_states = HashMap::new();
        inner_states.insert("inner_s".to_string(), RefCell::new(leaf()));
        inner_states.insert("inner_t".to_string(), RefCell::new(leaf()));
        let inner = Unit::Plain {
            upper: None,
            transitions: vec![("inner_t".to_string(), always_true())],
            state_transitions: HashMap::new(),
            state_nexts: HashMap::new(),
            states: inner_states,
            current: "inner_s".to_string(),
            always: vec![],
            enters: vec![],
            exits: vec![],
            variables: HashMap::new(),
        };

        let mut outer_states = HashMap::new();
        outer_states.insert("S".to_string(), RefCell::new(inner));
        let mut m = Unit::Plain {
            upper: None,
            transitions: vec![],
            state_transitions: HashMap::new(),
            state_nexts: HashMap::new(),
            states: outer_states,
            current: "S".to_string(),
            always: vec![],
            enters: vec![],
            exits: vec![],
            variables: HashMap::new(),
        };
        assert!(matches!(m.tick().unwrap(), Transition::Processing));
    }

    // ── Context: доступ к переменным ─────────────────────────────────────────

    #[test]
    fn test_plain_get_value_returns_variable() {
        let mut vars = HashMap::new();
        vars.insert("x".to_string(), Value::Number(42));
        let m = Unit::Plain {
            upper: None,
            transitions: vec![],
            state_transitions: HashMap::new(),
            state_nexts: HashMap::new(),
            states: HashMap::new(),
            current: String::new(),
            always: vec![],
            enters: vec![],
            exits: vec![],
            variables: vars,
        };
        assert!(matches!(m.get_value("x"), Some(Value::Number(42))));
    }

    #[test]
    fn test_get_value_missing_returns_none() {
        let m = leaf();
        assert!(m.get_value("unknown").is_none());
    }

    // ── Предикат читает переменную через Context ──────────────────────────────

    #[test]
    fn test_transition_predicate_reads_variable() {
        let pred: Predicate =
            Rc::new(|ctx: &dyn Context| matches!(ctx.get_value("ready"), Some(Value::Number(1))));

        let mut vars = HashMap::new();
        vars.insert("ready".to_string(), Value::Number(1));

        let mut states = HashMap::new();
        states.insert("S".to_string(), RefCell::new(leaf()));
        states.insert("T".to_string(), RefCell::new(leaf()));

        let mut m = Unit::Plain {
            upper: None,
            transitions: vec![("T".to_string(), pred)],
            state_transitions: HashMap::new(),
            state_nexts: HashMap::new(),
            states,
            current: "S".to_string(),
            always: vec![],
            enters: vec![],
            exits: vec![],
            variables: vars,
        };

        m.tick().unwrap();
        if let Unit::Plain { current, .. } = &m {
            assert_eq!(current, "T");
        } else {
            panic!("ожидался Unit::Plain");
        }
    }

    // ── enter/exit не паникуют ────────────────────────────────────────────────

    #[test]
    fn test_plain_enter_exit_do_not_panic() {
        let mut m = plain_with_leaf("S", vec![]);
        m.enter();
        m.exit();
    }

    // ── Context: делегирование к родителю через upper ────────────────────────

    #[test]
    fn test_get_value_delegates_to_upper() {
        let mut parent_vars = HashMap::new();
        parent_vars.insert("x".to_string(), Value::Number(99));
        let parent = Rc::new(RefCell::new(Unit::Plain {
            upper: None,
            transitions: vec![],
            state_transitions: HashMap::new(),
            state_nexts: HashMap::new(),
            states: HashMap::new(),
            current: String::new(),
            always: vec![],
            enters: vec![],
            exits: vec![],
            variables: parent_vars,
        }));
        let child = Unit::Plain {
            upper: Some(parent),
            transitions: vec![],
            state_transitions: HashMap::new(),
            state_nexts: HashMap::new(),
            states: HashMap::new(),
            current: String::new(),
            always: vec![],
            enters: vec![],
            exits: vec![],
            variables: HashMap::new(),
        };
        assert!(matches!(child.get_value("x"), Some(Value::Number(99))));
    }

    #[test]
    fn test_get_value_local_shadows_upper() {
        let mut parent_vars = HashMap::new();
        parent_vars.insert("x".to_string(), Value::Number(1));
        let parent = Rc::new(RefCell::new(Unit::Plain {
            upper: None,
            transitions: vec![],
            state_transitions: HashMap::new(),
            state_nexts: HashMap::new(),
            states: HashMap::new(),
            current: String::new(),
            always: vec![],
            enters: vec![],
            exits: vec![],
            variables: parent_vars,
        }));
        let mut child_vars = HashMap::new();
        child_vars.insert("x".to_string(), Value::Number(42));
        let child = Unit::Plain {
            upper: Some(parent),
            transitions: vec![],
            state_transitions: HashMap::new(),
            state_nexts: HashMap::new(),
            states: HashMap::new(),
            current: String::new(),
            always: vec![],
            enters: vec![],
            exits: vec![],
            variables: child_vars,
        };
        assert!(matches!(child.get_value("x"), Some(Value::Number(42))));
    }

    // ── Множественные enters/exits/always ─────────────────────────────────────

    #[test]
    fn test_plain_multiple_enters_all_called() {
        let (c1, e1) = call_counter();
        let (c2, e2) = call_counter();
        let (c3, e3) = call_counter();
        let mut m = Unit::Plain {
            upper: None,
            transitions: vec![],
            state_transitions: HashMap::new(),
            state_nexts: HashMap::new(),
            states: HashMap::new(),
            current: String::new(),
            always: vec![],
            enters: vec![e1, e2, e3],
            exits: vec![],
            variables: HashMap::new(),
        };
        m.enter();
        assert_eq!(c1.get(), 1);
        assert_eq!(c2.get(), 1);
        assert_eq!(c3.get(), 1);
    }

    #[test]
    fn test_plain_multiple_exits_all_called() {
        let (c1, e1) = call_counter();
        let (c2, e2) = call_counter();
        let mut m = Unit::Plain {
            upper: None,
            transitions: vec![],
            state_transitions: HashMap::new(),
            state_nexts: HashMap::new(),
            states: HashMap::new(),
            current: String::new(),
            always: vec![],
            enters: vec![],
            exits: vec![e1, e2],
            variables: HashMap::new(),
        };
        m.exit();
        assert_eq!(c1.get(), 1);
        assert_eq!(c2.get(), 1);
    }

    #[test]
    fn test_plain_multiple_always_all_called_each_tick() {
        let (c1, e1) = call_counter();
        let (c2, e2) = call_counter();
        let mut states = HashMap::new();
        states.insert("S".to_string(), RefCell::new(leaf()));
        let mut m = Unit::Plain {
            upper: None,
            transitions: vec![],
            state_transitions: HashMap::new(),
            state_nexts: HashMap::new(),
            states,
            current: "S".to_string(),
            always: vec![e1, e2],
            enters: vec![],
            exits: vec![],
            variables: HashMap::new(),
        };
        m.tick().unwrap();
        m.tick().unwrap();
        assert_eq!(c1.get(), 2);
        assert_eq!(c2.get(), 2);
    }

    // ── from_model: построение из семантической модели ───────────────────────

    use grammar::parse;
    use grammar::semantic::tree::construct_model;

    /// Двусостояниная FSM: A → B (безусловный переход).
    #[test]
    fn test_from_model_two_state_fsm() {
        let src = "start A { ref B; } state B;";
        let (ast, _) = parse(src, 0).unwrap();
        let model = construct_model(&ast, None, &[]).unwrap();
        let mut unit = from_model(model).unwrap();

        let Unit::Plain { current, .. } = &unit else {
            panic!("ожидался Unit::Plain");
        };
        assert_eq!(current, "A");

        let result = unit.tick().unwrap();
        assert!(matches!(result, Transition::Processing));
        let Unit::Plain { current, .. } = &unit else {
            panic!("ожидался Unit::Plain");
        };
        assert_eq!(current, "B");
    }

    /// Конечное состояние B: tick() из B возвращает Goto (нет переходов).
    #[test]
    fn test_from_model_final_state_returns_goto() {
        let src = "start A { ref B; } state B;";
        let (ast, _) = parse(src, 0).unwrap();
        let model = construct_model(&ast, None, &[]).unwrap();
        let mut unit = from_model(model).unwrap();

        unit.tick().unwrap(); // A → B
        let result = unit.tick().unwrap();
        assert!(matches!(result, Transition::Goto));
    }

    /// Последовательная композиция A + B → Unit::Sequence.
    #[test]
    fn test_from_model_concatenation_builds_sequence() {
        let src = "model A { start S; } model B { start S; } start Entry = A + B;";
        let (ast, _) = parse(src, 0).unwrap();
        let model = construct_model(&ast, None, &[]).unwrap();
        let root = model.borrow();
        let entry_state = root.search_state("Entry").unwrap();
        let state_node = entry_state.borrow();
        if let StateNode::Implement { implements, .. } = &*state_node {
            let unit = from_extend(implements, None).unwrap();
            assert!(matches!(unit, Unit::Sequence { .. }));
        } else {
            panic!("Entry должен быть StateNode::Implement");
        }
    }

    /// Параллельная композиция A | B → Unit::Parallel.
    #[test]
    fn test_from_model_parallel_builds_parallel() {
        let src = "model A { start S; } model B { start S; } start Entry = A | B;";
        let (ast, _) = parse(src, 0).unwrap();
        let model = construct_model(&ast, None, &[]).unwrap();
        let root = model.borrow();
        let entry_state = root.search_state("Entry").unwrap();
        let state_node = entry_state.borrow();
        if let StateNode::Implement { implements, .. } = &*state_node {
            let unit = from_extend(implements, None).unwrap();
            assert!(matches!(unit, Unit::Parallel { .. }));
        } else {
            panic!("Entry должен быть StateNode::Implement");
        }
    }

    /// Копии при повторном использовании: A + A → два независимых Unit.
    #[test]
    fn test_from_model_copies_are_independent() {
        let src = "model A { start S; } start Entry = A + A;";
        let (ast, _) = parse(src, 0).unwrap();
        let model = construct_model(&ast, None, &[]).unwrap();
        let root = model.borrow();
        let entry_state = root.search_state("Entry").unwrap();
        let state_node = entry_state.borrow();
        if let StateNode::Implement { implements, .. } = &*state_node {
            let unit = from_extend(implements, None).unwrap();
            let Unit::Sequence { units, .. } = &unit else {
                panic!("ожидался Unit::Sequence");
            };
            assert_eq!(units.len(), 2);
            assert!(!Rc::ptr_eq(&units[0], &units[1]));
        } else {
            panic!("Entry должен быть StateNode::Implement");
        }
    }

    /// state_transitions обновляются при смене состояния.
    #[test]
    fn test_state_transitions_updated_on_change() {
        let src = "start A { ref B; } state B { ref C; } state C;";
        let (ast, _) = parse(src, 0).unwrap();
        let model = construct_model(&ast, None, &[]).unwrap();
        let mut unit = from_model(model).unwrap();

        unit.tick().unwrap(); // A → B
        let Unit::Plain { current, .. } = &unit else {
            panic!()
        };
        assert_eq!(current, "B");

        unit.tick().unwrap(); // B → C
        let Unit::Plain { current, .. } = &unit else {
            panic!()
        };
        assert_eq!(current, "C");

        assert!(matches!(unit.tick().unwrap(), Transition::Goto));
    }

    // ── next-переход: безусловный переход после завершения расширения ─────────

    /// Простейший next: состояние с одной моделью переходит в End.
    #[test]
    fn test_next_after_single_model() {
        // S = A { next End; } — после завершения A переходим в End
        let src = "model A { start Start; } start S = A { next End; } state End;";
        let (ast, _) = parse(src, 0).unwrap();
        let model = construct_model(&ast, None, &[]).unwrap();
        let mut unit = from_model(model).unwrap();

        let Unit::Plain { current, .. } = &unit else {
            panic!("ожидался Unit::Plain")
        };
        assert_eq!(current, "S");

        // tick: A (листовой) → Goto → next срабатывает → переходим в End
        let result = unit.tick().unwrap();
        assert!(
            matches!(result, Transition::Processing),
            "ожидался Processing после next-перехода"
        );
        let Unit::Plain { current, .. } = &unit else {
            panic!()
        };
        assert_eq!(current, "End");
    }

    /// Последовательная композиция с next: S = A + B { next End; }.
    #[test]
    fn test_next_after_sequence() {
        let src = "model A { start Start; } model B { start Start; } start S = A + B { next End; } state End;";
        let (ast, _) = parse(src, 0).unwrap();
        let model = construct_model(&ast, None, &[]).unwrap();
        let mut unit = from_model(model).unwrap();

        let Unit::Plain { current, .. } = &unit else {
            panic!()
        };
        assert_eq!(current, "S");

        // tick 1: A листовой → Goto → Sequence переходит к B → Processing
        unit.tick().unwrap();

        // tick 2: B листовой → Goto → Sequence исчерпан → Final → next → End
        let result = unit.tick().unwrap();
        assert!(matches!(result, Transition::Processing));
        let Unit::Plain { current, .. } = &unit else {
            panic!()
        };
        assert_eq!(current, "End");
    }

    /// Параллельная композиция с next: S = A | B { next End; }.
    #[test]
    fn test_next_after_parallel() {
        let src = "model A { start Start; } model B { start Start; } start S = A | B { next End; } state End;";
        let (ast, _) = parse(src, 0).unwrap();
        let model = construct_model(&ast, None, &[]).unwrap();
        let mut unit = from_model(model).unwrap();

        // tick: оба листовых завершаются → Parallel Final → next → End
        let result = unit.tick().unwrap();
        assert!(matches!(result, Transition::Processing));
        let Unit::Plain { current, .. } = &unit else {
            panic!()
        };
        assert_eq!(current, "End");
    }

    /// После перехода в End tick() из листового End возвращает Goto.
    #[test]
    fn test_next_target_is_leaf_returns_goto() {
        let src = "model A { start Start; } start S = A { next End; } state End;";
        let (ast, _) = parse(src, 0).unwrap();
        let model = construct_model(&ast, None, &[]).unwrap();
        let mut unit = from_model(model).unwrap();

        unit.tick().unwrap(); // S → End через next
        let result = unit.tick().unwrap(); // End листовой
        assert!(matches!(result, Transition::Goto));
    }

    /// Без next: конечный внутренний Final пробрасывается наружу.
    #[test]
    fn test_no_next_final_propagates() {
        let src = "model A { start Start; } start S = A;";
        let (ast, _) = parse(src, 0).unwrap();
        let model = construct_model(&ast, None, &[]).unwrap();
        let mut unit = from_model(model).unwrap();

        // S не имеет next; A листовой → Goto → нет переходов → Goto (S единственное, нет FSM-переходов)
        let result = unit.tick().unwrap();
        assert!(
            matches!(result, Transition::Goto | Transition::Final),
            "ожидался Goto или Final, получен {:?}",
            result
        );
    }
}
