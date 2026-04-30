use crate::context::Context;

pub(crate) type Predicate = Box<dyn Fn(&dyn Context) -> bool>;
pub(crate) type Execution = Box<dyn Fn(&mut dyn Context)>;

use crate::value::Value;
use grammar::diagnostics::{Diagnostic, Location};
use std::cell::RefCell;
use std::collections::HashMap;
use std::rc::Rc;

enum Transition {
    Processing,
    Goto,
    Final,
}

enum Machine {
    Plain {
        upper: Option<Rc<RefCell<Machine>>>,
        transitions: Vec<(String, Predicate)>,
        states: HashMap<String, RefCell<Machine>>,
        current: String,

        always: Vec<Execution>,
        enters: Vec<Execution>,
        exits: Vec<Execution>,

        variables: HashMap<String, Value>,
    },
    Extend {
        upper: Option<Rc<RefCell<Machine>>>,
        always: Vec<Execution>,
        enters: Vec<Execution>,
        exits: Vec<Execution>,

        variables: HashMap<String, Value>,
    },
}

impl Context for Machine {
    fn get_value(&self, name: &str) -> Option<Value> {
        let result = match self {
            Machine::Plain { variables, .. } => variables.get(name).cloned(),
            Machine::Extend { variables, .. } => variables.get(name).cloned(),
        };
        result.or_else(|| {
            let upper = match self {
                Machine::Plain { upper, .. } => upper.as_ref().map(|u| &**u),
                Machine::Extend { upper, .. } => upper.as_ref().map(|u| &**u),
            };
            upper.and_then(|u| u.borrow().get_value(name))
        })
    }
}

impl Machine {
    fn enter(&mut self) {
        // Временно забираем enters, чтобы освободить заимствование self для замыканий
        let enters = match self {
            Machine::Plain { enters, .. } | Machine::Extend { enters, .. } => {
                std::mem::take(enters)
            }
        };
        for f in &enters {
            f(self as &mut dyn Context);
        }
        match self {
            Machine::Plain { enters: e, .. } | Machine::Extend { enters: e, .. } => *e = enters,
        }
    }

    fn exit(&mut self) {
        // Временно забираем exits, чтобы освободить заимствование self для замыканий
        let exits = match self {
            Machine::Plain { exits, .. } | Machine::Extend { exits, .. } => std::mem::take(exits),
        };
        for f in &exits {
            f(self as &mut dyn Context);
        }
        match self {
            Machine::Plain { exits: e, .. } | Machine::Extend { exits: e, .. } => *e = exits,
        }
    }

