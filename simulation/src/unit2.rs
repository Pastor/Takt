use crate::context::Context;
use crate::value::Value;
use std::cell::RefCell;
use std::rc::Rc;

#[derive(Clone, Default)]
enum Unit {
    #[default]
    None,
    Node {
        context: Option<Rc<RefCell<dyn Context>>>,
    },
    Parallel {
        units: Vec<Rc<RefCell<Unit>>>,
    },
    Sequential {
        units: Vec<Rc<RefCell<Unit>>>,
        index: usize,
    },
}

impl Context for Unit {
    fn get_value(&self, name: &str) -> Option<Value> {
        match self {
            Unit::None => None,
            Unit::Node { context } => {
                context.as_ref().and_then(|ctx| ctx.borrow().get_value(name))
            }
            Unit::Parallel { units } => {
                units.iter().find_map(|unit| unit.borrow().get_value(name))
            }
            Unit::Sequential { units, index } => {
                units.get(*index).and_then(|u| u.borrow().get_value(name))
            }
        }
    }
}

impl Unit {
    pub fn is_terminal(&self) -> bool {
        match self {
            Unit::None => true,
            Unit::Node { .. } => true, //TODO: Необходимо реализовать определение завершения модели
            Unit::Parallel { units, .. } | Unit::Sequential { units, .. } => {
                units.iter().all(|u| u.borrow().is_terminal())
            }
        }
    }

    pub fn union(&self, other: &Self) -> Self {
        match self.clone() {
            Unit::None => other.clone(),
            Unit::Node { .. } => self.union_parallel(other),
            Unit::Parallel { mut units, .. } => {
                if let Unit::Parallel {
                    units: other_units, ..
                } = other
                {
                    units.append(&mut other_units.clone());
                } else {
                    units.push(Rc::new(RefCell::new(other.clone())));
                }
                Unit::Parallel { units }
            }
            Unit::Sequential { .. } => self.union_parallel(other),
        }
    }

    fn union_parallel(&self, other: &Unit) -> Unit {
        if let Unit::Parallel { units: other_units } = other {
            let mut units = other_units.clone();
            units.insert(0, Rc::new(RefCell::new(self.clone())));
            Unit::Parallel { units }
        } else {
            Unit::Parallel {
                units: vec![
                    Rc::new(RefCell::new(self.clone())),
                    Rc::new(RefCell::new(other.clone())),
                ],
            }
        }
    }

