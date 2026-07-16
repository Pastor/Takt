//! Семантические узлы языка Lam.
//!
//! Этот модуль определяет структуры данных, представляющие результат
//! семантического анализа программы Lam:
//!
//! - [`ModelNode`] — семантическая модель: конечный автомат или компоновка автоматов;
//!   содержит словари состояний, переменных, условий, функций и вложенных моделей.
//! - [`StateNode`] — состояние автомата (неразрешённое, простое или с реализацией).
//! - [`ReferenceNode`] — ссылка-переход между состояниями с опциональным условием.
//! - [`ConditionNode`] — семантическое представление условия перехода.
//! - [`VariableNode`] — переменная, порт или константа с разрешённым типом.
//! - [`Extend`] — реализация модели: ссылка, последовательная или параллельная компоновка.

mod builtin;
mod condition;
pub(crate) mod docs;
pub mod enum_node;
mod expression;
pub mod extend;
pub mod formula;
mod function;
mod import;
pub mod index;
/// Карта семантических элементов модели для генератора кода.
///
/// Содержит снимок достижимых состояний и моделей в виде плоской карты [`Map`](minimap::Map),
/// используемый генератором C-заголовков.
pub mod minimap;
mod named_block;
pub(crate) mod naming;
mod reference;
mod statement;
pub mod struct_node;
mod test_constants;
pub mod tree;
mod type_inference;
pub mod type_node;
pub mod unused;
pub(crate) mod validate;

use crate::diagnostics::Location;
use crate::parser::ast;
pub use crate::parser::ast::PortDirection;
use crate::parser::ast::{Member, NamedArgument, ParameterList, Type};
pub use crate::semantic::enum_node::EnumDefinitionNode;
pub use crate::semantic::formula::Formula;
pub use crate::semantic::struct_node::StructDefinitionNode;
use extend::Extend;
use std::cell::RefCell;
use std::collections::BTreeMap;
use std::fmt::Debug;
use std::rc::Rc;
use std::rc::Weak;
use type_node::TypeNode;

/// Семантический узел модели (конечного автомата).
///
/// Содержит контекст модели, её имя, словарь состояний и
/// информацию о реализации (`implements`).
///
/// Поля [`doc`](ModelNode::doc) и [`docs`](ModelNode::docs) заполняются
/// отдельным вызовом [`construct_model_with_docs`](tree::construct_model_with_docs)
/// и содержат строки из `///`-комментариев исходного текста.
#[derive(Default, Debug)]
pub struct ModelNode {
    /// Имя модели (`None` для анонимной корневой модели).
    pub name: Option<String>,
    /// Позиция объявления модели в исходном тексте.
    pub loc: Location,
    /// Модель уровнем выше (слабая ссылка для предотвращения циклов Rc).
    pub upper: Option<Weak<RefCell<ModelNode>>>,
    /// Вложенные именованные модели.
    pub models: BTreeMap<String, Rc<RefCell<ModelNode>>>,
    /// Именованные блоки кода (`enter`, `exit`, `always`, …).
    pub named_blocks: Vec<NamedCodeBlockDefinitionNode>,
    /// Объявленные функции.
    pub functions: BTreeMap<String, FunctionDefinitionNode>,
    /// Объявленные переменные.
    pub variables: BTreeMap<String, VariableNode>,
    /// Объявленные псевдонимы типов.
    pub types: BTreeMap<String, TypeNode>,
    /// Позиции объявлений псевдонимов типов: имя → позиция в исходном тексте.
    pub type_locs: BTreeMap<String, Location>,
    /// Сырые АСД-типы псевдонимов: имя → оригинальный AST-тип до разрешения.
    ///
    /// Используется `check_recursive_type_aliases` в `validate.rs` для обнаружения
    /// циклических ссылок между псевдонимами (Ce16).
    pub raw_type_defs: BTreeMap<String, ast::Type>,
    /// Сырые АСД-операторы именованных блоков: `(имя_блока, оригинальный_оператор)`.
    ///
    /// Используется `SemanticIndex` (I8) для индексации объявлений локальных переменных
    /// внутри `enter`/`exit`/`always`-блоков, позиции которых теряются при разрешении.
    pub named_block_raw: Vec<(String, ast::Statement)>,
    /// Объявленные условия переходов.
    pub conditions: BTreeMap<String, ConditionDefinitionNode>,
    /// Объявленные перечисления (Ce4).
    pub enums: BTreeMap<String, EnumDefinitionNode>,
    /// Объявленные структурные типы (NI3).
    pub structs: BTreeMap<String, StructDefinitionNode>,
    /// Состояния модели: имя → узел состояния.
    pub states: BTreeMap<String, StateNode>,
    /// Информация о реализации (зарезервировано).
    pub implements: Extend,
    /// Документация самой модели (строки из `///`-комментариев перед `model`).
    ///
    /// Заполняется [`construct_model_with_docs`](tree::construct_model_with_docs).
    /// Пусто у анонимной корневой модели и при использовании [`construct_model`](tree::construct_model).
    pub doc: Vec<String>,
    /// Документация именованных элементов модели.
    ///
    /// Ключ — имя элемента (переменной, состояния, функции, типа, условия).
    /// Значение — список строк из `///`-комментариев, предшествующих объявлению.
    ///
    /// Заполняется [`construct_model_with_docs`](tree::construct_model_with_docs).
    pub docs: BTreeMap<String, Vec<String>>,
    /// Встроенные формулы модели.
    pub formulas: Vec<Formula>,
    /// Привязки адресов портов оператором `address` (фича 0020, слой AddressMap).
    ///
    /// Каждый элемент — один оператор `address Имя = <выражение>;`. Разрешение
    /// (привязка к порту, приоритет источников inline/`address`/внешняя карта) и
    /// диагностики выполняет [`check_port_addresses`](validate::check_port_addresses).
    pub address_defs: Vec<AddressBindingNode>,
}

/// Привязка адреса к порту оператором `address` (фича 0020).
///
/// Хранит имя целевого порта, позицию оператора и выражение-адрес. Выражение
/// остаётся «сырым» ([`ExpressionNode::Unresolved`]) до понижения в конкретный
/// адрес потребителем (C-генерация, задача 0020-05).
#[derive(Debug, Clone)]
pub struct AddressBindingNode {
    /// Имя порта, которому назначается адрес.
    pub port: String,
    /// Позиция оператора `address` в исходном тексте.
    pub loc: Location,
    /// Выражение-адрес (сырое АСД до понижения).
    pub value: ExpressionNode,
}

impl ModelNode {
    pub(crate) fn new(
        name: &str,
        parent: Option<Rc<RefCell<ModelNode>>>,
    ) -> Rc<RefCell<ModelNode>> {
        let model = ModelNode {
            name: Some(name.to_string()),
            loc: Location::Codegen,
            upper: parent.as_ref().map(Rc::downgrade),
            models: Default::default(),
            named_blocks: vec![],
            functions: Default::default(),
            variables: Default::default(),
            types: Default::default(),
            type_locs: Default::default(),
            raw_type_defs: Default::default(),
            named_block_raw: vec![],
            conditions: Default::default(),
            enums: Default::default(),
            structs: BTreeMap::new(),
            states: BTreeMap::new(),
            implements: Extend::None,
            doc: Vec::new(),
            docs: BTreeMap::new(),
            formulas: Vec::new(),
            address_defs: Vec::new(),
        };
        let model = Rc::new(RefCell::new(model));
        if let Some(parent) = &parent {
            parent
                .borrow_mut()
                .models
                .insert(name.to_string(), Rc::clone(&model));
        }
        model
    }

    pub(crate) fn name(&self) -> &str {
        if self.name.is_none() {
            return "";
        }
        self.name.as_ref().unwrap().as_str()
    }
}

impl ModelNode {
    pub(crate) fn get_start_state(&self) -> Option<StateNode> {
        self.states
            .clone()
            .into_values()
            .filter(|state| state.kind() == StateNodeKind::Start)
            .collect::<Vec<StateNode>>()
            .first()
            .cloned()
    }

