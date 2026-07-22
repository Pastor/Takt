use crate::eval::error::EvalError;
use crate::eval::value::Value;
use grammar::semantic::StructDefinitionNode;
use grammar::semantic::type_node::TypeNode;
use std::collections::HashMap;

/// Контекст выполнения: предоставляет доступ к переменным текущей области видимости.
pub(crate) trait Context {
    /// Возвращает клонированное значение переменной или `None`, если переменная не найдена.
    fn get_value(&self, name: &str) -> Option<Value>;
    /// Устанавливает значение переменной в текущей области видимости.
    fn set_value(&mut self, name: &str, value: Value);
    /// Определение структурного типа по имени (фича 0034) — для приведения
    /// инициализатора `{…}` к `Value::Struct`. Умолчание — `None` (контекст без
    /// модели, напр. мок в тестах): структур нет. Контексты над моделью
    /// переопределяют, делегируя `ModelNode::search_struct` (учитывает родителей),
    /// а вложенные области — своему `outer`.
    fn find_struct(&self, _name: &str) -> Option<StructDefinitionNode> {
        None
    }
    /// Перечисляет значения, составляющие состояние модели (для снимка, фича 0032).
    ///
    /// Включает значения родительских контекстов: для параллельных подмоделей
    /// родитель общий, и его переменные — часть их состояния. Константы и
    /// локальные переменные не включаются (см. анализ 0032). Реализация по
    /// умолчанию пуста — её достаточно для контекстов без собственного состояния.
    fn dump(&self) -> HashMap<String, Value> {
        HashMap::new()
    }
}

/// Приводит значение к типу цели, используя реестр структур из контекста
/// (фича 0034). Мост между слоем адаптеров (у которых есть `Context`) и ядром
/// [`crate::eval::coerce_to_type_with`] (которому нужен `StructRegistry`).
pub(crate) fn coerce_via(
    ctx: &dyn Context,
    value: Value,
    ty: &TypeNode,
) -> Result<Value, EvalError> {
    struct Reg<'a>(&'a dyn Context);
    impl crate::eval::StructRegistry for Reg<'_> {
        fn find_struct(&self, name: &str) -> Option<StructDefinitionNode> {
            self.0.find_struct(name)
        }
    }
    crate::eval::coerce_to_type_with(value, ty, &Reg(ctx))
}

/// Обновляет значение по пути сегментов (`p.x := …`, `data[i] := …`), используя
/// реестр структур из контекста (фичи 0034, 0076). `ty` — объявленный тип
/// корневой переменной (для приведения листа к типу поля/элемента). Мост к ядру
/// [`crate::eval::place::update`].
pub(crate) fn update_place_via(
    ctx: &dyn Context,
    value: Value,
    ty: Option<&TypeNode>,
    path: &[crate::eval::place::PlaceSegment],
    new: Value,
) -> Result<Value, EvalError> {
    struct Reg<'a>(&'a dyn Context);
    impl crate::eval::StructRegistry for Reg<'_> {
        fn find_struct(&self, name: &str) -> Option<StructDefinitionNode> {
            self.0.find_struct(name)
        }
    }
    crate::eval::place::update(value, ty, path, new, &Reg(ctx))
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::eval::value::Value;
    use std::collections::HashMap;

    struct MockContext {
        vars: HashMap<String, Value>,
    }

    impl Context for MockContext {
        fn get_value(&self, name: &str) -> Option<Value> {
            self.vars.get(name).cloned()
        }

        fn set_value(&mut self, name: &str, value: Value) {
            self.vars.insert(name.to_string(), value);
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
