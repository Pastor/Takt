//! Лексический анализатор языка BuT.
//!
//! Этот модуль реализует ручной лексер для языка BuT на базе итератора
//! [`CharIndices`] с односимвольным предпросмотром через [`PeekNth`].
//!
//! ## Основные типы
//!
//! - [`Token`] — перечисление всех лексем языка (ключевые слова, операторы,
//!   литералы, идентификаторы и прагмы).
//! - [`Lexer`] — итератор, преобразующий исходный текст в поток [`Spanned`]-токенов.
//! - [`LexicalError`] — ошибки, обнаруживаемые на этапе лексического анализа.
//!
//! ## Особенности
//!
//! - Ключевые слова (`if`, `while`, `model`, `state`, …) определены в статической
//!   таблице [`KEYWORDS`] и распознаются при сканировании идентификаторов.
//! - Числовые литералы поддерживают десятичную и шестнадцатеричную (`0x`) запись,
//!   а также числа с плавающей точкой и экспоненту.
//! - Строковые литералы заключаются в двойные кавычки `"..."` и допускают
//!   конкатенацию смежных строк.
//! - Комментарии: однострочные `//`, документационные `///` и блочные `/* */`.

use std::str::FromStr;
use std::{fmt, str::CharIndices};

use itertools::{PeekNth, peek_nth};
use phf::phf_map;
use thiserror::Error;
use unicode_xid::UnicodeXID;

use crate::ast::Comment;
use crate::diagnostics::Location;

/// Тип «токен с позицией»: `(начало, токен, конец)`.
pub type Spanned<'a> = (usize, Token<'a>, usize);

/// Специализированный `Result` для операций лексера.
pub type Result<'a, T = Spanned<'a>, E = LexicalError> = std::result::Result<T, E>;