    /// Возвращает все конечные состояния модели.
    #[allow(dead_code)]
    pub(crate) fn get_end_states(&self) -> Vec<StateNode> {
        self.states
            .clone()
            .into_values()
            .filter(|state| state.kind() == StateNodeKind::End)
            .collect::<Vec<StateNode>>()
    }

    pub(crate) fn copy(
        &self,
        new_name: Option<String>,
        upper: Option<Rc<RefCell<ModelNode>>>,
    ) -> ModelNode {
        let name = if new_name.is_none() {
            self.name.clone()
        } else {
            new_name.clone()
        };
        ModelNode {
            name,
            loc: self.loc,
            upper: upper.map(|p| Rc::downgrade(&p)),
            models: self.models.clone(),
            named_blocks: self.named_blocks.clone(),
            functions: self.functions.clone(),
            variables: self.variables.clone(),
            types: self.types.clone(),
            type_locs: self.type_locs.clone(),
            raw_type_defs: self.raw_type_defs.clone(),
            named_block_raw: self.named_block_raw.clone(),
            conditions: self.conditions.clone(),
            enums: self.enums.clone(),
            structs: self.structs.clone(),
            states: self.states.clone(),
            implements: self.implements.clone(),
            doc: self.doc.clone(),
            docs: self.docs.clone(),
            formulas: self.formulas.clone(),
            address_defs: self.address_defs.clone(),
        }
    }
}

impl PartialEq for ModelNode {
    fn eq(&self, other: &Self) -> bool {
        // upper, loc, type_locs игнорируются: не являются частью идентичности модели
        self.name == other.name
            && self.models == other.models
            && self.named_blocks == other.named_blocks
            && self.functions == other.functions
            && self.variables == other.variables
            && self.types == other.types
            && self.conditions == other.conditions
            && self.enums == other.enums
            && self.states == other.states
            && self.implements == other.implements
            && self.formulas == other.formulas
    }
}

impl Eq for ModelNode {}

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
    /// let node = construct_model(&ast, None, &[]).unwrap();
    /// assert!(!node.borrow().has_states());
    ///
    /// // Модель с состоянием
    /// let (ast, _) = parse("start S;", 0).unwrap();
    /// let node = construct_model(&ast, None, &[]).unwrap();
    /// assert!(node.borrow().has_states());
    /// ```
    pub fn has_states(&self) -> bool {
        !self.states.is_empty()
    }

    /// Ищет модель по имени в текущем контексте и во всех родительских.
    ///
    /// Обходит цепочку `upper`-ссылок вверх до тех пор, пока модель не найдена
    /// или цепочка не исчерпана.
    ///
    /// # Примеры
    ///
    /// ```
    /// use grammar::parse;
    /// use grammar::semantic::tree::construct_model;
    ///
    /// let (ast, _) = parse("model Inner { start S; }", 0).unwrap();
    /// let root = construct_model(&ast, None, &[]).unwrap();
    /// // Вложенная модель «Inner» доступна из корня
    /// assert!(root.borrow().search_model("Inner").is_some());
    /// // Несуществующая модель возвращает None
    /// assert!(root.borrow().search_model("Ghost").is_none());
    /// ```
    pub fn search_model(&self, name: &str) -> Option<Rc<RefCell<ModelNode>>> {
        if let Some(model) = self.models.get(name) {
            Some(Rc::clone(model))
        } else if let Some(model) = self.upper.as_ref().and_then(|w| w.upgrade()) {
            return model.borrow().search_model(name);
        } else {
            None
        }
    }

    /// Ищет переменную по имени в текущем контексте и во всех родительских.
    ///
    /// Аналогично [`search_model`](ModelNode::search_model), обходит цепочку
    /// `upper`-ссылок вверх.
    ///
    /// # Примеры
    ///
    /// ```
    /// use grammar::parse;
    /// use grammar::semantic::tree::construct_model;
    ///
    /// let (ast, _) = parse("var x: bit := false;", 0).unwrap();
    /// let root = construct_model(&ast, None, &[]).unwrap();
    /// assert!(root.borrow().search_var("x").is_some());
    /// assert!(root.borrow().search_var("y").is_none());
    /// ```
    pub fn search_var(&self, name: &str) -> Option<VariableNode> {
        if let Some(var) = self.variables.get(name) {
            Some(var.clone())
        } else if let Some(model) = self.upper.as_ref().and_then(|w| w.upgrade()) {
            return model.borrow().search_var(name);
        } else {
            None
        }
    }

    /// Ищет именованное условие по `name`, обходя цепочку `upper`.
    pub fn search_cond(&self, name: &str) -> Option<ConditionDefinitionNode> {
        if let Some(cond) = self.conditions.get(name) {
            Some(cond.clone())
        } else if let Some(model) = self.upper.as_ref().and_then(|w| w.upgrade()) {
            return model.borrow().search_cond(name);
        } else {
            None
        }
    }

    /// Ищет объявление перечисления по `name`, обходя цепочку `upper` (Ce4).
    ///
    /// Возвращает клон [`EnumDefinitionNode`], если перечисление найдено в текущем
    /// или родительском контексте.
    ///
    /// # Пример
    ///
    /// ```
    /// use grammar::semantic::{ModelNode, EnumDefinitionNode};
    ///
    /// let mut model = ModelNode::default();
    /// let e = EnumDefinitionNode::new("Color", &[("Red", None), ("Green", None)]);
    /// model.enums.insert("Color".to_string(), e);
    /// assert!(model.search_enum("Color").is_some());
    /// assert!(model.search_enum("Size").is_none());
    /// ```
    pub fn search_enum(&self, name: &str) -> Option<EnumDefinitionNode> {
        if let Some(e) = self.enums.get(name) {
            Some(e.clone())
        } else if let Some(model) = self.upper.as_ref().and_then(|w| w.upgrade()) {
            return model.borrow().search_enum(name);
        } else {
            None
        }
    }

    /// Ищет вариант перечисления по имени варианта среди всех доступных перечислений (NI6).
    ///
    /// Обходит все перечисления текущего контекста и родительских.
    /// Возвращает `(имя_перечисления, числовое_значение)` при нахождении.
    ///
    /// # Пример
    ///
    /// ```
    /// use grammar::semantic::{ModelNode, EnumDefinitionNode};
    ///
    /// let mut model = ModelNode::default();
    /// let e = EnumDefinitionNode::new("Direction", &[("North", None), ("South", Some(180))]);
    /// model.enums.insert("Direction".to_string(), e);
    /// let result = model.search_enum_variant("North");
    /// assert!(result.is_some());
    /// let (enum_node, value) = result.unwrap();
    /// assert_eq!(enum_node.name, "Direction");
    /// assert_eq!(value, 0);
    /// assert_eq!(model.search_enum_variant("East"), None);
    /// ```
    pub fn search_enum_variant(&self, variant_name: &str) -> Option<(EnumDefinitionNode, i64)> {
        for enum_node in self.enums.values() {
            if let Some(val) = enum_node.find_variant(variant_name) {
                return Some((enum_node.clone(), val));
            }
        }
        if let Some(model) = self.upper.as_ref().and_then(|w| w.upgrade()) {
            return model.borrow().search_enum_variant(variant_name);
        }
        None
    }

    /// Ищет структурный тип по имени, включая родительские модели (NI3).
    pub fn search_struct(&self, name: &str) -> Option<StructDefinitionNode> {
        if let Some(s) = self.structs.get(name) {
            Some(s.clone())
        } else if let Some(model) = self.upper.as_ref().and_then(|w| w.upgrade()) {
            model.borrow().search_struct(name)
        } else {
            None
        }
    }

    /// Возвращает список всех именованных блоков с заданным именем.
    pub fn get_named_blocks(&self, name: &str) -> Vec<&NamedCodeBlockDefinitionNode> {
        self.named_blocks
            .iter()
            .filter(|b| b.name() == name)
            .collect()
    }

    /// Возвращает первый именованный блок с заданным именем, если он есть.
    pub fn get_named_block(&self, name: &str) -> Option<&NamedCodeBlockDefinitionNode> {
        self.named_blocks.iter().find(|b| b.name() == name)
    }

    /// Ищет объявление функции по `name`, обходя цепочку `upper`.
    pub fn search_func(&self, name: &str) -> Option<Rc<RefCell<FunctionDefinitionNode>>> {
        if let Some(func) = self.functions.get(name) {
            Some(Rc::new(RefCell::new(func.clone())))
        } else if let Some(model) = self.upper.as_ref().and_then(|w| w.upgrade()) {
            return model.borrow().search_func(name);
        } else {
            None
        }
    }

    /// Ищет состояние по имени в текущей модели, затем рекурсивно в родительской.
    pub fn search_state(&self, name: &str) -> Option<Rc<RefCell<StateNode>>> {
        if let Some(state) = self.states.get(name) {
            Some(Rc::new(RefCell::new(state.clone())))
        } else if let Some(model) = self.upper.as_ref().and_then(|w| w.upgrade()) {
            return model.borrow().search_state(name);
        } else {
            None
        }
    }

    /// Ищет псевдоним типа по имени, поднимаясь по цепочке родительских моделей.
    pub fn search_type(&self, name: &str) -> Option<Rc<RefCell<TypeNode>>> {
        if let Some(type_) = self.types.get(name) {
            Some(Rc::new(RefCell::new(type_.clone())))
        } else if let Some(model) = self.upper.as_ref().and_then(|w| w.upgrade()) {
            return model.borrow().search_type(name);
        } else {
            None
        }
    }

    /// Возвращает документацию самой модели.
    ///
    /// Заполняется только при использовании
    /// [`construct_model_with_docs`](tree::construct_model_with_docs).
    /// Возвращает пустой срез для анонимной корневой модели или при
    /// использовании [`construct_model`](tree::construct_model).
    ///
    /// # Примеры
    ///
    /// ```
    /// # use grammar::parse;
    /// # use grammar::semantic::tree::construct_model_with_docs;
    /// let (ast, comments) = parse("/// Тест\nmodel M { start S; }", 0).unwrap();
    /// let root = construct_model_with_docs(&ast, None, &[], &comments).unwrap();
    /// let m = root.borrow().search_model("M").unwrap();
    /// assert_eq!(m.borrow().own_doc(), ["Тест"]);
    /// ```
    pub fn own_doc(&self) -> &[String] {
        &self.doc
    }

    /// Возвращает документацию именованного элемента (состояния, переменной,
    /// функции, типа, условия).
    ///
    /// Заполняется только при использовании
    /// [`construct_model_with_docs`](tree::construct_model_with_docs).
    /// Возвращает пустой срез, если документация отсутствует или не загружена.
    ///
    /// # Примеры
    ///
    /// ```
    /// # use grammar::parse;
    /// # use grammar::semantic::tree::construct_model_with_docs;
    /// let (ast, comments) = parse("/// Состояние.\nstart S;", 0).unwrap();
    /// let root = construct_model_with_docs(&ast, None, &[], &comments).unwrap();
    /// assert_eq!(root.borrow().element_doc("S"), ["Состояние."]);
    /// ```
    pub fn element_doc(&self, name: &str) -> &[String] {
        self.docs.get(name).map(|v| v.as_slice()).unwrap_or(&[])
    }
}

