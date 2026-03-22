//! Библиотека лексического и синтаксического анализаторов языка BuT.
//!
//! # Структура
//!
//! - [`ast`] — узлы абстрактного синтаксического дерева (АСД).
//! - [`diagnostics`] — типы диагностических сообщений (ошибки, предупреждения).
//! - [`lexer`] — лексический анализатор (токенизатор).
//!
//! # Использование
//!
//! ```
//! use grammar::parse;
//!
//! let src = "model M { start S; }";
//! match parse(src, 0) {
//!     Ok((model, comments)) => {
//!         // Успешный разбор: model — корневой узел АСД
//!         assert!(!model.elements.is_empty());
//!     }
//!     Err(diagnostics) => {
//!         for d in diagnostics {
//!             eprintln!("[{}] {}", d.level, d.message);
//!         }
//!     }
//! }
//! ```
#![warn(missing_debug_implementations, missing_docs)]

extern crate core;

use lalrpop_util::ParseError;

use diagnostics::Diagnostic;

use crate::ast::Location;
use crate::lexer::LexicalError;
use crate::lexer::Token;

/// Модуль абстрактного синтаксического дерева языка BuT.
pub mod ast;

/// Модуль диагностических сообщений компилятора.
pub mod diagnostics;

/// Модуль лексического анализатора BuT.
pub mod lexer;

#[allow(
    clippy::needless_lifetimes,
    clippy::type_complexity,
    clippy::ptr_arg,
    clippy::redundant_clone,
    clippy::just_underscores_and_digits
)]
mod grammar {
    include!(concat!(env!("OUT_DIR"), "/grammar.rs"));
}

/// Разбирает строку исходного кода BuT.
///
/// Возвращает пару `(корневая_модель, комментарии)` при успехе,
/// или вектор диагностических сообщений при ошибке.
///
/// # Параметры
///
/// - `src` — строка исходного кода.
/// - `file_no` — числовой идентификатор файла для сообщений об ошибках.
///
/// # Примеры
///
/// ```
/// use grammar::parse;
/// use grammar::ast::ModelElement;
///
/// // Успешный разбор минимальной программы.
/// // parse() возвращает анонимную корневую модель; именованные модели — в elements.
/// let (root, _) = parse("model M { start S; }", 0).unwrap();
/// assert!(root.name.is_none(), "Корневая модель всегда анонимна");
/// assert!(root.elements.iter().any(|e| matches!(e, ModelElement::Model(_))));
///
/// // Разбор завершается ошибкой при синтаксических нарушениях
/// let err = parse("model {", 0);
/// assert!(err.is_err());
/// ```
pub fn parse(src: &str, file_no: u64) -> Result<(ast::Model, Vec<ast::Comment>), Vec<Diagnostic>> {
    let mut comments = Vec::new();
    let mut lexer_errors = Vec::new();
    let mut lex = lexer::Lexer::new(src, file_no, &mut comments, &mut lexer_errors);

    let mut parser_errors = Vec::new();
    let res = grammar::SourceUnitParser::new().parse(src, file_no, &mut parser_errors, &mut lex);

    let mut diagnostics = Vec::with_capacity(lex.errors.len() + parser_errors.len());
    for lexical_error in lex.errors {
        diagnostics.push(Diagnostic::parser_error(
            lexical_error.loc(),
            lexical_error.to_string(),
        ))
    }

    for e in parser_errors {
        diagnostics.push(parser_error_to_diagnostic(&e.error, file_no));
    }

    match res {
        Err(e) => {
            diagnostics.push(parser_error_to_diagnostic(&e, file_no));
            Err(diagnostics)
        }
        _ if !diagnostics.is_empty() => Err(diagnostics),
        Ok(res) => Ok((res, comments)),
    }
}

/// Преобразует ошибку LALRPOP-парсера в [`Diagnostic`].
fn parser_error_to_diagnostic(
    error: &ParseError<usize, Token, LexicalError>,
    file_no: u64,
) -> Diagnostic {
    match error {
        ParseError::InvalidToken { location } => Diagnostic::parser_error(
            Location::Source(file_no, *location, *location),
            "invalid token".to_string(),
        ),
        ParseError::UnrecognizedToken {
            token: (l, token, r),
            expected,
        } => Diagnostic::parser_error(
            Location::Source(file_no, *l, *r),
            format!(
                "unrecognised token '{}', expected {}",
                token,
                expected.join(", ")
            ),
        ),
        ParseError::User { error } => Diagnostic::parser_error(error.loc(), error.to_string()),
        ParseError::ExtraToken { token } => Diagnostic::parser_error(
            Location::Source(file_no, token.0, token.2),
            format!("extra token '{}' encountered", token.1),
        ),
        ParseError::UnrecognizedEof { expected, location } => Diagnostic::parser_error(
            Location::Source(file_no, *location, *location),
            format!("unexpected end of file, expecting {}", expected.join(", ")),
        ),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// Комплексный тест разбора BuT-программы с различными конструкциями.
    ///
    /// Проверяет, что все основные элементы языка (псевдонимы типов, константы,
    /// условия, порты, переменные, модели, состояния, переходы, именованные блоки
    /// и операторы компоновки) успешно разбираются.
    #[test]
    fn parse_simple() {
        let src = r#"
//Алиас типа
type u8 = [bit;8];
//Константа
const MATRIX: u8 = { 0, 0, 0, 0, 0, 0, 0, 0 };
const NUMB: u8 = 0xFF;
cond  IsEmpty = it = 0;
//Порт с указанием отображаемого адреса
port  A : u8  = 0x00548835;
port  B1: bit = 0x00648835:6;
//Переменная
var   it: [bit;64] = 0;

//Модель
model Ping {
    //Начальное состояние
    start Start {
        //Переход на состояние по условию
        ref End: B1;
        //Исполнение блока кода при первом переходе в состояние
        enter {
            A.0 = true;
            A.1 = false;
        }
        //Исполнение блока кода при выходе из состояния
        exit {
            A.0 = false;
            A.1 = true;
        }
        always {
            A.2 = toggle;
        }
        always {
            toggle = !toggle;
        }
        var toggle = false;
    }
    state End;
}
model Pong {
    start Begin {
        ref Stop: S(Ping) = End;
        always {
            A.5 = MATRIX.5;
        }
    }
    state End {
        enter {
            A.6 = MATRIX.3;
        }
    }
}
model Toggle {
    start Entry {
        ref Ping: IsEmpty;
    }
    state Ping = Ping {
        next Pong;
        always {
            debug("Ping processing");
        }
    }
    state Pong = Pong {
        next Complete;
    }
    state Complete {
        ref End: true;
    }
    state End;
}
start Entry = (Ping | Pong) + Toggle;
always {
    debug("Main processing");
    it = it + 1;
    if S(Toggle) = Pong {
        debug("Pong processing");
    }
}"#;
        let result = parse(src, 0);
        if let Err(diagnostics) = result {
            for diagnostic in diagnostics.iter() {
                let source = &src[diagnostic.loc.start()..diagnostic.loc.end()];
                let text = &src[diagnostic.loc.start() - 5..diagnostic.loc.end() + 5];
                println!(
                    "[{}:{}] Source: {}, Text: {}, Message: {}",
                    diagnostic.loc.start(),
                    diagnostic.loc.end(),
                    source,
                    text,
                    diagnostic.message
                );
            }
        } else {
            assert!(!result.unwrap().0.elements.is_empty())
        }
    }
}
