use crate::context::Context;
use crate::unit::Predicate;
use crate::value::Value;
use grammar::semantic::minimap::Name;
use std::cell::RefCell;
use std::collections::HashMap;
use std::rc::Rc;

/// Snapshot изменяемый слепок модели, у каждой модели компановки свой слепок
struct Snapshot {
    upper: Option<Rc<RefCell<Snapshot>>>,
    states: HashMap<Name, (Name, Predicate)>,
    current_state: Name,
    variables: HashMap<String, Value>,
    ports: HashMap<String, Value>,
}

impl Context for Snapshot {
    fn get_value(&self, name: &str) -> Option<Value> {
        if let Some(value) = self.variables.get(name) {
            Some(value.clone())
        } else if let Some(upper) = &self.upper {
            upper.borrow().get_value(name)
        } else {
            None
        }
    }

    fn set_value(&mut self, name: &str, value: Value) {
        self.variables.insert(name.to_string(), value);
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use grammar::parse;
    use grammar::semantic::minimap::Map;
    use grammar::semantic::tree::construct_model;

    /// Создаёт `Name` для состояния `state` в анонимной корневой модели.
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

    /// Создаёт пустой `Snapshot` без переменных и без родителя.
    fn empty(current: Name) -> Snapshot {
        Snapshot {
            upper: None,
            states: HashMap::new(),
            current_state: current,
            variables: HashMap::new(),
            ports: HashMap::new(),
        }
    }

    /// Создаёт `Snapshot` с одной переменной `key = value`.
    fn with_var(current: Name, key: &str, value: Value) -> Snapshot {
        let mut variables = HashMap::new();
        variables.insert(key.to_string(), value);
        Snapshot {
            upper: None,
            states: HashMap::new(),
            current_state: current,
            variables,
            ports: HashMap::new(),
        }
    }

    // --- get_variable: локальный поиск ---

    #[test]
    fn test_not_found_in_empty_snapshot() {
        let name = make_name("S");
        assert!(empty(name.clone()).get_value("S").is_none());
    }

    #[test]
    fn test_not_found_different_key() {
        // Контрпример: сохранено S, запрашиваем T
        let s = make_name("S");
        let snap = with_var(s.clone(), "S", Value::Number(1));
        assert!(snap.get_value("T").is_none());
    }

    #[test]
    fn test_found_number() {
        let name = make_name("S");
        let snap = with_var(name.clone(), "S", Value::Number(42));
        assert!(matches!(snap.get_value("S"), Some(Value::Number(42))));
    }

    #[test]
    fn test_found_boolean() {
        let name = make_name("S");
        let snap = with_var(name.clone(), "S", Value::Boolean(true));
        assert!(matches!(snap.get_value("S"), Some(Value::Boolean(true))));
    }

    #[test]
    fn test_found_real() {
        let name = make_name("S");
        let snap = with_var(name.clone(), "S", Value::Real(2.71));
        let Some(Value::Real(v)) = snap.get_value("S") else {
            panic!("ожидалось Some(Real)");
        };
        assert!((v - 2.71).abs() < 1e-9);
    }

    #[test]
    fn test_returns_clone_not_reference() {
        // Два вызова возвращают независимые значения
        let name = make_name("S");
        let mut snap = with_var(name.clone(), "S", Value::Number(10));
        let v1 = snap.get_value("S").unwrap();
        // Перезаписываем переменную
        snap.variables.insert("S".to_string(), Value::Number(99));
        let v2 = snap.get_value("S").unwrap();
        assert!(matches!(v1, Value::Number(10)));
        assert!(matches!(v2, Value::Number(99)));
    }

    // --- get_variable: поиск через upper ---

    #[test]
    fn test_found_in_upper_scope() {
        let s = make_name("S");
        // parent хранит T=99, child не хранит T
        let parent = Rc::new(RefCell::new(with_var(s.clone(), "T", Value::Number(99))));
        let child = Snapshot {
            upper: Some(parent),
            states: HashMap::new(),
            current_state: s,
            variables: HashMap::new(),
            ports: HashMap::new(),
        };
        assert!(matches!(child.get_value("T"), Some(Value::Number(99))));
    }

    #[test]
    fn test_local_shadows_upper() {
        let s = make_name("S");
        // parent: T=1, child: T=99 — локальное значение перекрывает
        let parent = Rc::new(RefCell::new(with_var(s.clone(), "T", Value::Number(1))));
        let mut child_vars = HashMap::new();
        child_vars.insert("T".to_string(), Value::Number(99));
        let child = Snapshot {
            upper: Some(parent),
            states: HashMap::new(),
            current_state: s,
            variables: child_vars,
            ports: HashMap::new(),
        };
        assert!(matches!(child.get_value("T"), Some(Value::Number(99))));
    }

    #[test]
    fn test_not_found_in_upper_either() {
        // Контрпример: переменная отсутствует в обоих уровнях
        let s = make_name("S");
        let parent = Rc::new(RefCell::new(empty(s.clone())));
        let child = Snapshot {
            upper: Some(parent),
            states: HashMap::new(),
            current_state: s,
            variables: HashMap::new(),
            ports: HashMap::new(),
        };
        assert!(child.get_value("T").is_none());
    }

    #[test]
    fn test_deep_chain_three_levels() {
        // grandparent → parent → child: поиск доходит до третьего уровня
        let s = make_name("S");
        let grandparent = Rc::new(RefCell::new(with_var(s.clone(), "T", Value::Number(7))));
        let parent = Rc::new(RefCell::new(Snapshot {
            upper: Some(grandparent),
            states: HashMap::new(),
            current_state: s.clone(),
            variables: HashMap::new(),
            ports: HashMap::new(),
        }));
        let child = Snapshot {
            upper: Some(parent),
            states: HashMap::new(),
            current_state: s,
            variables: HashMap::new(),
            ports: HashMap::new(),
        };
        assert!(matches!(child.get_value("T"), Some(Value::Number(7))));
    }
}
