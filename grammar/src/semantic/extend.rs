//! Построение и развёртка структуры [`Extend`] — реализации модели.
//!
//! Модуль предоставляет две публичные функции:
//! - [`construct_implement`] — строит [`Extend`] из семантического выражения [`ExpressionNode`];
//! - [`unroll_extend_expression`] — разворачивает выражение в плоскую
//!   структуру [`Extend::Concatenation`] / [`Extend::Parallel`].

use crate::diagnostics::{Diagnostic, Location};
use crate::parser::ast;
use crate::semantic::{
    ConditionNode, ExpressionNode, ModelNode, ReferenceNode, StateNode, StateNodeKind,
};
use std::cell::RefCell;
use std::fmt::{Display, Formatter};
use std::rc::Rc;

/// Реализация модели: описывает, как состояние или корневой автомат
/// составлен из именованных моделей.
///
/// - [`Unresolved`](Extend::Unresolved) — временная заглушка до второго прохода.
/// - [`Model`](Extend::Model) — ссылка на конкретную именованную модель.
/// - [`Parentless`](Extend::Parentless) — обёртка без родителя (скобки).
/// - [`Add`](Extend::Add) — последовательная компоновка `A + B`.
/// - [`Or`](Extend::Or) — параллельная компоновка `A | B`.
#[derive(Default, Debug, PartialEq, Eq, Clone)]
pub enum Extend {
    /// Реализация не задана (значение по умолчанию для безымянной корневой модели).
    #[default]
    None,
    /// «Сырое» АСД-выражение реализации, ожидающее разрешения на этапе stage1.
    Unresolved(ast::Expression),
    /// Ссылка на конкретную именованную модель.
    Model(Rc<RefCell<ModelNode>>),
    /// Скобочная группировка: `(реализация)`.
    Parentless(Box<Extend>),

    /// Последовательная компоновка: `левое + правое + ...`.
    Concatenation(Vec<Box<Extend>>),
    /// Параллельная компоновка: `левое | правое | ...`.
    Parallel(Vec<Box<Extend>>),
}

impl Extend {
    /// Возвращает `true`, если вариант — конкретная ссылка на модель.
    #[inline]
    pub fn is_model(&self) -> bool {
        matches!(self, Extend::Model(_))
    }
    /// Возвращает `true`, если вариант — скобочная группировка.
    #[inline]
    pub fn is_parentless(&self) -> bool {
        matches!(self, Extend::Parentless(_))
    }
    /// Возвращает `true`, если вариант — последовательная компоновка (`+`).
    #[inline]
    pub fn is_sequence(&self) -> bool {
        matches!(self, Extend::Concatenation(_))
    }
    /// Возвращает `true`, если вариант — параллельная компоновка (`|`).
    #[inline]
    pub fn is_parallel(&self) -> bool {
        matches!(self, Extend::Parallel(_))
    }
    /// Возвращает человекочитаемое имя варианта или имя модели.
    pub fn name(&self) -> String {
        match self {
            Extend::None => "None".to_string(),
            Extend::Unresolved(_) => "Unresolved".to_string(),
            Extend::Model(model) => model.clone().borrow().name().to_string(),
            Extend::Parentless(implement) => implement.name(),
            Extend::Concatenation(_) => "Concatenation".to_string(),
            Extend::Parallel(_) => "Parallel".to_string(),
        }
    }
}

impl Display for Extend {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        match self {
            Extend::None => write!(f, "None"),
            Extend::Unresolved(_) => write!(f, "Unresolved"),
            Extend::Model(model) => {
                write!(f, "{}", model.borrow().name.clone().unwrap_or_default())
            }
            Extend::Parentless(extends) => write!(f, "({})", extends),
            Extend::Concatenation(extends) => write!(
                f,
                "{}",
                extends
                    .iter()
                    .map(|i| i.to_string())
                    .collect::<Vec<String>>()
                    .join(" + ")
            ),
            Extend::Parallel(implements) => write!(
                f,
                "{}",
                implements
                    .iter()
                    .map(|i| i.to_string())
                    .collect::<Vec<String>>()
                    .join(" | ")
            ),
        }
    }
}

