use crate::diagnostics::{Diagnostic, Location};
use crate::semantic::extend::Extend;
use crate::semantic::naming::{normalize_camelcase_name, normalize_lowercase_snakecase};
use crate::semantic::{ModelNode, StateNode};
use itertools::Itertools;
use std::cell::RefCell;
use std::collections::HashMap;
use std::fmt::{Debug, Display};
use std::rc::Rc;

#[derive(Clone, PartialEq, Eq, Hash)]
pub(crate) struct Name {
    local: String,
    unique: String,
}

impl Name {
    fn new(local: String, unique: String) -> Self {
        Name { local, unique }
    }

    pub fn local(&self) -> &str {
        &self.local
    }
    pub fn unique(&self) -> &str {
        &self.unique
    }

    pub fn unique_lowercase_snakecase(&self) -> String {
        normalize_lowercase_snakecase(self.unique.replace(":", "_"))
    }

    pub fn unique_uppercase_snakecase(&self) -> String {
        self.unique_lowercase_snakecase().to_uppercase()
    }

    pub fn unique_camelcase(&self) -> String {
        normalize_camelcase_name(&self.unique.replace(":", "_"))
    }
}

impl Display for Name {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(
            f,
            "{} ({})",
            self.local,
            self.unique
                .split(':')
                .map(|p| format!("{}{}", p.split_at(1).0.to_uppercase(), p.split_at(1).1))
                .join(":")
        )
    }
}

impl Debug for Name {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        Display::fmt(self, f)
    }
}

impl From<Rc<RefCell<ModelNode>>> for Name {
    fn from(model: Rc<RefCell<ModelNode>>) -> Self {
        let local_name = model.borrow().name.clone().unwrap_or_default();
        Name::new(local_name, unique_model_name(model.clone()))
    }
}

#[derive(Debug, Clone)]
pub(crate) enum StateExtend {
    None,
    Model(Name),
    Concatenation(Vec<StateExtend>),
    Parallel(Vec<StateExtend>),
}

#[derive(Debug, Clone)]
pub(crate) enum Element {
    Model {
        name: Name,
        states: Vec<Name>,
        start: Name,
    },
    StateExtend {
        name: Name,
        extend: StateExtend,
        next: Name,
    },
    State {
        name: Name,
        references: Vec<Name>,
    },
}

impl Element {
    pub(crate) fn is_state(&self) -> bool {
        matches!(self, Element::State { .. } | Element::StateExtend { .. })
    }
}

pub(crate) struct Map {
    root: Rc<RefCell<ModelNode>>,
    root_name: Name,
    elements: HashMap<Name, Element>,
    start: Name,
    states: Vec<Name>,
}

impl Map {
    /// Строит снимок модели: находит стартовое состояние и рекурсивно
    /// обходит все достижимые из него состояния и реализации.
    ///
    /// Должна вызываться после полного семантического разрешения модели.
    ///
    /// # Ошибки
    /// Возвращает [`Diagnostic`], если у модели нет стартового состояния.
    pub fn create(model: Rc<RefCell<ModelNode>>) -> Result<Self, Diagnostic> {
        let mut used_elements = HashMap::new();
        let Some(start) = model.borrow().get_start_state() else {
            return Err(Diagnostic::error(
                Location::Implicit,
                "Model must have a start state".to_string(),
            ));
        };
        let local_name = start.name().to_string();
        // Рекурсивный спуск: обходим все достижимые состояния начиная со стартового
        visit_state(&start, model.clone(), &mut used_elements);
        let mut states = Vec::new();
        model.borrow().states.iter().for_each(|state| {
            states.push(Name::new(
                state.0.clone(),
                unique_state_name(state.0, model.clone()),
            ));
        });
        Ok(Map {
            root: model.clone(),
            elements: used_elements,
            start: Name::new(
                local_name.clone(),
                unique_state_name(&local_name, model.clone()),
            ),
            root_name: Name::from(model),
            states,
        })
    }

    #[inline]
    pub(crate) fn model_at(&self, name: Option<String>) -> Option<Rc<RefCell<ModelNode>>> {
        if let Some(name) = name {
            return model_by_unique_name(&name, self.root.clone());
        }
        Some(self.root.clone())
    }

    #[inline]
    pub(crate) fn used_models(&self) -> Vec<Element> {
        self.elements
            .values()
            .filter(|e| matches!(e, Element::Model { .. }))
            .cloned()
            .collect::<Vec<Element>>()
    }

