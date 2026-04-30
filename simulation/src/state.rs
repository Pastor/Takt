use crate::context::Context;

pub(crate) type Predicate = Box<dyn Fn(&dyn Context) -> bool>;
pub(crate) type Execution = Box<dyn Fn(&mut dyn Context)>;

use grammar::diagnostics::{Diagnostic, Location};
use std::cell::RefCell;
use std::collections::HashMap;

enum Transition {
    Processing,
    Goto,
    Final,
}

enum Machine {
    Plain {
        transitions: Vec<(String, Predicate)>,
        states: HashMap<String, RefCell<Machine>>,
        current: String,

        always: Vec<Execution>,
        enters: Vec<Execution>,
        exits: Vec<Execution>,
    },
    Extend {
        always: Vec<Execution>,
        enters: Vec<Execution>,
        exits: Vec<Execution>,
    },
}

impl Machine {
    fn enter(&mut self, _c: &mut dyn Context) {
        let enters = match self {
            Machine::Plain { enters, .. } => enters,
            Machine::Extend { enters, .. } => enters,
        };
        enters.iter().for_each(|f| f(_c));
    }

    fn exit(&mut self, _c: &mut dyn Context) {
        let exits = match self {
            Machine::Plain { exits, .. } => exits,
            Machine::Extend { exits, .. } => exits,
        };
        exits.iter().for_each(|f| f(_c));
    }

