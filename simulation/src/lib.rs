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
// Крейт находится в стадии разработки: модули объявлены, но ещё не задействованы.
#![allow(dead_code)]

mod context;
mod execution;
mod predicate;
mod snapshot;
mod state;
mod value;

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
