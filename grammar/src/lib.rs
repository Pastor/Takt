#![doc = include_str!("../../README.md")]
#![warn(missing_debug_implementations, missing_docs)]

extern crate core;

use lalrpop_util::ParseError;

use diagnostics::Diagnostic;

use crate::ast::Location;
use crate::lexer::LexicalError;
use crate::lexer::Token;

pub mod ast;
pub mod diagnostics;
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

/// Convert lalrop parser error to a Diagnostic
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
            format!("extra token '{}' encountered", token.0),
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
port  A : u8  = 00548835;
port  B1: bit = 00648835:6;
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
