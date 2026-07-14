//! Крейт симуляции моделей языка Lam.
//!
//! Предоставляет инструменты для выполнения (симуляции) моделей,
//! построенных семантическим анализатором крейта [`grammar`].
//!
//! # Использование
//!
//! ```rust,ignore
//! use grammar::parse;
//! use grammar::semantic::{minimap::Map, tree::construct_model};
//!
//! let (ast, _) = parse("start S;", 0).unwrap();
//! let model = construct_model(&ast, None, &[]).unwrap();
//! let map = Map::create(model).unwrap();
//! ```
mod context;
pub(crate) mod eval;
pub(crate) mod gif;
pub mod graphics_config;
pub mod json_input;
mod predicate;
pub mod runner;
pub mod state_io;
pub(crate) mod svg;
mod unit;

/// Строит дерево симуляции из семантической модели.
pub fn build_unit(
    model: std::rc::Rc<std::cell::RefCell<grammar::semantic::ModelNode>>,
) -> Result<unit::Unit, grammar::diagnostics::Diagnostic> {
    unit::builder::build(model)
}

#[cfg(test)]
mod tests {
    use grammar::parse;
    use grammar::semantic::minimap::Map;
    use grammar::semantic::tree::construct_model;

    #[test]
    fn test_access() {
        let (model, _) = parse("start E;", 0).unwrap();
        let model = construct_model(&model, None, &[]).unwrap();
        let map = Map::create(model.clone()).unwrap();
        let raw = &*model.borrow();
        assert!(raw.search_state("E").is_some());
        map.states().iter().for_each(|state| {
            assert!(raw.search_state(state.local()).is_some());
        });
        assert!(map.used_models().is_empty());
    }
}
