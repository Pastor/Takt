use crate::context::Context;
use grammar::semantic::minimap::Name;

pub(crate) type Predicate = Box<dyn Fn(&dyn Context) -> bool>;
pub(crate) type Execution = Box<dyn Fn(&mut dyn Context)>;

/// Состояние автомата в процессе симуляции.
pub(crate) struct State {
    transitions: Vec<(Name, Predicate)>,
    always: Vec<Execution>,
    exits: Vec<Execution>,
    enter: Vec<Execution>,
}

impl State {
    pub(crate) fn next(&self, c: &dyn Context) -> Option<Name> {
        self.transitions
            .iter()
            .find(|(_, p)| p(c))
            .map(|(name, _)| name.clone())
    }

    pub(crate) fn always(&self, c: &mut dyn Context) {
        for exec in &self.always {
            exec(c);
        }
    }

    pub(crate) fn exits(&self, c: &mut dyn Context) {
        for exec in &self.exits {
            exec(c);
        }
    }

    pub(crate) fn enter(&self, c: &mut dyn Context) {
        for exec in &self.enter {
            exec(c);
        }
    }

    pub(crate) fn is_final(&self) -> bool {
        self.transitions.is_empty()
    }
}
