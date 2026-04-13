use crate::diagnostics::{Diagnostic, Location};
use crate::semantic::minimap::{Element, Map, Name};
use crate::semantic::naming::normalize_camelcase_name;
use crate::semantic::{ModelNode, StateNode};
use std::cell::RefCell;
use std::rc::Rc;

pub struct CMap {
    filename: String,
    map: Map,
}

impl CMap {
    pub(crate) fn raw_model_at(&self, name: Name) -> Result<Rc<RefCell<ModelNode>>, Diagnostic> {
        Ok(self
            .map
            .model_at(Some(name.unique().to_string()))
            .ok_or_else(|| {
                Diagnostic::error(
                    Location::Codegen,
                    format!("Model with name '{}' not found", name),
                )
            })?)
    }

    pub(crate) fn raw_state_at(&self, name: Name) -> Result<Rc<RefCell<StateNode>>, Diagnostic> {
        Ok(self
            .map
            .state_at(Some(name.unique().to_string()))
            .ok_or_else(|| {
                Diagnostic::error(
                    Location::Codegen,
                    format!("State with name '{}' not found", name),
                )
            })?)
    }
}

impl CMap {
    pub fn new(filename: &str, model: &ModelNode) -> Result<Self, Diagnostic> {
        Ok(Self {
            filename: filename.to_string(),
            map: Map::create(Rc::new(RefCell::new(model.copy(None, None))))?,
        })
    }

    pub fn get_filename(&self) -> &str {
        &self.filename
    }

    /// Возвращает имя корневой структуры в PascalCase (например, `ElevatorEngine`).
    #[allow(dead_code)]
    pub fn get_struct_name(&self) -> String {
        let name = self
            .map
            .model_at(None)
            .unwrap()
            .borrow()
            .name
            .clone()
            .unwrap();
        normalize_camelcase_name(&name)
    }

    pub fn using_models(&self) -> Vec<Element> {
        self.map.used_models()
    }

    /// Возвращает элемент корневой модели из карты (если есть).
    #[allow(dead_code)]
    pub fn own_model(&self) -> Option<Element> {
        self.map.own()
    }

    /// Возвращает имя стартового состояния корневой модели.
    #[allow(dead_code)]
    pub fn start(&self) -> Name {
        let Element::Model { start, .. } = self.map.model().clone() else {
            unreachable!()
        };
        start
    }

    pub(crate) fn model(&self) -> Element {
        self.map.model()
    }

    pub fn state_at(&self, name: Name) -> Option<Element> {
        if let Some(element) = self.map.element_at(name)
            && element.is_state()
        {
            Some(element)
        } else {
            None
        }
    }

    pub fn root_name(&self) -> Name {
        self.map.root_name()
    }

    pub fn states(&self) -> Vec<Name> {
        self.map.states()
    }
}
