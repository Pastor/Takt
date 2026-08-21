//! Цикл структур — `SE-124` (фича 0352).
//!
//! # Зачем
//!
//! `struct A { b: B }` и `struct B { a: A }` описывают тип **бесконечного
//! размера**: значение `A` содержит `B`, которое содержит `A`. Ни один
//! потребитель такого типа не имеет — в C, IEC 61131-3 и RTL структура
//! разворачивается по значению, и объявления отвергают их же инструменты.
//!
//! До фичи 0352 запись была недостижима иначе: одна из двух структур всегда
//! ссылалась **вперёд** и получала `TypeNode::Unsupported`, то есть цикл не
//! складывался — падало раньше и по другой причине. Сняв зависимость от
//! порядка объявления, фича сделала цикл разрешимым, поэтому диагностика
//! введена **той же** фичей: иначе правка породила бы вывод, отвергаемый
//! чужими инструментами при нулевом коде возврата `taktc`.
//!
//! # Почему по сырому АСД
//!
//! Проверка идёт **до** построения полей, тем же приёмом, что `SE-039` (Ce16)
//! для псевдонимов: в построенном дереве поле несёт `TypeNode::Struct(имя)`, и
//! цикл там тоже виден, но диагностика нужна раньше — до того, как модель
//! уедет к потребителям.
//!
//! ⚠️ **Узлами графа служат и псевдонимы:** `type Ref = A;` в поле структуры
//! `B` замыкает цикл через псевдоним, и без этих рёбер класс просочился бы.
//! Чисто псевдонимный цикл остаётся за `SE-039` — она проверяется раньше.

use crate::diagnostics::{Diagnostic, Location};
use crate::parser::ast::{self, ModelElement};
use std::collections::{BTreeMap, BTreeSet};

/// Имена типов, на которые ссылается сырой АСД-тип (включая элемент массива).
///
/// ⚠️ Спуск в массив обязателен: `struct A { xs: [A; 2] }` — тот же цикл,
/// только через элемент.
fn deps_of(ty: &ast::Type) -> Vec<String> {
    match ty {
        ast::Type::Alias(id) => vec![id.name.clone()],
        ast::Type::Array { element_type, .. } => deps_of(element_type),
        ast::Type::Struct(name) | ast::Type::Enum(name) => vec![name.clone()],
        _ => vec![],
    }
}

/// Граф «имя типа → имена, от которых он зависит по значению».
struct Graph {
    edges: BTreeMap<String, Vec<String>>,
    /// Позиция объявления — для диагностики.
    locs: BTreeMap<String, Location>,
    /// Какие имена являются структурами: цикл без структуры судит `SE-039`.
    structs: BTreeSet<String>,
}

fn build_graph(elements: &[ModelElement]) -> Graph {
    let mut edges: BTreeMap<String, Vec<String>> = BTreeMap::new();
    let mut locs: BTreeMap<String, Location> = BTreeMap::new();
    let mut structs: BTreeSet<String> = BTreeSet::new();
    for element in elements {
        match element {
            ModelElement::Struct(s) => {
                let Some(id) = s.name.as_ref() else { continue };
                structs.insert(id.name.clone());
                locs.insert(id.name.clone(), id.loc);
                let deps = s.fields.iter().flat_map(|f| deps_of(&f.ty)).collect();
                edges.insert(id.name.clone(), deps);
            }
            ModelElement::Type(def) => {
                locs.insert(def.name.name.clone(), def.name.loc);
                edges.insert(def.name.name.clone(), deps_of(&def.ty));
            }
            _ => {}
        }
    }
    Graph {
        edges,
        locs,
        structs,
    }
}

/// Ищет цикл, достижимый из `current`; возвращает имя структуры-участника.
///
/// Возвращается именно **структура**: цикл может пройти через псевдоним, но
/// сообщать автору надо о записи, которую он вправе исправить, а чисто
/// псевдонимный цикл судит `SE-039`.
fn find_cycle(
    current: &str,
    graph: &Graph,
    visited: &mut BTreeSet<String>,
    stack: &mut Vec<String>,
) -> Option<String> {
    if let Some(at) = stack.iter().position(|n| n == current) {
        // Участники цикла — хвост стека начиная с `at`.
        return stack[at..]
            .iter()
            .find(|n| graph.structs.contains(*n))
            .cloned();
    }
    if visited.contains(current) {
        return None;
    }
    stack.push(current.to_string());
    if let Some(deps) = graph.edges.get(current) {
        for dep in deps {
            if let Some(found) = find_cycle(dep, graph, visited, stack) {
                stack.pop();
                return Some(found);
            }
        }
    }
    stack.pop();
    visited.insert(current.to_string());
    None
}

/// Проверяет объявления модели на циклические структуры (`SE-124`).
///
/// Отдаёт **первую** находку: остальные — её следствия по тому же графу.
pub(crate) fn check_struct_cycles(elements: &[ModelElement]) -> Option<Diagnostic> {
    let graph = build_graph(elements);
    if graph.structs.is_empty() {
        return None;
    }
    let mut visited: BTreeSet<String> = BTreeSet::new();
    for name in graph.structs.iter() {
        let mut stack = Vec::new();
        if let Some(culprit) = find_cycle(name, &graph, &mut visited, &mut stack) {
            let loc = graph
                .locs
                .get(&culprit)
                .copied()
                .unwrap_or(Location::Implicit);
            return Some(
                Diagnostic::error(
                    loc,
                    format!(
                        "структура '{culprit}' содержит себя — прямо либо через цепочку \
                         полей: размер такого значения бесконечен, и представления у него \
                         нет ни у одной цели. Разорвите цепочку: храните вместо вложенной \
                         структуры её данные напрямую либо индекс элемента массива"
                    ),
                )
                .with_code("SE-124"),
            );
        }
    }
    None
}