    fn tick(&mut self) -> Result<Transition, Diagnostic> {
        // Extend: нет внутренней логики — сигнализируем о готовности к переходам
        if matches!(self, Machine::Extend { .. }) {
            return Ok(Transition::Goto);
        }

        // Временно забираем always, вызываем и возвращаем обратно
        let always = match self {
            Machine::Plain { always, .. } => std::mem::take(always),
            Machine::Extend { .. } => unreachable!(),
        };
        for f in &always {
            f(self as &mut dyn Context);
        }
        match self {
            Machine::Plain { always: a, .. } => *a = always,
            Machine::Extend { .. } => unreachable!(),
        }

        // Тикаем текущее вложенное состояние; RefMut освобождается в конце блока
        let result = {
            let Machine::Plain {
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
            Machine::Plain { transitions, .. } => std::mem::take(transitions),
            Machine::Extend { .. } => unreachable!(),
        };
        let next_name = transitions
            .iter()
            .find(|(_, predicate)| predicate(&*self))
            .map(|(name, _)| name.clone());
        match self {
            Machine::Plain { transitions: t, .. } => *t = transitions,
            Machine::Extend { .. } => unreachable!(),
        }

        let Some(next) = next_name else {
            return Ok(Transition::Goto);
        };

        // Переход: выходим из текущего, меняем current, входим в новое
        let Machine::Plain {
            current, states, ..
        } = &mut *self
        else {
            unreachable!()
        };
        let cur = current.clone();
        if let Some(state) = states.get(cur.as_str()) {
            state.borrow_mut().exit();
        }
        *current = next.clone();
        let next_machine = states.get(next.as_str()).ok_or_else(|| {
            Diagnostic::error(Location::Builtin, "Состояние не найдено".to_string())
        })?;
        next_machine.borrow_mut().enter();

        Ok(Transition::Processing)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::cell::Cell;
    use std::collections::HashMap;
    use std::rc::Rc;

    fn always_true() -> Predicate {
        Box::new(|_| true)
    }

    fn always_false() -> Predicate {
        Box::new(|_| false)
    }

    /// Счётчик вызовов через Rc<Cell>, доступный из Fn-замыкания.
    fn call_counter() -> (Rc<Cell<u32>>, Execution) {
        let count = Rc::new(Cell::new(0u32));
        let c = count.clone();
        (count, Box::new(move |_| c.set(c.get() + 1)))
    }

    /// Создаёт Machine::Extend без действий.
    fn extend() -> Machine {
        Machine::Extend {
            upper: None,
            always: vec![],
            enters: vec![],
            exits: vec![],
            variables: HashMap::new(),
        }
    }

    /// Создаёт Machine::Plain с одним дочерним Extend-состоянием и заданными переходами.
    fn plain_with_extend(name: &str, transitions: Vec<(String, Predicate)>) -> Machine {
        let mut states = HashMap::new();
        states.insert(name.to_string(), RefCell::new(extend()));
        Machine::Plain {
            upper: None,
            transitions,
            states,
            current: name.to_string(),
            always: vec![],
            enters: vec![],
            exits: vec![],
            variables: HashMap::new(),
        }
    }

    // ── Machine::Extend ──────────────────────────────────────────────────────

    #[test]
    fn test_extend_tick_returns_goto() {
        let mut m = extend();
        assert!(matches!(m.tick().unwrap(), Transition::Goto));
    }

    #[test]
    fn test_extend_enter_calls_enters() {
        let (count, exec) = call_counter();
        let mut m = Machine::Extend {
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
        let mut m = Machine::Extend {
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

    // ── Machine::Plain: отсутствующее текущее состояние ─────────────────────

    #[test]
    fn test_plain_missing_current_state_returns_final() {
        let mut m = Machine::Plain {
            upper: None,
            transitions: vec![("B".to_string(), always_true())],
            states: HashMap::new(),
            current: "A".to_string(),
            always: vec![],
            enters: vec![],
            exits: vec![],
            variables: HashMap::new(),
        };
        assert!(matches!(m.tick().unwrap(), Transition::Final));
    }

    // ── Machine::Plain: always выполняется каждый тик ───────────────────────

    #[test]
    fn test_plain_always_called_each_tick() {
        let (count, exec) = call_counter();
        let mut states = HashMap::new();
        states.insert("S".to_string(), RefCell::new(extend()));
        let mut m = Machine::Plain {
            upper: None,
            transitions: vec![],
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

    // ── Machine::Plain: дочерний Extend (Goto) ───────────────────────────────

    #[test]
    fn test_plain_child_goto_no_transitions_returns_goto() {
        // Extend возвращает Goto; переходов нет → Plain тоже возвращает Goto
        let mut m = plain_with_extend("S", vec![]);
        assert!(matches!(m.tick().unwrap(), Transition::Goto));
    }

    #[test]
    fn test_plain_child_goto_false_predicate_returns_goto() {
        // Предикат ложен → переход не срабатывает → Goto
        let mut m = plain_with_extend("S", vec![("T".to_string(), always_false())]);
        assert!(matches!(m.tick().unwrap(), Transition::Goto));
    }

    // ── Machine::Plain: срабатывание перехода ────────────────────────────────

    #[test]
    fn test_plain_transition_fires_returns_processing() {
        let mut m = plain_with_extend("S", vec![("T".to_string(), always_true())]);
        if let Machine::Plain { states, .. } = &mut m {
            states.insert("T".to_string(), RefCell::new(extend()));
        }
        assert!(matches!(m.tick().unwrap(), Transition::Processing));
    }

    #[test]
    fn test_plain_transition_fires_updates_current() {
        let mut m = plain_with_extend("S", vec![("T".to_string(), always_true())]);
        if let Machine::Plain { states, .. } = &mut m {
            states.insert("T".to_string(), RefCell::new(extend()));
        }
        m.tick().unwrap();
        if let Machine::Plain { current, .. } = &m {
            assert_eq!(current, "T");
        } else {
            panic!("ожидался Machine::Plain");
        }
    }

    #[test]
    fn test_plain_transition_skips_false_takes_first_true() {
        // Первый предикат ложен, второй истинен → переход в "U"
        let transitions = vec![
            ("T".to_string(), always_false()),
            ("U".to_string(), always_true()),
        ];
        let mut m = plain_with_extend("S", transitions);
        if let Machine::Plain { states, .. } = &mut m {
            states.insert("T".to_string(), RefCell::new(extend()));
            states.insert("U".to_string(), RefCell::new(extend()));
        }
        m.tick().unwrap();
        if let Machine::Plain { current, .. } = &m {
            assert_eq!(current, "U");
        } else {
            panic!("ожидался Machine::Plain");
        }
    }

    #[test]
    fn test_plain_transition_target_missing_returns_error() {
        // Переход в несуществующее состояние → Err
        let mut m = plain_with_extend("S", vec![("MISSING".to_string(), always_true())]);
        assert!(m.tick().is_err());
    }

    // ── exits/enters при переходе ─────────────────────────────────────────────

    #[test]
    fn test_plain_transition_calls_exit_then_enter() {
        let (exit_count, exit_exec) = call_counter();
        let (enter_count, enter_exec) = call_counter();

        let mut states: HashMap<String, RefCell<Machine>> = HashMap::new();
        states.insert(
            "S".to_string(),
            RefCell::new(Machine::Extend {
                upper: None,
                always: vec![],
                enters: vec![],
                exits: vec![exit_exec],
                variables: HashMap::new(),
            }),
        );
        states.insert(
            "T".to_string(),
            RefCell::new(Machine::Extend {
                upper: None,
                always: vec![],
                enters: vec![enter_exec],
                exits: vec![],
                variables: HashMap::new(),
            }),
        );

        let mut m = Machine::Plain {
            upper: None,
            transitions: vec![("T".to_string(), always_true())],
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

    // ── Machine::Plain: дочерний Final не запускает переходы ────────────────

    #[test]
    fn test_plain_child_final_propagates_without_checking_transitions() {
        // Дочерний Plain с отсутствующим состоянием → Final → родитель тоже Final,
        // даже если у него есть переход с истинным предикатом
        let inner = Machine::Plain {
            upper: None,
            transitions: vec![],
            states: HashMap::new(),
            current: "MISSING".to_string(),
            always: vec![],
            enters: vec![],
            exits: vec![],
            variables: HashMap::new(),
        };
        let mut states = HashMap::new();
        states.insert("S".to_string(), RefCell::new(inner));
        let mut m = Machine::Plain {
            upper: None,
            transitions: vec![("T".to_string(), always_true())],
            states,
            current: "S".to_string(),
            always: vec![],
            enters: vec![],
            exits: vec![],
            variables: HashMap::new(),
        };
        assert!(matches!(m.tick().unwrap(), Transition::Final));
    }

    // ── Machine::Plain: Processing от дочернего ──────────────────────────────

    #[test]
    fn test_plain_child_processing_propagates() {
        // Дочерняя Plain-машина сама переходит (Processing) → родитель тоже Processing
        let mut inner_states = HashMap::new();
        inner_states.insert("inner_s".to_string(), RefCell::new(extend()));
        inner_states.insert("inner_t".to_string(), RefCell::new(extend()));
        let inner = Machine::Plain {
            upper: None,
            transitions: vec![("inner_t".to_string(), always_true())],
            states: inner_states,
            current: "inner_s".to_string(),
            always: vec![],
            enters: vec![],
            exits: vec![],
            variables: HashMap::new(),
        };

        let mut outer_states = HashMap::new();
        outer_states.insert("S".to_string(), RefCell::new(inner));
        let mut m = Machine::Plain {
            upper: None,
            transitions: vec![],
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
        let m = Machine::Plain {
            upper: None,
            transitions: vec![],
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
        let m = Machine::Extend {
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
        // Переход срабатывает когда переменная "ready" == Number(1)
        let pred: Predicate =
            Box::new(|ctx| matches!(ctx.get_value("ready"), Some(Value::Number(1))));

        let mut vars = HashMap::new();
        vars.insert("ready".to_string(), Value::Number(1));

        let mut states = HashMap::new();
        states.insert("S".to_string(), RefCell::new(extend()));
        states.insert("T".to_string(), RefCell::new(extend()));

        let mut m = Machine::Plain {
            upper: None,
            transitions: vec![("T".to_string(), pred)],
            states,
            current: "S".to_string(),
            always: vec![],
            enters: vec![],
            exits: vec![],
            variables: vars,
        };

        m.tick().unwrap();
        if let Machine::Plain { current, .. } = &m {
            assert_eq!(current, "T");
        } else {
            panic!("ожидался Machine::Plain");
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
        // Переменная "x" задана в родителе, дочерний Extend делегирует поиск вверх
        let mut parent_vars = HashMap::new();
        parent_vars.insert("x".to_string(), Value::Number(99));
        let parent = Rc::new(RefCell::new(Machine::Extend {
            upper: None,
            always: vec![],
            enters: vec![],
            exits: vec![],
            variables: parent_vars,
        }));
        let child = Machine::Extend {
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
        // Контрпример: локальная переменная перекрывает значение из родителя
        let mut parent_vars = HashMap::new();
        parent_vars.insert("x".to_string(), Value::Number(1));
        let parent = Rc::new(RefCell::new(Machine::Extend {
            upper: None,
            always: vec![],
            enters: vec![],
            exits: vec![],
            variables: parent_vars,
        }));
        let mut child_vars = HashMap::new();
        child_vars.insert("x".to_string(), Value::Number(42));
        let child = Machine::Extend {
            upper: Some(parent),
            always: vec![],
            enters: vec![],
            exits: vec![],
            variables: child_vars,
        };
        assert!(matches!(child.get_value("x"), Some(Value::Number(42))));
    }

    // ── Множественные enters/exits/always ────────────────────────────────────

    #[test]
    fn test_plain_multiple_enters_all_called() {
        let (c1, e1) = call_counter();
        let (c2, e2) = call_counter();
        let (c3, e3) = call_counter();
        let mut m = Machine::Plain {
            upper: None,
            transitions: vec![],
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
        let mut m = Machine::Plain {
            upper: None,
            transitions: vec![],
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
        let mut m = Machine::Plain {
            upper: None,
            transitions: vec![],
            states,
            current: "S".to_string(),
            always: vec![e1, e2],
            enters: vec![],
            exits: vec![],
            variables: HashMap::new(),
        };
        m.tick().unwrap();
        m.tick().unwrap();
        // оба замыкания вызываются на каждом тике
        assert_eq!(c1.get(), 2);
        assert_eq!(c2.get(), 2);
    }

    #[test]
    fn test_extend_always_never_called_on_tick() {
        // Extend.always никогда не выполняется: tick() возвращает Goto немедленно
        let (count, exec) = call_counter();
        let mut m = Machine::Extend {
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
}
