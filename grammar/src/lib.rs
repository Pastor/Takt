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

use crate::parser::lexer::{LexicalError, Token};
use crate::parser::{ast, lexer};
use diagnostics::{Diagnostic, Location};
use lalrpop_util::ParseError;

/// Модуль диагностических сообщений компилятора.
pub mod diagnostics;
/// Модуль парсера
pub mod parser;
/// Модуль семантического анализа и построение семантического дерева
pub mod semantic;

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

/// Нормализует имя файла или идентификатора в CamelCase.
///
/// Преобразует `my_model`, `mein-leib`, `Mein_Leib` → `MyModel`, `MeinLeib`.
/// Небуквенно-цифровые символы (`_`, `-`, `#` и т.д.) используются как разделители слов.
pub fn normalize_model_name(name: &str) -> String {
    let mut result = String::new();
    let mut upper = true;
    for ch in name.chars() {
        if ch.is_alphabetic() && upper {
            result.push(ch.to_ascii_uppercase());
        } else if !ch.is_alphanumeric() {
            upper = true;
            continue;
        } else {
            result.push(ch);
        }
        upper = false;
    }
    result
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
/// use grammar::parser::ast::ModelElement;
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
            "недопустимый токен".to_string(),
        ),
        ParseError::UnrecognizedToken {
            token: (l, token, r),
            expected,
        } => Diagnostic::parser_error(
            Location::Source(file_no, *l, *r),
            format!(
                "нераспознанный токен '{}', ожидалось {}",
                token,
                expected.join(", ")
            ),
        ),
        ParseError::User { error } => Diagnostic::parser_error(error.loc(), error.to_string()),
        ParseError::ExtraToken { token } => Diagnostic::parser_error(
            Location::Source(file_no, token.0, token.2),
            format!("лишний токен '{}'", token.1),
        ),
        ParseError::UnrecognizedEof { expected, location } => Diagnostic::parser_error(
            Location::Source(file_no, *location, *location),
            format!("неожиданный конец файла, ожидалось {}", expected.join(", ")),
        ),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::semantic::tree::construct_model;
    #[cfg(feature = "ast-serde")]
    use serde_json;

    const SRC: &str = r#"
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
    }
    state End;
    var toggle = false;
}
model Pong {
    start Begin {
        ref Stop: S(Ping) = End;
        always {
            A.5 = MATRIX.5;
        }
    }
    state Stop {
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

    /// Комплексный тест разбора BuT-программы с различными конструкциями.
    ///
    /// Проверяет, что все основные элементы языка (псевдонимы типов, константы,
    /// условия, порты, переменные, модели, состояния, переходы, именованные блоки
    /// и операторы компоновки) успешно разбираются.
    #[test]
    fn parse_simple() {
        let result = parse(SRC, 0);
        if let Err(diagnostics) = result {
            for diagnostic in diagnostics.iter() {
                let source = &SRC[diagnostic.loc.start()..diagnostic.loc.end()];
                let text = &SRC[diagnostic.loc.start() - 5..diagnostic.loc.end() + 5];
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
            let (model, _) = result.unwrap();
            assert!(!model.elements.is_empty());
            #[cfg(feature = "ast-serde")]
            {
                let text = serde_json::to_string_pretty(&model).unwrap();
                println!("{}", text);
            }
        }
    }

    #[test]
    fn syntax_simple() {
        let (model, _) = parse(SRC, 0).unwrap();
        let model = construct_model(&model, None, &[]).unwrap();
        assert!(model.borrow().has_states());
    }

    const NAMES: &[(&str, &str)] = &[
        ("mein_leib", "MeinLeib"),
        ("mein-leib", "MeinLeib"),
        ("Mein_Leib", "MeinLeib"),
        ("mein_Leib", "MeinLeib"),
        ("Mein#Leib", "MeinLeib"),
    ];

    #[test]
    fn normalize_model_name() {
        use super::normalize_model_name;
        for (name, expected) in NAMES {
            let normalized = normalize_model_name(name);
            assert_eq!(&normalized, expected);
        }
    }

    // ── Дополнительные тесты нормализации имён ────────────────────────────────

    /// Пустая строка остаётся пустой.
    #[test]
    fn normalize_model_name_empty() {
        use super::normalize_model_name;
        assert_eq!(normalize_model_name(""), "");
    }

    /// Строка из одних цифр не изменяется.
    #[test]
    fn normalize_model_name_digits_only() {
        use super::normalize_model_name;
        assert_eq!(normalize_model_name("123"), "123");
    }

    /// Одно слово: первая буква становится заглавной.
    #[test]
    fn normalize_model_name_single_word() {
        use super::normalize_model_name;
        assert_eq!(normalize_model_name("hello"), "Hello");
    }

    // ── Тесты ошибок парсера ──────────────────────────────────────────────────

    /// Недопустимый токен: строка с управляющим символом вызывает ошибку парсера.
    #[test]
    fn parse_invalid_token_error() {
        // Управляющие символы не являются допустимыми токенами языка BuT
        let result = parse("\x00abc", 0);
        assert!(
            result.is_err(),
            "строка с управляющим символом должна давать ошибку"
        );
    }

    /// Неожиданный конец файла: незакрытая фигурная скобка.
    #[test]
    fn parse_unrecognized_eof_error() {
        let result = parse("model M {", 0);
        assert!(
            result.is_err(),
            "незакрытый блок модели должен давать ошибку EOF"
        );
        let diagnostics = result.unwrap_err();
        assert!(!diagnostics.is_empty(), "должна быть хотя бы одна диагностика");
    }

    /// Нераспознанный токен: объявление переменной с неверным синтаксисом.
    #[test]
    fn parse_unrecognized_token_error() {
        // «var» без имени переменной — нераспознанный токен
        let result = parse("var = 0;", 0);
        assert!(
            result.is_err(),
            "неверный синтаксис переменной должен давать ошибку"
        );
    }

    /// Разбор корректной программы не должен паниковать.
    #[test]
    fn syntax_simple_does_not_panic() {
        let result = parse("model M { start S; }", 0);
        assert!(result.is_ok(), "корректная программа должна разбираться без ошибок");
    }

    /// Ошибка парсера содержит непустой диагностический список.
    #[test]
    fn parse_error_produces_diagnostics() {
        let result = parse("model { }", 0);
        assert!(result.is_err(), "модель без имени должна давать ошибку");
        let diags = result.unwrap_err();
        assert!(
            !diags.is_empty(),
            "список диагностик не должен быть пустым при ошибке"
        );
    }
}
