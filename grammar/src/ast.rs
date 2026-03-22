#[cfg(feature = "ast-serde")]
use serde::{Deserialize, Serialize};

/// Местоположение узла АСД в исходном тексте.
///
/// Вариант [`Source`](Location::Source) хранит номер файла, байтовое смещение
/// начала и байтовое смещение конца (не включительно).
#[derive(Clone, Debug, PartialEq, Eq, PartialOrd, Ord, Hash)]
#[cfg_attr(feature = "ast-serde", derive(Serialize, Deserialize))]
pub enum Location {
    /// Встроенный элемент (не относится к пользовательскому коду).
    Builtin,
    /// Элемент, заданный через командную строку компилятора.
    CommandLine,
    /// Неявно сгенерированный элемент.
    Implicit,
    /// Элемент, сгенерированный кодогенератором.
    Codegen,
    /// Элемент в исходном файле: `(файл, начало, конец)`.
    Source(u64, usize, usize),
}

impl Default for Location {
    fn default() -> Self {
        Self::Source(0, 0, 0)
    }
}

/// Вызывается при попытке получить позицию из не-файлового варианта [`Location`].
#[inline(never)]
#[cold]
#[track_caller]
fn not_a_file() -> ! {
    panic!("location is not a file")
}

impl Location {
    /// Возвращает [`Location`] нулевой длины, указывающий на начало данного диапазона.
    #[inline]
    pub fn begin_range(&self) -> Self {
        match self {
            Location::Source(filename, start, _) => {
                Location::Source(filename.clone(), *start, *start)
            }
            loc => loc.clone(),
        }
    }

    /// Возвращает [`Location`] нулевой длины, указывающий на конец данного диапазона.
    #[inline]
    pub fn end_range(&self) -> Self {
        match self {
            Location::Source(filename, _, end) => Location::Source(filename.clone(), *end, *end),
            loc => loc.clone(),
        }
    }

    /// Возвращает строковое представление номера файла.
    ///
    /// # Паника
    ///
    /// Паникует, если `self` не является вариантом [`Location::Source`].
    #[track_caller]
    #[inline]
    pub fn filename(&self) -> String {
        match self {
            Location::Source(file_no, _, _) => format!("{}", file_no),
            _ => not_a_file(),
        }
    }

    /// Возвращает `Some(номер_файла)` для варианта [`Source`](Location::Source),
    /// или `None` для остальных вариантов.
    #[inline]
    pub fn try_file_no(&self) -> Option<String> {
        match self {
            Location::Source(file_no, _, _) => Some(format!("{}", file_no)),
            _ => None,
        }
    }

    /// Возвращает байтовое смещение начала диапазона.
    ///
    /// # Паника
    ///
    /// Паникует, если `self` не является вариантом [`Location::Source`].
    #[track_caller]
    #[inline]
    pub fn start(&self) -> usize {
        match self {
            Location::Source(_, start, _) => *start,
            _ => not_a_file(),
        }
    }

    /// Возвращает байтовое смещение конца диапазона (включительно).
    ///
    /// # Паника
    ///
    /// Паникует, если `self` не является вариантом [`Location::Source`].
    #[track_caller]
    #[inline]
    pub fn end(&self) -> usize {
        match self {
            Location::Source(_, _, end) => *end,
            _ => not_a_file(),
        }
    }

    /// Возвращает байтовое смещение конца диапазона (не включительно, `end + 1`).
    ///
    /// # Паника
    ///
    /// Паникует, если `self` не является вариантом [`Location::Source`].
    #[track_caller]
    #[inline]
    pub fn exclusive_end(&self) -> usize {
        self.end() + 1
    }

    /// Устанавливает начало текущего диапазона равным началу `other`.
    ///
    /// # Паника
    ///
    /// Паникует, если любой из операндов не является [`Location::Source`].
    #[track_caller]
    #[inline]
    pub fn use_start_from(&mut self, other: &Location) {
        match (self, other) {
            (Location::Source(_, start, _), Location::Source(_, other_start, _)) => {
                *start = *other_start;
            }
            _ => not_a_file(),
        }
    }

    /// Устанавливает конец текущего диапазона равным концу `other`.
    ///
    /// # Паника
    ///
    /// Паникует, если любой из операндов не является [`Location::Source`].
    #[track_caller]
    #[inline]
    pub fn use_end_from(&mut self, other: &Location) {
        match (self, other) {
            (Location::Source(_, _, end), Location::Source(_, _, other_end)) => {
                *end = *other_end;
            }
            _ => not_a_file(),
        }
    }

    /// Возвращает копию с началом, взятым из `other`.
    ///
    /// # Паника
    ///
    /// Паникует, если любой из операндов не является [`Location::Source`].
    #[track_caller]
    #[inline]
    pub fn with_start_from(mut self, other: &Self) -> Self {
        self.use_start_from(other);
        self
    }

    /// Возвращает копию с концом, взятым из `other`.
    ///
    /// # Паника
    ///
    /// Паникует, если любой из операндов не является [`Location::Source`].
    #[track_caller]
    #[inline]
    pub fn with_end_from(mut self, other: &Self) -> Self {
        self.use_end_from(other);
        self
    }

    /// Возвращает копию с заменённым началом.
    ///
    /// # Паника
    ///
    /// Паникует, если `self` не является [`Location::Source`].
    #[track_caller]
    #[inline]
    pub fn with_start(self, start: usize) -> Self {
        match self {
            Self::Source(no, _, end) => Self::Source(no, start, end),
            _ => not_a_file(),
        }
    }