    pub fn add(&self, other: &Self) -> Self {
        match self.clone() {
            Unit::None => other.clone(),
            Unit::Node { .. } => {
                let mut units = if let Unit::Sequential { units, .. } = other.clone() {
                    units
                } else {
                    vec![Rc::new(RefCell::new(other.clone()))]
                };
                units.insert(0, Rc::new(RefCell::new(self.clone())));
                Unit::Sequential { units, index: 0 }
            }
            Unit::Parallel { .. } => {
                let units = vec![
                    Rc::new(RefCell::new(self.clone())),
                    Rc::new(RefCell::new(other.clone())),
                ];
                Unit::Sequential { units, index: 0 }
            }
            Unit::Sequential { mut units, .. } => {
                if let Unit::Sequential {
                    units: mut other_units,
                    ..
                } = other.clone()
                {
                    units.append(&mut other_units)
                } else {
                    units.push(Rc::new(RefCell::new(other.clone())));
                }
                Unit::Sequential { units, index: 0 }
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::collections::HashMap;

    struct MockCtx(HashMap<String, Value>);

    impl Context for MockCtx {
        fn get_value(&self, name: &str) -> Option<Value> {
            self.0.get(name).cloned()
        }
    }

    fn ctx_with(key: &str, val: Value) -> Rc<RefCell<dyn Context>> {
        let mut m = HashMap::new();
        m.insert(key.to_string(), val);
        Rc::new(RefCell::new(MockCtx(m)))
    }

    fn node() -> Unit {
        Unit::Node { context: None }
    }

    fn node_with(key: &str, val: Value) -> Unit {
        Unit::Node {
            context: Some(ctx_with(key, val)),
        }
    }

    fn parallel(units: Vec<Unit>) -> Unit {
        Unit::Parallel {
            units: units
                .into_iter()
                .map(|u| Rc::new(RefCell::new(u)))
                .collect(),
        }
    }

    fn sequential(units: Vec<Unit>) -> Unit {
        Unit::Sequential {
            units: units
                .into_iter()
                .map(|u| Rc::new(RefCell::new(u)))
                .collect(),
            index: 0,
        }
    }

    fn len(u: &Unit) -> usize {
        match u {
            Unit::Parallel { units, .. } | Unit::Sequential { units, .. } => units.len(),
            _ => 0,
        }
    }

    #[test]
    fn test_union_none_with_node() {
        let result = Unit::None.union(&node());
        assert!(matches!(result, Unit::Node { .. }));
    }

    #[test]
    fn test_union_node_with_node() {
        let result = node().union(&node());
        assert!(matches!(result, Unit::Parallel { .. }));
        assert_eq!(len(&result), 2);
    }

    #[test]
    fn test_union_node_with_parallel() {
        // Node | Parallel([a, b]) → self вставляется в начало → Parallel([Node, a, b])
        let result = node().union(&parallel(vec![node(), node()]));
        assert!(matches!(result, Unit::Parallel { .. }));
        assert_eq!(len(&result), 3);
    }

    #[test]
    fn test_union_parallel_with_parallel() {
        let result = parallel(vec![node()]).union(&parallel(vec![node(), node()]));
        assert!(matches!(result, Unit::Parallel { .. }));
        assert_eq!(len(&result), 3);
    }

    #[test]
    fn test_union_parallel_with_node() {
        let result = parallel(vec![node()]).union(&node());
        assert!(matches!(result, Unit::Parallel { .. }));
        assert_eq!(len(&result), 2);
    }

    #[test]
    fn test_union_sequential_with_node() {
        let result = sequential(vec![node()]).union(&node());
        assert!(matches!(result, Unit::Parallel { .. }));
        assert_eq!(len(&result), 2);
    }

    #[test]
    fn test_union_sequential_with_parallel() {
        // Sequential | Parallel([a, b]) → self вставляется в начало → Parallel([Seq, a, b])
        let result = sequential(vec![node()]).union(&parallel(vec![node(), node()]));
        assert!(matches!(result, Unit::Parallel { .. }));
        assert_eq!(len(&result), 3);
    }

    #[test]
    fn test_add_none_with_node() {
        let result = Unit::None.add(&node());
        assert!(matches!(result, Unit::Node { .. }));
    }

    #[test]
    fn test_add_node_with_node() {
        let result = node().add(&node());
        assert!(matches!(result, Unit::Sequential { .. }));
        assert_eq!(len(&result), 2);
    }

    #[test]
    fn test_add_node_with_sequential() {
        // Node + Sequential([a, b]) → Sequential([Node, a, b]) — выравнивание
        let result = node().add(&sequential(vec![node(), node()]));
        assert!(matches!(result, Unit::Sequential { .. }));
        assert_eq!(len(&result), 3);
    }

    #[test]
    fn test_add_sequential_with_sequential() {
        // Sequential([a]) + Sequential([b, c]) → Sequential([a, b, c])
        let result = sequential(vec![node()]).add(&sequential(vec![node(), node()]));
        assert!(matches!(result, Unit::Sequential { .. }));
        assert_eq!(len(&result), 3);
    }

    #[test]
    fn test_add_parallel_with_node() {
        let result = parallel(vec![node(), node()]).add(&node());
        assert!(matches!(result, Unit::Sequential { .. }));
        assert_eq!(len(&result), 2);
    }

    // ── Context ──────────────────────────────────────────────────────────────

    #[test]
    fn test_context_none_returns_none() {
        assert!(Unit::None.get_value("x").is_none());
    }

    #[test]
    fn test_context_node_without_ctx_returns_none() {
        assert!(node().get_value("x").is_none());
    }

    #[test]
    fn test_context_node_with_ctx_returns_value() {
        let u = node_with("x", Value::Number(42));
        assert!(matches!(u.get_value("x"), Some(Value::Number(42))));
    }

    #[test]
    fn test_context_node_wrong_key_returns_none() {
        let u = node_with("x", Value::Number(1));
        assert!(u.get_value("y").is_none());
    }

    #[test]
    fn test_context_parallel_finds_in_children() {
        let u = parallel(vec![node_with("a", Value::Boolean(true)), node()]);
        assert!(matches!(u.get_value("a"), Some(Value::Boolean(true))));
    }

    #[test]
    fn test_context_parallel_missing_returns_none() {
        let u = parallel(vec![node_with("a", Value::Number(1))]);
        assert!(u.get_value("b").is_none());
    }

    #[test]
    fn test_context_sequential_reads_active_index() {
        // index=0: первый элемент имеет "a", второй — "b"
        let u = sequential(vec![
            node_with("a", Value::Number(10)),
            node_with("b", Value::Number(20)),
        ]);
        assert!(matches!(u.get_value("a"), Some(Value::Number(10))));
        // "b" находится в index=1, который ещё не активен
        assert!(u.get_value("b").is_none());
    }

    // ── is_terminal ──────────────────────────────────────────────────────────

    #[test]
    fn test_is_terminal_none() {
        assert!(Unit::None.is_terminal());
    }

    #[test]
    fn test_is_terminal_node() {
        assert!(node().is_terminal());
    }

    #[test]
    fn test_is_terminal_parallel_all_terminal() {
        assert!(parallel(vec![node(), Unit::None]).is_terminal());
    }

    #[test]
    fn test_is_terminal_sequential_all_terminal() {
        assert!(sequential(vec![node(), node()]).is_terminal());
    }
}
