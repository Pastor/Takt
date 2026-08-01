//! Семантический узел выражения ([`ExpressionNode`]).
//!
//! Вынесен из `semantic/mod.rs` **чистым перемещением** (фича 0189) по той же
//! причине и тем же приёмом, что [`ConditionNode`](crate::semantic::ConditionNode)
//! фичей 0134-02: файл пришпилен реестром размеров и расти не имеет права, а
//! узел — самостоятельное знание. Путь `semantic::ExpressionNode` сохранён
//! реэкспортом, поэтому потребители (генераторы, симулятор, LSP) не тронуты
//! (правило 11).

use super::*;

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
    ArraySlice(Rc<RefCell<VariableNode>>, Option<i128>, Option<i128>),
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
    /// Целочисленный литерал (носитель — `i128`, фича 0157).
    Number(i128),
    /// Литерал длительности в наносекундах (фича 0134).
    Duration(i64),
    /// Вещественный литерал: `(строка, отрицательный)`.
    Rational(String, bool),
    /// Конкатенация строковых литералов.
    String(Vec<String>),
    /// Тип как выражение.
    Type(Type),
    /// Адресный литерал: `адрес:бит`.
    Address(i64, i64),
    /// Анонимное обращение к ячейке по адресу: `#0x346619:0 as u64` (фича 0189).
    ///
    /// Свёрнутая тройка `{адрес, бит, тип}` — та же, которой оперируют
    /// дефолтный HAL цели `c-hal` и регистровый файл цели `sv-mmio`. Формы
    /// записи (`as` и `.N`) за этой границей не существует: свёртку делает
    /// [`anon_port`](crate::semantic::anon_port) в единой воронке.
    AnonPort(AnonPortAccess),
    /// Булевый литерал.
    Bool(bool),
    /// Ссылка на разрешённую переменную.
    Variable(Rc<RefCell<VariableNode>>),
    /// Ссылка на разрешённую модель.
    ///
    /// ⚠️ Позиции использования здесь **нет намеренно** (фича 0056): она нужна
    /// только реализации состояния (`= Helper`), а та разворачивается из АСД
    /// прямо в [`Extend::Model`](crate::semantic::extend::Extend::Model) —
    /// см. `extend::unroll_ast_extend`. Тащить позицию через ~40 вариантов
    /// `ExpressionNode`, где её никто не читает, значило бы ещё и втянуть её в
    /// автовыведённое равенство узла.
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