/// Все лексемы (токены) языка BuT.
///
/// Каждый вариант соответствует одному терминальному символу грамматики.
#[derive(Copy, Clone, PartialEq, Eq, Debug)]
#[allow(missing_docs)]
pub enum Token<'input> {
    /// Произвольный идентификатор, не являющийся ключевым словом.
    Identifier(&'input str),
    /// Строковый литерал: `(unicode, содержимое)`.
    StringLiteral(bool, &'input str),
    /// Адресный литерал (числовой адрес порта).
    AddressLiteral(&'input str),
    /// Целочисленный литерал.
    Number(i64),
    /// Рациональный (плавающий) литерал: `(строка, отрицательный)`.
    RationalNumber(&'input str, bool),
    /// Оператор деления `/`.
    Divide,
    /// Ключевое слово `fn`.
    Function,
    /// Ключевое слово `pragma`.
    Pragma,
    /// Ключевое слово `import`.
    Import,
    /// Ключевое слово `type`.
    Type,
    /// Ключевое слово `loop`.
    Loop,
    /// Ключевое слово `continue`.
    Continue,
    /// Ключевое слово `break`.
    Break,
    /// Ключевое слово `return`.
    Return,
    /// Ключевое слово `string`.
    String,
    /// Символ `#`.
    Sharp,
    /// Символ `;`.
    Semicolon,
    /// Символ `,`.
    Comma,
    /// Символ `(`.
    OpenParenthesis,
    /// Символ `)`.
    CloseParenthesis,
    /// Символ `{`.
    OpenCurlyBrace,
    /// Символ `}`.
    CloseCurlyBrace,

    /// Оператор побитового ИЛИ `|`.
    BitwiseOr,
    /// Оператор побитового исключающего ИЛИ `^`.
    BitwiseXor,
    /// Логический оператор ИЛИ `||`.
    Or,

    /// Оператор побитового И `&`.
    BitwiseAnd,
    /// Оператор побитового НЕ `~`.
    BitwiseNot,
    /// Логический оператор И `&&`.
    And,
    /// Оператор сложения `+`.
    Add,
    /// Оператор вычитания `-`.
    Subtract,
    /// Оператор умножения `*`.
    Mul,
    /// Оператор возведения в степень `**`.
    Power,
    /// Оператор взятия остатка `%`.
    Modulo,

    /// Оператор равенства `==`.
    Equal,
    /// Оператор присваивания `=`.
    Assign,

    /// Оператор неравенства `!=`.
    NotEqual,
    /// Логическое НЕ `!`.
    Not,

    /// Логическое значение `true`.
    True,
    /// Логическое значение `false`.
    False,
    /// Ключевое слово `else`.
    Else,
    /// Ключевое слово `for`.
    For,
    /// Ключевое слово `if`.
    If,

    /// Оператор сдвига вправо `>>`.
    ShiftRight,
    /// Оператор «меньше» `<`.
    Less,
    /// Оператор «меньше или равно» `<=`.
    LessEqual,

    /// Оператор сдвига влево `<<`.
    ShiftLeft,
    /// Оператор «больше» `>`.
    More,
    /// Оператор «больше или равно» `>=`.
    MoreEqual,

    /// Оператор доступа к члену `.`.
    Member,
    /// Двоеточие `:`.
    Colon,
    /// Символ вопроса `?` (тернарный оператор).
    Question,
    /// Символ `[`.
    OpenBracket,
    /// Символ `]`.
    CloseBracket,

    /// Ключевое слово `as` (приведение типов).
    As,

    /// Ключевое слово `assembly`.
    Assembly,
    /// Ключевое слово `formula`.
    Formula,

    /// Ключевое слово `const`.
    Constant,
    /// Ключевое слово `port`.
    Port,

    /// Стрелка `-->` (не используется в текущей грамматике).
    PeirceArrow,

    /// Ключевое слово `model`.
    Model,
    /// Ключевое слово `state`.
    State,
    /// Ключевое слово `start`.
    Start,
    /// Ключевое слово `ref`.
    Reference,
    /// Ключевое слово `template`.
    Template,
    /// Ключевое слово `cond`.
    Condition,
    /// Ключевое слово `var`.
    Variable,
    /// Ключевое слово `next`.
    Next,
    /// Ключевое слово `extern`.
    Extern,
    /// Ключевое слово `enum`.
    Enum,
    /// Ключевое слово `struct`.
    Struct,
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
            Token::Question => write!(f, "?"),
            Token::OpenBracket => write!(f, "["),
            Token::CloseBracket => write!(f, "]"),
            Token::ShiftRight => write!(f, ">>"),
            Token::Less => write!(f, "<"),
            Token::LessEqual => write!(f, "<="),
            Token::String => write!(f, "string"),
            Token::Function => write!(f, "fn"),
            Token::Pragma => write!(f, "pragma"),
            Token::Import => write!(f, "import"),
            Token::Type => write!(f, "type"),
            Token::Constant => write!(f, "const"),
            Token::Loop => write!(f, "loop"),
            Token::Continue => write!(f, "continue"),
            Token::Break => write!(f, "break"),
            Token::Return => write!(f, "return"),
            Token::True => write!(f, "true"),
            Token::False => write!(f, "false"),
            Token::Else => write!(f, "else"),
            Token::For => write!(f, "for"),
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
            Token::Enum => write!(f, "enum"),
            Token::Struct => write!(f, "struct"),
        }
    }
}

/// Лексический анализатор языка BuT.
///
/// Преобразует строку исходного кода в последовательность токенов ([`Token`]).
/// Для опережающего просмотра используется [`PeekNth`].
///
/// Комментарии (строчные `//`, документационные `///` и блочные `/* */`) собираются отдельно
/// в вектор [`Comment`] и не включаются в поток токенов.
///
/// # Примеры
///
/// ```
/// use grammar::parser::lexer::{Lexer, Token};
///
/// let source = "var x = 42;";
/// let mut comments = Vec::new();
/// let mut errors = Vec::new();
/// let mut lexer = Lexer::new(source, 0, &mut comments, &mut errors);
///
/// let mut next_token = || lexer.next().map(|(_, token, _)| token);
/// assert_eq!(next_token(), Some(Token::Variable));
/// assert_eq!(next_token(), Some(Token::Identifier("x")));
/// assert_eq!(next_token(), Some(Token::Assign));
/// assert_eq!(next_token(), Some(Token::Number(42i64)));
/// assert_eq!(next_token(), Some(Token::Semicolon));
/// assert_eq!(next_token(), None);
/// assert!(errors.is_empty());
/// assert!(comments.is_empty());
/// ```
#[derive(Debug)]
pub struct Lexer<'input> {
    /// Полный исходный текст.
    input: &'input str,
    /// Итератор по символам с позициями (поддерживает опережающий просмотр).
    chars: PeekNth<CharIndices<'input>>,
    /// Вектор для сохранения найденных комментариев.
    comments: &'input mut Vec<Comment>,
    /// Номер файла (используется в [`Location`]).
    file_no: u64,
    /// Последние два токена (для обработки `pragma`).
    last_tokens: [Option<Token<'input>>; 2],
    /// Вектор лексических ошибок, обнаруженных в ходе анализа.
    pub errors: &'input mut Vec<LexicalError>,
}

/// Ошибка лексического анализатора.
#[derive(Debug, Clone, PartialEq, Eq, Error)]
#[allow(missing_docs)]
pub enum LexicalError {
    /// Неожиданный конец файла внутри блочного комментария.
    #[error("неожиданный конец файла внутри комментария")]
    EndOfFileInComment(Location),

    /// Неожиданный конец файла внутри строкового литерала.
    #[error("неожиданный конец файла внутри строкового литерала")]
    EndOfFileInString(Location),

    /// Неожиданный конец файла внутри шестнадцатеричного литерала.
    #[error("неожиданный конец файла внутри шестнадцатеричного литерала")]
    EndOfFileInHex(Location),

    /// Отсутствуют цифры после `0x`.
    #[error("отсутствует число после '0x'")]
    MissingNumber(Location),

    /// Недопустимый символ в шестнадцатеричном литерале.
    #[error("недопустимый символ '{1}' в шестнадцатеричном литерале")]
    InvalidCharacterInHexLiteral(Location, char),

    /// Неизвестный токен.
    #[error("нераспознанный токен '{1}'")]
    UnrecognisedToken(Location, String),

    /// Отсутствует показатель степени после `e`/`E`.
    #[error("отсутствует показатель степени")]
    MissingExponent(Location),

    /// Ожидалось ключевое слово `from`, но встретилось другое слово.
    #[error("ожидалось ключевое слово 'from', но найдено '{1}'")]
    ExpectedFrom(Location, String),
}

impl LexicalError {
    /// Возвращает местоположение в исходном тексте, где возникла ошибка.
    pub fn loc(&self) -> Location {
        match self {
            LexicalError::EndOfFileInComment(loc) => *loc,
            LexicalError::EndOfFileInString(loc) => *loc,
            LexicalError::EndOfFileInHex(loc) => *loc,
            LexicalError::MissingNumber(loc) => *loc,
            LexicalError::InvalidCharacterInHexLiteral(loc, _) => *loc,
            LexicalError::UnrecognisedToken(loc, _) => *loc,
            LexicalError::MissingExponent(loc) => *loc,
            LexicalError::ExpectedFrom(loc, _) => *loc,
        }
    }

    /// Возвращает код ошибки в формате `LE-NNN`.
    pub fn code(&self) -> &'static str {
        match self {
            LexicalError::EndOfFileInComment(_) => "LE-001",
            LexicalError::EndOfFileInString(_) => "LE-002",
            LexicalError::EndOfFileInHex(_) => "LE-003",
            LexicalError::MissingNumber(_) => "LE-004",
            LexicalError::InvalidCharacterInHexLiteral(_, _) => "LE-005",
            LexicalError::UnrecognisedToken(_, _) => "LE-006",
            LexicalError::MissingExponent(_) => "LE-007",
            LexicalError::ExpectedFrom(_, _) => "LE-008",
        }
    }
}

