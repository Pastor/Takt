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
    /// Доступ к элементу массива: `база[индекс]` (фича 0358).
    ///
    /// ⚠️ База — **выражение**, а не переменная: `b.data[1]` прежде не
    /// разбирался вовсе (`SY-002`), тогда как обратная цепочка `ps[1].x`
    /// работала — постфиксные операции были асимметричны.
    ArraySubscript(Box<ExpressionNode>, Box<ExpressionNode>),
    /// Срез массива: `база[начало:конец]` (фича 0358 — база тоже выражение).
    ArraySlice(Box<ExpressionNode>, Option<i128>, Option<i128>),
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

impl ExpressionNode {
    /// Позиция выражения в исходном тексте.
    ///
    /// Узел позиции **не хранит** (её нет в вариантах), поэтому она берётся у
    /// первого потомка, который её знает: у переменной, функции или вложенного
    /// выражения. Литералы и ссылки на объявления позиции не имеют — для них
    /// ответ [`Location::Builtin`].
    ///
    /// # Почему метод здесь, а не у потребителя
    ///
    /// До фичи 0212 это знание жило **приватной копией в симуляторе**
    /// (`takt-sim/src/expression.rs::loc_of`) и было недоступно целям, из-за
    /// чего отказы генератора `c` не несли координаты вовсе. Вторая копия
    /// разошлась бы с первой при первом же новом варианте узла — класс
    /// 0084/0193/0195.
    ///
    /// ⚠️ У бинарных узлов позиция берётся у **левого** операнда, и только если
    /// он её не знает — у правого: `x + 1` указывает на `x`, а `1 + x` — на `x`
    /// же, потому что литерал позиции не несёт.
    pub fn loc(&self) -> Location {
        match self {
            // У литерала длительности позиции нет — как и у прочих литералов.
            ExpressionNode::Duration(_) => Location::Implicit,
            ExpressionNode::Variable(var) => var.borrow().loc(),
            ExpressionNode::ArraySubscript(base, _) | ExpressionNode::ArraySlice(base, _, _) => {
                base.loc()
            }
            ExpressionNode::Function(func, _) => func.borrow().loc(),
            ExpressionNode::Parenthesis(inner)
            | ExpressionNode::BitAccess(inner, _)
            | ExpressionNode::Not(inner)
            | ExpressionNode::BitwiseNot(inner)
            | ExpressionNode::UnaryPlus(inner)
            | ExpressionNode::Negate(inner)
            | ExpressionNode::Cast(inner, _)
            | ExpressionNode::CodeBlock(inner, _)
            | ExpressionNode::NamedFunctionBox(inner, _) => inner.loc(),
            ExpressionNode::Power(l, r)
            | ExpressionNode::Multiply(l, r)
            | ExpressionNode::Divide(l, r)
            | ExpressionNode::Modulo(l, r)
            | ExpressionNode::Add(l, r)
            | ExpressionNode::Subtract(l, r)
            | ExpressionNode::ShiftLeft(l, r)
            | ExpressionNode::ShiftRight(l, r)
            | ExpressionNode::BitwiseAnd(l, r)
            | ExpressionNode::BitwiseXor(l, r)
            | ExpressionNode::BitwiseOr(l, r)
            | ExpressionNode::Less(l, r)
            | ExpressionNode::More(l, r)
            | ExpressionNode::LessEqual(l, r)
            | ExpressionNode::MoreEqual(l, r)
            | ExpressionNode::Equal(l, r)
            | ExpressionNode::NotEqual(l, r)
            | ExpressionNode::And(l, r)
            | ExpressionNode::Or(l, r)
            | ExpressionNode::Assign(l, r) => match l.loc() {
                Location::Builtin => r.loc(),
                found => found,
            },
            ExpressionNode::ConditionalOperator(c, t, e) => match c.loc() {
                Location::Builtin => match t.loc() {
                    Location::Builtin => e.loc(),
                    found => found,
                },
                found => found,
            },
            ExpressionNode::Array(items) | ExpressionNode::Initializer(items) => items
                .iter()
                .map(ExpressionNode::loc)
                .find(|l| !matches!(l, Location::Builtin))
                .unwrap_or(Location::Builtin),
            // ⚠️ `Unresolved` несёт АСД, и позиция у него ЕСТЬ — но берётся она
            // не здесь: печатники зовут `raw.loc()` напрямую (фича 0236), чтобы
            // отказ указывал на сам неразрешённый узел.
            ExpressionNode::None
            | ExpressionNode::Unresolved(_)
            | ExpressionNode::Number(_)
            | ExpressionNode::Rational(_, _)
            | ExpressionNode::String(_)
            | ExpressionNode::Type(_)
            | ExpressionNode::Address(_, _)
            | ExpressionNode::AnonPort(_)
            | ExpressionNode::Bool(_)
            | ExpressionNode::Model(_)
            | ExpressionNode::Condition(_)
            | ExpressionNode::List(_) => Location::Builtin,
        }
    }
}