    /// Возвращает копию с заменённым концом.
    ///
    /// # Паника
    ///
    /// Паникует, если `self` не является [`Location::Source`].
    #[track_caller]
    #[inline]
    pub fn with_end(self, end: usize) -> Self {
        match self {
            Self::Source(no, start, _) => Self::Source(no, start, end),
            _ => not_a_file(),
        }
    }

    /// Преобразует [`Location`] в стандартный диапазон `start..end`.
    ///
    /// # Паника
    ///
    /// Паникует, если `self` не является [`Location::Source`].
    #[track_caller]
    #[inline]
    pub fn range(self) -> std::ops::Range<usize> {
        match self {
            Self::Source(_, start, end) => start..end,
            _ => not_a_file(),
        }
    }
}

/// Идентификатор с местоположением в исходном тексте.
#[derive(Debug, PartialEq, Eq, Clone)]
#[cfg_attr(feature = "ast-serde", derive(Serialize, Deserialize))]
pub struct Identifier {
    /// Местоположение идентификатора.
    pub loc: Location,
    /// Строковое имя идентификатора.
    pub name: String,
}

impl Identifier {
    /// Создаёт идентификатор с местоположением по умолчанию.
    pub fn new(s: impl Into<String>) -> Self {
        Self {
            loc: Location::default(),
            name: s.into(),
        }
    }
}

/// Путь из идентификаторов, разделённых `::` (например, `a::b::c`).
#[derive(Debug, PartialEq, Eq, Clone)]
#[cfg_attr(feature = "ast-serde", derive(Serialize, Deserialize))]
pub struct IdentifierPath {
    /// Местоположение всего пути.
    pub loc: Location,
    /// Последовательность идентификаторов.
    pub identifiers: Vec<Identifier>,
}

/// Комментарий в исходном тексте.
#[derive(Debug, PartialEq, Eq, Clone)]
#[cfg_attr(feature = "ast-serde", derive(Serialize, Deserialize))]
pub enum Comment {
    /// Обычный строчный комментарий `// ...`.
    Line(Location, String),
    /// Документационный строчный комментарий `/// ...`.
    DocLine(Location, String),
}

impl Comment {
    /// Возвращает текстовое содержимое комментария.
    #[inline]
    pub const fn value(&self) -> &String {
        match self {
            Self::Line(_, s) | Self::DocLine(_, s) => s,
        }
    }

    /// Возвращает `true`, если комментарий является документационным (`///`).
    #[inline]
    pub const fn is_doc(&self) -> bool {
        matches!(self, Self::DocLine(..))
    }

    /// Возвращает `true`, если комментарий является строчным (оба варианта).
    #[inline]
    pub const fn is_line(&self) -> bool {
        matches!(self, Self::Line(..) | Self::DocLine(..))
    }
}

/// Описание импорта.
#[derive(Debug, PartialEq, Eq, Clone)]
#[cfg_attr(feature = "ast-serde", derive(Serialize, Deserialize))]
pub enum ImportDefine {
    /// `import "путь";`
    Plain(ImportPath, Location),
    /// `import "путь" as Имя;` или `import * as Имя from "путь";`
    GlobalSymbol(ImportPath, Identifier, Location),
    /// `import { a, b as c } from "путь";`
    Rename(ImportPath, Vec<(Identifier, Option<Identifier>)>, Location),
}

/// Путь импорта.
#[derive(Debug, PartialEq, Eq, Clone)]
#[cfg_attr(feature = "ast-serde", derive(Serialize, Deserialize))]
pub enum ImportPath {
    /// Путь в виде строкового литерала.
    Filename(StringLiteral),
    /// Путь в виде идентификаторного пути.
    Path(IdentifierPath),
}

impl ImportDefine {
    /// Возвращает строковый литерал пути, если он есть.
    #[inline]
    pub const fn literal(&self) -> Option<&StringLiteral> {
        match self {
            Self::Plain(ImportPath::Filename(literal), _)
            | Self::GlobalSymbol(ImportPath::Filename(literal), _, _)
            | Self::Rename(ImportPath::Filename(literal), _, _) => Some(literal),
            _ => None,
        }
    }
}

/// Список параметров функции: `Vec<(позиция, параметр?)>`.
///
/// `None` используется при ошибке восстановления парсера.
pub type ParameterList = Vec<(Location, Option<Parameter>)>;

/// Тип данных в языке BuT.
#[derive(Debug, PartialEq, Eq, Clone)]
#[cfg_attr(feature = "ast-serde", derive(Serialize, Deserialize))]
pub enum Type {
    /// Адресный тип `порт[:бит]`.
    Address {
        /// Адрес порта.
        address: u64,
        /// Опциональный номер бита.
        bit: Option<u64>,
    },
    /// Примитивный тип `bit` (1 бит).
    Bit,
    /// Булевый тип `bool`.
    Bool,
    /// Тип с плавающей точкой `float`.
    Float,
    /// Псевдоним типа (ссылка по имени).
    Alias(Identifier),
    /// Массив битов: `[тип; N]`.
    Array {
        /// Местоположение в исходном тексте.
        loc: Location,
        /// Количество элементов.
        element_count: u16,
        /// Тип элемента.
        element_type: Box<Type>,
    },
    /// Функциональный тип `(параметры) -> возврат`.
    Function {
        /// Список входных параметров.
        params: ParameterList,
        /// Список возвращаемых значений.
        returns: Option<ParameterList>,
    },
}

/// Описание переменной (`var` или `const`).
#[derive(Debug, PartialEq, Eq, Clone)]
#[cfg_attr(feature = "ast-serde", derive(Serialize, Deserialize))]
pub struct VariableDefine {
    /// Местоположение в исходном тексте.
    pub loc: Location,
    /// Тип переменной (может быть не указан).
    pub ty: Option<Type>,
    /// Имя переменной (может быть не указано при ошибке).
    pub name: Option<Identifier>,
    /// Начальное значение (инициализатор).
    pub initializer: Option<Expression>,
    /// `true` — изменяемая (`var`), `false` — константа (`const`).
    pub mutability: bool,
}