    #[inline]
    pub(crate) fn start(&self) -> Name {
        self.start.clone()
    }

    pub(crate) fn element_at(&self, name: Name) -> Option<Element> {
        self.elements.get(&name).cloned()
    }

    pub(crate) fn own(&self) -> Option<Element> {
        self.elements.get(&self.root_name).cloned()
    }

    pub(crate) fn root_name(&self) -> Name {
        self.root_name.clone()
    }

    pub(crate) fn states(&self) -> Vec<Name> {
        self.states.clone()
    }
}

fn unique_model_name(model: Rc<RefCell<ModelNode>>) -> String {
    let name = model.borrow().name.clone().unwrap_or_default();
    if let Some(upper) = model.borrow().upper.as_ref() {
        // Weak может быть невалиден, если родительский Rc уже дропнут
        // (например, для моделей из импортированных файлов).
        // В этом случае используем локальное имя без префикса.
        if let Some(parent) = upper.upgrade() {
            let model_name = unique_model_name(parent);
            if model_name.is_empty() {
                return name;
            }
            return format!("{}:{}", model_name, name);
        }
    }
    name
}

fn unique_state_name(local_name: &str, model: Rc<RefCell<ModelNode>>) -> String {
    let name = unique_model_name(model);
    if name.is_empty() {
        return local_name.to_string();
    }
    format!("{}:{}", &name, local_name)
}

/// Ищет модель по уникальному имени вида `"Root:Child:Grandchild"`.
///
/// Рекурсивно отсекает первый сегмент, разделённый `':'`:
/// - если текущая модель совпадает с префиксом — углубляемся внутрь неё;
/// - иначе — ищем дочернюю модель с именем-префиксом и рекурсируем в неё.
///
/// Работает с именами произвольной длины.
fn model_by_unique_name(
    model_name: &str,
    owned: Rc<RefCell<ModelNode>>,
) -> Option<Rc<RefCell<ModelNode>>> {
    if let Some(index) = model_name.find(':') {
        let (prefix, rest) = model_name.split_at(index);
        let rest = &rest[1..]; // Отсекаем разделитель ':'
        if owned.borrow().name() == prefix {
            // Текущий узел совпадает с префиксом — рекурсируем внутрь
            return model_by_unique_name(rest, owned);
        } else if let Some(child) = owned.borrow().search_model(prefix) {
            // Нашли дочернюю модель — рекурсируем в неё
            return model_by_unique_name(rest, child);
        }
    } else {
        // Последний сегмент без ':'
        if owned.borrow().name() == model_name {
            return Some(owned);
        } else if let Some(model) = owned.borrow().search_model(model_name) {
            return Some(model);
        }
    }
    None
}

/// Рекурсивно обходит состояние и все достижимые из него по ссылкам и
/// `next`-переходу. Добавляет найденные элементы в `used`.
///
/// # Защита от циклов
/// Перед рекурсией в `used` вставляется заглушка с ключом текущего
/// состояния. При повторном входе проверка `used.contains_key(&key)`
/// в начале функции прерывает обход.
fn visit_state(
    state: &StateNode,
    model: Rc<RefCell<ModelNode>>,
    used: &mut HashMap<Name, Element>,
) {
    let name_str = state.name().to_string();
    let key = Name::new(
        name_str.clone(),
        unique_state_name(&name_str, model.clone()),
    );
    if used.contains_key(&key) {
        return; // Уже обработано — прерываем возможный цикл
    }
    match state {
        StateNode::Simple { references, .. } => {
            let ref_names: Vec<Name> = references
                .iter()
                .map(|r| Name::new(r.name.clone(), unique_state_name(&r.name, model.clone())))
                .collect();
            // Регистрируем состояние до рекурсии (защита от самоссылок)
            used.insert(
                key.clone(),
                Element::State {
                    name: key,
                    references: ref_names.clone(),
                },
            );
            // Рекурсивно обходим все исходящие переходы
            for ref_name in ref_names {
                let next_opt = model.borrow().search_state(&ref_name.local);
                if let Some(rc) = next_opt {
                    let next = rc.borrow().clone();
                    visit_state(&next, model.clone(), used);
                }
            }
        }
        StateNode::Implement {
            implements, next, ..
        } => {
            let next_name = next
                .as_ref()
                .map(|n| Name::new(n.name.clone(), unique_state_name(&n.name, model.clone())))
                .unwrap_or_else(|| Name::new(String::new(), String::new()));
            // Вставляем заглушку до обхода реализации (защита от циклов)
            used.insert(
                key.clone(),
                Element::StateExtend {
                    name: key.clone(),
                    extend: StateExtend::None,
                    next: next_name.clone(),
                },
            );
            visit_extend(implements, model.clone(), used);
            // Обновляем запись с реальным содержимым после обхода
            used.insert(
                key.clone(),
                Element::StateExtend {
                    name: key,
                    extend: build_extend(implements, model.clone()),
                    next: next_name.clone(),
                },
            );
            // Рекурсивно обходим next-переход
            if !next_name.local.is_empty() {
                let next_opt = model.borrow().search_state(&next_name.local);
                if let Some(rc) = next_opt {
                    let next_state = rc.borrow().clone();
                    visit_state(&next_state, model.clone(), used);
                }
            }
        }
        StateNode::Unresolved => {}
    }
}