/// Возвращает `true`, если переданная строка является ключевым словом BuT.
pub fn is_keyword(word: &str) -> bool {
    KEYWORDS.contains_key(word)
}

/// Статическая таблица ключевых слов языка BuT.
///
/// Отображает строку ключевого слова на соответствующий [`Token`].
static KEYWORDS: phf::Map<&'static str, Token> = phf_map! {
    "break"    => Token::Break,
    "const"    => Token::Constant,
    "continue" => Token::Continue,
    "else"     => Token::Else,
    "false"    => Token::False,
    "for"      => Token::For,
    "fn"       => Token::Function,
    "if"       => Token::If,
    "import"   => Token::Import,
    "loop"     => Token::Loop,
    "return"   => Token::Return,
    "string"   => Token::String,
    "true"     => Token::True,
    "type"     => Token::Type,
    "as"       => Token::As,
    "assembly" => Token::Assembly,
    "formula"  => Token::Formula,
    "port"     => Token::Port,
    "model"    => Token::Model,
    "state"    => Token::State,
    "start"    => Token::Start,
    "ref"      => Token::Reference,
    "template" => Token::Template,
    "cond"     => Token::Condition,
    "var"      => Token::Variable,
    "next"     => Token::Next,
    "extern"   => Token::Extern,
    "enum"     => Token::Enum,
    "struct"   => Token::Struct,
};

