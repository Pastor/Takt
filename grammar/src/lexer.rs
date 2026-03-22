use std::str::FromStr;
use std::{fmt, str::CharIndices};

use itertools::{peek_nth, PeekNth};
use phf::phf_map;
use thiserror::Error;
use unicode_xid::UnicodeXID;

use crate::ast::{Comment, Location};

pub type Spanned<'a> = (usize, Token<'a>, usize);
pub type Result<'a, T = Spanned<'a>, E = LexicalError> = std::result::Result<T, E>;

#[derive(Copy, Clone, PartialEq, Eq, Debug)]
#[allow(missing_docs)]
pub enum Token<'input> {
    Identifier(&'input str),
    StringLiteral(bool, &'input str),
    AddressLiteral(&'input str),
    Number(i64),
    RationalNumber(&'input str, bool),
    Divide,
    Function,
    Pragma,
    Import,
    Type,
    Do,
    Continue,
    Break,
    Return,
    String,
    Sharp,
    Semicolon,
    Comma,
    OpenParenthesis,
    CloseParenthesis,
    OpenCurlyBrace,
    CloseCurlyBrace,

    BitwiseOr,
    BitwiseXor,
    Or,

    BitwiseAnd,
    BitwiseNot,
    And,
    Add,
    Subtract,
    Mul,
    Power,
    Modulo,

    Equal,
    Assign,

    NotEqual,
    Not,

    True,
    False,
    Else,
    For,
    While,
    If,

    ShiftRight,
    Less,
    LessEqual,

    ShiftLeft,
    More,
    MoreEqual,

    Member,
    Colon,
    OpenBracket,
    CloseBracket,

    As,

    Assembly,
    Formula,

    Constant,
    Port,

    PeirceArrow,

    Model,
    State,
    Start,
    Reference,
    Template,
    Condition,
    Variable,
    Next,
    Extern,
}

impl<'input> fmt::Display for Token<'input> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Token::Identifier(id) => write!(f, "{id}"),
            Token::StringLiteral(false, s) => write!(f, "\"{s}\""),
            Token::StringLiteral(true, s) => write!(f, "unicode\"{s}\""),
            Token::AddressLiteral(address) => write!(f, "{address}"),
            Token::Number(n) => write!(f, "{n}"),
            Token::RationalNumber(n, d) => {
                if *d {
                    write!(f, "-")?;
                }
                write!(f, "{n}")
            }
            Token::Semicolon => write!(f, ";"),
            Token::Comma => write!(f, ","),
            Token::Sharp => write!(f, "#"),
            Token::OpenParenthesis => write!(f, "("),
            Token::CloseParenthesis => write!(f, ")"),
            Token::OpenCurlyBrace => write!(f, "{{"),
            Token::CloseCurlyBrace => write!(f, "}}"),
            Token::BitwiseOr => write!(f, "|"),
            Token::BitwiseXor => write!(f, "^"),
            Token::Or => write!(f, "||"),
            Token::BitwiseAnd => write!(f, "&"),
            Token::BitwiseNot => write!(f, "~"),
            Token::And => write!(f, "&&"),
            Token::Add => write!(f, "+"),
            Token::Subtract => write!(f, "-"),
            Token::Mul => write!(f, "*"),
            Token::Power => write!(f, "**"),
            Token::Divide => write!(f, "/"),
            Token::Modulo => write!(f, "%"),
            Token::Equal => write!(f, "=="),
            Token::Assign => write!(f, "="),
            Token::NotEqual => write!(f, "!="),
            Token::Not => write!(f, "!"),
            Token::ShiftLeft => write!(f, "<<"),
            Token::More => write!(f, ">"),
            Token::MoreEqual => write!(f, ">="),
            Token::Member => write!(f, "."),
            Token::Colon => write!(f, ":"),
            Token::OpenBracket => write!(f, "["),
            Token::CloseBracket => write!(f, "]"),
            Token::ShiftRight => write!(f, "<<"),
            Token::Less => write!(f, "<"),
            Token::LessEqual => write!(f, "<="),
            Token::String => write!(f, "string"),
            Token::Function => write!(f, "function"),
            Token::Pragma => write!(f, "pragma"),
            Token::Import => write!(f, "import"),
            Token::Type => write!(f, "type"),
            Token::Constant => write!(f, "const"),
            Token::Do => write!(f, "do"),
            Token::Continue => write!(f, "continue"),
            Token::Break => write!(f, "break"),
            Token::Return => write!(f, "return"),
            Token::True => write!(f, "true"),
            Token::False => write!(f, "false"),
            Token::Else => write!(f, "else"),
            Token::For => write!(f, "for"),
            Token::While => write!(f, "while"),
            Token::If => write!(f, "if"),
            Token::As => write!(f, "as"),
            Token::Assembly => write!(f, "assembly"),
            Token::Formula => write!(f, "formula"),
            Token::PeirceArrow => write!(f, "-->"),
            Token::Model => write!(f, "model"),
            Token::State => write!(f, "state"),
            Token::Start => write!(f, "start"),
            Token::Reference => write!(f, "ref"),
            Token::Template => write!(f, "template"),
            Token::Condition => write!(f, "cond"),
            Token::Port => write!(f, "port"),
            Token::Variable => write!(f, "var"),
            Token::Next => write!(f, "next"),
            Token::Extern => write!(f, "extern"),
        }
    }
}

