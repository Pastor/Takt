//! Построение и развёртка структуры [`Extend`] — реализации модели.
//!
//! Модуль предоставляет две публичные функции:
//! - [`unroll_extend_expression`] — разворачивает выражение в плоскую
//!   структуру [`Extend::Concatenation`] / [`Extend::Parallel`].

use crate::diagnostics::{Diagnostic, Location};
use crate::parser::ast;
use crate::semantic::{ConditionNode, ExpressionNode, ModelNode, ReferenceNode, StateNode, StateNodeKind};
use std::cell::RefCell;
use std::fmt::{Display, Formatter};
use std::rc::Rc;

/// Реализация модели: описывает, как состояние или корневой автомат
/// составлен из именованных моделей.
///
/// - [`Unresolved`](Extend::Unresolved) — временная заглушка до второго прохода.
/// - [`Model`](Extend::Model) — ссылка на конкретную именованную модель.
/// - [`Parentless`](Extend::Parentless) — обёртка без родителя (скобки).
/// - [`Concatenation`](Extend::Concatenation) — последовательная компоновка `A + B`.
/// - [`Parallel`](Extend::Parallel) — параллельная компоновка `A | B`.
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

/// Упаковывает плоскую [`Extend::Concatenation`] в синтетическую [`Extend::Model`].
///
/// Для `M1 + M2` создаёт новую анонимную модель с состояниями `Step0 = M1 { next Step1 }`
/// и `Step1 = M2`, возвращая `Extend::Model(synthetic)`. Одноэлементная конкатенация
/// сворачивается рекурсивно. `Parentless` прозрачно делегирует внутрь.
///
/// Вызывается сразу после [`unroll_extend_expression`] в стадии stage1.
pub fn compact_implement(
    extend: Extend,
    parent: Rc<RefCell<ModelNode>>,
    state_name: &str,
) -> Extend {
    match extend {
        // Одноэлементная последовательность — прозрачно разворачиваем.
        Extend::Concatenation(mut items) if items.len() == 1 => {
            compact_implement(*items.remove(0), parent, state_name)
        }
        // Несколько элементов: создаём синтетическую модель со ступенями Step0…StepN-1.
        Extend::Concatenation(items) => {
            let seq_name = format!("{}_Sequence", state_name);
            let seq_model = ModelNode::new(&seq_name, Some(Rc::clone(&parent)));
            let n = items.len();
            for (i, item) in items.into_iter().enumerate() {
                let step_name = format!("Step{}", i);
                let next: Option<ReferenceNode<StateNode>> = if i + 1 < n {
                    Some(ReferenceNode {
                        location: Location::Codegen,
                        name: format!("Step{}", i + 1),
                        cond: ConditionNode::None,
                        object: Box::new(StateNode::Unresolved),
                    })
                } else {
                    None
                };
                let kind = if i == 0 {
                    StateNodeKind::Start
                } else {
                    StateNodeKind::Simple
                };
                let inner = compact_implement(*item, Rc::clone(&seq_model), &step_name);
                let state = StateNode::Implement {
                    upper: Some(Rc::downgrade(&seq_model)),
                    loc: Location::Codegen,
                    named_blocks: vec![],
                    name: step_name.clone(),
                    references: vec![],
                    implements: inner,
                    next,
                    kind,
                };
                seq_model.borrow_mut().states.insert(step_name, state);
            }
            Extend::Model(seq_model)
        }
        // Скобочная группировка — делегируем внутрь.
        Extend::Parentless(inner) => compact_implement(*inner, parent, state_name),
        // Остальное не требует обработки.
        other => other,
    }
}