/// Преобразует АСД-выражение в семантический [`ExpressionNode`],
/// разрешая переменные в конкретные модели через контекст `model`.
///
/// Поддерживаемые операции: `Variable`, `Parenthesis`, `+`, `|`.
/// Для остальных возвращает [`Diagnostic`].
fn unroll_expression_ast(
    expr: ast::Expression,
    model: Rc<RefCell<ModelNode>>,
) -> Result<ExpressionNode, Diagnostic> {
    match expr {
        ast::Expression::Variable(id) => {
            let borrowed = model.as_ref().borrow();
            let found = borrowed.search_model(&id.name).ok_or_else(|| {
                Diagnostic::error(id.loc, format!("Модель '{}' не найдена", &id.name))
            })?;
            Ok(ExpressionNode::Model(Rc::clone(&found)))
        }
        ast::Expression::Parenthesis(_, inner) => unroll_expression_ast(*inner, model),
        ast::Expression::Add(_, left, right) => {
            let left = unroll_expression_ast(*left, model.clone())?;
            let right = unroll_expression_ast(*right, model.clone())?;
            Ok(ExpressionNode::Add(Box::new(left), Box::new(right)))
        }
        ast::Expression::BitwiseOr(_, left, right) => {
            let left = unroll_expression_ast(*left, model.clone())?;
            let right = unroll_expression_ast(*right, model.clone())?;
            Ok(ExpressionNode::BitwiseOr(Box::new(left), Box::new(right)))
        }
        other => Err(
            format!("Выражение AST расширения не поддерживается: {:?}", other)
                .as_str()
                .into(),
        ),
    }
}

/// Разворачивает семантическое выражение расширения в плоскую структуру [`Extend`],
/// объединяя цепочки `+` в [`Extend::Concatenation`] и `|` в [`Extend::Parallel`].
pub fn unroll_extend_expression(
    prefix_name: String,
    expression: ExpressionNode,
    model: Rc<RefCell<ModelNode>>,
) -> Result<Extend, Diagnostic> {
    let model = Rc::clone(&model);
    match expression {
        ExpressionNode::Unresolved(expr) => {
            let unrolled = unroll_expression_ast(expr, model.clone())?;
            unroll_extend_expression(prefix_name, unrolled, model)
        }
        ExpressionNode::Model(model) => Ok(Extend::Model(Rc::clone(&model))),
        ExpressionNode::Parenthesis(expression) => {
            unroll_extend_expression(prefix_name, *expression, model)
        }
        ExpressionNode::Add(left, right) => {
            let left = unroll_extend_expression(prefix_name.clone(), *left, model.clone())?;
            let right = unroll_extend_expression(prefix_name.clone(), *right, model.clone())?;
            // Плоская конкатенация: если операнд уже Sequence — разворачиваем его элементы.
            let mut items: Vec<Box<Extend>> = Vec::new();
            match left {
                Extend::Concatenation(seq) => items.extend(seq),
                other => items.push(Box::new(other)),
            }
            match right {
                Extend::Concatenation(seq) => items.extend(seq),
                other => items.push(Box::new(other)),
            }
            Ok(Extend::Concatenation(items))
        }
        ExpressionNode::BitwiseOr(left, right) => {
            let left = unroll_extend_expression(prefix_name.clone(), *left, model.clone())?;
            let right = unroll_extend_expression(prefix_name.clone(), *right, model.clone())?;
            // Плоское объединение: если операнд уже Parallel — разворачиваем его элементы.
            let mut items: Vec<Box<Extend>> = Vec::new();
            match left {
                Extend::Parallel(p) => items.extend(p),
                other => items.push(Box::new(other)),
            }
            match right {
                Extend::Parallel(p) => items.extend(p),
                other => items.push(Box::new(other)),
            }
            Ok(Extend::Parallel(items))
        }
        other => Err(format!("Неизвестное выражение расширения: {:?}", other)
            .as_str()
            .into()),
    }
}