/// Семантический узел именованного блока кода (`enter`, `exit`, `always`, …).
#[derive(Default, Debug, Clone)]
pub enum NamedCodeBlockDefinitionNode {
    /// Блок не задан.
    #[default]
    None,
    /// Неразрешённый блок кода: `(имя, AST-оператор)`.
    Unresolved(String, ast::Statement),
    /// Блок `enter` с разрешённым оператором.
    Enter {
        /// Родительская модель (слабая ссылка для предотвращения циклов Rc).
        upper: Option<Weak<RefCell<ModelNode>>>,
        /// Тело блока.
        body: StatementNode,
    },
    /// Блок `exit` с разрешённым оператором.
    Exit {
        /// Родительская модель (слабая ссылка для предотвращения циклов Rc).
        upper: Option<Weak<RefCell<ModelNode>>>,
        /// Тело блока.
        body: StatementNode,
    },
    /// Блок `always` с разрешённым оператором.
    Always {
        /// Родительская модель (слабая ссылка для предотвращения циклов Rc).
        upper: Option<Weak<RefCell<ModelNode>>>,
        /// Тело блока.
        body: StatementNode,
    },
    /// Пользовательский именованный блок.
    Unknown {
        /// Родительская модель (слабая ссылка для предотвращения циклов Rc).
        upper: Option<Weak<RefCell<ModelNode>>>,
        /// Имя блока.
        name: String,
        /// Тело блока.
        body: StatementNode,
    },
}

impl PartialEq for NamedCodeBlockDefinitionNode {
    fn eq(&self, other: &Self) -> bool {
        match (self, other) {
            (Self::None, Self::None) => true,
            (Self::Unresolved(n1, s1), Self::Unresolved(n2, s2)) => n1 == n2 && s1 == s2,
            (Self::Enter { body: b1, .. }, Self::Enter { body: b2, .. }) => b1 == b2,
            (Self::Exit { body: b1, .. }, Self::Exit { body: b2, .. }) => b1 == b2,
            (Self::Always { body: b1, .. }, Self::Always { body: b2, .. }) => b1 == b2,
            (
                Self::Unknown {
                    name: n1, body: b1, ..
                },
                Self::Unknown {
                    name: n2, body: b2, ..
                },
            ) => n1 == n2 && b1 == b2,
            _ => false,
        }
    }
}

impl Eq for NamedCodeBlockDefinitionNode {}

impl NamedCodeBlockDefinitionNode {
    /// Возвращает имя блока.
    pub fn name(&self) -> &str {
        match self {
            NamedCodeBlockDefinitionNode::None => "",
            NamedCodeBlockDefinitionNode::Unresolved(name, _) => name,
            NamedCodeBlockDefinitionNode::Enter { .. } => "enter",
            NamedCodeBlockDefinitionNode::Exit { .. } => "exit",
            NamedCodeBlockDefinitionNode::Always { .. } => "always",
            NamedCodeBlockDefinitionNode::Unknown { name, .. } => name,
        }
    }

    /// Возвращает ссылку на семантический оператор блока, если он разрешён.
    pub fn statement(&self) -> Option<&StatementNode> {
        match self {
            NamedCodeBlockDefinitionNode::Enter { body, .. }
            | NamedCodeBlockDefinitionNode::Exit { body, .. }
            | NamedCodeBlockDefinitionNode::Always { body, .. }
            | NamedCodeBlockDefinitionNode::Unknown { body, .. } => Some(body),
            _ => None,
        }
    }

    /// Возвращает ссылку на родительскую модель блока.
    pub fn upper(&self) -> Option<Rc<RefCell<ModelNode>>> {
        match self {
            NamedCodeBlockDefinitionNode::Enter { upper, .. }
            | NamedCodeBlockDefinitionNode::Exit { upper, .. }
            | NamedCodeBlockDefinitionNode::Always { upper, .. }
            | NamedCodeBlockDefinitionNode::Unknown { upper, .. } => {
                upper.as_ref().and_then(|w| w.upgrade())
            }
            _ => None,
        }
    }
}