/// Описание порта (`port`).
#[derive(Debug, PartialEq, Eq, Clone)]
#[cfg_attr(feature = "ast-serde", derive(Serialize, Deserialize))]
pub struct PortDefine {
    /// Местоположение в исходном тексте.
    pub loc: Location,
    /// Тип порта.
    pub ty: Type,
    /// Имя порта (может быть не указано при ошибке).
    pub name: Option<Identifier>,
    /// Адрес или выражение инициализации.
    pub initializer: Option<Expression>,
}

/// Пользовательский оператор, который может быть перегружен.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
#[cfg_attr(feature = "ast-serde", derive(Serialize, Deserialize))]
pub enum UserDefinedOperator {
    /// `&`
    BitwiseAnd,
    /// `~`
    BitwiseNot,
    /// Унарный минус `-` (псевдоним [`Subtract`](UserDefinedOperator::Subtract),
    /// в настоящее время не парсится).
    Negate,
    /// `|`
    BitwiseOr,
    /// `^`
    BitwiseXor,
    /// `+`
    Add,
    /// `/`
    Divide,
    /// `%`
    Modulo,
    /// `*`
    Multiply,
    /// `-`
    Subtract,
    /// `==`
    Equal,
    /// `>`
    More,
    /// `>=`
    MoreEqual,
    /// `<`
    Less,
    /// `<=`
    LessEqual,
    /// `!=`
    NotEqual,
}

impl UserDefinedOperator {
    /// Возвращает количество аргументов оператора.
    ///
    /// Унарные операторы возвращают `1`, бинарные — `2`.
    #[inline]
    pub const fn args(&self) -> usize {
        match self {
            UserDefinedOperator::BitwiseNot | UserDefinedOperator::Negate => 1,
            _ => 2,
        }
    }

    /// Возвращает `true`, если оператор унарный.
    #[inline]
    pub const fn is_unary(&self) -> bool {
        matches!(self, Self::BitwiseNot | Self::Negate)
    }

    /// Возвращает `true`, если оператор бинарный.
    #[inline]
    pub const fn is_binary(&self) -> bool {
        !self.is_unary()
    }

    /// Возвращает `true`, если оператор является побитовым.
    #[inline]
    pub const fn is_bitwise(&self) -> bool {
        matches!(
            self,
            Self::BitwiseAnd | Self::BitwiseOr | Self::BitwiseXor | Self::BitwiseNot
        )
    }

    /// Возвращает `true`, если оператор является арифметическим.
    #[inline]
    pub const fn is_arithmetic(&self) -> bool {
        matches!(
            self,
            Self::Add | Self::Subtract | Self::Multiply | Self::Divide | Self::Modulo
        )
    }

    /// Возвращает `true`, если оператор является оператором сравнения.
    #[inline]
    pub const fn is_comparison(&self) -> bool {
        matches!(
            self,
            Self::Equal
                | Self::NotEqual
                | Self::Less
                | Self::LessEqual
                | Self::More
                | Self::MoreEqual
        )
    }
}

/// Корневой узел АСД — модель (автомат или компоновка автоматов).
///
/// При разборе всего файла без явного `model` блока корень является
/// анонимной моделью (`name == None`), содержащей все элементы верхнего уровня.
#[derive(Debug, PartialEq, Eq, Clone)]
#[cfg_attr(feature = "ast-serde", derive(Serialize, Deserialize))]
pub struct Model {
    /// Местоположение в исходном тексте.
    pub loc: Location,
    /// Имя модели (`None` для анонимной корневой модели).
    pub name: Option<Identifier>,
    /// Элементы модели.
    pub elements: Vec<ModelElement>,
    /// Выражение реализации/компоновки (`= выражение`).
    pub implements: Option<Expression>,
}

/// Элемент модели верхнего уровня.
#[derive(Debug, PartialEq, Eq, Clone)]
#[cfg_attr(feature = "ast-serde", derive(Serialize, Deserialize))]
pub enum ModelElement {
    /// Директива импорта.
    Import(ImportDefine),
    /// Определение функции.
    Function(Box<FunctionDefine>),
    /// Формула.
    Formula(Box<FormulaDefine>),
    /// Определение условия.
    Condition(Box<ConditionDefine>),
    /// Определение переменной.
    Variable(Box<VariableDefine>),
    /// Определение порта.
    Port(Box<PortDefine>),
    /// Определение псевдонима типа.
    Type(Box<TypeDefine>),
    /// Определение состояния.
    State(Box<StateDefine>),
    /// Вложенная модель.
    Model(Box<Model>),
    /// Именованный блок кода (`enter`, `exit`, `always`, …).
    NamedBlockCode(Box<NamedBlockCodeDefine>),
    /// Одиночная точка с запятой (допускается для совместимости).
    StraySemicolon(Location),
}

/// Вид состояния автомата.
#[derive(Debug, PartialEq, Eq, Clone)]
#[cfg_attr(feature = "ast-serde", derive(Serialize, Deserialize))]
pub enum StateKind {
    /// Начальное состояние (`start`).
    Start,
    /// Конечное состояние.
    End,
    /// Переходное состояние.
    Next,
}

/// Определение состояния (`state` или `start`).
#[derive(Debug, PartialEq, Eq, Clone)]
#[cfg_attr(feature = "ast-serde", derive(Serialize, Deserialize))]
pub struct StateDefine {
    /// Местоположение в исходном тексте.
    pub loc: Location,
    /// Имя состояния.
    pub name: Option<Identifier>,
    /// Элементы состояния.
    pub elements: Vec<StateElement>,
    /// Выражение реализации.
    pub implements: Option<Expression>,
    /// Вид состояния.
    pub kind: Option<StateKind>,
}

