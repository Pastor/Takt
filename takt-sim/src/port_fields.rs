//! Поля портов модели для ручного ввода и наблюдения.
//!
//! Страница прогона строит поле ввода по роду порта, а языка не знает: род, границы
//! и варианты перечисления даёт эталон, тем же обходом моделей, что реестр имён
//! (`PortNames`). Двусмысленное имя отдаётся квалифицированными формами - иначе
//! ручная запись разошлась бы по всем ветвям.

use std::collections::BTreeMap;

use serde::Serialize;
use takt_lang::parser::ast::PortDirection;
use takt_lang::semantic::type_node::TypeNode;
use takt_lang::semantic::{ModelNode, VariableNode};

/// Поле одного порта.
#[derive(Debug, Clone, PartialEq, Serialize)]
pub struct PortField {
    /// Имя, которым порт адресует сценарий: голое либо `Модель::имя`.
    pub name: String,
    /// `in`, `out` либо `inout`.
    pub direction: &'static str,
    /// Тип, как его пишет автор модели.
    #[serde(rename = "type")]
    pub ty: String,
    /// Род поля ввода - плоско рядом с именем: `{"kind": "integer", "min": …}`.
    #[serde(flatten)]
    pub kind: FieldKind,
}

/// Род поля ввода.
#[derive(Debug, Clone, PartialEq, Serialize)]
#[serde(tag = "kind", rename_all = "snake_case")]
pub enum FieldKind {
    /// Переключатель: `0` либо `1`.
    Bit,
    /// Переключатель: `false` либо `true`.
    Bool,
    /// Целое в границах типа.
    Integer { min: String, max: String },
    /// Длительность: число миллисекунд, как у сценария.
    Duration,
    /// `q(m, n)`: вещественное число.
    Fixed { m: u8, n: u8, sat: bool },
    /// `float`.
    Float,
    /// Перечисление: значение варианта - число, как у сценария.
    Enum { variants: Vec<Variant> },
    /// Составной порт: наблюдается, но не вводится целиком.
    Composite,
}

/// Вариант перечисления.
#[derive(Debug, Clone, PartialEq, Serialize)]
pub struct Variant {
    pub name: String,
    pub value: String,
}

/// Поля всех портов модели и её под-моделей в порядке имён.
///
/// Границы - строками: `u64` и `i64` в число JSON без потери не укладываются.
pub fn port_fields(model: &ModelNode) -> Vec<PortField> {
    let mut found: Vec<(Option<String>, String, PortField)> = Vec::new();
    collect(model, &mut found);
    let mut owners: BTreeMap<String, Vec<Option<String>>> = BTreeMap::new();
    for (owner, bare, _) in &found {
        let slot = owners.entry(bare.clone()).or_default();
        if !slot.contains(owner) {
            slot.push(owner.clone());
        }
    }
    let mut fields: BTreeMap<String, PortField> = BTreeMap::new();
    for (owner, bare, mut field) in found {
        let ambiguous = owners.get(&bare).is_some_and(|list| list.len() > 1);
        field.name = match (&owner, ambiguous) {
            (Some(owner), true) => format!("{owner}::{bare}"),
            _ => bare,
        };
        fields.entry(field.name.clone()).or_insert(field);
    }
    fields.into_values().collect()
}

fn collect(model: &ModelNode, found: &mut Vec<(Option<String>, String, PortField)>) {
    for (name, var) in &model.variables {
        let VariableNode::Port { direction, .. } = var else {
            continue;
        };
        let direction = match direction {
            PortDirection::In => "in",
            PortDirection::Out => "out",
            PortDirection::InOut => "inout",
        };
        let ty = var.ty();
        found.push((
            model.name.clone(),
            name.clone(),
            PortField {
                name: name.clone(),
                direction,
                ty: ty.to_string(),
                kind: kind_of(model, ty),
            },
        ));
    }
    for sub in model.models.values() {
        collect(&sub.borrow(), found);
    }
}

fn kind_of(model: &ModelNode, ty: &TypeNode) -> FieldKind {
    match ty {
        TypeNode::Bit => FieldKind::Bit,
        TypeNode::Bool => FieldKind::Bool,
        TypeNode::Duration => FieldKind::Duration,
        TypeNode::Rational => FieldKind::Float,
        TypeNode::Fixed { m, n, sat } => FieldKind::Fixed {
            m: *m,
            n: *n,
            sat: *sat,
        },
        TypeNode::Enum(name) => match model.search_enum(name) {
            Some(definition) => FieldKind::Enum {
                variants: definition
                    .variants
                    .iter()
                    .map(|(name, value)| Variant {
                        name: name.clone(),
                        value: value.to_string(),
                    })
                    .collect(),
            },
            None => FieldKind::Composite,
        },
        // Бит-вектор эталон держит массивом разрядов, а не числом: поле целого дало
        // бы значение другой формы. Он наблюдается, как прочие составные.
        TypeNode::Integer { .. } => match takt_lang::semantic::type_node::type_range(ty) {
            Some((min, max)) => FieldKind::Integer {
                min: min.to_string(),
                max: max.to_string(),
            },
            None => FieldKind::Composite,
        },
        _ => FieldKind::Composite,
    }
}
