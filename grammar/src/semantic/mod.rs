//! Семантические узлы языка BuT.
//!
//! Этот модуль определяет структуры данных, представляющие результат
//! семантического анализа программы BuT:
//!
//! - [`ContextNode`] — область видимости с импортами, моделями, переменными и т.д.
//! - [`ModelNode`] — семантическая модель (конечный автомат или компоновка).
//! - [`StateNode`] — состояние автомата (неразрешённое, простое или с реализацией).
//! - [`Reference`] — ссылка на другой узел с условием перехода.
//! - [`Condition`] — условие перехода между состояниями.

pub mod tree;

use std::collections::HashMap;
use std::fmt::Debug;
use std::rc::Rc;

/// Диагностическое сообщение семантического анализатора.
///
/// Содержит список строк с описанием ошибок.
#[derive(Debug)]
pub enum Diagnostic {
    /// Одна или несколько ошибок.
    Error(Vec<String>),
}

impl Into<Diagnostic> for &str {
    fn into(self) -> Diagnostic {
        Diagnostic::Error(vec![self.to_owned()])
    }
}

/// Область видимости (контекст) в семантическом дереве.
///
/// Хранит все символы, объявленные в данной области: импорты, модели,
/// именованные блоки, функции, переменные, типы и условия.
///
/// Поле `upper` указывает на объемлющий контекст (если есть),
/// что позволяет реализовать поиск имён по цепочке областей видимости.
#[derive(Default, Debug, PartialEq, Eq, Clone)]
pub struct ContextNode {
    /// Объемлющий контекст (родительская область видимости).
    pub upper: Option<Rc<ContextNode>>,
    /// Импортированные модели (`import ... as Имя`).
    pub imports: HashMap<String, Rc<ModelNode>>,
    /// Вложенные именованные модели.
    pub models: HashMap<String, Rc<ModelNode>>,
    /// Именованные блоки кода (`enter`, `exit`, `always`, …).
    pub named_blocks: HashMap<String, NamedBlockNode>,
    /// Объявленные функции.
    pub functions: HashMap<String, FunctionNode>,
    /// Объявленные переменные.
    pub variables: HashMap<String, VariableNode>,
    /// Объявленные псевдонимы типов.
    pub types: HashMap<String, TypeNode>,
    /// Объявленные условия переходов.
    pub conditions: HashMap<String, ConditionNode>,
}

/// Семантический узел модели (конечного автомата).
///
/// Содержит контекст модели, её имя, словарь состояний и
/// информацию о реализации (`implements`).
#[derive(Default, Debug, PartialEq, Eq, Clone)]
pub struct ModelNode {
    /// Контекст (область видимости) модели.
    pub context: ContextNode,
    /// Имя модели (`None` для анонимной корневой модели).
    pub name: Option<String>,
    /// Состояния модели: имя → узел состояния.
    pub states: HashMap<String, StateNode>,
    /// Информация о реализации (зарезервировано).
    pub implements: (),
}

impl ModelNode {
    /// Возвращает `true`, если модель содержит хотя бы одно состояние.
    ///
    /// # Примеры
    ///
    /// ```
    /// use grammar::parse;
    /// use grammar::semantic::tree::construct_model;
    ///
    /// // Модель без состояний
    /// let (ast, _) = parse("type u8 = [bit;8];", 0).unwrap();
    /// let node = construct_model(&ast).unwrap();
    /// assert!(!node.has_states());
    ///
    /// // Модель с состоянием
    /// let (ast, _) = parse("start S;", 0).unwrap();
    /// let node = construct_model(&ast).unwrap();
    /// assert!(node.has_states());
    /// ```
    pub fn has_states(&self) -> bool {
        !self.states.is_empty()
    }
}

/// Семантический узел именованного блока кода (`enter`, `exit`, `always`, …).
///
/// В текущей реализации является заглушкой; будет расширен в будущих версиях.
#[derive(Default, Debug, PartialEq, Eq, Clone)]
pub struct NamedBlockNode {}

/// Семантический узел функции.
///
/// В текущей реализации является заглушкой; будет расширен в будущих версиях.
#[derive(Default, Debug, PartialEq, Eq, Clone)]
pub struct FunctionNode {}

/// Семантический узел переменной.
///
/// В текущей реализации является заглушкой; будет расширен в будущих версиях.
#[derive(Default, Debug, PartialEq, Eq, Clone)]
pub struct VariableNode {}

/// Семантический узел псевдонима типа.
///
/// В текущей реализации является заглушкой; будет расширен в будущих версиях.
#[derive(Default, Debug, PartialEq, Eq, Clone)]
pub struct TypeNode {}

/// Семантический узел условия перехода.
///
/// В текущей реализации является заглушкой; будет расширен в будущих версиях.
#[derive(Default, Debug, PartialEq, Eq, Clone)]
pub struct ConditionNode {}

/// Состояние конечного автомата.
///
/// Три варианта:
/// - [`Unresolved`](StateNode::Unresolved) — заглушка на время первого прохода построения.
/// - [`Simple`](StateNode::Simple) — обычное состояние без реализации.
/// - [`Implement`](StateNode::Implement) — состояние с реализацией (`= Модель`),
///   может иметь оператор `next`.
#[derive(Default, Debug, PartialEq, Eq, Clone)]
pub enum StateNode {
    /// Состояние не разрешено (временная заглушка при построении дерева).
    #[default]
    Unresolved,
    /// Обычное состояние: контекст, имя и список ссылок на переходы.
    Simple {
        /// Контекст состояния.
        context: ContextNode,
        /// Имя состояния.
        name: String,
        /// Ссылки-переходы (`ref Имя [: Условие]`).
        references: Vec<Reference<StateNode>>,
    },
    /// Состояние с реализацией (`= Модель`): может иметь `next`-переход.
    Implement {
        /// Контекст состояния.
        context: ContextNode,
        /// Имя состояния.
        name: String,
        /// Ссылки-переходы.
        references: Vec<Reference<StateNode>>,
        /// Информация о реализации (зарезервировано).
        implements: (),
        /// Единственный `next`-переход (если задан).
        next: Option<Reference<StateNode>>,
    },
}