/// Элемент состояния.
#[derive(Debug, PartialEq, Eq, Clone)]
#[cfg_attr(feature = "ast-serde", derive(Serialize, Deserialize))]
pub enum StateElement {
    /// Директива импорта.
    Import(ImportDefine),
    /// Переход к следующему состоянию (`next Имя`).
    Next(Identifier),
    /// Определение функции.
    Function(Box<FunctionDefine>),
    /// Формула.
    Formula(Box<FormulaDefine>),
    /// Определение условия.
    Condition(Box<ConditionDefine>),
    /// Определение переменной.
    Variable(Box<VariableDefine>),
    /// Определение псевдонима типа.
    Type(Box<TypeDefine>),
    /// Ссылка на состояние с условием перехода: `ref Имя [: Условие]`.
    Reference(Location, Identifier, Option<Condition>),
    /// Вложенная модель.
    Model(Box<Model>),
    /// Именованный блок кода.
    NamedBlockCode(Box<NamedBlockCodeDefine>),
    /// Одиночная точка с запятой.
    StraySemicolon(Location),
}

/// База наследования: `Имя[(аргументы,*)]`.
#[derive(Debug, PartialEq, Eq, Clone)]
#[cfg_attr(feature = "ast-serde", derive(Serialize, Deserialize))]
pub struct Base {
    /// Местоположение в исходном тексте.
    pub loc: Location,
    /// Путь идентификатора.
    pub name: IdentifierPath,
    /// Аргументы (опционально).
    pub args: Option<Vec<Expression>>,
}

/// Переменная с типом и инициализатором.
#[derive(Debug, PartialEq, Eq, Clone)]
#[cfg_attr(feature = "ast-serde", derive(Serialize, Deserialize))]
pub struct Variable {
    /// Местоположение в исходном тексте.
    pub loc: Location,
    /// Тип переменной.
    pub ty: Box<TypeDefine>,
    /// Имя переменной.
    pub name: Option<Identifier>,
    /// Начальное значение.
    pub initializer: Option<Expression>,
}

/// Определение псевдонима типа: `type Имя = Тип`.
#[derive(Debug, PartialEq, Eq, Clone)]
#[cfg_attr(feature = "ast-serde", derive(Serialize, Deserialize))]
pub struct TypeDefine {
    /// Местоположение в исходном тексте.
    pub loc: Location,
    /// Имя псевдонима.
    pub name: Identifier,
    /// Исходный тип.
    pub ty: Type,
}

/// Аннотация (метаданные, прикреплённые к объявлению).
#[derive(Debug, PartialEq, Eq, Clone)]
#[cfg_attr(feature = "ast-serde", derive(Serialize, Deserialize))]
pub enum Annotation {
    /// Идентификаторный путь.
    Identifier(Location, IdentifierPath),
    /// Вызов функции-аннотации.
    Function {
        /// Местоположение.
        loc: Location,
        /// Имя функции.
        name: Identifier,
        /// Аргументы.
        args: Vec<Annotation>,
    },
    /// Присваивание: `ключ = значение`.
    Assign {
        /// Местоположение.
        loc: Location,
        /// Левая часть.
        name: IdentifierPath,
        /// Правая часть.
        value: Expression,
    },
    /// Строковый аргумент.
    String(StringLiteral),
    /// Числовой аргумент.
    Number(Location, i64),
    /// Рациональный аргумент.
    Rational(Location, String, bool),
    /// Булевый аргумент.
    Boolean(Location, bool),
    /// Аргумент видимости.
    Visibility(Location, Expression),
}

/// Строковый литерал с признаком Unicode.
#[derive(Debug, PartialEq, Eq, Clone)]
#[cfg_attr(feature = "ast-serde", derive(Serialize, Deserialize))]
pub struct StringLiteral {
    /// Местоположение в исходном тексте.
    pub loc: Location,
    /// `true` — строка объявлена с префиксом `unicode`.
    pub unicode: bool,
    /// Содержимое строки (без кавычек).
    pub string: String,
}

/// Именованный аргумент вызова функции: `имя: выражение`.
#[derive(Debug, PartialEq, Eq, Clone)]
#[cfg_attr(feature = "ast-serde", derive(Serialize, Deserialize))]
pub struct NamedArgument {
    /// Местоположение в исходном тексте.
    pub loc: Location,
    /// Имя аргумента (может отсутствовать при ошибке).
    pub name: Option<Identifier>,
    /// Значение аргумента.
    pub expr: Expression,
}