    fn tick(&mut self, c: &mut dyn Context) -> Result<Transition, Diagnostic> {
        let Machine::Plain {
            transitions,
            states,
            current,
            always,
            ..
        } = self
        else {
            // Machine::Extend: делегирует другой модели; без подмодели — проверяем переходы
            return Ok(Transition::Goto);
        };
        always.iter().for_each(|f| f(c));

        // Тикаем текущее вложенное состояние; RefMut освобождается в конце блока
        let result = {
            let Some(state) = states.get(current.as_str()) else {
                // Текущее состояние не найдено — машина завершена
                return Ok(Transition::Final);
            };
            state.borrow_mut().tick(c)?
        };

        match result {
            Transition::Processing => return Ok(Transition::Processing),
            Transition::Final => return Ok(Transition::Final),
            Transition::Goto => {}
        }

        // Ищем первый переход, чей предикат истинен
        let next_name = transitions
            .iter()
            .find(|(_, predicate)| predicate(c))
            .map(|(name, _)| name.clone());

        let Some(next) = next_name else {
            return Ok(Transition::Goto);
        };

        // Выходим из текущего состояния (borrow ограничен блоком)
        if let Some(state) = states.get(current.as_str()) {
            state.borrow_mut().exit(c);
        }

        // Обновляем текущее состояние через мутабельную ссылку на поле
        *current = next.clone();

        // Входим в новое состояние
        let next_machine = states.get(next.as_str()).ok_or_else(|| {
            Diagnostic::error(Location::Builtin, "Состояние не найдено".to_string())
        })?;
        next_machine.borrow_mut().enter(c);

        Ok(Transition::Processing)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::value::Value;
    use std::cell::Cell;
    use std::collections::HashMap;
    use std::rc::Rc;

    struct MockCtx;

    impl Context for MockCtx {
        fn get_variable(&self, _name: &str) -> Option<Value> {
            None
        }
    }

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
            always: vec![],
            enters: vec![],
            exits: vec![],
        }
    }

    /// Создаёт Machine::Plain с одним дочерним Extend-состоянием и заданными переходами.
    fn plain_with_extend(name: &str, transitions: Vec<(String, Predicate)>) -> Machine {
        let mut states = HashMap::new();
        states.insert(name.to_string(), RefCell::new(extend()));
        Machine::Plain {
            transitions,
            states,
            current: name.to_string(),
            always: vec![],
            enters: vec![],
            exits: vec![],
        }
    }

    // ── Machine::Extend ──────────────────────────────────────────────────────

    #[test]
    fn test_extend_tick_returns_goto() {
        let mut m = extend();
        assert!(matches!(m.tick(&mut MockCtx).unwrap(), Transition::Goto));
    }

    #[test]
    fn test_extend_enter_calls_enters() {
        let (count, exec) = call_counter();
        let mut m = Machine::Extend {
            always: vec![],
            enters: vec![exec],
            exits: vec![],
        };
        m.enter(&mut MockCtx);
        assert_eq!(count.get(), 1);
    }

    #[test]
    fn test_extend_exit_calls_exits() {
        let (count, exec) = call_counter();
        let mut m = Machine::Extend {
            always: vec![],
            enters: vec![],
            exits: vec![exec],
        };
        m.exit(&mut MockCtx);
        assert_eq!(count.get(), 1);
    }

    #[test]
    fn test_extend_enter_exit_empty_do_not_panic() {
        let mut m = extend();
        m.enter(&mut MockCtx);
        m.exit(&mut MockCtx);
    }

    // ── Machine::Plain: отсутствующее текущее состояние ─────────────────────

    #[test]
    fn test_plain_missing_current_state_returns_final() {
        let mut m = Machine::Plain {
            transitions: vec![("B".to_string(), always_true())],
            states: HashMap::new(),
            current: "A".to_string(),
            always: vec![],
            enters: vec![],
            exits: vec![],
        };
        assert!(matches!(m.tick(&mut MockCtx).unwrap(), Transition::Final));
    }

    // ── Machine::Plain: always выполняется каждый тик ───────────────────────

    #[test]
    fn test_plain_always_called_each_tick() {
        let (count, exec) = call_counter();
        let mut states = HashMap::new();
        states.insert("S".to_string(), RefCell::new(extend()));
        let mut m = Machine::Plain {
            transitions: vec![],
            states,
            current: "S".to_string(),
            always: vec![exec],
            enters: vec![],
            exits: vec![],
        };
        m.tick(&mut MockCtx).unwrap();
        m.tick(&mut MockCtx).unwrap();
        assert_eq!(count.get(), 2);
    }

    // ── Machine::Plain: дочерний Extend (Goto) ───────────────────────────────

    #[test]
    fn test_plain_child_goto_no_transitions_returns_goto() {
        // Extend возвращает Goto; переходов нет → Plain тоже возвращает Goto
        let mut m = plain_with_extend("S", vec![]);
        assert!(matches!(m.tick(&mut MockCtx).unwrap(), Transition::Goto));
    }

    #[test]
    fn test_plain_child_goto_false_predicate_returns_goto() {
        // Предикат ложен → переход не срабатывает → Goto
        let mut m = plain_with_extend("S", vec![("T".to_string(), always_false())]);
        assert!(matches!(m.tick(&mut MockCtx).unwrap(), Transition::Goto));
    }

    // ── Machine::Plain: срабатывание перехода ────────────────────────────────

    #[test]
    fn test_plain_transition_fires_returns_processing() {
        let mut m = plain_with_extend("S", vec![("T".to_string(), always_true())]);
        if let Machine::Plain { states, .. } = &mut m {
            states.insert("T".to_string(), RefCell::new(extend()));
        }
        assert!(matches!(
            m.tick(&mut MockCtx).unwrap(),
            Transition::Processing
        ));
    }

    #[test]
    fn test_plain_transition_fires_updates_current() {
        let mut m = plain_with_extend("S", vec![("T".to_string(), always_true())]);
        if let Machine::Plain { states, .. } = &mut m {
            states.insert("T".to_string(), RefCell::new(extend()));
        }
        m.tick(&mut MockCtx).unwrap();
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
        m.tick(&mut MockCtx).unwrap();
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
        assert!(m.tick(&mut MockCtx).is_err());
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
                always: vec![],
                enters: vec![],
                exits: vec![exit_exec],
            }),
        );
        states.insert(
            "T".to_string(),
            RefCell::new(Machine::Extend {
                always: vec![],
                enters: vec![enter_exec],
                exits: vec![],
            }),
        );

        let mut m = Machine::Plain {
            transitions: vec![("T".to_string(), always_true())],
            states,
            current: "S".to_string(),
            always: vec![],
            enters: vec![],
            exits: vec![],
        };

        m.tick(&mut MockCtx).unwrap();
        assert_eq!(exit_count.get(), 1, "exit состояния S должен вызваться");
        assert_eq!(enter_count.get(), 1, "enter состояния T должен вызваться");
    }

    // ── Machine::Plain: дочерний Final не запускает переходы ────────────────

    #[test]
    fn test_plain_child_final_propagates_without_checking_transitions() {
        // Дочерний Plain с отсутствующим состоянием → Final → родитель тоже Final,
        // даже если у него есть переход с истинным предикатом
        let inner = Machine::Plain {
            transitions: vec![],
            states: HashMap::new(),
            current: "MISSING".to_string(),
            always: vec![],
            enters: vec![],
            exits: vec![],
        };
        let mut states = HashMap::new();
        states.insert("S".to_string(), RefCell::new(inner));
        let mut m = Machine::Plain {
            transitions: vec![("T".to_string(), always_true())],
            states,
            current: "S".to_string(),
            always: vec![],
            enters: vec![],
            exits: vec![],
        };
        assert!(matches!(m.tick(&mut MockCtx).unwrap(), Transition::Final));
    }

    // ── Machine::Plain: Processing от дочернего ──────────────────────────────

    #[test]
    fn test_plain_child_processing_propagates() {
        // Дочерняя Plain-машина сама переходит (Processing) → родитель тоже Processing
        let mut inner_states = HashMap::new();
        inner_states.insert("inner_s".to_string(), RefCell::new(extend()));
        inner_states.insert("inner_t".to_string(), RefCell::new(extend()));
        let inner = Machine::Plain {
            transitions: vec![("inner_t".to_string(), always_true())],
            states: inner_states,
            current: "inner_s".to_string(),
            always: vec![],
            enters: vec![],
            exits: vec![],
        };

        let mut outer_states = HashMap::new();
        outer_states.insert("S".to_string(), RefCell::new(inner));
        let mut m = Machine::Plain {
            transitions: vec![],
            states: outer_states,
            current: "S".to_string(),
            always: vec![],
            enters: vec![],
            exits: vec![],
        };
        assert!(matches!(
            m.tick(&mut MockCtx).unwrap(),
            Transition::Processing
        ));
    }

    // ── enter/exit не паникуют ────────────────────────────────────────────────

    #[test]
    fn test_plain_enter_exit_do_not_panic() {
        let mut m = plain_with_extend("S", vec![]);
        m.enter(&mut MockCtx);
        m.exit(&mut MockCtx);
    }
}
