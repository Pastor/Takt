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
mod but {
    include!(concat!(env!("OUT_DIR"), "/grammar.rs"));
}

pub fn parse(
    src: &str,
    filename: String,
) -> Result<(ast::ModelElement, Vec<ast::Comment>), Vec<Diagnostic>> {
    let mut comments = Vec::new();
    let mut lexer_errors = Vec::new();
    let mut lex = lexer::Lexer::new(src, filename.clone(), &mut comments, &mut lexer_errors);

    let mut parser_errors = Vec::new();
    let res =
        but::SourceUnitParser::new().parse(src, filename.clone(), &mut parser_errors, &mut lex);

    let mut diagnostics = Vec::with_capacity(lex.errors.len() + parser_errors.len());
    for lexical_error in lex.errors {
        diagnostics.push(Diagnostic::parser_error(
            lexical_error.loc(),
            lexical_error.to_string(),
        ))
    }

    for e in parser_errors {
        diagnostics.push(parser_error_to_diagnostic(&e.error, filename.clone()));
    }

    match res {
        Err(e) => {
            diagnostics.push(parser_error_to_diagnostic(&e, filename.clone()));
            Err(diagnostics)
        }
        _ if !diagnostics.is_empty() => Err(diagnostics),
        Ok(res) => Ok((res, comments)),
    }
}

/// Convert lalrop parser error to a Diagnostic
fn parser_error_to_diagnostic(
    error: &ParseError<usize, Token, LexicalError>,
    filename: String,
) -> Diagnostic {
    match error {
        ParseError::InvalidToken { location } => Diagnostic::parser_error(
            Location::Source(filename, *location, *location),
            "invalid token".to_string(),
        ),
        ParseError::UnrecognizedToken {
            token: (l, token, r),
            expected,
        } => Diagnostic::parser_error(
            Location::Source(filename, *l, *r),
            format!(
                "unrecognised token '{}', expected {}",
                token,
                expected.join(", ")
            ),
        ),
        ParseError::User { error } => Diagnostic::parser_error(error.loc(), error.to_string()),
        ParseError::ExtraToken { token } => Diagnostic::parser_error(
            Location::Source(filename, token.0, token.2),
            format!("extra token '{}' encountered", token.0),
        ),
        ParseError::UnrecognizedEof { expected, location } => Diagnostic::parser_error(
            Location::Source(filename, *location, *location),
            format!("unexpected end of file, expecting {}", expected.join(", ")),
        ),
    }
}