/// Custom Solidity lexer.
///
/// # Examples
///
/// ```
/// use but_grammar::lexer::{Lexer, Token};
///
/// let source = "int number = 0;";
/// let mut comments = Vec::new();
/// let mut errors = Vec::new();
/// let mut lexer = Lexer::new(source, 0, &mut comments, &mut errors);
///
/// let mut next_token = || lexer.next().map(|(_, token, _)| token);
/// assert_eq!(next_token(), Some(Token::Identifier("int")));
/// assert_eq!(next_token(), Some(Token::Identifier("number")));
/// assert_eq!(next_token(), Some(Token::Assign));
/// assert_eq!(next_token(), Some(Token::Number(0i64)));
/// assert_eq!(next_token(), Some(Token::Semicolon));
/// assert_eq!(next_token(), None);
/// assert!(errors.is_empty());
/// assert!(comments.is_empty());
/// ```
#[derive(Debug)]
pub struct Lexer<'input> {
    input: &'input str,
    chars: PeekNth<CharIndices<'input>>,
    comments: &'input mut Vec<Comment>,
    file_no: u64,
    last_tokens: [Option<Token<'input>>; 2],
    /// The mutable reference to the error vector.
    pub errors: &'input mut Vec<LexicalError>,
}

/// An error thrown by [Lexer].
#[derive(Debug, Clone, PartialEq, Eq, Error)]
#[allow(missing_docs)]
pub enum LexicalError {
    #[error("end of file found in comment")]
    EndOfFileInComment(Location),

    #[error("end of file found in string literal")]
    EndOfFileInString(Location),

    #[error("end of file found in hex literal string")]
    EndOfFileInHex(Location),

    #[error("missing number")]
    MissingNumber(Location),

    #[error("invalid character '{1}' in hex literal string")]
    InvalidCharacterInHexLiteral(Location, char),

    #[error("unrecognised token '{1}'")]
    UnrecognisedToken(Location, String),

    #[error("missing exponent")]
    MissingExponent(Location),

    #[error("'{1}' found where 'from' expected")]
    ExpectedFrom(Location, String),
}

impl LexicalError {
    pub fn loc(&self) -> Location {
        match self {
            LexicalError::EndOfFileInComment(loc) => loc.clone(),
            LexicalError::EndOfFileInString(loc) => loc.clone(),
            LexicalError::EndOfFileInHex(loc) => loc.clone(),
            LexicalError::MissingNumber(loc) => loc.clone(),
            LexicalError::InvalidCharacterInHexLiteral(loc, _) => loc.clone(),
            LexicalError::UnrecognisedToken(loc, _) => loc.clone(),
            LexicalError::MissingExponent(loc) => loc.clone(),
            LexicalError::ExpectedFrom(loc, _) => loc.clone(),
        }
    }
}

