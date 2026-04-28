use crate::value::Value;
use grammar::semantic::minimap::Name;

/// Контекст выполнения: предоставляет доступ к переменным текущей области видимости.
pub(crate) trait Context {
    /// Возвращает клонированное значение переменной или `None`, если переменная не найдена.
    fn get_variable(&self, name: &Name) -> Option<Value>;
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::value::Value;
    use grammar::parse;
    use grammar::semantic::minimap::Map;
    use grammar::semantic::tree::construct_model;
    use std::collections::HashMap;

    struct MockContext {
        vars: HashMap<Name, Value>,
    }

    impl Context for MockContext {
        fn get_variable(&self, name: &Name) -> Option<Value> {
            self.vars.get(name).cloned()
        }
    }

    fn make_name(state: &str) -> Name {
        let (ast, _) = parse(&format!("start {};", state), 0).unwrap();
        let model = construct_model(&ast, None, &[]).unwrap();
        Map::create(model).unwrap().states().into_iter().next().unwrap()
    }

    #[test]
    fn test_context_not_found() {
        let ctx = MockContext { vars: HashMap::new() };
        assert!(ctx.get_variable(&make_name("S")).is_none());
    }

    #[test]
    fn test_context_found_number() {
        let name = make_name("S");
        let mut vars = HashMap::new();
        vars.insert(name.clone(), Value::Number(5));
        let ctx = MockContext { vars };
        assert!(matches!(ctx.get_variable(&name), Some(Value::Number(5))));
    }

    #[test]
    fn test_context_found_boolean() {
        let name = make_name("S");
        let mut vars = HashMap::new();
        vars.insert(name.clone(), Value::Boolean(false));
        let ctx = MockContext { vars };
        assert!(matches!(ctx.get_variable(&name), Some(Value::Boolean(false))));
    }

    #[test]
    fn test_context_different_key_not_found() {
        // Контрпример: запрашиваем имя T, а сохранено S
        let s = make_name("S");
        let t = make_name("T");
        let mut vars = HashMap::new();
        vars.insert(s.clone(), Value::Number(1));
        let ctx = MockContext { vars };
        assert!(ctx.get_variable(&t).is_none());
    }

    #[test]
    fn test_context_via_dyn() {
        // Трейт-объект: dyn Context диспетчеризуется корректно
        let name = make_name("S");
        let mut vars = HashMap::new();
        vars.insert(name.clone(), Value::Number(99));
        let ctx: Box<dyn Context> = Box::new(MockContext { vars });
        assert!(matches!(ctx.get_variable(&name), Some(Value::Number(99))));
    }
}