/// Компактирует [`Extend`] в конкретную модель (или `Parallel`), копируя
/// исходные модели с новыми именами вида `"<prefix><OriginalName>"`.
///
/// - [`Extend::Model`] — копирует модель под именем `prefix + orig_name`.
/// - [`Extend::Concatenation`] — создаёт промежуточную модель `prefix_Concatenation`
///   с цепочкой `Implement`-состояний в обратном порядке.
/// - [`Extend::Parallel`] — рекурсивно компактирует каждый операнд.
/// - [`Extend::None`] / [`Extend::Unresolved`] — возвращает ошибку.
pub fn compact_extend(
    prefix_name: String,
    extend: &Extend,
    owned: Rc<RefCell<ModelNode>>,
) -> Result<Extend, Diagnostic> {
    match extend {
        Extend::None | Extend::Unresolved(_) => Err("Неизвестная реализация".into()),
        Extend::Model(model) => {
            // Копируем содержимое модели и меняем его владельца, добавляем в список моделей.
            // Для анонимных моделей (корневой import) имя состоит только из префикса.
            let base = model.borrow().name.clone().unwrap_or_default();
            let model_name = format!("{}{}", prefix_name, base);
            let model_node = owned.borrow().search_model(&model_name);
            if model_node.is_some() {
                return Ok(Extend::Model(model_node.unwrap()));
            }
            let model_node = model.borrow().copy(&model_name, Some(owned.clone()));
            let mode_rc = Rc::new(RefCell::new(model_node));
            owned
                .borrow_mut()
                .models
                .insert(model_name.clone(), mode_rc.clone());
            Ok(Extend::Model(mode_rc))
        }
        Extend::Parentless(extend_item) => compact_extend(prefix_name, extend_item, owned.clone()),
        Extend::Concatenation(extends) => {
            let prefix_name = format!("{}_Concatenation", prefix_name.clone());
            let model = ModelNode::new(prefix_name.clone().as_str(), Some(owned.clone()));
            let mut prev = None;
            let mut n = 0;
            let max_sequence_length: usize = extends.len();
            for implement in extends.iter().rev() {
                let model_name = format!("{}_{}", prefix_name, n);
                let extend_model = compact_extend(model_name.clone(), implement, owned.clone())?;
                n += 1;
                let kind = if n >= max_sequence_length {
                    StateNodeKind::Start
                } else {
                    StateNodeKind::Simple
                };
                let state = StateNode::Implement {
                    upper: Some(Rc::downgrade(&model)),
                    loc: Location::Codegen,
                    named_blocks: vec![],
                    name: model_name.clone(),
                    references: vec![],
                    implements: extend_model.clone(),
                    next: prev.clone(),
                    kind,
                };
                prev = Some(ReferenceNode {
                    location: Location::Codegen,
                    name: state.name().to_string(),
                    cond: ConditionNode::None,
                    object: Box::new(state.clone()),
                });
                model.borrow_mut().states.insert(model_name.clone(), state);
            }
            Ok(Extend::Model(model))
        }
        Extend::Parallel(extends) => {
            let extends = extends
                .iter()
                .map(|implement| {
                    Box::new(compact_extend(prefix_name.clone(), implement, owned.clone()).unwrap())
                })
                .collect::<Vec<_>>();
            Ok(Extend::Parallel(extends))
        }
    }
}

mod minimalistic {
    use crate::diagnostics::{Diagnostic, Location};
    use crate::semantic::extend::Extend;
    use crate::semantic::{ModelNode, StateNode};
    use std::cell::RefCell;
    use std::collections::HashMap;
    use std::rc::Rc;

    #[derive(Debug, Clone, PartialEq, Eq, Hash)]
    struct Name {
        local: String,
        unique: String,
    }

    impl Name {
        fn new(local: String, unique: String) -> Self {
            Name { local, unique }
        }
    }