/// Returns whether `word` is a keyword.
pub fn is_keyword(word: &str) -> bool {
    KEYWORDS.contains_key(word)
}

static KEYWORDS: phf::Map<&'static str, Token> = phf_map! {
    "break" => Token::Break,
    "const" => Token::Constant,
    "continue" => Token::Continue,
    "do" => Token::Do,
    "else" => Token::Else,
    "false" => Token::False,
    "for" => Token::For,
    "fn" => Token::Function,
    "if" => Token::If,
    "import" => Token::Import,
    "return" => Token::Return,
    "string" => Token::String,
    "true" => Token::True,
    "type" => Token::Type,
    "while" => Token::While,
    "as" => Token::As,
    "assembly" => Token::Assembly,
    "formula" => Token::Formula,
    "port" => Token::Port,
    "model" => Token::Model,
    "state" => Token::State,
    "start" => Token::Start,
    "ref" => Token::Reference,
    "template" => Token::Template,
    "cond" => Token::Condition,
    "var" => Token::Variable,
    "next" => Token::Next,
};

impl<'input> Lexer<'input> {
    /// Instantiates a new Lexer.
    ///
    /// # Examples
    ///
    /// ```
    /// use but_grammar::lexer::Lexer;
    ///
    /// let source = "let number: uint256 = 0;";
    /// let mut comments = Vec::new();
    /// let mut errors = Vec::new();
    /// let mut lexer = Lexer::new(source, 0, &mut comments, &mut errors);
    /// ```
    pub fn new(
        input: &'input str,
        file_no: u64,
        comments: &'input mut Vec<Comment>,
        errors: &'input mut Vec<LexicalError>,
    ) -> Self {
        Lexer {
            input,
            chars: peek_nth(input.char_indices()),
            comments,
            file_no,
            last_tokens: [None, None],
            errors,
        }
    }

    fn parse_number(&mut self, mut start: usize, ch: char) -> Result<'input> {
        let mut is_rational = false;
        let mut is_minus = false;
        if ch == '0' {
            if let Some((_, 'x')) = self.chars.peek() {
                // hex number
                self.chars.next();

                let mut end = match self.chars.next() {
                    Some((end, ch)) if ch.is_ascii_hexdigit() => end,
                    Some((..)) => {
                        return Err(LexicalError::MissingNumber(Location::Source(
                            self.file_no,
                            start,
                            start + 1,
                        )));
                    }
                    None => {
                        return Err(LexicalError::EndOfFileInHex(Location::Source(
                            self.file_no,
                            start,
                            self.input.len(),
                        )));
                    }
                };

                while let Some((i, ch)) = self.chars.peek() {
                    if !ch.is_ascii_hexdigit() && *ch != '_' {
                        break;
                    }
                    end = *i;
                    self.chars.next();
                }

                let hex = &self.input[start + 2..=end];
                return Ok((
                    start,
                    Token::Number(i64::from_str_radix(hex, 16).unwrap()),
                    end + 1,
                ));
            }
        }

        if ch == '.' {
            is_rational = true;
            start -= 1;
        }
        if ch == '-' {
            is_minus = true;
        }

        let mut end = start;
        while let Some((i, ch)) = self.chars.peek() {
            if !ch.is_ascii_digit() && *ch != '_' {
                break;
            }
            end = *i;
            self.chars.next();
        }
        let mut rational_end = end;
        let mut end_before_rational = end + 1;
        let mut rational_start = end;
        if is_rational {
            end_before_rational = start;
            rational_start = start + 1;
        }