/// Условие перехода между состояниями.
///
/// Является упрощённым подмножеством [`Expression`], допускаемым
/// в позиции условия перехода `ref Имя: Условие`.
#[derive(Debug, PartialEq, Eq, Clone)]
#[cfg_attr(feature = "ast-serde", derive(Serialize, Deserialize))]
pub enum Condition {
    /// Доступ к элементу массива: `id[n]`.
    ArraySubscript(Location, Identifier, i64),
    /// Скобки: `(условие)`.
    Parenthesis(Location, Box<Condition>),
    /// Доступ к биту: `условие.член`.
    BitAccess(Location, Box<Condition>, Member),
    /// Вызов функции: `id(аргументы,*)`.
    Function(Location, Identifier, Vec<Condition>),
    /// Логическое НЕ: `!условие`.
    Not(Location, Box<Condition>),
    /// Сложение: `левое + правое`.
    Add(Location, Box<Condition>, Box<Condition>),
    /// Вычитание: `левое - правое`.
    Subtract(Location, Box<Condition>, Box<Condition>),
    /// Побитовое И: `левое & правое`.
    And(Location, Box<Condition>, Box<Condition>),
    /// Побитовое ИЛИ: `левое | правое`.
    Or(Location, Box<Condition>, Box<Condition>),
    /// Меньше: `левое < правое`.
    Less(Location, Box<Condition>, Box<Condition>),
    /// Больше: `левое > правое`.
    More(Location, Box<Condition>, Box<Condition>),
    /// Меньше или равно: `левое <= правое`.
    LessEqual(Location, Box<Condition>, Box<Condition>),
    /// Больше или равно: `левое >= правое`.
    MoreEqual(Location, Box<Condition>, Box<Condition>),
    /// Равенство: `левое = правое`.
    Equal(Location, Box<Condition>, Box<Condition>),
    /// Неравенство: `левое != правое`.
    NotEqual(Location, Box<Condition>, Box<Condition>),
    /// Целочисленный литерал.
    Number(Location, i64),
    /// Вещественный литерал: `(строка, отрицательный)`.
    Float(Location, String, bool),
    /// Конкатенация строковых литералов.
    String(Vec<StringLiteral>),
    /// Булевый литерал.
    Bool(Location, bool),
    /// Переменная.
    Variable(Identifier),
}

/// Элемент доступа к члену (для оператора `.`).
#[derive(Debug, PartialEq, Eq, Clone)]
#[cfg_attr(feature = "ast-serde", derive(Serialize, Deserialize))]
pub enum Member {
    /// Доступ по имени: `.имя`.
    Identifier(Identifier),
    /// Доступ по индексу: `.0`, `.1`, …
    Number(i64),
}

/// Выражение языка BuT.
///
/// Поддерживает полный спектр операций: арифметику, побитовые операции,
/// сравнения, логику, обращение к массивам, вызовы функций и т.д.
#[derive(Debug, PartialEq, Eq, Clone)]
#[cfg_attr(feature = "ast-serde", derive(Serialize, Deserialize))]
pub enum Expression {
    /// Доступ к элементу массива: `id[n]`.
    ArraySubscript(Location, Identifier, i64),
    /// Срез массива: `id[начало:конец]`.
    ArraySlice(Location, Identifier, Option<i64>, Option<i64>),
    /// Скобки: `(выражение)`.
    Parenthesis(Location, Box<Expression>),
    /// Доступ к биту: `выражение.член`.
    BitAccess(Location, Box<Expression>, Member),
    /// Вызов функции: `id(аргументы,*)`.
    Function(Location, Identifier, Vec<Expression>),
    /// Блок кода как выражение: `выражение { ... }`.
    CodeBlock(Location, Box<Expression>, Box<Statement>),
    /// Вызов с именованными аргументами: `выражение({ ключ: значение, … })`.
    NamedFunction(Location, Box<Expression>, Vec<NamedArgument>),
    /// Логическое НЕ: `!выражение`.
    Not(Location, Box<Expression>),
    /// Побитовое НЕ: `~выражение`.
    BitwiseNot(Location, Box<Expression>),
    /// Унарный плюс: `+выражение`.
    UnaryPlus(Location, Box<Expression>),
    /// Унарный минус: `-выражение`.
    Negate(Location, Box<Expression>),

    /// Возведение в степень: `левое ** правое`.
    Power(Location, Box<Expression>, Box<Expression>),
    /// Умножение: `левое * правое`.
    Multiply(Location, Box<Expression>, Box<Expression>),
    /// Деление: `левое / правое`.
    Divide(Location, Box<Expression>, Box<Expression>),
    /// Остаток от деления: `левое % правое`.
    Modulo(Location, Box<Expression>, Box<Expression>),
    /// Сложение: `левое + правое`.
    Add(Location, Box<Expression>, Box<Expression>),
    /// Вычитание: `левое - правое`.
    Subtract(Location, Box<Expression>, Box<Expression>),
    /// Сдвиг влево: `левое << правое`.
    ShiftLeft(Location, Box<Expression>, Box<Expression>),
    /// Сдвиг вправо: `левое >> правое`.
    ShiftRight(Location, Box<Expression>, Box<Expression>),
    /// Побитовое И: `левое & правое`.
    BitwiseAnd(Location, Box<Expression>, Box<Expression>),
    /// Побитовое исключающее ИЛИ: `левое ^ правое`.
    BitwiseXor(Location, Box<Expression>, Box<Expression>),
    /// Побитовое ИЛИ: `левое | правое`.
    BitwiseOr(Location, Box<Expression>, Box<Expression>),
    /// Меньше: `левое < правое`.
    Less(Location, Box<Expression>, Box<Expression>),
    /// Больше: `левое > правое`.
    More(Location, Box<Expression>, Box<Expression>),
    /// Меньше или равно: `левое <= правое`.
    LessEqual(Location, Box<Expression>, Box<Expression>),
    /// Больше или равно: `левое >= правое`.
    MoreEqual(Location, Box<Expression>, Box<Expression>),
    /// Равенство: `левое == правое`.
    Equal(Location, Box<Expression>, Box<Expression>),
    /// Неравенство: `левое != правое`.
    NotEqual(Location, Box<Expression>, Box<Expression>),
    /// Логическое И: `левое && правое`.
    And(Location, Box<Expression>, Box<Expression>),
    /// Логическое ИЛИ: `левое || правое`.
    Or(Location, Box<Expression>, Box<Expression>),
    /// Тернарный оператор: `условие ? тогда : иначе`.
    ConditionalOperator(Location, Box<Expression>, Box<Expression>, Box<Expression>),
    /// Присваивание: `левое = правое`.
    Assign(Location, Box<Expression>, Box<Expression>),
    /// Целочисленный литерал.
    Number(Location, i64),
    /// Вещественный литерал: `(строка, отрицательный)`.
    Float(Location, String, bool),
    /// Конкатенация строковых литералов.
    String(Vec<StringLiteral>),
    /// Тип как выражение.
    Type(Location, Type),
    /// Адресный литерал: `адрес:бит`.
    Address(Location, i64, i64),
    /// Булевый литерал.
    Bool(Location, bool),
    /// Ссылка на переменную.
    Variable(Identifier),
    /// Список параметров: `(параметр,*)`.
    List(Location, ParameterList),
    /// Массивный литерал: `[элемент,*]`.
    Array(Location, Vec<Expression>),
    /// Инициализатор структуры: `{ элемент,* }`.
    Initializer(Location, Vec<Expression>),
    /// Приведение типа: `выражение as Тип`.
    Cast(Location, Box<Expression>, Type),
}

