use crate::context::Context;
use crate::predicate::create_predicate;

pub(crate) type Predicate = Rc<dyn Fn(&dyn Context) -> bool>;
pub(crate) type Execution = Box<dyn Fn(&mut dyn Context)>;

use crate::unit::Transition::Final;
use crate::value::Value;
use grammar::diagnostics::{Diagnostic, Location};
use grammar::semantic::extend::Extend;
use grammar::semantic::{ConditionNode, ModelNode, StateNode, StateNodeKind};
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
/// Каждый вариант соответствует одному из структурных паттернов:
/// - [`Plain`](Unit::Plain) — конечный автомат с состояниями и переходами;
/// - [`Extend`](Unit::Extend) — листовое состояние (tick всегда возвращает Goto);
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
        states: HashMap<String, RefCell<Unit>>,
        current: String,

        always: Vec<Execution>,
        enters: Vec<Execution>,
        exits: Vec<Execution>,

        variables: HashMap<String, Value>,
    },
    /// Листовое состояние: tick() немедленно возвращает Goto.
    /// always не вызывается, enter/exit — вызываются при входе/выходе.
    Extend {
        upper: Option<Rc<RefCell<Unit>>>,
        always: Vec<Execution>,
        enters: Vec<Execution>,
        exits: Vec<Execution>,
        variables: HashMap<String, Value>,
    },
    /// Параллельная композиция: все дочерние Unit тикаются одновременно.
    Parallel {
        upper: Option<Rc<RefCell<Unit>>>,
        units: Vec<Rc<RefCell<Unit>>>,
    },
    /// Последовательная композиция: дочерние Unit выполняются по очереди.
    Sequence {
        upper: Option<Rc<RefCell<Unit>>>,
        units: Vec<Rc<RefCell<Unit>>>,
        index: usize,
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
        from_extend(&borrowed.implements)
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
    }

    let transitions = make_transitions(&state_transitions, &start_name);

    Ok(Unit::Plain {
        upper: None,
        transitions,
        state_transitions,
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
        StateNode::Simple { .. } => Ok(Unit::Extend {
            upper: None,
            always: vec![],
            enters: vec![],
            exits: vec![],
            variables: HashMap::new(),
        }),
        StateNode::Implement { implements, .. } => from_extend(implements),
    }
}