        if let Some((_, '.')) = self.chars.peek() {
            if let Some((i, ch)) = self.chars.peek_nth(1) {
                if ch.is_ascii_digit() && !is_rational {
                    rational_start = *i;
                    rational_end = *i;
                    is_rational = true;
                    self.chars.next(); // advance over '.'
                    while let Some((i, ch)) = self.chars.peek() {
                        if !ch.is_ascii_digit() && *ch != '_' {
                            break;
                        }
                        rational_end = *i;
                        end = *i;
                        self.chars.next();
                    }
                }
            }
        }

        let old_end = end;
        let mut exp_start = end + 1;

        if let Some((i, 'e' | 'E')) = self.chars.peek() {
            exp_start = *i + 1;
            self.chars.next();
            // Negative exponent
            while matches!(self.chars.peek(), Some((_, '-'))) {
                self.chars.next();
            }
            while let Some((i, ch)) = self.chars.peek() {
                if !ch.is_ascii_digit() && *ch != '_' {
                    break;
                }
                end = *i;
                self.chars.next();
            }

            if exp_start > end {
                return Err(LexicalError::MissingExponent(Location::Source(
                    self.file_no,
                    start,
                    self.input.len(),
                )));
            }
        }

        if is_rational {
            let _integer = &self.input[start..end_before_rational];
            let _fraction = &self.input[rational_start..=rational_end];
            let _exp = &self.input[exp_start..=end];

            return Ok((
                start,
                Token::RationalNumber(&self.input[start..=rational_end], is_minus),
                end + 1,
            ));
        }

        let n = &self.input[start..=old_end];
        let _exp = &self.input[exp_start..=end];