/// Вспомогательный макрос для получения компонент выражения.
///
/// Используется в методах [`Expression::components`] и [`Expression::components_mut`].
macro_rules! expr_components {
    ($s:ident) => {
        match $s {
            // Унарные: (None, Some)
            Not(_, expr)
            | BitwiseNot(_, expr)
            | UnaryPlus(_, expr)
            | Negate(_, expr)
            | Parenthesis(_, expr) => (None, Some(expr)),

            // Бинарные: (Some, Some)
            Power(_, left, right)
            | Multiply(_, left, right)
            | Divide(_, left, right)
            | Modulo(_, left, right)
            | Add(_, left, right)
            | Subtract(_, left, right)
            | ShiftLeft(_, left, right)
            | ShiftRight(_, left, right)
            | BitwiseAnd(_, left, right)
            | BitwiseXor(_, left, right)
            | BitwiseOr(_, left, right)
            | Less(_, left, right)
            | More(_, left, right)
            | LessEqual(_, left, right)
            | MoreEqual(_, left, right)
            | Equal(_, left, right)
            | NotEqual(_, left, right)
            | And(_, left, right)
            | Or(_, left, right)
            | Assign(_, left, right) => (Some(left), Some(right)),

            // Листовые: (None, None)
            BitAccess(..)
            | ConditionalOperator(..)
            | ArraySubscript(..)
            | ArraySlice(..)
            | Function(..)
            | CodeBlock(..)
            | NamedFunction(..)
            | Number(..)
            | Float(..)
            | String(..)
            | Type(..)
            | Bool(..)
            | Address(..)
            | Variable(..)
            | List(..)
            | Cast(..)
            | Initializer(..)
            | Array(..) => (None, None),
        }
    };
}

impl Expression {
    /// Убирает один уровень скобок.
    ///
    /// Если `self` является [`Parenthesis`](Expression::Parenthesis), возвращает
    /// внутреннее выражение; иначе возвращает `self`.
    #[inline]
    pub fn remove_parenthesis(&self) -> &Expression {
        if let Expression::Parenthesis(_, expr) = self {
            expr
        } else {
            self
        }
    }

    /// Рекурсивно убирает все уровни скобок.
    pub fn strip_parentheses(&self) -> &Expression {
        match self {
            Expression::Parenthesis(_, expr) => expr.strip_parentheses(),
            _ => self,
        }
    }

    /// Возвращает разделяемые ссылки на компоненты выражения.
    ///
    /// Возвращает пару `(левая_часть, правая_часть)`:
    /// - для унарных операторов — `(None, Some(операнд))`,
    /// - для бинарных — `(Some(левый), Some(правый))`,
    /// - для литералов и вызовов — `(None, None)`.
    ///
    /// # Примеры
    ///
    /// ```
    /// use grammar::ast::{Expression, Identifier, Location};
    ///
    /// // Унарный: ~a
    /// let var = Expression::Variable(Identifier::new("a"));
    /// let bitwise_not = Expression::BitwiseNot(Location::default(), Box::new(var.clone()));
    /// assert_eq!(bitwise_not.components(), (None, Some(&var)));
    ///
    /// // Бинарный: a + b
    /// let var_a = Expression::Variable(Identifier::new("a"));
    /// let var_b = Expression::Variable(Identifier::new("b"));
    /// let add = Expression::Add(
    ///     Location::default(),
    ///     Box::new(var_a.clone()),
    ///     Box::new(var_b.clone()),
    /// );
    /// assert_eq!(add.components(), (Some(&var_a), Some(&var_b)));
    ///
    /// // Литерал: 42
    /// let num = Expression::Number(Location::default(), 42);
    /// assert_eq!(num.components(), (None, None));
    /// ```
    #[inline]
    pub fn components(&self) -> (Option<&Self>, Option<&Self>) {
        use Expression::*;
        expr_components!(self)
    }

    /// Возвращает изменяемые ссылки на компоненты выражения.
    ///
    /// См. также [`Expression::components`].
    #[inline]
    pub fn components_mut(&mut self) -> (Option<&mut Self>, Option<&mut Self>) {
        use Expression::*;
        expr_components!(self)
    }

    /// Возвращает `true`, если выражение нельзя разбить на несколько строк.
    #[inline]
    pub const fn is_unsplittable(&self) -> bool {
        use Expression::*;
        matches!(
            self,
            Number(..) | Float(..) | String(..) | Address(..) | Variable(..)
        )
    }

    /// Возвращает `true`, если вокруг оператора нужны пробелы.
    #[inline]
    pub const fn has_space_around(&self) -> bool {
        use Expression::*;
        !matches!(self, Not(..) | BitwiseNot(..) | UnaryPlus(..) | Negate(..))
    }