impl<'input> Lexer<'input> {
    /// Создаёт новый экземпляр лексического анализатора.
    ///
    /// # Параметры
    ///
    /// - `input` — строка исходного кода для анализа.
    /// - `file_no` — числовой идентификатор файла (используется в [`Location`]).
    /// - `comments` — вектор для сохранения найденных комментариев.
    /// - `errors` — вектор для накопления лексических ошибок.
    ///
    /// # Примеры
    ///
    /// ```
    /// use grammar::parser::lexer::Lexer;
    ///
    /// let source = "model M { start S; }";
    /// let mut comments = Vec::new();
    /// let mut errors = Vec::new();
    /// let _lexer = Lexer::new(source, 0, &mut comments, &mut errors);
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

    /// Читает числовой литерал начиная с символа `ch` в позиции `start`.
    ///
    /// Поддерживает десятичные, шестнадцатеричные (`0x`), рациональные
    /// (`3.14`) и числа с показателем степени (`1e10`).
    fn parse_number(&mut self, mut start: usize, ch: char) -> Result<'input> {
        let mut is_rational = false;
        let mut is_minus = false;
        if ch == '0'
            && let Some((_, 'x')) = self.chars.peek()
        {
            // Шестнадцатеричный литерал: 0x...
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

                // Удаляем разделители `_` перед разбором hex-числа
                let hex_raw = &self.input[start + 2..=end];
                let hex: String = hex_raw.chars().filter(|&c| c != '_').collect();
                let hex_val = i64::from_str_radix(&hex, 16).unwrap();

                // Проверяем, является ли это адресным литералом `0xNNNN:bit`
                // (токен AddressLiteral, чтобы избежать LR(1)-конфликта с тернарным `?:`)
                if matches!(self.chars.peek(), Some((_, ':')))
                    && matches!(self.chars.peek_nth(1), Some((_, '0'..='9')))
                {
                    self.chars.next(); // потребляем ':'
                    let mut bit_end = end + 1;
                    while let Some((i, ch)) = self.chars.peek() {
                        if ch.is_ascii_digit() {
                            bit_end = *i;
                            self.chars.next();
                        } else {
                            break;
                        }
                    }
                    return Ok((
                        start,
                        Token::AddressLiteral(&self.input[start..=bit_end]),
                        bit_end + 1,
                    ));
                }

                return Ok((start, Token::Number(hex_val), end + 1));
        }