fn from_extend(extend: &Extend) -> Result<Unit, Diagnostic> {
    match extend {
        Extend::None | Extend::Unresolved(_) => Ok(Unit::Extend {
            upper: None,
            always: vec![],
            enters: vec![],
            exits: vec![],
            variables: HashMap::new(),
        }),
        Extend::Model(model) => {
            let borrowed = model.borrow();
            if borrowed.has_states() {
                from_model_plain(&borrowed)
            } else {
                from_extend(&borrowed.implements)
            }
        }
        Extend::Parentless(inner) => from_extend(inner),
        Extend::Concatenation(items) => {
            let units: Vec<Rc<RefCell<Unit>>> = items
                .iter()
                .map(|item| Ok(Rc::new(RefCell::new(from_extend(item)?))))
                .collect::<Result<_, Diagnostic>>()?;
            Ok(Unit::Sequence {
                upper: None,
                units,
                index: 0,
            })
        }
        Extend::Parallel(items) => {
            let units: Vec<Rc<RefCell<Unit>>> = items
                .iter()
                .map(|item| Ok(Rc::new(RefCell::new(from_extend(item)?))))
                .collect::<Result<_, Diagnostic>>()?;
            Ok(Unit::Parallel { upper: None, units })
        }
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
            Unit::Extend { variables, .. } => variables.get(name).cloned(),
            Unit::Parallel { units, .. } | Unit::Sequence { units, .. } => {
                units.iter().flat_map(|u| u.borrow().get_value(name)).next()
            }
        };
        result.or_else(|| {
            let upper = match self {
                Unit::Plain { upper, .. }
                | Unit::Extend { upper, .. }
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
            Unit::Extend { enters, .. } => std::mem::take(enters),
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
            Unit::Extend { enters: e, .. } => *e = enters,
            Unit::Parallel { .. } | Unit::Sequence { .. } => unreachable!(),
        }
    }

    fn exit(&mut self) {
        let exits = match self {
            Unit::Plain { exits, .. } => std::mem::take(exits),
            Unit::Extend { exits, .. } => std::mem::take(exits),
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
            Unit::Extend { exits: e, .. } => *e = exits,
            Unit::Parallel { .. } | Unit::Sequence { .. } => unreachable!(),
        }
    }

    fn tick(&mut self) -> Result<Transition, Diagnostic> {
        // Листовое состояние: немедленный Goto, always не вызывается
        if matches!(self, Unit::Extend { .. }) {
            return Ok(Transition::Goto);
        }

        if let Unit::Parallel { units, .. } = self {
            if units
                .iter()
                .map(|u| u.borrow_mut().tick())
                .all(|ret| ret.is_ok_and(|v| v == Final))
            {
                return Ok(Transition::Final);
            }
            return Ok(Transition::Processing);
        } else if let Unit::Sequence { units, index, .. } = self {
            let result = units[*index].borrow_mut().tick()?;
            if result == Final {
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
            _ => unreachable!(),
        };
        for f in &always {
            f(self as &mut dyn Context);
        }
        match self {
            Unit::Plain { always: a, .. } => *a = always,
            _ => unreachable!(),
        }

        // Тикаем текущее вложенное состояние
        let result = {
            let Unit::Plain {
                current, states, ..
            } = &mut *self
            else {
                unreachable!()
            };
            let Some(state) = states.get(current.as_str()) else {
                return Ok(Transition::Final);
            };
            state.borrow_mut().tick()?
        };

        match result {
            Transition::Processing => return Ok(Transition::Processing),
            Transition::Final => return Ok(Transition::Final),
            Transition::Goto => {}
        }

        // Временно забираем transitions; предикаты читают self через &dyn Context
        let transitions = match self {
            Unit::Plain { transitions, .. } => std::mem::take(transitions),
            _ => unreachable!(),
        };
        let next_name = transitions
            .iter()
            .find(|(_, predicate)| predicate(&*self))
            .map(|(name, _)| name.clone());
        match self {
            Unit::Plain { transitions: t, .. } => *t = transitions,
            _ => unreachable!(),
        }

        let Some(next) = next_name else {
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

    /// Создаёт Unit::Extend без действий.
    fn extend() -> Unit {
        Unit::Extend {
            upper: None,
            always: vec![],
            enters: vec![],
            exits: vec![],
            variables: HashMap::new(),
        }
    }

    /// Создаёт Unit::Plain с одним дочерним Extend-состоянием и заданными переходами.
    fn plain_with_extend(name: &str, transitions: Vec<(String, Predicate)>) -> Unit {
        let mut states = HashMap::new();
        states.insert(name.to_string(), RefCell::new(extend()));
        Unit::Plain {
            upper: None,
            transitions,
            state_transitions: HashMap::new(),
            states,
            current: name.to_string(),
            always: vec![],
            enters: vec![],
            exits: vec![],
            variables: HashMap::new(),
        }
    }

    // ── Unit::Extend ──────────────────────────────────────────────────────────

    #[test]
    fn test_extend_tick_returns_goto() {
        let mut m = extend();
        assert!(matches!(m.tick().unwrap(), Transition::Goto));
    }

    #[test]
    fn test_extend_enter_calls_enters() {
        let (count, exec) = call_counter();
        let mut m = Unit::Extend {
            upper: None,
            always: vec![],
            enters: vec![exec],
            exits: vec![],
            variables: HashMap::new(),
        };
        m.enter();
        assert_eq!(count.get(), 1);
    }

    #[test]
    fn test_extend_exit_calls_exits() {
        let (count, exec) = call_counter();
        let mut m = Unit::Extend {
            upper: None,
            always: vec![],
            enters: vec![],
            exits: vec![exec],
            variables: HashMap::new(),
        };
        m.exit();
        assert_eq!(count.get(), 1);
    }

    #[test]
    fn test_extend_enter_exit_empty_do_not_panic() {
        let mut m = extend();
        m.enter();
        m.exit();
    }

    // ── Unit::Plain: отсутствующее текущее состояние ─────────────────────────

    #[test]
    fn test_plain_missing_current_state_returns_final() {
        let mut m = Unit::Plain {
            upper: None,
            transitions: vec![("B".to_string(), always_true())],
            state_transitions: HashMap::new(),
            states: HashMap::new(),
            current: "A".to_string(),
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
        states.insert("S".to_string(), RefCell::new(extend()));
        let mut m = Unit::Plain {
            upper: None,
            transitions: vec![],
            state_transitions: HashMap::new(),
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

    // ── Unit::Plain: дочерний Extend (Goto) ──────────────────────────────────

    #[test]
    fn test_plain_child_goto_no_transitions_returns_goto() {
        let mut m = plain_with_extend("S", vec![]);
        assert!(matches!(m.tick().unwrap(), Transition::Goto));
    }

    #[test]
    fn test_plain_child_goto_false_predicate_returns_goto() {
        let mut m = plain_with_extend("S", vec![("T".to_string(), always_false())]);
        assert!(matches!(m.tick().unwrap(), Transition::Goto));
    }

    // ── Unit::Plain: срабатывание перехода ───────────────────────────────────

    #[test]
    fn test_plain_transition_fires_returns_processing() {
        let mut m = plain_with_extend("S", vec![("T".to_string(), always_true())]);
        if let Unit::Plain { states, .. } = &mut m {
            states.insert("T".to_string(), RefCell::new(extend()));
        }
        assert!(matches!(m.tick().unwrap(), Transition::Processing));
    }

    #[test]
    fn test_plain_transition_fires_updates_current() {
        let mut m = plain_with_extend("S", vec![("T".to_string(), always_true())]);
        if let Unit::Plain { states, .. } = &mut m {
            states.insert("T".to_string(), RefCell::new(extend()));
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
        let mut m = plain_with_extend("S", transitions);
        if let Unit::Plain { states, .. } = &mut m {
            states.insert("T".to_string(), RefCell::new(extend()));
            states.insert("U".to_string(), RefCell::new(extend()));
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
        let mut m = plain_with_extend("S", vec![("MISSING".to_string(), always_true())]);
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
            RefCell::new(Unit::Extend {
                upper: None,
                always: vec![],
                enters: vec![],
                exits: vec![exit_exec],
                variables: HashMap::new(),
            }),
        );
        states.insert(
            "T".to_string(),
            RefCell::new(Unit::Extend {
                upper: None,
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
        let inner = Unit::Plain {
            upper: None,
            transitions: vec![],
            state_transitions: HashMap::new(),
            states: HashMap::new(),
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
        inner_states.insert("inner_s".to_string(), RefCell::new(extend()));
        inner_states.insert("inner_t".to_string(), RefCell::new(extend()));
        let inner = Unit::Plain {
            upper: None,
            transitions: vec![("inner_t".to_string(), always_true())],
            state_transitions: HashMap::new(),
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
            states: HashMap::new(),
            current: "S".to_string(),
            always: vec![],
            enters: vec![],
            exits: vec![],
            variables: vars,
        };
        assert!(matches!(m.get_value("x"), Some(Value::Number(42))));
    }

    #[test]
    fn test_extend_get_value_returns_variable() {
        let mut vars = HashMap::new();
        vars.insert("flag".to_string(), Value::Boolean(true));
        let m = Unit::Extend {
            upper: None,
            always: vec![],
            enters: vec![],
            exits: vec![],
            variables: vars,
        };
        assert!(matches!(m.get_value("flag"), Some(Value::Boolean(true))));
    }

    #[test]
    fn test_get_value_missing_returns_none() {
        let m = extend();
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
        states.insert("S".to_string(), RefCell::new(extend()));
        states.insert("T".to_string(), RefCell::new(extend()));

        let mut m = Unit::Plain {
            upper: None,
            transitions: vec![("T".to_string(), pred)],
            state_transitions: HashMap::new(),
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
        let mut m = plain_with_extend("S", vec![]);
        m.enter();
        m.exit();
    }

    // ── Context: делегирование к родителю через upper ────────────────────────

    #[test]
    fn test_get_value_delegates_to_upper_extend() {
        let mut parent_vars = HashMap::new();
        parent_vars.insert("x".to_string(), Value::Number(99));
        let parent = Rc::new(RefCell::new(Unit::Extend {
            upper: None,
            always: vec![],
            enters: vec![],
            exits: vec![],
            variables: parent_vars,
        }));
        let child = Unit::Extend {
            upper: Some(parent),
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
        let parent = Rc::new(RefCell::new(Unit::Extend {
            upper: None,
            always: vec![],
            enters: vec![],
            exits: vec![],
            variables: parent_vars,
        }));
        let mut child_vars = HashMap::new();
        child_vars.insert("x".to_string(), Value::Number(42));
        let child = Unit::Extend {
            upper: Some(parent),
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
            states: HashMap::new(),
            current: "S".to_string(),
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
            states: HashMap::new(),
            current: "S".to_string(),
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
        states.insert("S".to_string(), RefCell::new(extend()));
        let mut m = Unit::Plain {
            upper: None,
            transitions: vec![],
            state_transitions: HashMap::new(),
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

    #[test]
    fn test_extend_always_never_called_on_tick() {
        // Extend.always никогда не выполняется: tick() возвращает Goto немедленно
        let (count, exec) = call_counter();
        let mut m = Unit::Extend {
            upper: None,
            always: vec![exec],
            enters: vec![],
            exits: vec![],
            variables: HashMap::new(),
        };
        m.tick().unwrap();
        m.tick().unwrap();
        assert_eq!(count.get(), 0);
    }

    // ── from_model: построение из семантической модели ───────────────────────

    use grammar::parse;
    use grammar::semantic::tree::construct_model;

    /// Двусостояниная FSM: A → B (безусловный переход).
    #[test]
    fn test_from_model_two_state_fsm() {
        // start A { ref B; } — состояние A с безусловным переходом в B
        let src = "start A { ref B; } state B;";
        let (ast, _) = parse(src, 0).unwrap();
        let model = construct_model(&ast, None, &[]).unwrap();
        let mut unit = from_model(model).unwrap();

        // Начальное состояние — A
        let Unit::Plain { current, .. } = &unit else {
            panic!("ожидался Unit::Plain");
        };
        assert_eq!(current, "A");

        // После одного тика — переход в B (безусловный)
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
        // В B нет переходов → Goto
        let result = unit.tick().unwrap();
        assert!(matches!(result, Transition::Goto));
    }

    /// Последовательная композиция A + B → Unit::Sequence.
    #[test]
    fn test_from_model_concatenation_builds_sequence() {
        let src = "model A { start S; } model B { start S; } start Entry = A + B;";
        let (ast, _) = parse(src, 0).unwrap();
        let model = construct_model(&ast, None, &[]).unwrap();
        let entry = model.borrow().search_model("A").unwrap(); // использован для проверки
        let _ = entry;

        let root = model.borrow();
        let entry_state = root.search_state("Entry").unwrap();
        let state_node = entry_state.borrow();
        if let StateNode::Implement { implements, .. } = &*state_node {
            let unit = from_extend(implements).unwrap();
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
            let unit = from_extend(implements).unwrap();
            assert!(matches!(unit, Unit::Parallel { .. }));
        } else {
            panic!("Entry должен быть StateNode::Implement");
        }
    }

    /// Копии при повторном использовании: A + A → Sequence из двух независимых Unit.
    #[test]
    fn test_from_model_copies_are_independent() {
        let src = "model A { start S; } start Entry = A + A;";
        let (ast, _) = parse(src, 0).unwrap();
        let model = construct_model(&ast, None, &[]).unwrap();
        let root = model.borrow();
        let entry_state = root.search_state("Entry").unwrap();
        let state_node = entry_state.borrow();
        if let StateNode::Implement { implements, .. } = &*state_node {
            let unit = from_extend(implements).unwrap();
            // Должны быть ДВЕ отдельные копии в Sequence
            let Unit::Sequence { units, .. } = &unit else {
                panic!("ожидался Unit::Sequence");
            };
            assert_eq!(units.len(), 2);
            // Убеждаемся, что это разные Rc (независимые копии)
            assert!(!Rc::ptr_eq(&units[0], &units[1]));
        } else {
            panic!("Entry должен быть StateNode::Implement");
        }
    }

    /// state_transitions обновляются при смене состояния.
    #[test]
    fn test_state_transitions_updated_on_change() {
        // Три состояния: A → B (всегда), B → C (всегда)
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

        // В C нет переходов → Goto
        assert!(matches!(unit.tick().unwrap(), Transition::Goto));
    }
}