fn build_extend(extend: &Extend, model: Rc<RefCell<ModelNode>>) -> StateExtend {
    match extend {
        Extend::None => StateExtend::None,
        Extend::Unresolved(_) => StateExtend::None,
        Extend::Model(model) => StateExtend::Model(Name::from(Rc::clone(&model))),
        Extend::Parentless(extend) => build_extend(extend, model),
        Extend::Concatenation(extends) => StateExtend::Concatenation(
            extends
                .iter()
                .map(|extend| build_extend(extend, model.clone()))
                .collect(),
        ),
        Extend::Parallel(extends) => StateExtend::Parallel(
            extends
                .iter()
                .map(|extend| build_extend(extend, model.clone()))
                .collect(),
        ),
    }
}

/// Рекурсивно обходит выражение реализации [`Extend`], регистрирует
/// используемые модели в `used` и возвращает плоский список вложенных элементов.
///
/// - [`Extend::Model`] — регистрирует модель и запускает обход её состояний.
/// - [`Extend::Concatenation`] / [`Extend::Parallel`] — рекурсирует в каждый операнд.
/// - [`Extend::Parentless`] — прозрачная обёртка, делегирует внутрь.
/// - [`Extend::None`] / [`Extend::Unresolved`] — пропускаются.
fn visit_extend(extend: &Extend, model: Rc<RefCell<ModelNode>>, used: &mut HashMap<Name, Element>) {
    match extend {
        Extend::Model(m_rc) => {
            let unique = unique_model_name(m_rc.clone());
            let local = m_rc.borrow().name.clone().unwrap_or_default();
            let key = Name::new(local, unique);
            if !used.contains_key(&key) {
                // Собираем имена состояний отдельным borrow, чтобы не
                // удерживать его при последующих вызовах
                let state_keys: Vec<String> = m_rc.borrow().states.keys().cloned().collect();
                let start_opt = m_rc.borrow().get_start_state();
                let states: Vec<Name> = state_keys
                    .iter()
                    .map(|s| Name::new(s.clone(), unique_state_name(s, m_rc.clone())))
                    .collect();
                let start_name = start_opt
                    .as_ref()
                    .map(|s| {
                        Name::new(
                            s.name().to_string(),
                            unique_state_name(s.name(), m_rc.clone()),
                        )
                    })
                    .unwrap_or_else(|| Name::new(String::new(), String::new()));
                used.insert(
                    key.clone(),
                    Element::Model {
                        name: key,
                        states,
                        start: start_name,
                    },
                );
                // Рекурсивно обходим состояния модели
                if let Some(start) = start_opt {
                    visit_state(&start, m_rc.clone(), used);
                }
            }
        }
        Extend::Parentless(inner) => visit_extend(inner, model, used),
        Extend::Concatenation(items) | Extend::Parallel(items) => {
            for item in items {
                visit_extend(item, model.clone(), used)
            }
        }
        Extend::None | Extend::Unresolved(_) => (),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::parse;
    use crate::semantic::test_constants::tests::SRC;
    use crate::semantic::tree::construct_model;

    #[test]
    fn test_unique_model_name() {
        let model = ModelNode::new("A", None);
        assert_eq!(unique_model_name(model.clone()), "A");

        let model = ModelNode::new("B", Some(model.clone()));
        assert_eq!(unique_model_name(model.clone()), "A:B");
    }

    /// Тест с однобуквенными именами (базовые случаи).
    #[test]
    fn test_model_by_unique_name() {
        let global = ModelNode::new("A", None);
        let model = ModelNode::new("B", Some(global.clone()));
        let _ = ModelNode::new("C", Some(model.clone()));
        assert_eq!(
            model_by_unique_name("A", global.clone())
                .unwrap()
                .borrow()
                .name(),
            "A"
        );
        assert_eq!(
            model_by_unique_name("A:B", global.clone())
                .unwrap()
                .borrow()
                .name(),
            "B"
        );
        assert_eq!(
            model_by_unique_name("A:B:C", global.clone())
                .unwrap()
                .borrow()
                .name(),
            "C"
        );
    }

    /// Тест с многосимвольными именами — проверяет корректность отсечения ':'.
    #[test]
    fn test_model_by_unique_name_multichar() {
        let global = ModelNode::new("Root", None);
        let child = ModelNode::new("Child", Some(global.clone()));
        let _ = ModelNode::new("Leaf", Some(child.clone()));
        assert_eq!(
            model_by_unique_name("Root", global.clone())
                .unwrap()
                .borrow()
                .name(),
            "Root"
        );
        assert_eq!(
            model_by_unique_name("Root:Child", global.clone())
                .unwrap()
                .borrow()
                .name(),
            "Child"
        );
        assert_eq!(
            model_by_unique_name("Root:Child:Leaf", global.clone())
                .unwrap()
                .borrow()
                .name(),
            "Leaf"
        );
        // Контрпример: несуществующая модель возвращает None
        assert!(model_by_unique_name("Root:Ghost", global.clone()).is_none());
        assert!(model_by_unique_name("Ghost", global.clone()).is_none());
    }

    /// Snapshot::create возвращает ошибку, если нет стартового состояния.
    #[test]
    fn test_snapshot_create_no_start() {
        // Создаём пустую модель без состояний напрямую,
        // обходя валидацию construct_model
        let model_rc = ModelNode::new("Root", None);
        let result = Map::create(model_rc);
        assert!(result.is_err());
    }

    /// Snapshot::create обходит простые состояния.
    #[test]
    fn test_snapshot_create_simple_states() {
        let (ast, _) = parse("start S; state T;", 0).unwrap();
        let model_rc = construct_model(&ast, None, &[]).unwrap();
        let snap = Map::create(model_rc).unwrap();
        // Стартовое состояние найдено
        assert_eq!(snap.start.local, "S");
        // S зарегистрировано среди используемых элементов
        assert!(
            snap.elements
                .values()
                .any(|e| matches!(e, Element::State { name, .. } if name.local == "S"))
        );
    }

    /// Snapshot::create регистрирует модели из выражений реализации.
    ///
    /// После compact_extend модель A регистрируется под именем "EntryA"
    /// (префикс состояния + исходное имя модели).
    #[test]
    fn test_snapshot_create_with_extend() {
        let src = "model A { start S; }\nstart Entry = A;";
        let (ast, _) = parse(src, 0).unwrap();
        let model_rc = construct_model(&ast, None, &[]).unwrap();
        let snap = Map::create(model_rc).unwrap();
        // Стартовое состояние корневой модели — Entry
        assert_eq!(snap.start.local, "Entry");
        // compact_extend именует копию как "Entry" + "A" = "EntryA"
        assert!(
            snap.used_models()
                .iter()
                .any(|e| matches!(e, Element::Model { name, .. } if name.local == "A"))
        );
    }

    /// Snapshot::create не дублирует элементы при параллельных ссылках на одну модель.
    ///
    /// compact_extend кэширует уже созданную копию ("EntryA"), поэтому
    /// оба операнда `A | A` указывают на один и тот же Rc.
    #[test]
    fn test_snapshot_create_dedup() {
        let src = "model A { start S; }\nstart Entry = A | A;";
        let (ast, _) = parse(src, 0).unwrap();
        let model_rc = construct_model(&ast, None, &[]).unwrap();
        let snap = Map::create(model_rc).unwrap();
        // Модель EntryA должна быть зарегистрирована ровно один раз
        let model_count = snap
            .used_models()
            .iter()
            .filter(|e| matches!(e, Element::Model { name, .. } if name.local == "A"))
            .count();
        assert_eq!(model_count, 1);
    }

    #[test]
    fn test_snapshot_create_complex() {
        let (ast, _) = parse(SRC, 0).unwrap();
        let model_rc = construct_model(&ast, None, &[]).unwrap();
        let snap = Map::create(model_rc).unwrap();
        assert_eq!(snap.start.local, "Entry");
    }
}