    #[derive(Debug, Clone)]
    enum Element {
        Model {
            name: Name,
            states: Vec<Name>,
            start: Name,
        },
        StateExtend {
            name: Name,
            extend: Vec<Element>,
            next: Name,
        },
        State {
            name: Name,
            references: Vec<Name>,
        },
    }

    struct Snapshot {
        root: Rc<RefCell<ModelNode>>,
        used_elements: HashMap<Name, Element>,
        start: Name,
    }

    impl Snapshot {
        /// Строит снимок модели: находит стартовое состояние и рекурсивно
        /// обходит все достижимые из него состояния и реализации.
        ///
        /// Должна вызываться после полного семантического разрешения модели.
        ///
        /// # Ошибки
        /// Возвращает [`Diagnostic`], если у модели нет стартового состояния.
        pub(crate) fn create(model: Rc<RefCell<ModelNode>>) -> Result<Self, Diagnostic> {
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
            Ok(Snapshot {
                root: model.clone(),
                used_elements,
                start: Name::new(
                    local_name.clone(),
                    unique_state_name(&local_name, model.clone()),
                ),
            })
        }

        #[inline]
        fn model_at(&self, name: &str) -> Option<Rc<RefCell<ModelNode>>> {
            model_by_unique_name(name, self.root.clone())
        }

        #[inline]
        fn used_models(&self) -> Vec<Element> {
            self.used_elements
                .values()
                .filter(|e| matches!(e, Element::Model { .. }))
                .cloned()
                .collect::<Vec<Element>>()
        }
    }

    fn unique_model_name(model: Rc<RefCell<ModelNode>>) -> String {
        let name = model.borrow().name.clone().unwrap_or_default();
        if let Some(upper) = model.borrow().upper.as_ref() {
            return format!("{}:{}", unique_model_name(upper.upgrade().unwrap()), name);
        }
        name
    }

