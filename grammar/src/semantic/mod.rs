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

use crate::parser::ast::Expression;
use std::cell::RefCell;
use std::collections::HashMap;
use std::fmt::Debug;
use std::rc::Rc;

/// Семантический узел модели (конечного автомата).
///
/// Содержит контекст модели, её имя, словарь состояний и
/// информацию о реализации (`implements`).
#[derive(Default, Debug, PartialEq, Eq)]
pub struct ModelNode {
    /// Имя модели (`None` для анонимной корневой модели).
    pub name: Option<String>,
    /// Модель уровнем выше.
    pub upper: Option<Rc<RefCell<ModelNode>>>,
    /// Вложенные именованные модели.
    pub models: HashMap<String, Rc<RefCell<ModelNode>>>,
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
    /// Состояния модели: имя → узел состояния.
    pub states: HashMap<String, StateNode>,
    /// Информация о реализации (зарезервировано).
    pub implements: Implement,
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
    /// let node = construct_model(&ast, None).unwrap();
    /// assert!(!node.borrow().has_states());
    ///
    /// // Модель с состоянием
    /// let (ast, _) = parse("start S;", 0).unwrap();
    /// let node = construct_model(&ast, None).unwrap();
    /// assert!(node.borrow().has_states());
    /// ```
    pub fn has_states(&self) -> bool {
        !self.states.is_empty()
    }

    pub fn search_model(&self, name: &str) -> Option<Rc<RefCell<ModelNode>>> {
        if let Some(model) = self.models.get(name) {
            Some(Rc::clone(model))
        } else if let Some(model) = self.upper.as_ref() {
            return model.borrow().search_model(name);
        } else {
            None
        }
    }

    pub fn search_var(&self, name: &str) -> Option<VariableNode> {
        if let Some(var) = self.variables.get(name) {
            Some(var.clone())
        } else if let Some(model) = self.upper.as_ref() {
            return model.borrow().search_var(name);
        } else {
            None
        }
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
pub enum VariableNode {
    #[default]
    Unresolved,
    Simple(String, TypeNode, Option<Expression>),
    Port(String, TypeNode, Expression),
    Const(String, TypeNode, Expression),
}

/// Семантический узел псевдонима типа.
///
/// В текущей реализации является заглушкой; будет расширен в будущих версиях.
#[derive(Default, Debug, PartialEq, Eq, Clone)]
pub enum TypeNode {
    #[default]
    Detecting,
    Address(u64, Option<u64>),
    Bit,
    Rational,
    Array(u16, Box<TypeNode>),
    Unsupported,
}

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
        /// Именованные блоки кода (`enter`, `exit`, `always`, …).
        named_blocks: HashMap<String, NamedBlockNode>,
        /// Имя состояния.
        name: String,
        /// Ссылки-переходы (`ref Имя [: Условие]`).
        references: Vec<Reference<StateNode>>,
    },
    /// Состояние с реализацией (`= Модель`): может иметь `next`-переход.
    Implement {
        /// Именованные блоки кода (`enter`, `exit`, `always`, …).
        named_blocks: HashMap<String, NamedBlockNode>,
        /// Имя состояния.
        name: String,
        /// Ссылки-переходы.
        references: Vec<Reference<StateNode>>,
        /// Информация о реализации (зарезервировано).
        implements: Implement,
        expression: Option<Expression>,
        /// Единственный `next`-переход (если задан).
        next: Option<Reference<StateNode>>,
    },
}

#[derive(Default, Debug, PartialEq, Eq, Clone)]
pub enum Implement {
    #[default]
    Unresolved,
    Model(Rc<RefCell<ModelNode>>),
    Parentless(Box<Implement>),
    Add(Box<Implement>, Box<Implement>),
    Or(Box<Implement>, Box<Implement>),
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
    use crate::diagnostics::Diagnostic;

    // ─── Diagnostic ──────────────────────────────────────────────────────

    /// Конвертация `&str` в `Diagnostic::Error`.
    #[test]
    fn diagnostic_from_str() {
        let d: Diagnostic = "что-то пошло не так".into();
        let Diagnostic { message: msgs, .. } = d;
        assert!(msgs.len() > 0);
        assert_eq!(msgs, "что-то пошло не так");
    }

    /// Debug-вывод Diagnostic не паникует.
    #[test]
    fn diagnostic_debug() {
        let d: Diagnostic = "ошибка".into();
        let _ = format!("{:?}", d);
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
