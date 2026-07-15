//! Снимок семантической карты модели для генератора Structured Text.
//!
//! Обёртка над [`Map`] из [`crate::semantic::minimap`] по образцу
//! [`CMap`](crate::generator::c) — снимок дерева плюс множество используемых
//! имён ([`UsageSet`]), чтобы не эмитить в `FUNCTION_BLOCK` объявления, которых
//! модель не использует.

use crate::diagnostics::{Diagnostic, Location};
use crate::semantic::ModelNode;
use crate::semantic::minimap::{Element, Map, Name};
use crate::semantic::unused::UsageSet;
use std::cell::RefCell;
use std::rc::Rc;

/// Снимок модели, подготовленный для генерации Structured Text.
pub(crate) struct StMap {
    filename: String,
    map: Map,
    /// Множество используемых имён модели (для фильтрации неиспользуемых элементов).
    usage: UsageSet,
    /// Режим потребления адресов (`st-at`): эмитить `AT %…` у портов.
    ///
    /// Соответствует [`GenerateOptions::hal`](crate::generator::GenerateOptions::hal);
    /// потребляется задачей 0041-05, здесь только переносится в снимок.
    at_addresses: bool,
}

impl StMap {
    /// Строит снимок модели из семантического дерева.
    ///
    /// # Ошибки
    /// Возвращает [`Diagnostic`], если у модели нет стартового состояния.
    pub fn new(filename: &str, model: &ModelNode, at_addresses: bool) -> Result<Self, Diagnostic> {
        let model_rc = Rc::new(RefCell::new(model.copy(None, None)));
        let usage = crate::semantic::unused::compute_usage(Rc::clone(&model_rc));
        Ok(Self {
            filename: filename.to_string(),
            map: Map::create(model_rc)?,
            usage,
            at_addresses,
        })
    }

    /// Возвращает базовое имя файла (без расширения).
    pub fn get_filename(&self) -> &str {
        &self.filename
    }

    /// Возвращает имя корневой модели.
    pub fn root_name(&self) -> Name {
        self.map.root_name()
    }

    /// Возвращает элемент корневой модели (`Element::Model`).
    pub fn model(&self) -> Element {
        self.map.model()
    }

    /// Возвращает список имён состояний корневой модели.
    #[allow(dead_code)]
    pub fn states(&self) -> Vec<Name> {
        self.map.states()
    }

    /// Возвращает элемент состояния по имени, или `None`, если не найден либо не
    /// является состоянием.
    #[allow(dead_code)]
    pub fn state_at(&self, name: Name) -> Option<Element> {
        if let Some(element) = self.map.element_at(name)
            && element.is_state()
        {
            Some(element)
        } else {
            None
        }
    }

    /// Возвращает список подмоделей, используемых через `StateExtend`.
    pub fn using_models(&self) -> Vec<Element> {
        self.map.used_models()
    }

    /// Возвращает корневую модель (только для чтения).
    #[allow(dead_code)]
    pub(crate) fn root_model_node(&self) -> Option<Rc<RefCell<ModelNode>>> {
        self.map.model_at(None)
    }

    /// Возвращает модель по имени.
    ///
    /// # Ошибки
    /// [`Diagnostic`] с кодом `ST-012`, если модели с таким именем нет.
    pub(crate) fn raw_model_at(&self, name: Name) -> Result<Rc<RefCell<ModelNode>>, Diagnostic> {
        self.map
            .model_at(Some(name.unique().to_string()))
            .ok_or_else(|| {
                Diagnostic::error(Location::Codegen, format!("Модель '{}' не найдена", name))
                    .with_code("ST-012")
            })
    }

    /// Возвращает ссылку на множество используемых имён модели.
    #[allow(dead_code)]
    pub fn usage(&self) -> &UsageSet {
        &self.usage
    }

    /// Включён ли режим потребления адресов (`st-at`).
    #[allow(dead_code)]
    pub fn at_addresses(&self) -> bool {
        self.at_addresses
    }
}