/// Условие перехода между состояниями.
///
/// В текущей реализации поддерживается только вариант [`None`](Condition::None),
/// означающий безусловный переход. Полный набор условий — в будущих версиях.
#[derive(Default, Debug, PartialEq, Eq, Clone)]
pub enum Condition {
    /// Безусловный переход (условие не задано или не разрешено).
    #[default]
    None,
}

/// Ссылка на узел семантического дерева с условием перехода.
///
/// Параметр `T` — тип целевого узла (обычно [`StateNode`]).
#[derive(Default, Debug, PartialEq, Eq, Clone)]
pub struct Reference<T: Clone + PartialEq + Eq + Debug> {
    /// Имя целевого состояния.
    pub name: String,
    /// Условие перехода.
    pub cond: Condition,
    /// Целевой узел (может быть [`StateNode::Unresolved`] до второго прохода).
    pub object: Box<T>,
}

#[cfg(test)]
mod tests {
    use super::*;

    // ─── Diagnostic ──────────────────────────────────────────────────────

    /// Конвертация `&str` в `Diagnostic::Error`.
    #[test]
    fn diagnostic_from_str() {
        let d: Diagnostic = "что-то пошло не так".into();
        let Diagnostic::Error(msgs) = d;
        assert_eq!(msgs.len(), 1);
        assert_eq!(msgs[0], "что-то пошло не так");
    }

    /// Debug-вывод Diagnostic не паникует.
    #[test]
    fn diagnostic_debug() {
        let d: Diagnostic = "ошибка".into();
        let _ = format!("{:?}", d);
    }

    // ─── ContextNode ─────────────────────────────────────────────────────

    /// Default-контекст пустой.
    #[test]
    fn context_node_default_is_empty() {
        let ctx = ContextNode::default();
        assert!(ctx.imports.is_empty());
        assert!(ctx.models.is_empty());
        assert!(ctx.named_blocks.is_empty());
        assert!(ctx.functions.is_empty());
        assert!(ctx.variables.is_empty());
        assert!(ctx.types.is_empty());
        assert!(ctx.conditions.is_empty());
        assert!(ctx.upper.is_none());
    }

    // ─── ModelNode ───────────────────────────────────────────────────────

    /// ModelNode по умолчанию не содержит состояний.
    #[test]
    fn model_node_default_has_no_states() {
        let node = ModelNode::default();
        assert!(!node.has_states());
    }

    /// ModelNode с одним состоянием: has_states() → true.
    #[test]
    fn model_node_with_state_has_states() {
        let mut node = ModelNode::default();
        node.states.insert("S".to_string(), StateNode::default());
        assert!(node.has_states());
    }

    /// Debug-вывод ModelNode не паникует.
    #[test]
    fn model_node_debug() {
        let node = ModelNode::default();
        let _ = format!("{:?}", node);
    }

    // ─── StateNode ───────────────────────────────────────────────────────

    /// StateNode::default() равен Unresolved.
    #[test]
    fn state_node_default_is_unresolved() {
        assert_eq!(StateNode::default(), StateNode::Unresolved);
    }

    /// Debug-вывод StateNode::Simple не паникует.
    #[test]
    fn state_node_simple_debug() {
        let s = StateNode::Simple {
            context: ContextNode::default(),
            name: "S".to_string(),
            references: vec![],
        };
        let _ = format!("{:?}", s);
    }

    /// Debug-вывод StateNode::Implement не паникует.
    #[test]
    fn state_node_implement_debug() {
        let s = StateNode::Implement {
            context: ContextNode::default(),
            name: "A".to_string(),
            references: vec![],
            implements: (),
            next: None,
        };
        let _ = format!("{:?}", s);
    }

    // ─── Reference ──────────────────────────────────────────────────────

    /// Создание Reference<StateNode> с Unresolved-объектом.
    #[test]
    fn reference_unresolved() {
        let r: Reference<StateNode> = Reference {
            name: "X".to_string(),
            cond: Condition::None,
            object: Box::new(StateNode::Unresolved),
        };
        assert_eq!(r.name, "X");
        assert_eq!(r.cond, Condition::None);
        assert_eq!(*r.object, StateNode::Unresolved);
    }

    /// Reference по умолчанию (Default).
    #[test]
    fn reference_default() {
        let r: Reference<StateNode> = Reference::default();
        assert!(r.name.is_empty());
    }

    // ─── Condition ──────────────────────────────────────────────────────

    /// Condition::default() равен None.
    #[test]
    fn condition_default_is_none() {
        assert_eq!(Condition::default(), Condition::None);
    }

    /// Заглушки-узлы реализуют Default.
    #[test]
    fn stub_nodes_default() {
        let _ = NamedBlockNode::default();
        let _ = FunctionNode::default();
        let _ = VariableNode::default();
        let _ = TypeNode::default();
        let _ = ConditionNode::default();
    }
}