        if ch == '.' {
            // Начало дробной части (например, `.5`)
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

        if let Some((_, '.')) = self.chars.peek()
            && let Some((i, ch)) = self.chars.peek_nth(1)
            && ch.is_ascii_digit()
            && !is_rational
        {
            // Дробная часть: 3.14
            rational_start = *i;
            rational_end = *i;
            is_rational = true;
            self.chars.next(); // пропускаем '.'
            while let Some((i, ch)) = self.chars.peek() {
                if !ch.is_ascii_digit() && *ch != '_' {
                    break;
                }
                rational_end = *i;
                end = *i;
                self.chars.next();
            }
        }

        let old_end = end;
        let mut exp_start = end + 1;

        if let Some((i, 'e' | 'E')) = self.chars.peek() {
            // Показатель степени: 1e10, 2.5E-3
            exp_start = *i + 1;
            self.chars.next();
            // Опциональный знак минус перед показателем
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

        // Удаляем разделители `_` перед разбором десятичного числа
        let n_raw = &self.input[start..=old_end];
        let n_clean: String = n_raw.chars().filter(|&c| c != '_').collect();
        let _exp = &self.input[exp_start..=end];

        let mut n = i64::from_str(&n_clean).unwrap();
        if is_minus {
            n = -n;
        }
        Ok((start, Token::Number(n), end + 1))
    }

    /// Читает строковый литерал, заключённый в кавычки `quote_char`.
    ///
    /// - `unicode` — признак Unicode-строки (`unicode"..."`)
    /// - `token_start` — позиция начала токена (включая открывающую кавычку)
    /// - `string_start` — позиция первого символа содержимого строки
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

    /// Основной цикл токенизации.
    ///
    /// Возвращает следующий токен или `None` при достижении конца файла.
    fn next(&mut self) -> Option<Spanned<'input>> {
        loop {
            match self.chars.next() {
                // Идентификатор или ключевое слово
                Some((start, ch)) if ch == '_' || ch == '$' || UnicodeXID::is_xid_start(ch) => {
                    let (id, end) = self.match_identifier(start);

                    // Специальная обработка Unicode-строк: unicode"..."
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
                // Строковый литерал в одинарных или двойных кавычках
                Some((start, quote_char @ '"')) | Some((start, quote_char @ '\'')) => {
                    let str_res = self.string(false, start, start + 1, quote_char);
                    match str_res {
                        Err(lex_err) => self.errors.push(lex_err),
                        Ok(val) => return Some(val),
                    }
                }
                // Слэш: деление `/`, начало строчного `//` или блочного `/* */` комментария
                Some((start, '/')) => {
                    match self.chars.peek() {
                        Some((_, '/')) => {
                            // Строчный комментарий
                            self.chars.next();

                            let mut newline = false;

                            let doc_comment = match self.chars.next() {
                                Some((_, '/')) => {
                                    // `////` и далее — не документационный, а обычный
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
                                // Документационный комментарий `///`
                                self.comments.push(Comment::DocLine(
                                    Location::Source(self.file_no, start, last),
                                    self.input[start..last].to_owned(),
                                ));
                            } else {
                                // Обычный строчный комментарий `//`
                                self.comments.push(Comment::Line(
                                    Location::Source(self.file_no, start, last),
                                    self.input[start..last].to_owned(),
                                ));
                            }
                        }
                        Some((_, '*')) => {
                            // Блочный комментарий `/* ... */`
                            self.chars.next(); // потребляем `*`

                            // Сканируем до закрывающей пары `*/`; возвращаем позицию за `*/`
                            let last = 'scan: {
                                loop {
                                    match self.chars.next() {
                                        None => {
                                            // Неожиданный конец файла внутри комментария
                                            self.errors.push(LexicalError::EndOfFileInComment(
                                                Location::Source(
                                                    self.file_no,
                                                    start,
                                                    self.input.len(),
                                                ),
                                            ));
                                            return None;
                                        }
                                        Some((offset, '*')) => {
                                            if matches!(self.chars.peek(), Some((_, '/'))) {
                                                self.chars.next(); // потребляем `/`
                                                break 'scan offset + 2;
                                            }
                                        }
                                        _ => {}
                                    }
                                }
                            };

                            // Блочный комментарий не производит токены, только собирается
                            self.comments.push(Comment::Block(
                                Location::Source(self.file_no, start, last),
                                self.input[start..last].to_owned(),
                            ));
                        }
                        _ => {
                            return Some((start, Token::Divide, start + 1));
                        }
                    }
                }
                // Цифра — начало числового литерала
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
                    return Some((i, Token::BitwiseXor, i + 1));
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
                    return Some((i, Token::Add, i + 1));
                }
                Some((i, '-')) => {
                    return match self.chars.peek() {
                        Some((_, other)) if other.is_ascii_digit() => {
                            // Отрицательный числовой литерал
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
                            // Оператор возведения в степень `**`
                            self.chars.next();
                            Some((i, Token::Power, i + 2))
                        }
                        _ => Some((i, Token::Mul, i + 1)),
                    };
                }
                Some((i, '%')) => {
                    return Some((i, Token::Modulo, i + 1));
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
                Some((i, '?')) => return Some((i, Token::Question, i + 1)),
                Some((i, '~')) => return Some((i, Token::BitwiseNot, i + 1)),
                // Пробельные символы игнорируются
                Some((_, ch)) if ch.is_whitespace() => (),
                // Неизвестный символ — лексическая ошибка
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
                None => return None, // Конец файла
            }
        }
    }

    /// Читает значение после директивы `pragma` вплоть до `;`.
    ///
    /// Возвращает содержимое как строковый литерал, пробелы на концах обрезаются.
    fn pragma_value(&mut self) -> Option<Spanned<'input>> {
        // Аналог поведения solc: всё до следующей точки с запятой
        let mut start = None;
        let mut end = 0;

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

                    // end указывает на байт после текущего символа
                    end = match self.chars.peek() {
                        Some((i, _)) => *i,
                        None => self.input.len(),
                    }
                }
            }
        }
    }

    /// Читает идентификатор начиная с позиции `start`.
    ///
    /// Возвращает срез строки идентификатора и позицию его конца.
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
        // Если предыдущие два токена были `pragma <идентификатор>`,
        // следующий токен читается как значение pragma-директивы.
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
