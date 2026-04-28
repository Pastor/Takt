use crate::context::Context;

#[derive(Debug)]
pub(crate) struct Predicate {}

impl Predicate {
    /// Вычисляет предикат в заданном контексте. Возвращает `true`, если условие выполнено.
    pub(crate) fn test(&self, _context: &impl Context) -> bool {
        true
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::value::Value;
    use grammar::semantic::minimap::Name;

    struct MockCtx;
    impl Context for MockCtx {
        fn get_variable(&self, _: &Name) -> Option<Value> {
            None
        }
    }

    #[test]
    fn test_predicate_stub_always_true() {
        // Текущая реализация — заглушка, всегда истинна
        let p = Predicate {};
        assert!(p.test(&MockCtx));
    }

    #[test]
    fn test_predicate_test_multiple_calls() {
        // Повторные вызовы дают тот же результат
        let p = Predicate {};
        assert!(p.test(&MockCtx));
        assert!(p.test(&MockCtx));
    }

    #[test]
    fn test_predicate_debug() {
        let s = format!("{:?}", Predicate {});
        assert!(!s.is_empty());
    }
}