/// Разворачивает семантическое выражение расширения в плоскую структуру [`Extend`],
/// объединяя цепочки `+` в [`Extend::Concatenation`] и `|` в [`Extend::Parallel`].
pub fn unroll_extend_expression(
    expression: ExpressionNode,
    model: Rc<RefCell<ModelNode>>,
) -> Result<Extend, Diagnostic> {
    let model = Rc::clone(&model);
    match expression {
        ExpressionNode::Unresolved(expr) => {
            let unrolled = unroll_expression_ast(expr, model.clone())?;
            unroll_extend_expression(unrolled, model)
        }
        ExpressionNode::Model(model) => Ok(Extend::Model(Rc::clone(&model))),
        ExpressionNode::Parenthesis(expression) => unroll_extend_expression(*expression, model),
        ExpressionNode::Add(left, right) => {
            let left = unroll_extend_expression(*left, model.clone())?;
            let right = unroll_extend_expression(*right, model.clone())?;
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
            let left = unroll_extend_expression(*left, model.clone())?;
            let right = unroll_extend_expression(*right, model.clone())?;
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

pub mod minimalistic {
    use crate::diagnostics::{Diagnostic, Location};
    use crate::semantic::extend::Extend;
    use crate::semantic::{ModelNode, StateNode};
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
    }

    impl Display for Name {
        fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
            write!(f, "{} ({})", self.local, self.unique)
        }
    }

    impl Debug for Name {
        fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
            Display::fmt(self, f)
        }
    }

    impl From<Rc<RefCell<ModelNode>>> for Name {
        fn from(model: Rc<RefCell<ModelNode>>) -> Self {
            Name::new(
                model.borrow().name.clone().unwrap().clone(),
                unique_model_name(model.clone()),
            )
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

    pub(crate) struct Snapshot {
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
        pub(crate) fn model_at(&self, name: &str) -> Option<Rc<RefCell<ModelNode>>> {
            model_by_unique_name(name, self.root.clone())
        }

        #[inline]
        pub(crate) fn used_models(&self) -> Vec<Element> {
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
    fn visit_extend(
        extend: &Extend,
        model: Rc<RefCell<ModelNode>>,
        used: &mut HashMap<Name, Element>,
    ) {
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
        use crate::semantic::extend::tests::SRC;
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
            assert!(
                snap.used_elements
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
            let snap = Snapshot::create(model_rc).unwrap();
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
            let snap = Snapshot::create(model_rc).unwrap();
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
            let snap = Snapshot::create(model_rc).unwrap();
            assert_eq!(snap.start.local, "Entry");
        }
    }
}

#[cfg(test)]
mod tests {
    use crate::diagnostics::Location;
    use crate::parse;
    use crate::parser::ast;
    use crate::semantic::extend::{Extend, unroll_extend_expression};
    use crate::semantic::tree::construct_model;
    use crate::semantic::{ExpressionNode, StateNode};

    pub const SRC: &str = r#"
model A {
    start Start;
}
model B {
    start Start;
}
start Entry = A | B | (A + B) {
    next Next1;
}
state Next1 = A + B + (A | B) {
    next Next2;
}
state Next2 = A + (B | A) + B {
    next Next3;
}
state Next3 = A + (B + A) + B {
    next Next4;
}
state Next4 = A + (B + A) + (B | A) {
    next Next5;
}
state Next5 = (A | B) + (A + B) {
    next Next6;
}
state Next6 = (A | B) + (A + B) + (A | B) {
    next Next7;
}
state Next7 = (A | B) + (A + B) + (A | B) + (A + B) {
    next Next8;
}
state Next8 = (A | B) + (A + B) + (A | B) + (A + B) + (A | B) {
    next Next9;
}
state Next9 = (A | B) + (A + B) + (A | B) + (A + B) + (A | B) + (A + B) {
    next Next10;
}
state Next10 = (A | B) + (A + B) + (A | B) + (A + B) + (A | B) + (A + B) + (A + B);
"#;

    #[test]
    fn test_unroll_implement_expression() {
        let (ast, _) = parse(SRC, 0).unwrap();
        let model_rc = construct_model(&ast, None, &[]).unwrap();

        let implement = unroll_extend_expression(
            ExpressionNode::Unresolved(ast::Expression::Variable(ast::Identifier::new("A"))),
            model_rc.clone(),
        )
        .unwrap();
        assert!(matches!(implement, Extend::Model(_)));
        let implement = unroll_extend_expression(
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
        // После compact_implement все состояния с `+` на верхнем уровне дают Model(synthetic).
        // Это тест пост-компактного состояния семантического дерева.
        let (ast, _) = parse(SRC, 0).unwrap();
        let model_rc = construct_model(&ast, None, &[]).unwrap();

        // Next1..Next10: верхний уровень — конкатенация → упаковывается в Model(synthetic).
        let seq_states_with_step_count = [
            ("Next1", 3usize),  // A + B + (A|B)      → 3 ступени
            ("Next2", 3),       // A + (B|A) + B       → 3 ступени
            ("Next3", 4),       // A + B + A + B        → 4 ступени
            ("Next4", 4),       // A + B + A + (B|A)   → 4 ступени
            ("Next5", 3),       // (A|B) + A + B        → 3 ступени
            ("Next6", 4),       // (A|B) + A + B + (A|B) → 4 ступени
            ("Next7", 6),       // (A|B)+A+B+(A|B)+A+B → 6 ступеней
            ("Next8", 7),       // … + (A|B)            → 7 ступеней
            ("Next9", 9),       // … + A + B            → 9 ступеней
            ("Next10", 11),     // … + A + B            → 11 ступеней
        ];
        for (name, expected_steps) in seq_states_with_step_count {
            let state = model_rc.borrow().search_state(name).unwrap();
            let StateNode::Implement { ref implements, .. } = *state.borrow() else {
                panic!("State {name} is not an Implement node")
            };
            let Extend::Model(seq_rc) = implements else {
                panic!("State {name}: после compact_implement ожидался Extend::Model, получили: {implements}");
            };
            let step_count = seq_rc.borrow().states.len();
            assert_eq!(
                step_count, expected_steps,
                "State {name}: синтетическая модель должна содержать {expected_steps} ступеней, содержит {step_count}"
            );
            // Стартовая ступень существует и имеет kind=Start.
            let seq = seq_rc.borrow();
            let start = seq.get_start_state();
            assert!(
                start.is_some(),
                "State {name}: в синтетической модели должно быть стартовое состояние"
            );
        }

        // Entry = A | B | (A + B): верхний уровень — параллель → Parallel(_).
        // compact_implement не трогает Parallel, поэтому вложенная конкатенация
        // остаётся как Concatenation внутри Parallel.
        {
            let state = model_rc.borrow().search_state("Entry").unwrap();
            let StateNode::Implement { ref implements, .. } = *state.borrow() else {
                panic!("State Entry is not an Implement node")
            };
            assert!(
                matches!(implements, Extend::Parallel(_)),
                "Entry = A | B | (A + B): ожидался Extend::Parallel, получили: {implements}"
            );
        }
    }

    #[test]
    fn test_extend_predicates() {
        use crate::semantic::ModelNode;

        let model = ModelNode::new("A", None);

        assert!(!Extend::None.is_model());
        assert!(!Extend::None.is_parentless());
        assert!(!Extend::None.is_sequence());
        assert!(!Extend::None.is_parallel());

        let extend_model = Extend::Model(model.clone());
        assert!(extend_model.is_model());
        assert!(!extend_model.is_parentless());
        assert!(!extend_model.is_sequence());
        assert!(!extend_model.is_parallel());

        let parentless = Extend::Parentless(Box::new(Extend::Model(model.clone())));
        assert!(!parentless.is_model());
        assert!(parentless.is_parentless());
        assert!(!parentless.is_sequence());
        assert!(!parentless.is_parallel());

        let seq = Extend::Concatenation(vec![Box::new(Extend::Model(model.clone()))]);
        assert!(!seq.is_model());
        assert!(!seq.is_parentless());
        assert!(seq.is_sequence());
        assert!(!seq.is_parallel());

        let par = Extend::Parallel(vec![Box::new(Extend::Model(model.clone()))]);
        assert!(!par.is_model());
        assert!(!par.is_parentless());
        assert!(!par.is_sequence());
        assert!(par.is_parallel());
    }

    #[test]
    fn test_extend_name() {
        use crate::semantic::ModelNode;

        let model = ModelNode::new("MyModel", None);

        assert_eq!(Extend::None.name(), "None");
        assert_eq!(
            Extend::Unresolved(ast::Expression::Variable(ast::Identifier::new("X"))).name(),
            "Unresolved"
        );
        assert_eq!(Extend::Model(model.clone()).name(), "MyModel");
        assert_eq!(
            Extend::Parentless(Box::new(Extend::Model(model.clone()))).name(),
            "MyModel"
        );
        assert_eq!(
            Extend::Concatenation(vec![Box::new(Extend::Model(model.clone()))]).name(),
            "Concatenation"
        );
        assert_eq!(
            Extend::Parallel(vec![Box::new(Extend::Model(model.clone()))]).name(),
            "Parallel"
        );
    }

    #[test]
    fn test_extend_display() {
        use crate::semantic::ModelNode;

        let a = ModelNode::new("A", None);
        let b = ModelNode::new("B", None);

        assert_eq!(format!("{}", Extend::None), "None");
        assert_eq!(
            format!(
                "{}",
                Extend::Unresolved(ast::Expression::Variable(ast::Identifier::new("X")))
            ),
            "Unresolved"
        );
        assert_eq!(format!("{}", Extend::Model(a.clone())), "A");
        assert_eq!(
            format!("{}", Extend::Parentless(Box::new(Extend::Model(a.clone())))),
            "(A)"
        );
        assert_eq!(
            format!(
                "{}",
                Extend::Concatenation(vec![
                    Box::new(Extend::Model(a.clone())),
                    Box::new(Extend::Model(b.clone())),
                ])
            ),
            "A + B"
        );
        assert_eq!(
            format!(
                "{}",
                Extend::Parallel(vec![
                    Box::new(Extend::Model(a.clone())),
                    Box::new(Extend::Model(b.clone())),
                ])
            ),
            "A | B"
        );
    }

    #[test]
    fn test_unroll_model_not_found() {
        let (ast, _) = parse(SRC, 0).unwrap();
        let model_rc = construct_model(&ast, None, &[]).unwrap();

        let result = unroll_extend_expression(
            ExpressionNode::Unresolved(ast::Expression::Variable(ast::Identifier::new(
                "NonExistent",
            ))),
            model_rc.clone(),
        );
        assert!(result.is_err());
        let err = result.unwrap_err();
        assert!(err.message.contains("NonExistent"));
    }

    #[test]
    fn test_unroll_unsupported_expression() {
        use crate::semantic::ModelNode;

        let model = ModelNode::new("Root", None);

        // ExpressionNode::BitwiseAnd не поддерживается — должна вернуть ошибку
        let result = unroll_extend_expression(
            ExpressionNode::BitwiseAnd(
                Box::new(ExpressionNode::Model(model.clone())),
                Box::new(ExpressionNode::Model(model.clone())),
            ),
            model.clone(),
        );
        assert!(result.is_err());
    }
}