        let mut n = i64::from_str(n).unwrap();
        if is_minus {
            n = -n;
        }
        Ok((start, Token::Number(n), end + 1))
    }

    fn string(
        &mut self,
        unicode: bool,
        token_start: usize,
        string_start: usize,
        quote_char: char,
    ) -> Result<'input> {
        let mut end;

        let mut last_was_escape = false;

        loop {
            if let Some((i, ch)) = self.chars.next() {
                end = i;
                if !last_was_escape {
                    if ch == quote_char {
                        break;
                    }
                    last_was_escape = ch == '\\';
                } else {
                    last_was_escape = false;
                }
            } else {
                return Err(LexicalError::EndOfFileInString(Location::Source(
                    self.file_no,
                    token_start,
                    self.input.len(),
                )));
            }
        }

        Ok((
            token_start,
            Token::StringLiteral(unicode, &self.input[string_start..end]),
            end + 1,
        ))
    }

    fn next(&mut self) -> Option<Spanned<'input>> {
        'toplevel: loop {
            match self.chars.next() {
                Some((start, ch)) if ch == '_' || ch == '$' || UnicodeXID::is_xid_start(ch) => {
                    let (id, end) = self.match_identifier(start);

                    if id == "unicode" {
                        match self.chars.peek() {
                            Some((_, quote_char @ '"')) | Some((_, quote_char @ '\'')) => {
                                let quote_char = *quote_char;

                                self.chars.next();
                                let str_res = self.string(true, start, start + 8, quote_char);
                                match str_res {
                                    Err(lex_err) => self.errors.push(lex_err),
                                    Ok(val) => return Some(val),
                                }
                            }
                            _ => (),
                        }
                    }

                    return if let Some(w) = KEYWORDS.get(id) {
                        Some((start, *w, end))
                    } else {
                        Some((start, Token::Identifier(id), end))
                    };
                }
                Some((start, quote_char @ '"')) | Some((start, quote_char @ '\'')) => {
                    let str_res = self.string(false, start, start + 1, quote_char);
                    match str_res {
                        Err(lex_err) => self.errors.push(lex_err),
                        Ok(val) => return Some(val),
                    }
                }
                Some((start, '/')) => {
                    match self.chars.peek() {
                        Some((_, '/')) => {
                            // line comment
                            self.chars.next();

                            let mut newline = false;

                            let doc_comment = match self.chars.next() {
                                Some((_, '/')) => {
                                    // ///(/)+ is still a line comment
                                    !matches!(self.chars.peek(), Some((_, '/')))
                                }
                                Some((_, ch)) if ch == '\n' || ch == '\r' => {
                                    newline = true;
                                    false
                                }
                                _ => false,
                            };

                            let mut last = start + 3;

                            if !newline {
                                loop {
                                    match self.chars.next() {
                                        None => {
                                            last = self.input.len();
                                            break;
                                        }
                                        Some((offset, '\n' | '\r')) => {
                                            last = offset;
                                            break;
                                        }
                                        Some(_) => (),
                                    }
                                }
                            }

                            if doc_comment {
                                self.comments.push(Comment::DocLine(
                                    Location::Source(self.file_no, start, last),
                                    self.input[start..last].to_owned(),
                                ));
                            } else {
                                self.comments.push(Comment::Line(
                                    Location::Source(self.file_no, start, last),
                                    self.input[start..last].to_owned(),
                                ));
                            }
                        }
                        _ => {
                            return Some((start, Token::Divide, start + 1));
                        }
                    }
                }
                Some((start, ch)) if ch.is_ascii_digit() => {
                    let parse_result = self.parse_number(start, ch);
                    match parse_result {
                        Err(lex_err) => {
                            self.errors.push(lex_err.clone());
                            if matches!(lex_err, LexicalError::EndOfFileInHex(_)) {
                                return None;
                            }
                        }
                        Ok(parse_result) => return Some(parse_result),
                    }
                }
                Some((i, '#')) => return Some((i, Token::Sharp, i + 1)),
                Some((i, ';')) => return Some((i, Token::Semicolon, i + 1)),
                Some((i, ',')) => return Some((i, Token::Comma, i + 1)),
                Some((i, '(')) => return Some((i, Token::OpenParenthesis, i + 1)),
                Some((i, ')')) => return Some((i, Token::CloseParenthesis, i + 1)),
                Some((i, '{')) => return Some((i, Token::OpenCurlyBrace, i + 1)),
                Some((i, '}')) => return Some((i, Token::CloseCurlyBrace, i + 1)),
                Some((i, '=')) => {
                    return match self.chars.peek() {
                        Some((_, '=')) => {
                            self.chars.next();
                            Some((i, Token::Equal, i + 2))
                        }
                        _ => Some((i, Token::Assign, i + 1)),
                    };
                }
                Some((i, '!')) => {
                    return if let Some((_, '=')) = self.chars.peek() {
                        self.chars.next();
                        Some((i, Token::NotEqual, i + 2))
                    } else {
                        Some((i, Token::Not, i + 1))
                    };
                }
                Some((i, '|')) => {
                    return match self.chars.peek() {
                        Some((_, '|')) => {
                            self.chars.next();
                            Some((i, Token::Or, i + 2))
                        }
                        _ => Some((i, Token::BitwiseOr, i + 1)),
                    };
                }
                Some((i, '^')) => {
                    return match self.chars.peek() {
                        _ => Some((i, Token::BitwiseXor, i + 1)),
                    };
                }
                Some((i, '&')) => {
                    return match self.chars.peek() {
                        Some((_, '&')) => {
                            self.chars.next();
                            Some((i, Token::And, i + 2))
                        }
                        _ => Some((i, Token::BitwiseAnd, i + 1)),
                    };
                }
                Some((i, '+')) => {
                    return match self.chars.peek() {
                        _ => Some((i, Token::Add, i + 1)),
                    };
                }
                Some((i, '-')) => {
                    return match self.chars.peek() {
                        Some((_, other)) if other.is_ascii_digit() => {
                            return match self.parse_number(i + 1, '-') {
                                Err(lex_error) => {
                                    self.errors.push(lex_error);
                                    None
                                }
                                Ok(parse_result) => Some(parse_result),
                            };
                        }
                        _ => Some((i, Token::Subtract, i + 1)),
                    };
                }
                Some((i, '*')) => {
                    return match self.chars.peek() {
                        Some((_, '*')) => {
                            self.chars.next();
                            Some((i, Token::Power, i + 2))
                        }
                        _ => Some((i, Token::Mul, i + 1)),
                    };
                }
                Some((i, '%')) => {
                    return match self.chars.peek() {
                        _ => Some((i, Token::Modulo, i + 1)),
                    };
                }
                Some((i, '<')) => {
                    return match self.chars.peek() {
                        Some((_, '<')) => {
                            self.chars.next();
                            Some((i, Token::ShiftLeft, i + 2))
                        }
                        Some((_, '=')) => {
                            self.chars.next();
                            Some((i, Token::LessEqual, i + 2))
                        }
                        _ => Some((i, Token::Less, i + 1)),
                    };
                }
                Some((i, '>')) => {
                    return match self.chars.peek() {
                        Some((_, '>')) => {
                            self.chars.next();
                            Some((i, Token::ShiftRight, i + 2))
                        }
                        Some((_, '=')) => {
                            self.chars.next();
                            Some((i, Token::MoreEqual, i + 2))
                        }
                        _ => Some((i, Token::More, i + 1)),
                    };
                }
                Some((i, '.')) => {
                    return Some((i, Token::Member, i + 1));
                }
                Some((i, '[')) => return Some((i, Token::OpenBracket, i + 1)),
                Some((i, ']')) => return Some((i, Token::CloseBracket, i + 1)),
                Some((i, ':')) => return Some((i, Token::Colon, i + 1)),
                Some((i, '~')) => return Some((i, Token::BitwiseNot, i + 1)),
                Some((_, ch)) if ch.is_whitespace() => (),
                Some((start, _)) => {
                    let mut end;

                    loop {
                        if let Some((i, ch)) = self.chars.next() {
                            end = i;

                            if ch.is_whitespace() {
                                break;
                            }
                        } else {
                            end = self.input.len();
                            break;
                        }
                    }

                    self.errors.push(LexicalError::UnrecognisedToken(
                        Location::Source(self.file_no, start, end),
                        self.input[start..end].to_owned(),
                    ));
                }
                None => return None, // End of file
            }
        }
    }

    /// Next token is pragma value. Return it
    fn pragma_value(&mut self) -> Option<Spanned<'input>> {
        // special parser for pragma solidity >=0.4.22 <0.7.0;
        let mut start = None;
        let mut end = 0;

        // solc will include anything upto the next semicolon, whitespace
        // trimmed on left and right
        loop {
            match self.chars.peek() {
                Some((_, ';')) | None => {
                    return if let Some(start) = start {
                        Some((
                            start,
                            Token::StringLiteral(false, &self.input[start..end]),
                            end,
                        ))
                    } else {
                        self.next()
                    };
                }
                Some((_, ch)) if ch.is_whitespace() => {
                    self.chars.next();
                }
                Some((i, _)) => {
                    if start.is_none() {
                        start = Some(*i);
                    }
                    self.chars.next();

                    // end should point to the byte _after_ the character
                    end = match self.chars.peek() {
                        Some((i, _)) => *i,
                        None => self.input.len(),
                    }
                }
            }
        }
    }

    fn match_identifier(&mut self, start: usize) -> (&'input str, usize) {
        let end;
        loop {
            if let Some((i, ch)) = self.chars.peek() {
                if !UnicodeXID::is_xid_continue(*ch) && *ch != '$' {
                    end = *i;
                    break;
                }
                self.chars.next();
            } else {
                end = self.input.len();
                break;
            }
        }

        (&self.input[start..end], end)
    }
}

impl<'input> Iterator for Lexer<'input> {
    type Item = Spanned<'input>;

    fn next(&mut self) -> Option<Self::Item> {
        let token = if let [Some(Token::Pragma), Some(Token::Identifier(_))] = self.last_tokens {
            self.pragma_value()
        } else {
            self.next()
        };

        self.last_tokens = [
            self.last_tokens[1],
            match token {
                Some((_, n, _)) => Some(n),
                _ => None,
            },
        ];

        token
    }
}
