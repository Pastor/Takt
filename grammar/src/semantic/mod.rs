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

mod builtin;
mod condition;
mod expression;
mod function;
mod include;
mod named_block;
mod statement;
pub mod tree;
mod type_;
mod type_inference;
mod validate;

use crate::parser::ast;
use crate::parser::ast::{Member, NamedArgument, ParameterList, Type};
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
    pub named_blocks: Vec<NamedCodeBlock>,
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
        } else if let Some(model) = self.upper.as_ref() {
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
    /// let (ast, _) = parse("var x: bit = false;", 0).unwrap();
    /// let root = construct_model(&ast, None, &[]).unwrap();
    /// assert!(root.borrow().search_var("x").is_some());
    /// assert!(root.borrow().search_var("y").is_none());
    /// ```
    pub fn search_var(&self, name: &str) -> Option<VariableNode> {
        if let Some(var) = self.variables.get(name) {
            Some(var.clone())
        } else if let Some(model) = self.upper.as_ref() {
            return model.borrow().search_var(name);
        } else {
            None
        }
    }

    /// Ищет именованное условие по `name`, обходя цепочку `upper`.
    pub fn search_cond(&self, name: &str) -> Option<ConditionNode> {
        if let Some(cond) = self.conditions.get(name) {
            Some(cond.clone())
        } else if let Some(model) = self.upper.as_ref() {
            return model.borrow().search_cond(name);
        } else {
            None
        }
    }

    /// Возвращает список всех именованных блоков с заданным именем.
    pub fn get_named_blocks(&self, name: &str) -> Vec<&NamedCodeBlock> {
        self.named_blocks
            .iter()
            .filter(|b| b.name() == name)
            .collect()
    }

    /// Возвращает первый именованный блок с заданным именем, если он есть.
    pub fn get_named_block(&self, name: &str) -> Option<&NamedCodeBlock> {
        self.named_blocks.iter().find(|b| b.name() == name)
    }

    /// Ищет объявление функции по `name`, обходя цепочку `upper`.
    pub fn search_func(&self, name: &str) -> Option<Rc<RefCell<FunctionNode>>> {
        if let Some(func) = self.functions.get(name) {
            Some(Rc::new(RefCell::new(func.clone())))
        } else if let Some(model) = self.upper.as_ref() {
            return model.borrow().search_func(name);
        } else {
            None
        }
    }
}

/// Семантический узел именованного блока кода (`enter`, `exit`, `always`, …).
#[derive(Default, Debug, PartialEq, Eq, Clone)]
pub enum NamedCodeBlock {
    /// Блок не задан.
    #[default]
    None,
    /// Неразрешённый блок кода: `(имя, AST-оператор)`.
    Unresolved(String, ast::Statement),
    /// Блок `enter` с разрешённым оператором.
    Enter(Statement),
    /// Блок `exit` с разрешённым оператором.
    Exit(Statement),
    /// Блок `always` с разрешённым оператором.
    Always(Statement),
    /// Пользовательский именованный блок: `(имя, разрешённый_оператор)`.
    Unknown(String, Statement),
}

impl NamedCodeBlock {
    /// Возвращает имя блока.
    pub fn name(&self) -> &str {
        match self {
            NamedCodeBlock::None => "",
            NamedCodeBlock::Unresolved(name, _) => name,
            NamedCodeBlock::Enter(_) => "enter",
            NamedCodeBlock::Exit(_) => "exit",
            NamedCodeBlock::Always(_) => "always",
            NamedCodeBlock::Unknown(name, _) => name,
        }
    }

    /// Возвращает ссылку на семантический оператор блока, если он разрешён.
    pub fn statement(&self) -> Option<&Statement> {
        match self {
            NamedCodeBlock::Enter(s)
            | NamedCodeBlock::Exit(s)
            | NamedCodeBlock::Always(s)
            | NamedCodeBlock::Unknown(_, s) => Some(s),
            _ => None,
        }
    }
}