    /// Возвращает `true`, если выражение является литералом.
    pub fn is_literal(&self) -> bool {
        matches!(
            self,
            Expression::Address(..)
                | Expression::Number(..)
                | Expression::Array(..)
                | Expression::Float(..)
                | Expression::String(..)
        )
    }

    /// Возвращает местоположение выражения в исходном тексте.
    pub fn loc(&self) -> Location {
        match self {
            Expression::ArraySubscript(loc, _, _) => loc.clone(),
            Expression::ArraySlice(loc, _, _, _) => loc.clone(),
            Expression::Parenthesis(loc, _) => loc.clone(),
            Expression::BitAccess(loc, _, _) => loc.clone(),
            Expression::Function(loc, _, _) => loc.clone(),
            Expression::CodeBlock(loc, _, _) => loc.clone(),
            Expression::NamedFunction(loc, _, _) => loc.clone(),
            Expression::Not(loc, _) => loc.clone(),
            Expression::BitwiseNot(loc, _) => loc.clone(),
            Expression::UnaryPlus(loc, _) => loc.clone(),
            Expression::Negate(loc, _) => loc.clone(),
            Expression::Power(loc, _, _) => loc.clone(),
            Expression::Multiply(loc, _, _) => loc.clone(),
            Expression::Divide(loc, _, _) => loc.clone(),
            Expression::Modulo(loc, _, _) => loc.clone(),
            Expression::Add(loc, _, _) => loc.clone(),
            Expression::Subtract(loc, _, _) => loc.clone(),
            Expression::ShiftLeft(loc, _, _) => loc.clone(),
            Expression::ShiftRight(loc, _, _) => loc.clone(),
            Expression::BitwiseAnd(loc, _, _) => loc.clone(),
            Expression::BitwiseXor(loc, _, _) => loc.clone(),
            Expression::BitwiseOr(loc, _, _) => loc.clone(),
            Expression::Less(loc, _, _) => loc.clone(),
            Expression::More(loc, _, _) => loc.clone(),
            Expression::LessEqual(loc, _, _) => loc.clone(),
            Expression::MoreEqual(loc, _, _) => loc.clone(),
            Expression::Equal(loc, _, _) => loc.clone(),
            Expression::NotEqual(loc, _, _) => loc.clone(),
            Expression::And(loc, _, _) => loc.clone(),
            Expression::Or(loc, _, _) => loc.clone(),
            Expression::ConditionalOperator(loc, _, _, _) => loc.clone(),
            Expression::Assign(loc, _, _) => loc.clone(),
            Expression::Number(loc, _) => loc.clone(),
            Expression::Float(loc, _, _) => loc.clone(),
            Expression::String(_) => Location::Builtin,
            Expression::Type(loc, _) => loc.clone(),
            Expression::Address(loc, _, _) => loc.clone(),
            Expression::Bool(loc, _) => loc.clone(),
            Expression::Variable(var) => var.loc.clone(),
            Expression::List(loc, _) => loc.clone(),
            Expression::Array(loc, _) => loc.clone(),
            Expression::Initializer(loc, _) => loc.clone(),
            Expression::Cast(loc, _, _) => loc.clone(),
        }
    }
}

/// Параметр функции: опциональное имя и тип.
#[derive(Debug, PartialEq, Eq, Clone)]
#[cfg_attr(feature = "ast-serde", derive(Serialize, Deserialize))]
pub struct Parameter {
    /// Местоположение в исходном тексте.
    pub loc: Location,
    /// Тип параметра (выражение).
    pub ty: Expression,
    /// Имя параметра (может отсутствовать).
    pub name: Option<Identifier>,
}

/// Определение формулы (`formula`).
#[derive(Debug, PartialEq, Eq, Clone)]
#[cfg_attr(feature = "ast-serde", derive(Serialize, Deserialize))]
pub struct FormulaDefine {
    /// Местоположение в исходном тексте.
    pub loc: Location,
    /// Блок тела формулы.
    pub formula: FormulaBlock,
}

/// Определение условия перехода (`cond Имя = Условие`).
#[derive(Debug, PartialEq, Eq, Clone)]
#[cfg_attr(feature = "ast-serde", derive(Serialize, Deserialize))]
pub struct ConditionDefine {
    /// Местоположение в исходном тексте.
    pub loc: Location,
    /// Имя условия.
    pub name: Option<Identifier>,
    /// Выражение условия.
    pub value: Condition,
}

/// Именованный блок кода (`enter`, `exit`, `always`, …).
#[derive(Debug, PartialEq, Eq, Clone)]
#[cfg_attr(feature = "ast-serde", derive(Serialize, Deserialize))]
pub struct NamedBlockCodeDefine {
    /// Местоположение в исходном тексте.
    pub loc: Location,
    /// Имя блока.
    pub name: Option<Identifier>,
    /// Тело блока (оператор).
    pub statement: Statement,
}

/// Определение функции (`fn Имя(параметры) [-> Тип] [{ тело }]`).
#[derive(Debug, PartialEq, Eq, Clone)]
#[cfg_attr(feature = "ast-serde", derive(Serialize, Deserialize))]
pub struct FunctionDefine {
    /// Местоположение всего определения.
    pub loc: Location,
    /// Имя функции.
    pub name: Option<Identifier>,
    /// Местоположение имени.
    pub name_loc: Location,
    /// Список параметров.
    pub params: ParameterList,
    /// Возвращаемый тип (если есть).
    pub return_type: Option<Type>,
    /// Тело функции (если есть).
    pub body: Option<Statement>,
    /// Является ли функция внешней.
    pub external: bool,
}