/// Семантическое определение функции.
#[derive(Default, Debug, Clone)]
pub enum FunctionDefinitionNode {
    /// Определение отсутствует.
    #[default]
    None,
    /// Неразрешённое AST-определение.
    Unresolved(ast::FunctionDefine),
    /// Локальная функция с телом.
    Local {
        /// Родительская модель (слабая ссылка для предотвращения циклов Rc).
        upper: Option<Weak<RefCell<ModelNode>>>,
        /// Позиция объявления функции в исходном тексте.
        loc: Location,
        /// Имя функции.
        name: String,
        /// Список параметров: `(имя, тип)`.
        params: Vec<(String, TypeNode)>,
        /// Возвращаемый тип.
        ret: TypeNode,
        /// Тело функции.
        body: StatementNode,
    },
    /// Внешняя функция (без тела).
    External {
        /// Родительская модель (слабая ссылка для предотвращения циклов Rc).
        upper: Option<Weak<RefCell<ModelNode>>>,
        /// Позиция объявления функции в исходном тексте.
        loc: Location,
        /// Имя функции.
        name: String,
        /// Список параметров: `(имя, тип)`.
        params: Vec<(String, TypeNode)>,
        /// Возвращаемый тип.
        ret: TypeNode,
    },
    /// Встроенная функция языка.
    Builtin(&'static str, &'static [(&'static str, TypeNode)], TypeNode),
}

impl PartialEq for FunctionDefinitionNode {
    fn eq(&self, other: &Self) -> bool {
        // upper и loc игнорируются: не являются частью семантической идентичности функции
        match (self, other) {
            (Self::None, Self::None) => true,
            (Self::Unresolved(a), Self::Unresolved(b)) => a == b,
            (
                Self::Local {
                    name: n1,
                    params: p1,
                    ret: r1,
                    body: b1,
                    ..
                },
                Self::Local {
                    name: n2,
                    params: p2,
                    ret: r2,
                    body: b2,
                    ..
                },
            ) => n1 == n2 && p1 == p2 && r1 == r2 && b1 == b2,
            (
                Self::External {
                    name: n1,
                    params: p1,
                    ret: r1,
                    ..
                },
                Self::External {
                    name: n2,
                    params: p2,
                    ret: r2,
                    ..
                },
            ) => n1 == n2 && p1 == p2 && r1 == r2,
            (Self::Builtin(n1, p1, r1), Self::Builtin(n2, p2, r2)) => {
                n1 == n2 && p1 == p2 && r1 == r2
            }
            _ => false,
        }
    }
}

impl Eq for FunctionDefinitionNode {}

impl FunctionDefinitionNode {
    /// Возвращает позицию объявления функции в исходном тексте.
    ///
    /// Для [`None`](FunctionDefinitionNode::None), [`Unresolved`](FunctionDefinitionNode::Unresolved) и
    /// [`Builtin`](FunctionDefinitionNode::Builtin) возвращает [`Location::Implicit`].
    pub fn loc(&self) -> Location {
        match self {
            FunctionDefinitionNode::Local { loc, .. }
            | FunctionDefinitionNode::External { loc, .. } => *loc,
            _ => Location::Implicit,
        }
    }

    /// Возвращает имя функции (пустая строка для `None` и `Unresolved`).
    pub fn name(&self) -> &str {
        match self {
            FunctionDefinitionNode::Local { name, .. }
            | FunctionDefinitionNode::External { name, .. } => name,
            FunctionDefinitionNode::Builtin(name, ..) => name,
            _ => "",
        }
    }
}

/// Семантический узел вызова или ссылки на функцию.
#[derive(Default, Debug, PartialEq, Eq, Clone)]
pub enum FunctionNode {
    /// Функция не задана.
    #[default]
    None,
    /// Неразрешённый вызов: `(имя, аргументы)`.
    Unresolved(String, Vec<ExpressionNode>),
    /// Вызов локальной функции.
    Local(Rc<RefCell<FunctionDefinitionNode>>, Vec<ExpressionNode>),
    /// Вызов внешней функции.
    External(Rc<RefCell<FunctionDefinitionNode>>, Vec<ExpressionNode>),
}

/// Семантический оператор языка Lam.
///
/// После семантического анализа (этап 4) все варианты `Unresolved` заменяются
/// конкретными разрешёнными вариантами.
#[derive(Default, Debug, PartialEq, Eq, Clone)]
pub enum StatementNode {
    /// Оператор отсутствует (значение по умолчанию).
    #[default]
    None,
    /// «Сырой» АСД-оператор, ещё не прошедший семантическое понижение.
    Unresolved(ast::Statement),
    /// Блок операторов: `{ операторы* }`.
    Block(Vec<StatementNode>),
    /// Оператор-выражение: присваивание, вызов функции и т.п.
    Expression(Box<ExpressionNode>),
    /// Условный оператор `if`.
    If {
        /// Условие.
        cond: Box<ExpressionNode>,
        /// Тело.
        then_: Box<StatementNode>,
        /// Ветка `else` (если задана).
        else_: Option<Box<StatementNode>>,
    },
    /// Цикл `loop [условие] { тело }`.
    /// Условие `None` означает бесконечный цикл.
    Loop {
        /// Условие продолжения (`None` — бесконечный цикл).
        cond: Option<Box<ExpressionNode>>,
        /// Тело цикла.
        body: Box<StatementNode>,
    },
    /// Оператор цикла `for`.
    For {
        /// Инициализация (опционально).
        init: Option<Box<StatementNode>>,
        /// Условие продолжения (опционально).
        cond: Option<Box<ExpressionNode>>,
        /// Выражение шага (опционально).
        step: Option<Box<ExpressionNode>>,
        /// Тело цикла.
        body: Box<StatementNode>,
    },
    /// Объявление локальной переменной: `(имя, тип, инициализатор?)`.
    Variable(String, TypeNode, Option<Box<ExpressionNode>>),
    /// Оператор `return [выражение]`.
    Return(Option<Box<ExpressionNode>>),
    /// Оператор `continue`.
    Continue,
    /// Оператор `break`.
    Break,
    /// Встроенная формула: `: условие1[, условие2, …];`
    InlineFormula(Vec<Formula>),
    /// Оператор `match`: `match expr { patterns => body, … }`.
    Match {
        /// Разбираемое выражение.
        expr: Box<ExpressionNode>,
        /// Ветки оператора.
        arms: Vec<MatchArmNode>,
    },
}

/// Семантическая ветка оператора `match`.
#[derive(Default, Debug, PartialEq, Eq, Clone)]
pub struct MatchArmNode {
    /// Образцы (хотя бы один).
    pub patterns: Vec<MatchPatternNode>,
    /// Тело ветки.
    pub body: Box<StatementNode>,
}

/// Семантический образец ветки `match`.
#[derive(Debug, PartialEq, Eq, Clone)]
pub enum MatchPatternNode {
    /// Конкретное значение.
    Value(Box<ExpressionNode>),
    /// Подстановочный образец `_`.
    Wildcard,
}

impl Default for MatchPatternNode {
    fn default() -> Self {
        Self::Wildcard
    }
}

/// Семантический узел переменной.
///
/// Варианты:
/// - [`Unresolved`](VariableNode::Unresolved) — временная заглушка.
/// - [`Simple`](VariableNode::Simple) — обычная изменяемая переменная.
/// - [`Port`](VariableNode::Port) — порт, отображённый на адрес.
/// - [`Const`](VariableNode::Const) — константа.
///
/// Каждый разрешённый вариант хранит [`Location`] из исходного текста —
/// позицию объявления переменной. Это поле используется при формировании
/// диагностических сообщений, чтобы указывать конкретное место ошибки.
#[derive(Default, Debug, Clone)]
pub enum VariableNode {
    /// Не разрешено (временная заглушка при построении дерева).
    #[default]
    Unresolved,
    /// Изменяемая переменная.
    Simple {
        /// Родительская модель (слабая ссылка для предотвращения циклов Rc).
        upper: Option<Weak<RefCell<ModelNode>>>,
        /// Позиция объявления переменной в исходном тексте.
        loc: Location,
        /// Имя переменной.
        name: String,
        /// Тип переменной.
        ty: TypeNode,
        /// Инициализирующее выражение.
        expr: ExpressionNode,
    },
    /// Порт ввода-вывода, объявляется через `in` (входной) или `out` (выходной).
    Port {
        /// Родительская модель (слабая ссылка для предотвращения циклов Rc).
        upper: Option<Weak<RefCell<ModelNode>>>,
        /// Позиция объявления порта в исходном тексте.
        loc: Location,
        /// Имя переменной.
        name: String,
        /// Тип переменной.
        ty: TypeNode,
        /// Адрес порта (необязателен).
        expr: ExpressionNode,
        /// Направление порта (входной / выходной).
        direction: PortDirection,
    },
    /// Константа.
    Const {
        /// Родительская модель (слабая ссылка для предотвращения циклов Rc).
        upper: Option<Weak<RefCell<ModelNode>>>,
        /// Позиция объявления константы в исходном тексте.
        loc: Location,
        /// Имя константы.
        name: String,
        /// Тип константы.
        ty: TypeNode,
        /// Значение константы.
        expr: ExpressionNode,
    },
}

impl PartialEq for VariableNode {
    fn eq(&self, other: &Self) -> bool {
        match (self, other) {
            (Self::Unresolved, Self::Unresolved) => true,
            (
                Self::Simple {
                    name: n1,
                    ty: t1,
                    expr: e1,
                    ..
                },
                Self::Simple {
                    name: n2,
                    ty: t2,
                    expr: e2,
                    ..
                },
            ) => n1 == n2 && t1 == t2 && e1 == e2,
            (
                Self::Port {
                    name: n1,
                    ty: t1,
                    expr: e1,
                    ..
                },
                Self::Port {
                    name: n2,
                    ty: t2,
                    expr: e2,
                    ..
                },
            ) => n1 == n2 && t1 == t2 && e1 == e2,
            (
                Self::Const {
                    name: n1,
                    ty: t1,
                    expr: e1,
                    ..
                },
                Self::Const {
                    name: n2,
                    ty: t2,
                    expr: e2,
                    ..
                },
            ) => n1 == n2 && t1 == t2 && e1 == e2,
            _ => false,
        }
    }
}

impl Eq for VariableNode {}

impl VariableNode {
    /// Возвращает позицию объявления переменной в исходном тексте.
    ///
    /// Для [`Unresolved`](VariableNode::Unresolved) возвращает [`Location::Implicit`],
    /// так как заглушка не привязана к конкретному месту в коде.
    pub fn loc(&self) -> Location {
        match self {
            VariableNode::Simple { loc, .. }
            | VariableNode::Port { loc, .. }
            | VariableNode::Const { loc, .. } => *loc,
            VariableNode::Unresolved => Location::Implicit,
        }
    }

    /// Возвращает имя переменной (пустая строка для `Unresolved`).
    pub fn name(&self) -> &str {
        match self {
            VariableNode::Simple { name, .. }
            | VariableNode::Port { name, .. }
            | VariableNode::Const { name, .. } => name,
            VariableNode::Unresolved => "",
        }
    }

    /// Возвращает тип переменной (`Inference` для `Unresolved`).
    pub fn ty(&self) -> &TypeNode {
        match self {
            VariableNode::Simple { ty, .. }
            | VariableNode::Port { ty, .. }
            | VariableNode::Const { ty, .. } => ty,
            VariableNode::Unresolved => &TypeNode::Inference,
        }
    }

    /// Возвращает ссылку на родительскую модель.
    pub fn upper(&self) -> Option<Rc<RefCell<ModelNode>>> {
        match self {
            VariableNode::Simple { upper, .. }
            | VariableNode::Port { upper, .. }
            | VariableNode::Const { upper, .. } => upper.as_ref().and_then(|w| w.upgrade()),
            VariableNode::Unresolved => None,
        }
    }
}

// ─── Ce4: Перечисления ────────────────────────────────────────────────────────

/// Семантический узел именованного условия.
///
/// Хранит имя условия и его разрешённое значение.
/// Заполняется в ходе третьего прохода построения модели ([`extract_conditions`]).
///
/// [`extract_conditions`]: crate::semantic::condition::extract_conditions
#[derive(Default, Debug, Clone)]
pub struct ConditionDefinitionNode {
    /// Имя условия, как объявлено в источнике (`cond имя = …`).
    pub name: String,
    /// Позиция объявления условия в исходном тексте.
    pub loc: Location,
    /// Разрешённое значение условия.
    pub value: ConditionNode,
    /// Родительская модель (слабая ссылка для предотвращения циклов Rc).
    pub upper: Option<Weak<RefCell<ModelNode>>>,
}

impl PartialEq for ConditionDefinitionNode {
    fn eq(&self, other: &Self) -> bool {
        // upper и loc игнорируются: не являются частью идентичности узла
        self.name == other.name && self.value == other.value
    }
}

impl Eq for ConditionDefinitionNode {}

impl ConditionDefinitionNode {
    /// Возвращает имя условия.
    pub fn name(&self) -> &str {
        &self.name
    }
}

/// Состояние конечного автомата.
///
/// Три варианта:
/// - [`Unresolved`](StateNode::Unresolved) — заглушка на время первого прохода построения.
/// - [`Simple`](StateNode::Simple) — обычное состояние без реализации.
/// - [`Implement`](StateNode::Implement) — состояние с реализацией (`= Модель`),
///   может иметь оператор `next`.
#[derive(Default, Debug, Clone)]
// Вариант Implement крупнее Simple из-за поля `implements`, которое содержит векторы.
// Боксирование поля `next` допустимо, но требует масштабного рефакторинга — откладываем.
#[allow(clippy::large_enum_variant)]
pub enum StateNode {
    /// Состояние не разрешено (временная заглушка при построении дерева).
    #[default]
    Unresolved,
    /// Обычное состояние: контекст, имя и список ссылок на переходы.
    Simple {
        /// Родительская модель (слабая ссылка для предотвращения циклов Rc).
        upper: Option<Weak<RefCell<ModelNode>>>,
        /// Позиция объявления состояния в исходном тексте.
        loc: Location,
        /// Именованные блоки кода (`enter`, `exit`, `always`, …).
        named_blocks: Vec<NamedCodeBlockDefinitionNode>,
        /// Имя состояния.
        name: String,
        /// Ссылки-переходы (`ref Имя [: Условие]`).
        references: Vec<ReferenceNode<StateNode>>,
        /// Разновидность состояния (обычное, начальное, конечное).
        kind: StateNodeKind,
        /// Встроенные формулы состояния.
        formulas: Vec<Formula>,
    },
    /// Состояние с реализацией (`= Модель`): может иметь `next`-переход.
    Implement {
        /// Родительская модель (слабая ссылка для предотвращения циклов Rc).
        upper: Option<Weak<RefCell<ModelNode>>>,
        /// Позиция объявления состояния в исходном тексте.
        loc: Location,
        /// Именованные блоки кода (`enter`, `exit`, `always`, …).
        named_blocks: Vec<NamedCodeBlockDefinitionNode>,
        /// Имя состояния.
        name: String,
        /// Ссылки-переходы.
        references: Vec<ReferenceNode<StateNode>>,
        /// Информация о реализации (зарезервировано).
        implements: Extend,
        /// Единственный `next`-переход (если задан).
        next: Option<ReferenceNode<StateNode>>,
        /// Разновидность состояния (обычное, начальное, конечное).
        kind: StateNodeKind,
        /// Встроенные формулы состояния.
        formulas: Vec<Formula>,
    },
}

impl StateNode {
    pub(crate) fn kind(&self) -> StateNodeKind {
        match self {
            StateNode::Unresolved => StateNodeKind::Simple,
            StateNode::Simple { kind, .. } | StateNode::Implement { kind, .. } => *kind,
        }
    }

    pub(crate) fn is_terminated(&self) -> bool {
        match self {
            StateNode::Unresolved => false,
            StateNode::Simple { references, .. } => references.is_empty(),
            StateNode::Implement {
                references, next, ..
            } => references.is_empty() && next.is_none(),
        }
    }

    /// Возвращает срез формул, связанных с состоянием.
    pub fn formulas(&self) -> &[Formula] {
        match self {
            StateNode::Simple { formulas, .. } | StateNode::Implement { formulas, .. } => formulas,
            _ => &[],
        }
    }
}

impl PartialEq for StateNode {
    fn eq(&self, other: &Self) -> bool {
        // upper и loc игнорируются: не являются частью семантической идентичности состояния
        match (self, other) {
            (StateNode::Unresolved, StateNode::Unresolved) => true,
            (
                StateNode::Simple {
                    name: n1,
                    named_blocks: nb1,
                    references: r1,
                    kind: k1,
                    formulas: f1,
                    ..
                },
                StateNode::Simple {
                    name: n2,
                    named_blocks: nb2,
                    references: r2,
                    kind: k2,
                    formulas: f2,
                    ..
                },
            ) => n1 == n2 && nb1 == nb2 && r1 == r2 && k1 == k2 && f1 == f2,
            (
                StateNode::Implement {
                    name: n1,
                    named_blocks: nb1,
                    references: r1,
                    implements: i1,
                    next: nx1,
                    kind: k1,
                    formulas: f1,
                    ..
                },
                StateNode::Implement {
                    name: n2,
                    named_blocks: nb2,
                    references: r2,
                    implements: i2,
                    next: nx2,
                    kind: k2,
                    formulas: f2,
                    ..
                },
            ) => {
                n1 == n2 && nb1 == nb2 && r1 == r2 && i1 == i2 && nx1 == nx2 && k1 == k2 && f1 == f2
            }
            _ => false,
        }
    }
}

impl Eq for StateNode {}

/// Разновидность состояния FSM.
#[derive(Default, Debug, PartialEq, Eq, Clone, Copy)]
pub enum StateNodeKind {
    /// Обычное состояние.
    #[default]
    Simple,
    /// Начальное состояние (`start`).
    Start,
    /// Конечное состояние (`end`).
    End,
}

impl StateNode {
    /// Возвращает позицию объявления состояния в исходном тексте.
    ///
    /// Для [`Unresolved`](StateNode::Unresolved) возвращает [`Location::Implicit`].
    pub fn loc(&self) -> Location {
        match self {
            StateNode::Simple { loc, .. } | StateNode::Implement { loc, .. } => *loc,
            StateNode::Unresolved => Location::Implicit,
        }
    }

    /// Возвращает имя состояния.
    pub fn name(&self) -> &str {
        match self {
            StateNode::Unresolved => "",
            StateNode::Simple { name, .. } => name,
            StateNode::Implement { name, .. } => name,
        }
    }

    /// Возвращает ссылку на родительскую модель состояния.
    pub fn upper(&self) -> Option<Rc<RefCell<ModelNode>>> {
        match self {
            StateNode::Simple { upper, .. } | StateNode::Implement { upper, .. } => {
                upper.as_ref().and_then(|w| w.upgrade())
            }
            StateNode::Unresolved => None,
        }
    }

    /// Возвращает список именованных блоков состояния.
    pub fn named_blocks(&self) -> &[NamedCodeBlockDefinitionNode] {
        match self {
            StateNode::Unresolved => &[],
            StateNode::Simple { named_blocks, .. } => named_blocks,
            StateNode::Implement { named_blocks, .. } => named_blocks,
        }
    }

    /// Возвращает список ссылок-переходов состояния (`ref`-рёбер).
    pub fn references(&self) -> &[ReferenceNode<StateNode>] {
        match self {
            StateNode::Unresolved => &[],
            StateNode::Simple { references, .. } => references,
            StateNode::Implement { references, .. } => references,
        }
    }

    /// Ищет именованный блок в состоянии по его имени.
    /// Возвращает список всех именованных блоков с заданным именем.
    pub fn get_named_blocks(&self, name: &str) -> Vec<&NamedCodeBlockDefinitionNode> {
        self.named_blocks()
            .iter()
            .filter(|b| b.name() == name)
            .collect()
    }

    /// Возвращает первый именованный блок с заданным именем, если он есть.
    pub fn get_named_block(&self, name: &str) -> Option<&NamedCodeBlockDefinitionNode> {
        self.named_blocks().iter().find(|b| b.name() == name)
    }
}

/// Условие перехода между состояниями.
///
/// В текущей реализации поддерживается только вариант [`None`](ConditionNode::None),
/// означающий безусловный переход. Полный набор условий — в будущих версиях.
///
/// `PartialEq` реализован вручную: поле `Location` в вариантах `Variable` и
/// `Function` не участвует в сравнении — оно несёт позицию использования
/// (use-site), а не часть семантической идентичности условия.
#[derive(Default, Debug, Clone)]
pub enum ConditionNode {
    /// Безусловный переход (условие не задано или не разрешено).
    #[default]
    None,
    /// Заглушка для условия, которое ещё не было разрешено.
    Unresolved(ast::Condition),
    /// Доступ к элементу массива: `id[индекс]`.
    ArraySubscript(Rc<RefCell<VariableNode>>, Box<ConditionNode>),
    /// Скобки: `(условие)`.
    Parenthesis(Box<ConditionNode>),
    /// Доступ к биту: `условие.член`.
    BitAccess(Box<ConditionNode>, Member),
    /// Вызов функции: `id(аргументы,*)`.
    ///
    /// Третье поле — позиция имени функции в исходном тексте (use-site).
    Function(
        Rc<RefCell<FunctionDefinitionNode>>,
        Vec<Box<ConditionNode>>,
        Location,
    ),
    /// Логическое НЕ: `!условие`.
    Not(Box<ConditionNode>),
    /// Сложение: `левое + правое`.
    Add(Box<ConditionNode>, Box<ConditionNode>),
    /// Вычитание: `левое - правое`.
    Subtract(Box<ConditionNode>, Box<ConditionNode>),
    /// Побитовое И: `левое & правое`.
    And(Box<ConditionNode>, Box<ConditionNode>),
    /// Побитовое ИЛИ: `левое | правое`.
    Or(Box<ConditionNode>, Box<ConditionNode>),
    /// Меньше: `левое < правое`.
    Less(Box<ConditionNode>, Box<ConditionNode>),
    /// Больше: `левое > правое`.
    More(Box<ConditionNode>, Box<ConditionNode>),
    /// Меньше или равно: `левое <= правое`.
    LessEqual(Box<ConditionNode>, Box<ConditionNode>),
    /// Больше или равно: `левое >= правое`.
    MoreEqual(Box<ConditionNode>, Box<ConditionNode>),
    /// Равенство: `левое = правое`.
    Equal(Box<ConditionNode>, Box<ConditionNode>),
    /// Неравенство: `левое != правое`.
    NotEqual(Box<ConditionNode>, Box<ConditionNode>),
    /// Целочисленный литерал.
    Number(i64),
    /// Вещественный литерал: `(строка, отрицательный)`.
    Rational(String, bool),
    /// Конкатенация строковых литералов.
    String(Vec<String>),
    /// Булевый литерал.
    Bool(bool),
    /// Переменная.
    ///
    /// Второе поле — позиция использования переменной в исходном тексте (use-site),
    /// а не позиция объявления. Позволяет индексу LSP найти узел по курсору.
    Variable(Rc<RefCell<VariableNode>>, Location),
    /// Ссылка на модель.
    Model(Rc<RefCell<ModelNode>>),
    /// Ссылка на состояние.
    State(Rc<RefCell<StateNode>>),
    /// Вариант перечисления (Ce4/NI6).
    ///
    /// Поля: `(определение перечисления, имя варианта, числовое значение варианта)`.
    EnumVariant(Rc<RefCell<EnumDefinitionNode>>, String, i64),
}

impl PartialEq for ConditionNode {
    fn eq(&self, other: &Self) -> bool {
        match (self, other) {
            (Self::None, Self::None) => true,
            (Self::Unresolved(a), Self::Unresolved(b)) => a == b,
            (Self::ArraySubscript(v1, n1), Self::ArraySubscript(v2, n2)) => v1 == v2 && n1 == n2,
            (Self::Parenthesis(a), Self::Parenthesis(b)) => a == b,
            (Self::BitAccess(a, ma), Self::BitAccess(b, mb)) => a == b && ma == mb,
            // Location (use-site) намеренно игнорируется: идентичность — семантическая
            (Self::Function(f1, args1, _), Self::Function(f2, args2, _)) => {
                f1 == f2 && args1 == args2
            }
            (Self::Not(a), Self::Not(b)) => a == b,
            (Self::Add(l1, r1), Self::Add(l2, r2)) => l1 == l2 && r1 == r2,
            (Self::Subtract(l1, r1), Self::Subtract(l2, r2)) => l1 == l2 && r1 == r2,
            (Self::And(l1, r1), Self::And(l2, r2)) => l1 == l2 && r1 == r2,
            (Self::Or(l1, r1), Self::Or(l2, r2)) => l1 == l2 && r1 == r2,
            (Self::Less(l1, r1), Self::Less(l2, r2)) => l1 == l2 && r1 == r2,
            (Self::More(l1, r1), Self::More(l2, r2)) => l1 == l2 && r1 == r2,
            (Self::LessEqual(l1, r1), Self::LessEqual(l2, r2)) => l1 == l2 && r1 == r2,
            (Self::MoreEqual(l1, r1), Self::MoreEqual(l2, r2)) => l1 == l2 && r1 == r2,
            (Self::Equal(l1, r1), Self::Equal(l2, r2)) => l1 == l2 && r1 == r2,
            (Self::NotEqual(l1, r1), Self::NotEqual(l2, r2)) => l1 == l2 && r1 == r2,
            (Self::Number(a), Self::Number(b)) => a == b,
            (Self::Rational(s1, n1), Self::Rational(s2, n2)) => s1 == s2 && n1 == n2,
            (Self::String(a), Self::String(b)) => a == b,
            (Self::Bool(a), Self::Bool(b)) => a == b,
            // Location (use-site) намеренно игнорируется
            (Self::Variable(v1, _), Self::Variable(v2, _)) => v1 == v2,
            (Self::Model(a), Self::Model(b)) => a == b,
            (Self::State(a), Self::State(b)) => a == b,
            _ => false,
        }
    }
}

impl Eq for ConditionNode {}

/// Разрешённый семантический узел выражения (заглушка — будет расширено).
#[derive(Default, Debug, PartialEq, Eq, Clone)]
pub enum ExpressionDefinitionNode {
    /// Узел выражения ещё не разрешён (значение по умолчанию).
    #[default]
    None,
}

/// Полностью типизированный семантический узел выражения.
///
/// Большинство вариантов повторяют соответствующие варианты АСД, но работают
/// с уже разрешёнными семантическими подвыражениями. [`Unresolved`](ExpressionNode::Unresolved) —
/// временная обёртка вокруг «сырого» АСД-выражения, ещё не прошедшего семантическое понижение.
#[derive(Default, Debug, PartialEq, Eq, Clone)]
pub enum ExpressionNode {
    /// Выражение отсутствует (значение по умолчанию).
    #[default]
    None,
    /// «Сырое» АСД-выражение, ожидающее семантического понижения.
    Unresolved(ast::Expression),
    /// Доступ к элементу массива: `id[индекс]`.
    ArraySubscript(Rc<RefCell<VariableNode>>, Box<ExpressionNode>),
    /// Срез массива: `id[начало:конец]`.
    ArraySlice(Rc<RefCell<VariableNode>>, Option<i64>, Option<i64>),
    /// Скобки: `(выражение)`.
    Parenthesis(Box<ExpressionNode>),
    /// Доступ к биту: `выражение.член`.
    BitAccess(Box<ExpressionNode>, Member),
    /// Вызов функции: `id(аргументы,*)`.
    Function(Rc<RefCell<FunctionDefinitionNode>>, Vec<ExpressionNode>),
    /// Блок кода как выражение: `выражение { ... }`.
    CodeBlock(Box<ExpressionNode>, StatementNode),
    /// Вызов с именованными аргументами: `выражение({ ключ: значение, … })`.
    NamedFunctionBox(Box<ExpressionNode>, Vec<NamedArgument>),
    /// Логическое НЕ: `!выражение`.
    Not(Box<ExpressionNode>),
    /// Побитовое НЕ: `~выражение`.
    BitwiseNot(Box<ExpressionNode>),
    /// Унарный плюс: `+выражение`.
    UnaryPlus(Box<ExpressionNode>),
    /// Унарный минус: `-выражение`.
    Negate(Box<ExpressionNode>),
    /// Возведение в степень: `левое ** правое`.
    Power(Box<ExpressionNode>, Box<ExpressionNode>),
    /// Умножение: `левое * правое`.
    Multiply(Box<ExpressionNode>, Box<ExpressionNode>),
    /// Деление: `левое / правое`.
    Divide(Box<ExpressionNode>, Box<ExpressionNode>),
    /// Остаток от деления: `левое % правое`.
    Modulo(Box<ExpressionNode>, Box<ExpressionNode>),
    /// Сложение: `левое + правое`.
    Add(Box<ExpressionNode>, Box<ExpressionNode>),
    /// Вычитание: `левое - правое`.
    Subtract(Box<ExpressionNode>, Box<ExpressionNode>),
    /// Сдвиг влево: `левое << правое`.
    ShiftLeft(Box<ExpressionNode>, Box<ExpressionNode>),
    /// Сдвиг вправо: `левое >> правое`.
    ShiftRight(Box<ExpressionNode>, Box<ExpressionNode>),
    /// Побитовое И: `левое & правое`.
    BitwiseAnd(Box<ExpressionNode>, Box<ExpressionNode>),
    /// Побитовое исключающее ИЛИ: `левое ^ правое`.
    BitwiseXor(Box<ExpressionNode>, Box<ExpressionNode>),
    /// Побитовое ИЛИ: `левое | правое`.
    BitwiseOr(Box<ExpressionNode>, Box<ExpressionNode>),
    /// Меньше: `левое < правое`.
    Less(Box<ExpressionNode>, Box<ExpressionNode>),
    /// Больше: `левое > правое`.
    More(Box<ExpressionNode>, Box<ExpressionNode>),
    /// Меньше или равно: `левое <= правое`.
    LessEqual(Box<ExpressionNode>, Box<ExpressionNode>),
    /// Больше или равно: `левое >= правое`.
    MoreEqual(Box<ExpressionNode>, Box<ExpressionNode>),
    /// Равенство: `левое == правое`.
    Equal(Box<ExpressionNode>, Box<ExpressionNode>),
    /// Неравенство: `левое != правое`.
    NotEqual(Box<ExpressionNode>, Box<ExpressionNode>),
    /// Логическое И: `левое && правое`.
    And(Box<ExpressionNode>, Box<ExpressionNode>),
    /// Логическое ИЛИ: `левое || правое`.
    Or(Box<ExpressionNode>, Box<ExpressionNode>),
    /// Тернарный оператор: `условие ? тогда : иначе`.
    ConditionalOperator(
        Box<ExpressionNode>,
        Box<ExpressionNode>,
        Box<ExpressionNode>,
    ),
    /// Присваивание: `левое = правое`.
    Assign(Box<ExpressionNode>, Box<ExpressionNode>),
    /// Целочисленный литерал.
    Number(i64),
    /// Вещественный литерал: `(строка, отрицательный)`.
    Rational(String, bool),
    /// Конкатенация строковых литералов.
    String(Vec<String>),
    /// Тип как выражение.
    Type(Type),
    /// Адресный литерал: `адрес:бит`.
    Address(i64, i64),
    /// Булевый литерал.
    Bool(bool),
    /// Ссылка на разрешённую переменную.
    Variable(Rc<RefCell<VariableNode>>),
    /// Ссылка на разрешённую модель.
    Model(Rc<RefCell<ModelNode>>),
    /// Ссылка на разрешённое именованное условие.
    Condition(Rc<RefCell<ConditionDefinitionNode>>),
    /// Список параметров: `(параметр,*)`.
    List(ParameterList),
    /// Массивный литерал: `[элемент,*]`.
    Array(Vec<ExpressionNode>),
    /// Инициализатор структуры: `{ элемент,* }`.
    Initializer(Vec<ExpressionNode>),
    /// Приведение типа: `выражение as Тип`.
    Cast(Box<ExpressionNode>, TypeNode),
}

/// Ссылка на узел семантического дерева с условием перехода.
///
/// Параметр `T` — тип целевого узла (обычно [`StateNode`]).
#[derive(Default, Debug, PartialEq, Eq, Clone)]
pub struct ReferenceNode<T: Clone + PartialEq + Eq + Debug> {
    /// Позиция ссылки в исходном тексте.
    pub location: Location,
    /// Имя целевого состояния.
    pub name: String,
    /// Условие перехода.
    pub cond: ConditionNode,
    /// Целевой узел (может быть [`StateNode::Unresolved`] до второго прохода).
    pub object: Box<T>,
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::diagnostics::Diagnostic;
    use crate::parse;
    use crate::semantic::tree::construct_model;

    // ─── Тесты: отсутствие циклических сильных ссылок (SA8) ──────────────────

    /// Корневая модель с переменными не создаёт сильных циклов.
    ///
    /// После построения модели счётчик сильных ссылок на корневой Rc должен
    /// быть равен 1 — только наш handle. Если бы `upper` переменных был Rc,
    /// он увеличил бы счётчик до 1 + N (по количеству переменных).
    #[test]
    fn model_with_vars_has_no_strong_cycle() {
        let (ast, _) = parse("var a: bit := false; var b: bit := false; start S;", 0).unwrap();
        let root = construct_model(&ast, None, &[]).unwrap();
        // Единственный сильный владелец — наша переменная `root`
        assert_eq!(
            Rc::strong_count(&root),
            1,
            "корневой Rc должен иметь счётчик 1 (нет циклических Rc-ссылок)"
        );
    }

    /// Вложенная модель не создаёт сильных циклов через upper.
    ///
    /// Модель M имеет `upper` → корень через Weak. Счётчик корня = 1.
    #[test]
    fn nested_model_has_no_strong_cycle() {
        let (ast, _) = parse("model M { start S; } start Main = M;", 0).unwrap();
        let root = construct_model(&ast, None, &[]).unwrap();
        assert_eq!(
            Rc::strong_count(&root),
            1,
            "корень с вложенной моделью должен иметь счётчик 1"
        );
    }

    /// При удалении корневой Rc все Weak-ссылки из переменных становятся недействительными.
    ///
    /// Это доказывает, что циклов нет: дерево освобождается корректно.
    #[test]
    fn upper_weak_invalidated_after_drop() {
        let (ast, _) = parse("var x: bit := false; start S;", 0).unwrap();
        let root = construct_model(&ast, None, &[]).unwrap();
        // Получаем переменную и сохраняем Weak через upper
        let var_x = root
            .borrow()
            .search_var("x")
            .expect("x должна быть найдена");
        let weak_upper = match var_x {
            VariableNode::Simple { ref upper, .. } => upper.clone(),
            _ => panic!("ожидался Simple"),
        };
        // Пока root жив — upgrade() работает
        assert!(
            weak_upper.as_ref().and_then(|w| w.upgrade()).is_some(),
            "upper должен быть жив, пока root существует"
        );
        // Удаляем root — Weak должен стать недействительным
        drop(root);
        assert!(
            weak_upper.as_ref().and_then(|w| w.upgrade()).is_none(),
            "upper должен стать недействительным после drop(root)"
        );
    }

    /// `upper()` метод переменной возвращает Some, если модель жива.
    #[test]
    fn variable_upper_returns_some_while_model_alive() {
        let (ast, _) = parse("var x: bit := false;", 0).unwrap();
        let root = construct_model(&ast, None, &[]).unwrap();
        let var_x = root
            .borrow()
            .search_var("x")
            .expect("x должна быть найдена");
        assert!(
            var_x.upper().is_some(),
            "upper() переменной должен возвращать Some пока модель жива"
        );
    }

    /// Вложенная модель имеет upper() указывающий на родителя.
    #[test]
    fn nested_model_upper_points_to_parent() {
        let (ast, _) = parse("model Inner { start S; } start Main = Inner;", 0).unwrap();
        let root = construct_model(&ast, None, &[]).unwrap();
        let inner = root
            .borrow()
            .search_model("Inner")
            .expect("Inner не найдена");
        let parent = inner.borrow().upper.as_ref().and_then(|w| w.upgrade());
        assert!(
            parent.is_some(),
            "Inner должна иметь upper → родительскую модель"
        );
        // Родитель — анонимная корневая модель (name = None)
        assert_eq!(
            parent.unwrap().borrow().name,
            None,
            "родитель Inner должен быть анонимной корневой моделью"
        );
    }

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

    /// Поиск именованного блока в ModelNode.
    #[test]
    fn model_node_get_named_block() {
        let mut node = ModelNode::default();
        node.named_blocks
            .push(NamedCodeBlockDefinitionNode::Always {
                upper: None,
                body: StatementNode::None,
            });
        assert!(node.get_named_block("always").is_some());
        assert!(node.get_named_block("enter").is_none());
    }

    // ─── StateNode ───────────────────────────────────────────────────────

    /// StateNode::default() равен Unresolved.
    #[test]
    fn state_node_default_is_unresolved() {
        assert_eq!(StateNode::default(), StateNode::Unresolved);
    }

    /// Поиск именованного блока в StateNode.
    #[test]
    fn state_node_get_named_block() {
        let state = StateNode::Simple {
            upper: None,
            name: "S".to_string(),
            named_blocks: vec![NamedCodeBlockDefinitionNode::Enter {
                upper: None,
                body: StatementNode::None,
            }],
            references: vec![],
            kind: StateNodeKind::Simple,
            loc: Default::default(),
            formulas: vec![],
        };
        assert!(state.get_named_block("enter").is_some());
        assert!(state.get_named_block("exit").is_none());
        assert_eq!(state.name(), "S");
    }

    // ─── Reference ──────────────────────────────────────────────────────

    /// Создание Reference<StateNode> с Unresolved-объектом.
    #[test]
    fn reference_unresolved() {
        let r: ReferenceNode<StateNode> = ReferenceNode {
            location: Default::default(),
            name: "X".to_string(),
            cond: ConditionNode::None,
            object: Box::new(StateNode::Unresolved),
        };
        assert_eq!(r.name, "X");
        assert_eq!(r.cond, ConditionNode::None);
        assert_eq!(*r.object, StateNode::Unresolved);
    }

    /// Reference по умолчанию (Default).
    #[test]
    fn reference_default() {
        let r: ReferenceNode<StateNode> = ReferenceNode::default();
        assert!(r.name.is_empty());
    }

    // ─── Condition ──────────────────────────────────────────────────────

    /// Condition::default() равен None.
    #[test]
    fn condition_default_is_none() {
        assert_eq!(ConditionNode::default(), ConditionNode::None);
    }

    /// Заглушки-узлы реализуют Default.
    #[test]
    fn stub_nodes_default() {
        let _ = NamedCodeBlockDefinitionNode::default();
        let _ = FunctionNode::default();
        let _ = VariableNode::default();
        let _ = TypeNode::default();
        let _ = ConditionDefinitionNode::default();
    }

    /// NamedCodeBlock методы name() и statement().
    #[test]
    fn named_code_block_methods() {
        let nb = NamedCodeBlockDefinitionNode::Always {
            upper: None,
            body: StatementNode::Continue,
        };
        assert_eq!(nb.name(), "always");
        assert_eq!(nb.statement(), Some(&StatementNode::Continue));

        let nb_none = NamedCodeBlockDefinitionNode::None;
        assert_eq!(nb_none.name(), "");
        assert_eq!(nb_none.statement(), None);
    }
}