    fn unique_state_name(local_name: &str, model: Rc<RefCell<ModelNode>>) -> String {
        let name = unique_model_name(model);
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
        let key = Name::new(name_str.clone(), unique_state_name(&name_str, model.clone()));
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
            StateNode::Implement { implements, next, .. } => {
                let next_name = next
                    .as_ref()
                    .map(|n| Name::new(n.name.clone(), unique_state_name(&n.name, model.clone())))
                    .unwrap_or_else(|| Name::new(String::new(), String::new()));
                // Вставляем заглушку до обхода реализации (защита от циклов)
                used.insert(
                    key.clone(),
                    Element::StateExtend {
                        name: key.clone(),
                        extend: vec![],
                        next: next_name.clone(),
                    },
                );
                // Обходим вложенные модели через выражение реализации
                let extend_elems = visit_extend(implements, model.clone(), used);
                // Обновляем запись с реальным содержимым после обхода
                used.insert(
                    key.clone(),
                    Element::StateExtend {
                        name: key,
                        extend: extend_elems,
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

    /// Рекурсивно обходит выражение реализации [`Extend`], регистрирует
    /// используемые модели в `used` и возвращает плоский список вложенных элементов.
    ///
    /// - [`Extend::Model`] — регистрирует модель и запускает обход её состояний.
    /// - [`Extend::Concatenation`] / [`Extend::Parallel`] — рекурсирует в каждый операнд.
    /// - [`Extend::Parentless`] — прозрачная обёртка, делегирует внутрь.
    /// - [`Extend::None`] / [`Extend::Unresolved`] — пропускаются.
    fn visit_extend(
        extend: &Extend,
        model: Rc<RefCell<ModelNode>>,
        used: &mut HashMap<Name, Element>,
    ) -> Vec<Element> {
        match extend {
            Extend::Model(m_rc) => {
                let unique = unique_model_name(m_rc.clone());
                let local = m_rc.borrow().name.clone().unwrap_or_default();
                let key = Name::new(local, unique);
                if !used.contains_key(&key) {
                    // Собираем имена состояний отдельным borrow, чтобы не
                    // удерживать его при последующих вызовах
                    let state_keys: Vec<String> =
                        m_rc.borrow().states.keys().cloned().collect();
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
                vec![]
            }
            Extend::Parentless(inner) => visit_extend(inner, model, used),
            Extend::Concatenation(items) | Extend::Parallel(items) => items
                .iter()
                .flat_map(|item| visit_extend(item, model.clone(), used))
                .collect(),
            Extend::None | Extend::Unresolved(_) => vec![],
        }
    }

    #[cfg(test)]
    mod tests {
        use super::*;
        use crate::parse;
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
            let result = Snapshot::create(model_rc);
            assert!(result.is_err());
        }

        /// Snapshot::create обходит простые состояния.
        #[test]
        fn test_snapshot_create_simple_states() {
            let (ast, _) = parse("start S; state T;", 0).unwrap();
            let model_rc = construct_model(&ast, None, &[]).unwrap();
            let snap = Snapshot::create(model_rc).unwrap();
            // Стартовое состояние найдено
            assert_eq!(snap.start.local, "S");
            // S зарегистрировано среди используемых элементов
            assert!(snap
                .used_elements
                .values()
                .any(|e| matches!(e, Element::State { name, .. } if name.local == "S")));
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
            let snap = Snapshot::create(model_rc).unwrap();
            // Стартовое состояние корневой модели — Entry
            assert_eq!(snap.start.local, "Entry");
            // compact_extend именует копию как "Entry" + "A" = "EntryA"
            assert!(snap
                .used_models()
                .iter()
                .any(|e| matches!(e, Element::Model { name, .. } if name.local == "EntryA")));
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
            let snap = Snapshot::create(model_rc).unwrap();
            // Модель EntryA должна быть зарегистрирована ровно один раз
            let model_count = snap
                .used_models()
                .iter()
                .filter(|e| matches!(e, Element::Model { name, .. } if name.local == "EntryA"))
                .count();
            assert_eq!(model_count, 1);
        }
    }
}

#[cfg(test)]
mod tests {
    use crate::diagnostics::Location;
    use crate::parse;
    use crate::parser::ast;
    use crate::semantic::extend::{Extend, compact_extend, unroll_extend_expression};
    use crate::semantic::tree::construct_model;
    use crate::semantic::{ExpressionNode, StateNode};

    const SRC: &str = r#"
model A {
    start Start;
}
model B {
    start Start;
}
start Entry = A | B | (A + B);
state Next1 = A + B + (A | B);
state Next2 = A + (B | A) + B;
state Next3 = A + (B + A) + B;
state Next4 = A + (B + A) + (B | A);
state Next5 = (A | B) + (A + B);
state Next6 = (A | B) + (A + B) + (A | B);
state Next7 = (A | B) + (A + B) + (A | B) + (A + B);
state Next8 = (A | B) + (A + B) + (A | B) + (A + B) + (A | B);
state Next9 = (A | B) + (A + B) + (A | B) + (A + B) + (A | B) + (A + B);
state Next10 = (A | B) + (A + B) + (A | B) + (A + B) + (A | B) + (A + B) + (A + B);
"#;

    #[test]
    fn test_unroll_implement_expression() {
        let (ast, _) = parse(SRC, 0).unwrap();
        let model_rc = construct_model(&ast, None, &[]).unwrap();

        let implement = unroll_extend_expression(
            String::from("O"),
            ExpressionNode::Unresolved(ast::Expression::Variable(ast::Identifier::new("A"))),
            model_rc.clone(),
        )
        .unwrap();
        assert!(matches!(implement, Extend::Model(_)));
        let implement = unroll_extend_expression(
            String::from("O"),
            ExpressionNode::Unresolved(ast::Expression::BitwiseOr(
                Location::Implicit,
                Box::new(ast::Expression::Variable(ast::Identifier::new("A"))),
                Box::new(ast::Expression::Variable(ast::Identifier::new("B"))),
            )),
            model_rc.clone(),
        )
        .unwrap();
        assert!(matches!(implement, Extend::Parallel(_)));
        // start Entry = A | B | (A + B)  →  Parallel([A, B, Sequence([A, B])])
        let implement = unroll_extend_expression(
            String::from("O"),
            ExpressionNode::Unresolved(ast::Expression::BitwiseOr(
                Location::Implicit,
                Box::new(ast::Expression::BitwiseOr(
                    Location::Implicit,
                    Box::new(ast::Expression::Variable(ast::Identifier::new("A"))),
                    Box::new(ast::Expression::Variable(ast::Identifier::new("B"))),
                )),
                Box::new(ast::Expression::Parenthesis(
                    Location::Implicit,
                    Box::new(ast::Expression::Add(
                        Location::Implicit,
                        Box::new(ast::Expression::Variable(ast::Identifier::new("A"))),
                        Box::new(ast::Expression::Variable(ast::Identifier::new("B"))),
                    )),
                )),
            )),
            model_rc.clone(),
        )
        .unwrap();
        assert_eq!(
            implement,
            Extend::Parallel(vec![
                Box::new(Extend::Model(model_rc.borrow().search_model("A").unwrap())),
                Box::new(Extend::Model(model_rc.borrow().search_model("B").unwrap())),
                Box::new(Extend::Concatenation(vec![
                    Box::new(Extend::Model(model_rc.borrow().search_model("A").unwrap())),
                    Box::new(Extend::Model(model_rc.borrow().search_model("B").unwrap())),
                ]))
            ])
        );
    }

    #[test]
    fn test_unroll_implement_expressions() {
        let (ast, _) = parse(SRC, 0).unwrap();
        let model_rc = construct_model(&ast, None, &[]).unwrap();

        let ma = || Box::new(Extend::Model(model_rc.borrow().search_model("A").unwrap()));
        let mb = || Box::new(Extend::Model(model_rc.borrow().search_model("B").unwrap()));
        let par_ab = || Extend::Parallel(vec![ma(), mb()]);

        let pairs = vec![
            // Next1 = A + B + (A | B)
            (
                "Next1",
                Extend::Concatenation(vec![ma(), mb(), Box::new(par_ab())]),
                "Next1_Concatenation",
            ),
            // Next2 = A + (B | A) + B
            (
                "Next2",
                Extend::Concatenation(vec![
                    ma(),
                    Box::new(Extend::Parallel(vec![mb(), ma()])),
                    mb(),
                ]),
                "Next2_Concatenation",
            ),
            // Next3 = A + (B + A) + B  →  все элементы разворачиваются в одну последовательность
            (
                "Next3",
                Extend::Concatenation(vec![ma(), mb(), ma(), mb()]),
                "Next3_Concatenation",
            ),
            // Next4 = A + (B + A) + (B | A)
            (
                "Next4",
                Extend::Concatenation(vec![
                    ma(),
                    mb(),
                    ma(),
                    Box::new(Extend::Parallel(vec![mb(), ma()])),
                ]),
                "Next4_Concatenation",
            ),
            // Next5 = (A | B) + (A + B)
            (
                "Next5",
                Extend::Concatenation(vec![Box::new(par_ab()), ma(), mb()]),
                "Next5_Concatenation",
            ),
            // Next6 = (A | B) + (A + B) + (A | B)
            (
                "Next6",
                Extend::Concatenation(vec![Box::new(par_ab()), ma(), mb(), Box::new(par_ab())]),
                "Next6_Concatenation",
            ),
            // Next7 = (A | B) + (A + B) + (A | B) + (A + B)
            (
                "Next7",
                Extend::Concatenation(vec![
                    Box::new(par_ab()),
                    ma(),
                    mb(),
                    Box::new(par_ab()),
                    ma(),
                    mb(),
                ]),
                "Next7_Concatenation",
            ),
            // Next8 = (A | B) + (A + B) + (A | B) + (A + B) + (A | B)
            (
                "Next8",
                Extend::Concatenation(vec![
                    Box::new(par_ab()),
                    ma(),
                    mb(),
                    Box::new(par_ab()),
                    ma(),
                    mb(),
                    Box::new(par_ab()),
                ]),
                "Next8_Concatenation",
            ),
            // Next9 = (A | B) + (A + B) + (A | B) + (A + B) + (A | B) + (A + B)
            (
                "Next9",
                Extend::Concatenation(vec![
                    Box::new(par_ab()),
                    ma(),
                    mb(),
                    Box::new(par_ab()),
                    ma(),
                    mb(),
                    Box::new(par_ab()),
                    ma(),
                    mb(),
                ]),
                "Next9_Concatenation",
            ),
            // Next10 = (A | B) + (A + B) + (A | B) + (A + B) + (A | B) + (A + B) + (A + B)
            (
                "Next10",
                Extend::Concatenation(vec![
                    Box::new(par_ab()),
                    ma(),
                    mb(),
                    Box::new(par_ab()),
                    ma(),
                    mb(),
                    Box::new(par_ab()),
                    ma(),
                    mb(),
                    ma(),
                    mb(),
                ]),
                "Next10_Concatenation",
            ),
            // Entry = A | B | (A + B)
            (
                "Entry",
                Extend::Parallel(vec![
                    ma(),
                    mb(),
                    Box::new(Extend::Concatenation(vec![ma(), mb()])),
                ]),
                "EntryA | EntryB | Entry_Concatenation",
            ),
        ];
        for (name, _expected, expected_name) in pairs {
            let state = model_rc.borrow().search_state(name).unwrap();
            let StateNode::Implement { ref implements, .. } = *state.borrow() else {
                panic!("State is not an implement")
            };
            assert_eq!(
                implements.to_string(),
                expected_name,
                "State {} is not unrolled. {} != {}",
                name,
                implements,
                expected_name
            );
        }
    }

    #[test]
    fn test_implement_to_model() {
        let (ast, _) = parse(SRC, 0).unwrap();
        let model_rc = construct_model(&ast, None, &[]).unwrap();

        let result = compact_extend(String::from("O"), &Extend::None, model_rc.clone());
        assert!(result.is_err());

        let result = compact_extend(
            String::from("O"),
            &Extend::Unresolved(ast::Expression::Variable(ast::Identifier::new("A"))),
            model_rc.clone(),
        );
        assert!(result.is_err());

        let model_a = model_rc.borrow().search_model("A").unwrap();
        let model_b = model_rc.borrow().search_model("B").unwrap();
        {
            let result = compact_extend(
                String::from("O"),
                &Extend::Model(model_a.clone()),
                model_rc.clone(),
            );
            assert!(result.is_ok());
            let Extend::Model(result) = result.unwrap() else {
                panic!("Result is not a model")
            };
            let result = result.borrow();
            assert_eq!(
                format!("O{}", model_a.borrow().name.clone().unwrap()),
                result.name.clone().unwrap()
            );
        }
        {
            let result = compact_extend(
                String::from("O"),
                &Extend::Concatenation(vec![
                    Box::new(Extend::Model(model_a.clone())),
                    Box::new(Extend::Model(model_b.clone())),
                ]),
                model_rc.clone(),
            );
            assert!(result.is_ok());
            let result = result.unwrap();
            let Extend::Model(result) = result else {
                panic!("Result is not a model")
            };
            let result = result.borrow();
            assert_eq!(result.name(), "O_Concatenation".to_string());
            assert!(result.search_state("O_Concatenation_1").is_some());
            assert!(result.search_state("O_Concatenation_0").is_some());
        }
        {
            let result = compact_extend(
                String::from("O"),
                &Extend::Parallel(vec![
                    Box::new(Extend::Model(model_a.clone())),
                    Box::new(Extend::Model(model_b.clone())),
                ]),
                model_rc.clone(),
            );
            assert!(result.is_ok());
            let result = result.unwrap();
            let Extend::Parallel(implements) = result.clone() else {
                panic!("Result is not a parallel implement")
            };
            assert_eq!(result.name(), "Parallel".to_string());
            assert_eq!(implements.len(), 2);
            assert!(implements[0].is_model());
            assert!(implements[1].is_model());
        }
    }
}
