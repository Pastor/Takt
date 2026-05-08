use crate::context::Context;

pub(crate) trait Execution {
    /// Выполняет шаг действия в заданном контексте.
    fn execute(&self, context: &mut dyn Context);
    /// Возвращает `true`, если действие завершено.
    fn is_final(&self) -> bool;
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::value::Value;

    struct MockCtx;
    impl Context for MockCtx {
        fn get_value(&self, _: &str) -> Option<Value> {
            None
        }

        fn set_value(&mut self, _: &str, _: Value) {}
    }

    /// Реализация, которая всегда завершена.
    struct AlwaysDone;
    impl Execution for AlwaysDone {
        fn execute(&self, _: &mut dyn Context) {}
        fn is_final(&self) -> bool {
            true
        }
    }

    /// Реализация, которая никогда не завершается.
    struct NeverDone;
    impl Execution for NeverDone {
        fn execute(&self, _: &mut dyn Context) {}
        fn is_final(&self) -> bool {
            false
        }
    }

    #[test]
    fn test_is_final_true() {
        assert!(AlwaysDone.is_final());
    }

    #[test]
    fn test_is_final_false() {
        assert!(!NeverDone.is_final());
    }

    #[test]
    fn test_execute_does_not_panic2() {
        let mut ctx = MockCtx;
        let always = AlwaysDone {};
        always.execute(&mut ctx);
        let never = NeverDone;
        never.execute(&mut ctx);
    }

    #[test]
    fn test_execute_does_not_panic() {
        let mut ctx = MockCtx;
        AlwaysDone.execute(&mut ctx);
        NeverDone.execute(&mut ctx);
    }

    #[test]
    fn test_execution_via_dyn() {
        // Трейт-объект: dyn Execution корректно диспетчеризуется
        let boxed: Box<dyn Execution> = Box::new(AlwaysDone);
        assert!(boxed.is_final());
        let boxed: Box<dyn Execution> = Box::new(NeverDone);
        assert!(!boxed.is_final());
    }
}
