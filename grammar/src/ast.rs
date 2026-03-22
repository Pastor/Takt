#[cfg(feature = "ast-serde")]
use serde::{Deserialize, Serialize};

#[derive(Clone, Debug, PartialEq, Eq, PartialOrd, Ord, Hash)]
#[cfg_attr(feature = "ast-serde", derive(Serialize, Deserialize))]
pub enum Location {
    Builtin,
    CommandLine,
    Implicit,
    Codegen,
    Source(u64, usize, usize),
}

impl Default for Location {
    fn default() -> Self {
        Self::Source(0, 0, 0)
    }
}

#[inline(never)]
#[cold]
#[track_caller]
fn not_a_file() -> ! {
    panic!("location is not a file")
}

impl Location {
    #[inline]
    pub fn begin_range(&self) -> Self {
        match self {
            Location::Source(filename, start, _) => {
                Location::Source(filename.clone(), *start, *start)
            }
            loc => loc.clone(),
        }
    }

    #[inline]
    pub fn end_range(&self) -> Self {
        match self {
            Location::Source(filename, _, end) => Location::Source(filename.clone(), *end, *end),
            loc => loc.clone(),
        }
    }

    #[track_caller]
    #[inline]
    pub fn filename(&self) -> String {
        match self {
            Location::Source(file_no, _, _) => format!("{}", file_no),
            _ => not_a_file(),
        }
    }

    #[inline]
    pub fn try_file_no(&self) -> Option<String> {
        match self {
            Location::Source(file_no, _, _) => Some(format!("{}", file_no)),
            _ => None,
        }
    }

    #[track_caller]
    #[inline]
    pub fn start(&self) -> usize {
        match self {
            Location::Source(_, start, _) => *start,
            _ => not_a_file(),
        }
    }

    #[track_caller]
    #[inline]
    pub fn end(&self) -> usize {
        match self {
            Location::Source(_, _, end) => *end,
            _ => not_a_file(),
        }
    }

    #[track_caller]
    #[inline]
    pub fn exclusive_end(&self) -> usize {
        self.end() + 1
    }

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

    #[track_caller]
    #[inline]
    pub fn with_start_from(mut self, other: &Self) -> Self {
        self.use_start_from(other);
        self
    }

    #[track_caller]
    #[inline]
    pub fn with_end_from(mut self, other: &Self) -> Self {
        self.use_end_from(other);
        self
    }

    #[track_caller]
    #[inline]
    pub fn with_start(self, start: usize) -> Self {
        match self {
            Self::Source(no, _, end) => Self::Source(no, start, end),
            _ => not_a_file(),
        }
    }

    #[track_caller]
    #[inline]
    pub fn with_end(self, end: usize) -> Self {
        match self {
            Self::Source(no, start, _) => Self::Source(no, start, end),
            _ => not_a_file(),
        }
    }

    #[track_caller]
    #[inline]
    pub fn range(self) -> std::ops::Range<usize> {
        match self {
            Self::Source(_, start, end) => start..end,
            _ => not_a_file(),
        }
    }
}

#[derive(Debug, PartialEq, Eq, Clone)]
#[cfg_attr(feature = "ast-serde", derive(Serialize, Deserialize))]
pub struct Identifier {
    pub loc: Location,
    pub name: String,
}

impl Identifier {
    pub fn new(s: impl Into<String>) -> Self {
        Self {
            loc: Location::default(),
            name: s.into(),
        }
    }
}

#[derive(Debug, PartialEq, Eq, Clone)]
#[cfg_attr(feature = "ast-serde", derive(Serialize, Deserialize))]
pub struct IdentifierPath {
    pub loc: Location,
    pub identifiers: Vec<Identifier>,
}

#[derive(Debug, PartialEq, Eq, Clone)]
#[cfg_attr(feature = "ast-serde", derive(Serialize, Deserialize))]
pub enum Comment {
    Line(Location, String),
    DocLine(Location, String),
}

