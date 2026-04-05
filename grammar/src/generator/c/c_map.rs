use crate::diagnostics::Diagnostic;
use crate::semantic::ModelNode;
use crate::semantic::minimap::{Element, Map, Name};
use crate::semantic::naming::normalize_camelcase_name;
use std::cell::RefCell;
use std::rc::Rc;

pub struct CMap {
    filename: String,
    map: Map,
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

    pub fn own_model(&self) -> Option<Element> {
        self.map.own()
    }

    pub fn start(&self) -> Name {
        self.map.start()
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