impl FunctionDefine {
    /// Возвращает `true`, если функция не возвращает значения.
    #[inline]
    pub fn is_void(&self) -> bool {
        self.return_type.is_none()
    }

    /// Возвращает `true`, если тело функции отсутствует или пусто.
    #[inline]
    pub fn is_empty(&self) -> bool {
        self.body.as_ref().map_or(true, Statement::is_empty)
    }
}

/// Оператор языка BuT.
#[derive(Debug, PartialEq, Eq, Clone)]
#[cfg_attr(feature = "ast-serde", derive(Serialize, Deserialize))]
#[allow(clippy::large_enum_variant, clippy::type_complexity)]
pub enum Statement {
    /// Блок операторов: `{ операторы* }`.
    Block {
        /// Местоположение блока.
        loc: Location,
        /// Признак проверяемого блока (зарезервировано).
        unchecked: bool,
        /// Список операторов.
        statements: Vec<Statement>,
    },
    /// Блок ассемблерного кода: `assembly [диалект] { ... }`.
    Assembly {
        /// Местоположение.
        loc: Location,
        /// Диалект ассемблера.
        dialect: Option<StringLiteral>,
        /// Флаги.
        flags: Option<Vec<StringLiteral>>,
        /// Блок тела.
        block: Box<Statement>,
    },
    /// Блок формулы: `formula [диалект] { ... }`.
    Formula {
        /// Местоположение.
        loc: Location,
        /// Диалект.
        dialect: Option<StringLiteral>,
        /// Флаги.
        flags: Option<Vec<StringLiteral>>,
        /// Блок тела.
        block: Box<FormulaBlock>,
    },
    /// Именованные аргументы: `{ ключ: значение, … }`.
    Args(Location, Vec<NamedArgument>),
    /// Оператор `if`: `if условие блок [else ветка]`.
    If(Location, Expression, Box<Statement>, Option<Box<Statement>>),
    /// Оператор `while`: `while (условие) тело`.
    While(Location, Expression, Box<Statement>),
    /// Оператор-выражение.
    Expression(Location, Expression),
    /// Объявление переменной с опциональным инициализатором.
    Variable(Location, Box<VariableDefine>, Option<Expression>),
    /// Оператор `for`: `for (инит; условие; шаг) тело`.
    For(
        Location,
        Option<Box<Statement>>,
        Option<Box<Expression>>,
        Option<Box<Expression>>,
        Option<Box<Statement>>,
    ),
    /// Оператор `do ... while`: `do тело while (условие)`.
    DoWhile(Location, Box<Statement>, Expression),
    /// Оператор `continue`.
    Continue(Location),
    /// Оператор `break`.
    Break(Location),
    /// Оператор `return [выражение]`.
    Return(Location, Option<Expression>),
    /// Ошибка (вставляется при восстановлении после сбоя парсера).
    Error(Location),
    /// Одиночная точка с запятой.
    StraySemicolon(Location),
}

impl Statement {
    /// Возвращает `true`, если блочный оператор не содержит элементов.
    #[inline]
    pub fn is_empty(&self) -> bool {
        match self {
            Self::Block { statements, .. } => statements.is_empty(),
            Self::Assembly { block, .. } => block.is_empty(),
            Self::Args(_, args) => args.is_empty(),
            _ => false,
        }
    }
}

/// Оператор в блоке формулы.
#[derive(Debug, PartialEq, Eq, Clone)]
#[cfg_attr(feature = "ast-serde", derive(Serialize, Deserialize))]
pub enum FormulaStatement {
    /// Выражение формулы.
    Expression(Location, FormulaExpression),
    /// Вложенный блок формулы.
    Block(FormulaBlock),
    /// Вызов функции формулы.
    Function(Box<FormulaFunction>),
    /// Ошибка при разборе.
    Error(Location),
}

/// Блок формулы: `{ операторы* }`.
#[derive(Debug, PartialEq, Eq, Clone)]
#[cfg_attr(feature = "ast-serde", derive(Serialize, Deserialize))]
pub struct FormulaBlock {
    /// Местоположение блока.
    pub loc: Location,
    /// Список операторов формулы.
    pub statements: Vec<FormulaStatement>,
}

impl FormulaBlock {
    /// Возвращает `true`, если блок не содержит операторов.
    #[inline]
    pub fn is_empty(&self) -> bool {
        self.statements.is_empty()
    }
}

/// Выражение в формуле.
#[derive(Debug, PartialEq, Eq, Clone)]
#[cfg_attr(feature = "ast-serde", derive(Serialize, Deserialize))]
pub enum FormulaExpression {
    /// Булевый литерал с опциональной аннотацией типа: `true[:тип]`.
    Bool(Location, bool, Option<Identifier>),
    /// Числовой литерал с опциональной аннотацией: `42[:тип]`.
    Number(Location, i64, Option<Identifier>),
    /// Строковый литерал с опциональной аннотацией: `"s"[:тип]`.
    String(StringLiteral, Option<Identifier>),
    /// Ссылка на переменную.
    Variable(Identifier),
    /// Вызов функции.
    Function(Box<FormulaFunction>),
    /// Доступ к члену: `выражение.имя`.
    SuffixAccess(Location, Box<FormulaExpression>, Identifier),
    /// Скобки.
    Parenthesis(Location, Box<FormulaExpression>),
}

/// Вызов функции в формуле: `id(аргументы,*)`.
#[derive(Debug, PartialEq, Eq, Clone)]
#[cfg_attr(feature = "ast-serde", derive(Serialize, Deserialize))]
pub struct FormulaFunction {
    /// Местоположение в исходном тексте.
    pub loc: Location,
    /// Имя функции.
    pub id: Identifier,
    /// Аргументы вызова.
    pub arguments: Vec<FormulaExpression>,
}
