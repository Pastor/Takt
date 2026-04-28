use crate::context::Context;
use crate::execution::Execution;
use crate::predicate::Predicate;
use grammar::semantic::minimap::Name;

/// Состояние автомата в процессе симуляции.
enum State {
    /// Обычное состояние: список переходов и действий.
    Simple {
        transitions: Vec<(Name, Predicate)>,
        always: Vec<Box<dyn Execution>>,
        exits: Vec<Box<dyn Execution>>,
    },
    /// Состояние с реализацией: последовательность действий и переход после завершения.
    Extend {
        sequence: Vec<Box<dyn Execution>>,
        next: Name,
        always: Vec<Box<dyn Execution>>,
        exits: Vec<Box<dyn Execution>>,
    },
}

impl Execution for State {
    fn execute(&self, _context: &mut dyn Context) {
        unimplemented!()
    }

    fn is_final(&self) -> bool {
        unimplemented!()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use grammar::parse;
    use grammar::semantic::minimap::Map;
    use grammar::semantic::tree::construct_model;

    fn make_name(state: &str) -> Name {
        let (ast, _) = parse(&format!("start {};", state), 0).unwrap();
        let model = construct_model(&ast, None, &[]).unwrap();
        Map::create(model)
            .unwrap()
            .states()
            .into_iter()
            .next()
            .unwrap()
    }

    /// Заглушка `Execution` для заполнения полей `always`/`exits`/`sequence`.
    struct NoopExec;
    impl Execution for NoopExec {
        fn execute(&self, _: &mut dyn Context) {}
        fn is_final(&self) -> bool {
            false
        }
    }

    #[test]
    fn test_simple_state_empty_transitions() {
        let s = make_name("S");
        let state = State::Simple {
            transitions: vec![],
            always: vec![],
            exits: vec![],
        };
        drop(s);
        assert!(matches!(state, State::Simple { .. }));
    }

    #[test]
    fn test_simple_state_with_transition() {
        let s = make_name("S");
        let t = make_name("T");
        let state = State::Simple {
            transitions: vec![(t, Predicate {})],
            always: vec![Box::new(NoopExec)],
            exits: vec![Box::new(NoopExec)],
        };
        drop(s);
        assert!(matches!(state, State::Simple { .. }));
    }

    #[test]
    fn test_extend_state_construction() {
        let s = make_name("S");
        let state = State::Extend {
            sequence: vec![Box::new(NoopExec)],
            next: s,
            always: vec![],
            exits: vec![],
        };
        assert!(matches!(state, State::Extend { .. }));
    }

    #[test]
    fn test_state_is_execution() {
        // State реализует Execution, поэтому его можно поместить в Box<dyn Execution>
        // (методы unimplemented!(), поэтому вызывать их не будем)
        let s = make_name("S");
        let _: Box<dyn Execution> = Box::new(State::Simple {
            transitions: vec![(s, Predicate {})],
            always: vec![],
            exits: vec![],
        });
    }
}