impl Comment {
    #[inline]
    pub const fn value(&self) -> &String {
        match self {
            Self::Line(_, s) | Self::DocLine(_, s) => s,
        }
    }
    #[inline]
    pub const fn is_doc(&self) -> bool {
        matches!(self, Self::DocLine(..))
    }

    #[inline]
    pub const fn is_line(&self) -> bool {
        matches!(self, Self::Line(..) | Self::DocLine(..))
    }
}

#[derive(Debug, PartialEq, Eq, Clone)]
#[cfg_attr(feature = "ast-serde", derive(Serialize, Deserialize))]
pub enum ImportDefine {
    Plain(ImportPath, Location),
    GlobalSymbol(ImportPath, Identifier, Location),
    Rename(ImportPath, Vec<(Identifier, Option<Identifier>)>, Location),
}

#[derive(Debug, PartialEq, Eq, Clone)]
#[cfg_attr(feature = "ast-serde", derive(Serialize, Deserialize))]
pub enum ImportPath {
    Filename(StringLiteral),
    Path(IdentifierPath),
}

impl ImportDefine {
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

pub type ParameterList = Vec<(Location, Option<Parameter>)>;

#[derive(Debug, PartialEq, Eq, Clone)]
#[cfg_attr(feature = "ast-serde", derive(Serialize, Deserialize))]
pub enum Type {
    Address {
        address: u64,
        bit: Option<u64>,
    },
    Bit,
    Bool,
    Float,
    Alias(Identifier),
    Array {
        loc: Location,
        element_count: u16,
        element_type: Box<Type>,
    },
    Function {
        params: ParameterList,
        returns: Option<ParameterList>,
    },
}

#[derive(Debug, PartialEq, Eq, Clone)]
#[cfg_attr(feature = "ast-serde", derive(Serialize, Deserialize))]
pub struct VariableDefine {
    pub loc: Location,
    pub ty: Option<Type>,
    pub name: Option<Identifier>,
    pub initializer: Option<Expression>,
    pub mutability: bool,
}

#[derive(Debug, PartialEq, Eq, Clone)]
#[cfg_attr(feature = "ast-serde", derive(Serialize, Deserialize))]
pub struct PortDefine {
    pub loc: Location,
    pub ty: Type,
    pub name: Option<Identifier>,
    pub initializer: Option<Expression>,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
#[cfg_attr(feature = "ast-serde", derive(Serialize, Deserialize))]
pub enum UserDefinedOperator {
    /// `&`
    BitwiseAnd,
    /// `~`
    ///
    BitwiseNot,
    /// `-`
    ///
    /// Note that this is the same as `Subtract`, and that it is currently not being parsed.
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
    /// Returns the number of arguments needed for this operator's operation.
    #[inline]
    pub const fn args(&self) -> usize {
        match self {
            UserDefinedOperator::BitwiseNot | UserDefinedOperator::Negate => 1,
            _ => 2,
        }
    }

    /// Returns whether `self` is a unary operator.
    #[inline]
    pub const fn is_unary(&self) -> bool {
        matches!(self, Self::BitwiseNot | Self::Negate)
    }

    /// Returns whether `self` is a binary operator.
    #[inline]
    pub const fn is_binary(&self) -> bool {
        !self.is_unary()
    }

    /// Returns whether `self` is a bitwise operator.
    #[inline]
    pub const fn is_bitwise(&self) -> bool {
        matches!(
            self,
            Self::BitwiseAnd | Self::BitwiseOr | Self::BitwiseXor | Self::BitwiseNot
        )
    }

    /// Returns whether `self` is an arithmetic operator.
    #[inline]
    pub const fn is_arithmetic(&self) -> bool {
        matches!(
            self,
            Self::Add | Self::Subtract | Self::Multiply | Self::Divide | Self::Modulo
        )
    }