/// Семантическое определение функции.
#[derive(Default, Debug, PartialEq, Eq, Clone)]
pub enum FunctionNode {
    /// Определение отсутствует.
    #[default]
    None,
    /// Неразрешённое AST-определение.
    Unresolved(ast::FunctionDefine),
    /// Локальная функция: `(имя, параметры, возвращаемый_тип, тело)`.
    Local(String, Vec<(String, TypeNode)>, TypeNode, Statement),
    /// Внешняя функция (без тела).
    External(String, Vec<(String, TypeNode)>, TypeNode),
    Builtin(&'static str, &'static [(&'static str, TypeNode)], TypeNode),
}

/// Семантический узел вызова или ссылки на функцию.
#[derive(Default, Debug, PartialEq, Eq, Clone)]
pub enum Function {
    /// Функция не задана.
    #[default]
    None,
    /// Неразрешённый вызов: `(имя, аргументы)`.
    Unresolved(String, Vec<Expression>),
    /// Вызов локальной функции.
    Local(Rc<RefCell<FunctionNode>>, Vec<Expression>),
    /// Вызов внешней функции.
    External(Rc<RefCell<FunctionNode>>, Vec<Expression>),
}

/// Семантический оператор языка BuT.
///
/// После семантического анализа (этап 4) все варианты `Unresolved` заменяются
/// конкретными разрешёнными вариантами.
#[derive(Default, Debug, PartialEq, Eq, Clone)]
pub enum Statement {
    /// Оператор отсутствует (значение по умолчанию).
    #[default]
    None,
    /// «Сырой» АСД-оператор, ещё не прошедший семантическое понижение.
    Unresolved(ast::Statement),
    /// Блок операторов: `{ операторы* }`.
    Block(Vec<Statement>),
    /// Оператор-выражение: присваивание, вызов функции и т.п.
    Expression(Box<Expression>),
    /// Условный оператор `if`.
    If {
        /// Условие.
        cond: Box<Expression>,
        /// Тело.
        then_: Box<Statement>,
        /// Ветка `else` (если задана).
        else_: Option<Box<Statement>>,
    },
    /// Цикл `loop [условие] { тело }`.
    /// Условие `None` означает бесконечный цикл.
    Loop {
        /// Условие продолжения (`None` — бесконечный цикл).
        cond: Option<Box<Expression>>,
        /// Тело цикла.
        body: Box<Statement>,
    },
    /// Оператор цикла `for`.
    For {
        /// Инициализация (опционально).
        init: Option<Box<Statement>>,
        /// Условие продолжения (опционально).
        cond: Option<Box<Expression>>,
        /// Выражение шага (опционально).
        step: Option<Box<Expression>>,
        /// Тело цикла.
        body: Box<Statement>,
    },
    /// Объявление локальной переменной: `(имя, тип, инициализатор?)`.
    Variable(String, TypeNode, Option<Box<Expression>>),
    /// Оператор `return [выражение]`.
    Return(Option<Box<Expression>>),
    /// Оператор `continue`.
    Continue,
    /// Оператор `break`.
    Break,
}

/// Семантический узел переменной.
///
/// Варианты:
/// - [`Unresolved`](VariableNode::Unresolved) — временная заглушка.
/// - [`Simple`](VariableNode::Simple) — обычная изменяемая переменная `(имя, тип, инициализатор?)`.
/// - [`Port`](VariableNode::Port) — порт, отображённый на адрес `(имя, тип, адрес)`.
/// - [`Const`](VariableNode::Const) — константа `(имя, тип, значение)`.
#[derive(Default, Debug, PartialEq, Eq, Clone)]
pub enum VariableNode {
    /// Не разрешено (временная заглушка при построении дерева).
    #[default]
    Unresolved,
    /// Изменяемая переменная: `(имя, тип, инициализатор?)`.
    Simple(String, TypeNode, Expression),
    /// Порт, отображённый на аппаратный адрес: `(имя, тип, адрес)`.
    Port(String, TypeNode, Expression),
    /// Константа: `(имя, тип, значение)`.
    Const(String, TypeNode, Expression),
}

/// Семантический узел типа данных.
///
/// Варианты:
/// - [`Detecting`](TypeNode::Inference) — тип выводится (временная заглушка).
/// - [`Address`](TypeNode::Address) — адресный тип порта `(адрес, бит?)`.
/// - [`Bit`](TypeNode::Bit) — 1-битный примитив (`bit`).
/// - [`Bool`](TypeNode::Bool) — булев тип (`bool`).
/// - [`Rational`](TypeNode::Rational) — вещественное число (`float`).
/// - [`Array`](TypeNode::Array) — массив фиксированного размера `(N, элемент)`.
/// - [`Unsupported`](TypeNode::Unsupported) — неподдерживаемый тип (например, функциональный).
#[derive(Default, Debug, PartialEq, Eq, Clone)]
pub enum TypeNode {
    /// Тип ещё не определён (вывод типа в процессе).
    #[default]
    Inference,
    /// Адресный тип порта: `(адрес, номер_бита?)`.
    Address(u64, Option<u64>),
    /// 1-битный примитив (`bit`).
    Bit,
    /// Тип `bool` — булев тип (`true`/`false`).
    Bool,
    /// Тип с плавающей точкой (`float`).
    Rational,
    /// Массив фиксированного размера: `(количество_элементов, тип_элемента)`.
    Array(u16, Box<TypeNode>),
    /// Неподдерживаемый тип (например, функциональный).
    Unsupported,
    /// Пустой тип.
    Unit,
    BuiltinString,
    BuiltinModel,
    BuiltinState,
}

/// Семантический узел именованного условия.
///
/// Хранит имя условия и его разрешённое значение.
/// Заполняется в ходе третьего прохода построения модели ([`extract_conditions`]).
///
/// [`extract_conditions`]: crate::semantic::condition::extract_conditions
#[derive(Default, Debug, PartialEq, Eq, Clone)]
pub struct ConditionNode {
    /// Имя условия, как объявлено в источнике (`cond имя = …`).
    pub name: String,
    /// Разрешённое значение условия.
    pub value: Condition,
}

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
        named_blocks: Vec<NamedCodeBlock>,
        /// Имя состояния.
        name: String,
        /// Ссылки-переходы (`ref Имя [: Условие]`).
        references: Vec<Reference<StateNode>>,
        kind: StateNodeKind,
    },
    /// Состояние с реализацией (`= Модель`): может иметь `next`-переход.
    Implement {
        /// Именованные блоки кода (`enter`, `exit`, `always`, …).
        named_blocks: Vec<NamedCodeBlock>,
        /// Имя состояния.
        name: String,
        /// Ссылки-переходы.
        references: Vec<Reference<StateNode>>,
        /// Информация о реализации (зарезервировано).
        implements: Implement,
        /// Единственный `next`-переход (если задан).
        next: Option<Reference<StateNode>>,
        kind: StateNodeKind,
    },
}

#[derive(Default, Debug, PartialEq, Eq, Clone)]
enum StateNodeKind {
    #[default]
    Simple,
    Start,
    End,
}

impl StateNode {
    /// Возвращает имя состояния.
    pub fn name(&self) -> &str {
        match self {
            StateNode::Unresolved => "",
            StateNode::Simple { name, .. } => name,
            StateNode::Implement { name, .. } => name,
        }
    }

