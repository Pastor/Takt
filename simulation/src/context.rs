use crate::value::Value;

/// Контекст выполнения: предоставляет доступ к переменным текущей области видимости.
pub(crate) trait Context {
    /// Возвращает клонированное значение переменной или `None`, если переменная не найдена.
    fn get_value(&self, name: &str) -> Option<Value>;
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::value::Value;
    use std::collections::HashMap;

    struct MockContext {
        vars: HashMap<String, Value>,
    }

    impl Context for MockContext {
        fn get_value(&self, name: &str) -> Option<Value> {
            self.vars.get(name).cloned()
        }
    }

    #[test]
    fn test_context_not_found() {
        let ctx = MockContext {
            vars: HashMap::new(),
        };
        assert!(ctx.get_value("S").is_none());
    }

    #[test]
    fn test_context_found_number() {
        let mut vars = HashMap::new();
        vars.insert("S".to_string(), Value::Number(5));
        let ctx = MockContext { vars };
        assert!(matches!(ctx.get_value("S"), Some(Value::Number(5))));
    }

    #[test]
    fn test_context_found_boolean() {
        let mut vars = HashMap::new();
        vars.insert("S".to_string(), Value::Boolean(false));
        let ctx = MockContext { vars };
        assert!(matches!(ctx.get_value("S"), Some(Value::Boolean(false))));
    }

    #[test]
    fn test_context_different_key_not_found() {
        // Контрпример: запрашиваем имя T, а сохранено S
        let mut vars = HashMap::new();
        vars.insert("S".to_string(), Value::Number(1));
        let ctx = MockContext { vars };
        assert!(ctx.get_value("T").is_none());
    }

    #[test]
    fn test_context_via_dyn() {
        // Трейт-объект: dyn Context диспетчеризуется корректно
        let mut vars = HashMap::new();
        vars.insert("S".to_string(), Value::Number(99));
        let ctx: Box<dyn Context> = Box::new(MockContext { vars });
        assert!(matches!(ctx.get_value("S"), Some(Value::Number(99))));
    }
}