    /// Returns whether this is a comparison operator.
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

#[derive(Debug, PartialEq, Eq, Clone)]
#[cfg_attr(feature = "ast-serde", derive(Serialize, Deserialize))]
pub struct Model {
    pub loc: Location,
    pub name: Option<Identifier>,
    pub elements: Vec<ModelElement>,
    pub implements: Option<Expression>,
}

#[derive(Debug, PartialEq, Eq, Clone)]
#[cfg_attr(feature = "ast-serde", derive(Serialize, Deserialize))]
pub enum ModelElement {
    Import(ImportDefine),
    Function(Box<FunctionDefine>),
    Formula(Box<FormulaDefine>),
    Condition(Box<ConditionDefine>),
    Variable(Box<VariableDefine>),
    Port(Box<PortDefine>),
    Type(Box<TypeDefine>),
    State(Box<StateDefine>),
    Model(Box<Model>),
    NamedBlockCode(Box<NamedBlockCodeDefine>),
    StraySemicolon(Location),
}

#[derive(Debug, PartialEq, Eq, Clone)]
#[cfg_attr(feature = "ast-serde", derive(Serialize, Deserialize))]
pub enum StateKind {
    Start,
    End,
    Next,
}

#[derive(Debug, PartialEq, Eq, Clone)]
#[cfg_attr(feature = "ast-serde", derive(Serialize, Deserialize))]
pub struct StateDefine {
    pub loc: Location,
    pub name: Option<Identifier>,
    pub elements: Vec<StateElement>,
    pub implements: Option<Expression>,
    pub kind: Option<StateKind>,
}

#[derive(Debug, PartialEq, Eq, Clone)]
#[cfg_attr(feature = "ast-serde", derive(Serialize, Deserialize))]
pub enum StateElement {
    Import(ImportDefine),
    Next(Identifier),
    Function(Box<FunctionDefine>),
    Formula(Box<FormulaDefine>),
    Condition(Box<ConditionDefine>),
    Variable(Box<VariableDefine>),
    Type(Box<TypeDefine>),
    Reference(Location, Identifier, Option<Condition>),
    Model(Box<Model>),
    NamedBlockCode(Box<NamedBlockCodeDefine>),
    StraySemicolon(Location),
}

/// Both have the same semantics:
///
/// `<name>[(<args>,*)]`
#[derive(Debug, PartialEq, Eq, Clone)]
#[cfg_attr(feature = "ast-serde", derive(Serialize, Deserialize))]
pub struct Base {
    /// The code location.
    pub loc: Location,
    /// The identifier path.
    pub name: IdentifierPath,
    /// The optional arguments.
    pub args: Option<Vec<Expression>>,
}

#[derive(Debug, PartialEq, Eq, Clone)]
#[cfg_attr(feature = "ast-serde", derive(Serialize, Deserialize))]
pub struct Variable {
    pub loc: Location,
    pub ty: Box<TypeDefine>,
    pub name: Option<Identifier>,
    pub initializer: Option<Expression>,
}

#[derive(Debug, PartialEq, Eq, Clone)]
#[cfg_attr(feature = "ast-serde", derive(Serialize, Deserialize))]
pub struct TypeDefine {
    pub loc: Location,
    pub name: Identifier,
    pub ty: Type,
}

/// An annotation.
#[derive(Debug, PartialEq, Eq, Clone)]
#[cfg_attr(feature = "ast-serde", derive(Serialize, Deserialize))]
pub enum Annotation {
    Identifier(Location, IdentifierPath),
    Function {
        loc: Location,
        name: Identifier,
        args: Vec<Annotation>,
    },
    Assign {
        loc: Location,
        name: IdentifierPath,
        value: Expression,
    },
    String(StringLiteral),
    Number(Location, i64),
    Rational(Location, String, bool),
    Boolean(Location, bool),
    Visibility(Location, Expression),
}

#[derive(Debug, PartialEq, Eq, Clone)]
#[cfg_attr(feature = "ast-serde", derive(Serialize, Deserialize))]
pub struct StringLiteral {
    pub loc: Location,
    pub unicode: bool,
    pub string: String,
}

#[derive(Debug, PartialEq, Eq, Clone)]
#[cfg_attr(feature = "ast-serde", derive(Serialize, Deserialize))]
pub struct NamedArgument {
    pub loc: Location,
    pub name: Option<Identifier>,
    pub expr: Expression,
}

#[derive(Debug, PartialEq, Eq, Clone)]
#[cfg_attr(feature = "ast-serde", derive(Serialize, Deserialize))]
pub enum Condition {
    /// `<1>\[ [2] \]`
    ArraySubscript(Location, Identifier, i64),
    /// `(<1>)`
    Parenthesis(Location, Box<Condition>),
    /// `<1>.<2>`
    BitAccess(Location, Box<Condition>, Member),
    /// `<1>(<2>,*)`
    Function(Location, Identifier, Vec<Condition>),
    /// `!<1>`
    Not(Location, Box<Condition>),
    /// `<1> + <2>`
    Add(Location, Box<Condition>, Box<Condition>),
    /// `<1> - <2>`
    Subtract(Location, Box<Condition>, Box<Condition>),
    /// `<1> & <2>`
    And(Location, Box<Condition>, Box<Condition>),
    /// `<1> | <2>`
    Or(Location, Box<Condition>, Box<Condition>),
    /// `<1> < <2>`
    Less(Location, Box<Condition>, Box<Condition>),
    /// `<1> > <2>`
    More(Location, Box<Condition>, Box<Condition>),
    /// `<1> <= <2>`
    LessEqual(Location, Box<Condition>, Box<Condition>),
    /// `<1> >= <2>`
    MoreEqual(Location, Box<Condition>, Box<Condition>),
    /// `<1> = <2>`
    Equal(Location, Box<Condition>, Box<Condition>),
    /// `<1> != <2>`
    NotEqual(Location, Box<Condition>, Box<Condition>),
    Number(Location, i64),
    Float(Location, String, bool),
    String(Vec<StringLiteral>),
    Bool(Location, bool),
    Variable(Identifier),
}

#[derive(Debug, PartialEq, Eq, Clone)]
#[cfg_attr(feature = "ast-serde", derive(Serialize, Deserialize))]
pub enum Member {
    Identifier(Identifier),
    Number(i64),
}

#[derive(Debug, PartialEq, Eq, Clone)]
#[cfg_attr(feature = "ast-serde", derive(Serialize, Deserialize))]
pub enum Expression {
    /// `<1>\[ [2] \]`
    ArraySubscript(Location, Identifier, i64),
    /// `<1>\[ [2] : [3] \]`
    ArraySlice(Location, Identifier, Option<i64>, Option<i64>),
    /// `(<1>)`
    Parenthesis(Location, Box<Expression>),
    /// `<1>.<2>`
    BitAccess(Location, Box<Expression>, Member),
    /// `<1>(<2>,*)`
    Function(Location, Identifier, Vec<Expression>),
    /// `<1><2>` where <2> is a block.
    CodeBlock(Location, Box<Expression>, Box<Statement>),
    /// `<1>({ <2>,* })`
    NamedFunction(Location, Box<Expression>, Vec<NamedArgument>),
    /// `!<1>`
    Not(Location, Box<Expression>),
    /// `~<1>`
    BitwiseNot(Location, Box<Expression>),
    UnaryPlus(Location, Box<Expression>),
    /// `-<1>`
    Negate(Location, Box<Expression>),