    /// Возвращает список именованных блоков состояния.
    pub fn named_blocks(&self) -> &[NamedCodeBlock] {
        match self {
            StateNode::Unresolved => &[],
            StateNode::Simple { named_blocks, .. } => named_blocks,
            StateNode::Implement { named_blocks, .. } => named_blocks,
        }
    }

    /// Ищет именованный блок в состоянии по его имени.
    /// Возвращает список всех именованных блоков с заданным именем.
    pub fn get_named_blocks(&self, name: &str) -> Vec<&NamedCodeBlock> {
        self.named_blocks()
            .iter()
            .filter(|b| b.name() == name)
            .collect()
    }

    /// Возвращает первый именованный блок с заданным именем, если он есть.
    pub fn get_named_block(&self, name: &str) -> Option<&NamedCodeBlock> {
        self.named_blocks().iter().find(|b| b.name() == name)
    }
}

/// Реализация модели: описывает, как состояние или корневой автомат
/// составлен из именованных моделей.
///
/// - [`Unresolved`](Implement::Unresolved) — временная заглушка до второго прохода.
/// - [`Model`](Implement::Model) — ссылка на конкретную именованную модель.
/// - [`Parentless`](Implement::Parentless) — обёртка без родителя (скобки).
/// - [`Add`](Implement::Add) — последовательная компоновка `A + B`.
/// - [`Or`](Implement::Or) — параллельная компоновка `A | B`.
#[derive(Default, Debug, PartialEq, Eq, Clone)]
pub enum Implement {
    /// Реализация не задана (значение по умолчанию для безымянной корневой модели).
    #[default]
    None,
    /// «Сырое» АСД-выражение реализации, ожидающее разрешения на этапе stage1.
    Unresolved(ast::Expression),
    /// Ссылка на конкретную именованную модель.
    Model(Rc<RefCell<ModelNode>>),
    /// Скобочная группировка: `(реализация)`.
    Parentless(Box<Implement>),
    /// Последовательная компоновка: `левое + правое`.
    Add(Box<Implement>, Box<Implement>),
    /// Параллельная компоновка: `левое | правое`.
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
    /// Заглушка для условия, которое ещё не было разрешено.
    Unresolved(ast::Condition),
    /// Доступ к элементу массива: `id[n]`.
    ArraySubscript(Rc<RefCell<VariableNode>>, i64),
    /// Скобки: `(условие)`.
    Parenthesis(Box<Condition>),
    /// Доступ к биту: `условие.член`.
    BitAccess(Box<Condition>, Member),
    /// Вызов функции: `id(аргументы,*)`.
    Function(Rc<RefCell<FunctionNode>>, Vec<Box<Condition>>),
    /// Логическое НЕ: `!условие`.
    Not(Box<Condition>),
    /// Сложение: `левое + правое`.
    Add(Box<Condition>, Box<Condition>),
    /// Вычитание: `левое - правое`.
    Subtract(Box<Condition>, Box<Condition>),
    /// Побитовое И: `левое & правое`.
    And(Box<Condition>, Box<Condition>),
    /// Побитовое ИЛИ: `левое | правое`.
    Or(Box<Condition>, Box<Condition>),
    /// Меньше: `левое < правое`.
    Less(Box<Condition>, Box<Condition>),
    /// Больше: `левое > правое`.
    More(Box<Condition>, Box<Condition>),
    /// Меньше или равно: `левое <= правое`.
    LessEqual(Box<Condition>, Box<Condition>),
    /// Больше или равно: `левое >= правое`.
    MoreEqual(Box<Condition>, Box<Condition>),
    /// Равенство: `левое = правое`.
    Equal(Box<Condition>, Box<Condition>),
    /// Неравенство: `левое != правое`.
    NotEqual(Box<Condition>, Box<Condition>),
    /// Целочисленный литерал.
    Number(i64),
    /// Вещественный литерал: `(строка, отрицательный)`.
    Rational(String, bool),
    /// Конкатенация строковых литералов.
    String(Vec<String>),
    /// Булевый литерал.
    Bool(bool),
    /// Переменная.
    Variable(Rc<RefCell<VariableNode>>),
}

/// Разрешённый семантический узел выражения (заглушка — будет расширено).
#[derive(Default, Debug, PartialEq, Eq, Clone)]
pub enum ExpressionNode {
    /// Узел выражения ещё не разрешён (значение по умолчанию).
    #[default]
    None,
}

/// Полностью типизированный семантический узел выражения.
///
/// Большинство вариантов повторяют соответствующие варианты АСД, но работают
/// с уже разрешёнными семантическими подвыражениями. [`Unresolved`](Expression::Unresolved) —
/// временная обёртка вокруг «сырого» АСД-выражения, ещё не прошедшего семантическое понижение.
#[derive(Default, Debug, PartialEq, Eq, Clone)]
pub enum Expression {
    /// Выражение отсутствует (значение по умолчанию).
    #[default]
    None,
    /// «Сырое» АСД-выражение, ожидающее семантического понижения.
    Unresolved(ast::Expression),
    /// Доступ к элементу массива: `id[n]`.
    ArraySubscript(Rc<RefCell<VariableNode>>, i64),
    /// Срез массива: `id[начало:конец]`.
    ArraySlice(Rc<RefCell<VariableNode>>, Option<i64>, Option<i64>),
    /// Скобки: `(выражение)`.
    Parenthesis(Box<Expression>),
    /// Доступ к биту: `выражение.член`.
    BitAccess(Box<Expression>, Member),
    /// Вызов функции: `id(аргументы,*)`.
    Function(Rc<RefCell<FunctionNode>>, Vec<Expression>),
    /// Блок кода как выражение: `выражение { ... }`.
    CodeBlock(Box<Expression>, Statement),
    /// Вызов с именованными аргументами: `выражение({ ключ: значение, … })`.
    NamedFunctionBox(Box<Expression>, Vec<NamedArgument>),
    /// Логическое НЕ: `!выражение`.
    Not(Box<Expression>),
    /// Побитовое НЕ: `~выражение`.
    BitwiseNot(Box<Expression>),
    /// Унарный плюс: `+выражение`.
    UnaryPlus(Box<Expression>),
    /// Унарный минус: `-выражение`.
    Negate(Box<Expression>),
    /// Возведение в степень: `левое ** правое`.
    Power(Box<Expression>, Box<Expression>),
    /// Умножение: `левое * правое`.
    Multiply(Box<Expression>, Box<Expression>),
    /// Деление: `левое / правое`.
    Divide(Box<Expression>, Box<Expression>),
    /// Остаток от деления: `левое % правое`.
    Modulo(Box<Expression>, Box<Expression>),
    /// Сложение: `левое + правое`.
    Add(Box<Expression>, Box<Expression>),
    /// Вычитание: `левое - правое`.
    Subtract(Box<Expression>, Box<Expression>),
    /// Сдвиг влево: `левое << правое`.
    ShiftLeft(Box<Expression>, Box<Expression>),
    /// Сдвиг вправо: `левое >> правое`.
    ShiftRight(Box<Expression>, Box<Expression>),
    /// Побитовое И: `левое & правое`.
    BitwiseAnd(Box<Expression>, Box<Expression>),
    /// Побитовое исключающее ИЛИ: `левое ^ правое`.
    BitwiseXor(Box<Expression>, Box<Expression>),
    /// Побитовое ИЛИ: `левое | правое`.
    BitwiseOr(Box<Expression>, Box<Expression>),
    /// Меньше: `левое < правое`.
    Less(Box<Expression>, Box<Expression>),
    /// Больше: `левое > правое`.
    More(Box<Expression>, Box<Expression>),
    /// Меньше или равно: `левое <= правое`.
    LessEqual(Box<Expression>, Box<Expression>),
    /// Больше или равно: `левое >= правое`.
    MoreEqual(Box<Expression>, Box<Expression>),
    /// Равенство: `левое == правое`.
    Equal(Box<Expression>, Box<Expression>),
    /// Неравенство: `левое != правое`.
    NotEqual(Box<Expression>, Box<Expression>),
    /// Логическое И: `левое && правое`.
    And(Box<Expression>, Box<Expression>),
    /// Логическое ИЛИ: `левое || правое`.
    Or(Box<Expression>, Box<Expression>),
    /// Тернарный оператор: `условие ? тогда : иначе`.
    ConditionalOperator(Box<Expression>, Box<Expression>, Box<Expression>),
    /// Присваивание: `левое = правое`.
    Assign(Box<Expression>, Box<Expression>),
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
    Condition(Rc<RefCell<ConditionNode>>),
    /// Список параметров: `(параметр,*)`.
    List(ParameterList),
    /// Массивный литерал: `[элемент,*]`.
    Array(Vec<Expression>),
    /// Инициализатор структуры: `{ элемент,* }`.
    Initializer(Vec<Expression>),
    /// Приведение типа: `выражение as Тип`.
    Cast(Box<Expression>, Type),
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

    /// Поиск именованного блока в ModelNode.
    #[test]
    fn model_node_get_named_block() {
        let mut node = ModelNode::default();
        node.named_blocks
            .push(NamedCodeBlock::Always(Statement::None));
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
            name: "S".to_string(),
            named_blocks: vec![NamedCodeBlock::Enter(Statement::None)],
            references: vec![],
            kind: StateNodeKind::Simple,
        };
        assert!(state.get_named_block("enter").is_some());
        assert!(state.get_named_block("exit").is_none());
        assert_eq!(state.name(), "S");
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
        let _ = NamedCodeBlock::default();
        let _ = Function::default();
        let _ = VariableNode::default();
        let _ = TypeNode::default();
        let _ = ConditionNode::default();
    }

    /// NamedCodeBlock методы name() и statement().
    #[test]
    fn named_code_block_methods() {
        let nb = NamedCodeBlock::Always(Statement::Continue);
        assert_eq!(nb.name(), "always");
        assert_eq!(nb.statement(), Some(&Statement::Continue));

        let nb_none = NamedCodeBlock::None;
        assert_eq!(nb_none.name(), "");
        assert_eq!(nb_none.statement(), None);
    }
}