    /// `<1> ** <2>`
    Power(Location, Box<Expression>, Box<Expression>),
    /// `<1> * <2>`
    Multiply(Location, Box<Expression>, Box<Expression>),
    /// `<1> / <2>`
    Divide(Location, Box<Expression>, Box<Expression>),
    /// `<1> % <2>`
    Modulo(Location, Box<Expression>, Box<Expression>),
    /// `<1> + <2>`
    Add(Location, Box<Expression>, Box<Expression>),
    /// `<1> - <2>`
    Subtract(Location, Box<Expression>, Box<Expression>),
    /// `<1> << <2>`
    ShiftLeft(Location, Box<Expression>, Box<Expression>),
    /// `<1> >> <2>`
    ShiftRight(Location, Box<Expression>, Box<Expression>),
    /// `<1> & <2>`
    BitwiseAnd(Location, Box<Expression>, Box<Expression>),
    /// `<1> ^ <2>`
    BitwiseXor(Location, Box<Expression>, Box<Expression>),
    /// `<1> | <2>`
    BitwiseOr(Location, Box<Expression>, Box<Expression>),
    /// `<1> < <2>`
    Less(Location, Box<Expression>, Box<Expression>),
    /// `<1> > <2>`
    More(Location, Box<Expression>, Box<Expression>),
    /// `<1> <= <2>`
    LessEqual(Location, Box<Expression>, Box<Expression>),
    /// `<1> >= <2>`
    MoreEqual(Location, Box<Expression>, Box<Expression>),
    /// `<1> == <2>`
    Equal(Location, Box<Expression>, Box<Expression>),
    /// `<1> != <2>`
    NotEqual(Location, Box<Expression>, Box<Expression>),
    /// `<1> && <2>`
    And(Location, Box<Expression>, Box<Expression>),
    /// `<1> || <2>`
    Or(Location, Box<Expression>, Box<Expression>),
    /// `<1> ? <2> : <3>`
    ///
    /// AKA ternary operator.
    ConditionalOperator(Location, Box<Expression>, Box<Expression>, Box<Expression>),
    /// `<1> = <2>`
    Assign(Location, Box<Expression>, Box<Expression>),
    Number(Location, i64),
    Float(Location, String, bool),
    String(Vec<StringLiteral>),
    Type(Location, Type),
    Address(Location, i64, i64),
    Bool(Location, bool),
    Variable(Identifier),
    /// `(<1>,*)`
    List(Location, ParameterList),
    /// `\[ <1>.* \]`
    Array(Location, Vec<Expression>),
    Initializer(Location, Vec<Expression>),
    Cast(Location, Box<Expression>, Type),
}

/// See `Expression::components`.
macro_rules! expr_components {
    ($s:ident) => {
        match $s {
            // (None, Some)
            Not(_, expr)
            | BitwiseNot(_, expr)
            | UnaryPlus(_, expr)
            | Negate(_, expr)
            | Parenthesis(_, expr) => (None, Some(expr)),

            // (Some, Some)
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

            // (None, None)
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
    /// Removes one layer of parentheses.
    #[inline]
    pub fn remove_parenthesis(&self) -> &Expression {
        if let Expression::Parenthesis(_, expr) = self {
            expr
        } else {
            self
        }
    }

    /// Strips all parentheses recursively.
    pub fn strip_parentheses(&self) -> &Expression {
        match self {
            Expression::Parenthesis(_, expr) => expr.strip_parentheses(),
            _ => self,
        }
    }

    /// Returns shared references to the components of this expression.
    ///
    /// `(left_component, right_component)`
    ///
    /// # Examples
    ///
    /// ```
    /// use but_grammar::ast::{Expression, Identifier, Loc};
    ///
    /// // `a++`
    /// let var = Expression::Variable(Identifier::new("a"));
    /// let post_increment = Expression::PostIncrement(Loc::default(), Box::new(var.clone()));
    /// assert_eq!(post_increment.components(), (Some(&var), None));
    ///
    /// // `++a`
    /// let var = Expression::Variable(Identifier::new("a"));
    /// let pre_increment = Expression::PreIncrement(Loc::default(), Box::new(var.clone()));
    /// assert_eq!(pre_increment.components(), (None, Some(&var)));
    ///
    /// // `a + b`
    /// let var_a = Expression::Variable(Identifier::new("a"));
    /// let var_b = Expression::Variable(Identifier::new("b"));
    /// let pre_increment = Expression::Add(Loc::default(), Box::new(var_a.clone()), Box::new(var_b.clone()));
    /// assert_eq!(pre_increment.components(), (Some(&var_a), Some(&var_b)));
    /// ```
    #[inline]
    pub fn components(&self) -> (Option<&Self>, Option<&Self>) {
        use Expression::*;
        expr_components!(self)
    }

    /// Returns mutable references to the components of this expression.
    ///
    /// See also [`Expression::components`].
    #[inline]
    pub fn components_mut(&mut self) -> (Option<&mut Self>, Option<&mut Self>) {
        use Expression::*;
        expr_components!(self)
    }

    /// Returns whether this expression can be split across multiple lines.
    #[inline]
    pub const fn is_unsplittable(&self) -> bool {
        use Expression::*;
        matches!(
            self,
            Number(..) | Float(..) | String(..) | Address(..) | Variable(..)
        )
    }

    /// Returns whether this expression has spaces around it.
    #[inline]
    pub const fn has_space_around(&self) -> bool {
        use Expression::*;
        !matches!(self, Not(..) | BitwiseNot(..) | UnaryPlus(..) | Negate(..))
    }

    /// Returns if the expression is a literal
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

#[derive(Debug, PartialEq, Eq, Clone)]
#[cfg_attr(feature = "ast-serde", derive(Serialize, Deserialize))]
pub struct Parameter {
    pub loc: Location,
    pub ty: Expression,
    pub name: Option<Identifier>,
}

#[derive(Debug, PartialEq, Eq, Clone)]
#[cfg_attr(feature = "ast-serde", derive(Serialize, Deserialize))]
pub struct FormulaDefine {
    pub loc: Location,
    pub formula: FormulaBlock,
}

#[derive(Debug, PartialEq, Eq, Clone)]
#[cfg_attr(feature = "ast-serde", derive(Serialize, Deserialize))]
pub struct ConditionDefine {
    pub loc: Location,
    pub name: Option<Identifier>,
    pub value: Condition,
}

#[derive(Debug, PartialEq, Eq, Clone)]
#[cfg_attr(feature = "ast-serde", derive(Serialize, Deserialize))]
pub struct NamedBlockCodeDefine {
    pub loc: Location,
    pub name: Option<Identifier>,
    pub statement: Statement,
}

#[derive(Debug, PartialEq, Eq, Clone)]
#[cfg_attr(feature = "ast-serde", derive(Serialize, Deserialize))]
pub struct FunctionDefine {
    pub loc: Location,
    pub name: Option<Identifier>,
    /// The identifier's code location.
    pub name_loc: Location,
    pub params: ParameterList,
    pub return_type: Option<Type>,
    pub body: Option<Statement>,
}

impl FunctionDefine {
    /// Returns `true` if the function has no return parameters.
    #[inline]
    pub fn is_void(&self) -> bool {
        self.return_type.is_none()
    }

    /// Returns `true` if the function body is empty.
    #[inline]
    pub fn is_empty(&self) -> bool {
        self.body.as_ref().map_or(true, Statement::is_empty)
    }
}

/// A statement.
#[derive(Debug, PartialEq, Eq, Clone)]
#[cfg_attr(feature = "ast-serde", derive(Serialize, Deserialize))]
#[allow(clippy::large_enum_variant, clippy::type_complexity)]
pub enum Statement {
    Block {
        loc: Location,
        unchecked: bool,
        statements: Vec<Statement>,
    },
    /// `assembly [dialect] [(<flags>,*)] <block>`
    Assembly {
        /// The code location.
        loc: Location,
        /// The assembly dialect.
        dialect: Option<StringLiteral>,
        /// The assembly flags.
        flags: Option<Vec<StringLiteral>>,
        /// The assembly block.
        block: Box<Statement>,
    },
    Formula {
        /// The code location.
        loc: Location,
        /// The assembly dialect.
        dialect: Option<StringLiteral>,
        /// The assembly flags.
        flags: Option<Vec<StringLiteral>>,
        /// The assembly block.
        block: Box<FormulaBlock>,
    },
    /// `{ <1>,* }`
    Args(Location, Vec<NamedArgument>),
    /// `if ({1}) <2> [else <3>]`
    ///
    /// Note that the `<1>` expression does not contain the parentheses.
    If(Location, Expression, Box<Statement>, Option<Box<Statement>>),
    /// `while ({1}) <2>`
    ///
    /// Note that the `<1>` expression does not contain the parentheses.
    While(Location, Expression, Box<Statement>),
    /// An [Expression].
    Expression(Location, Expression),
    /// `<1> [= <2>];`
    Variable(Location, Box<VariableDefine>, Option<Expression>),
    /// `for ([1]; [2]; [3]) [4]`
    ///
    /// The `[4]` block statement is `None` when the `for` statement ends with a semicolon.
    For(
        Location,
        Option<Box<Statement>>,
        Option<Box<Expression>>,
        Option<Box<Expression>>,
        Option<Box<Statement>>,
    ),
    /// `do <1> while ({2});`
    ///
    /// Note that the `<2>` expression does not contain the parentheses.
    DoWhile(Location, Box<Statement>, Expression),
    /// `continue;`
    Continue(Location),
    /// `break;`
    Break(Location),
    /// `return [1];`
    Return(Location, Option<Expression>),
    /// An error occurred during parsing.
    Error(Location),
    StraySemicolon(Location),
}

impl Statement {
    /// Returns `true` if the block statement contains no elements.
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

/// A Formula statement.
#[derive(Debug, PartialEq, Eq, Clone)]
#[cfg_attr(feature = "ast-serde", derive(Serialize, Deserialize))]
pub enum FormulaStatement {
    Expression(Location, FormulaExpression),
    /// A [FormulaBlock] statement.
    Block(FormulaBlock),
    /// A [FormulaFunction] statement.
    Function(Box<FormulaFunction>),
    /// An error occurred during parsing.
    Error(Location),
}

/// A Formula block statement.
///
/// `{ <statements>* }`
#[derive(Debug, PartialEq, Eq, Clone)]
#[cfg_attr(feature = "ast-serde", derive(Serialize, Deserialize))]
pub struct FormulaBlock {
    /// The code location.
    pub loc: Location,
    /// The block statements.
    pub statements: Vec<FormulaStatement>,
}

impl FormulaBlock {
    /// Returns `true` if the block contains no elements.
    #[inline]
    pub fn is_empty(&self) -> bool {
        self.statements.is_empty()
    }
}

/// A Formula expression.
#[derive(Debug, PartialEq, Eq, Clone)]
#[cfg_attr(feature = "ast-serde", derive(Serialize, Deserialize))]
pub enum FormulaExpression {
    /// `<1> [: <2>]`
    Bool(Location, bool, Option<Identifier>),
    /// `<1>[e<2>] [: <2>]`
    Number(Location, i64, Option<Identifier>),
    /// `<0> [: <1>]`
    String(StringLiteral, Option<Identifier>),
    /// Any valid [Identifier].
    Variable(Identifier),
    /// [FormulaFunction].
    Function(Box<FormulaFunction>),
    /// `<1>.<2>`
    SuffixAccess(Location, Box<FormulaExpression>, Identifier),
    Parenthesis(Location, Box<FormulaExpression>),
}

/// A Formula function call.
///
/// `<id>(<arguments>,*)`
#[derive(Debug, PartialEq, Eq, Clone)]
#[cfg_attr(feature = "ast-serde", derive(Serialize, Deserialize))]
pub struct FormulaFunction {
    /// The code location.
    pub loc: Location,
    /// The identifier.
    pub id: Identifier,
    /// The function call arguments.
    pub arguments: Vec<FormulaExpression>,
}
