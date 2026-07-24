//! Интеграционные тесты лексического анализатора Takt.
//!
//! Тесты разделены на несколько групп:
//! - **Позитивные по файлам** — файлы из `tests/data/lexer/valid/` лексируются без ошибок.
//! - **Негативные по файлам** — файлы из `tests/data/lexer/invalid/` порождают ошибку.
//! - **Модульные** — проверяют конкретные свойства лексера напрямую.
//!
//! При добавлении нового `.lam`-файла в директорию тест автоматически его подхватит.

use std::fs;
use std::path::Path;
use takt_lang::diagnostics::Location;
use takt_lang::parser::lexer::{Lexer, LexicalError, Token};

// ─────────────────────────────── Вспомогательные функции ─────────────────────

/// Запускает лексер и возвращает вектор лексических ошибок.
fn collect_errors(input: &str) -> Vec<LexicalError> {
    let src = input.to_string();
    let mut comments = Vec::new();
    let mut errors = Vec::new();
    {
        let _: Vec<_> = Lexer::new(&src, 0, &mut comments, &mut errors).collect();
    }
    errors
}

/// Возвращает количество токенов в строке (ошибки игнорируются).
fn token_count(input: &str) -> usize {
    let src = input.to_string();
    let mut comments = Vec::new();
    let mut errors = Vec::new();
    Lexer::new(&src, 0, &mut comments, &mut errors).count()
}

/// Собирает Display-представления всех токенов из строки.
///
/// Используется вместо возврата `Vec<Token<'_>>` для обхода ограничений
/// времени жизни: `Token` заимствует из входной строки и буферов лексера.
fn collect_token_strings(input: &str) -> Vec<String> {
    let src = input.to_string();
    let mut comments = Vec::new();
    let mut errors = Vec::new();
    Lexer::new(&src, 0, &mut comments, &mut errors)
        .map(|(_, tok, _)| tok.to_string())
        .collect()
}

/// Возвращает количество комментариев в строке.
fn comment_count(input: &str) -> usize {
    let src = input.to_string();
    let mut comments = Vec::new();
    let mut errors = Vec::new();
    {
        let _: Vec<_> = Lexer::new(&src, 0, &mut comments, &mut errors).collect();
    }
    comments.len()
}

/// Возвращает `true`, если первый комментарий является документационным.
fn first_comment_is_doc(input: &str) -> bool {
    let src = input.to_string();
    let mut comments = Vec::new();
    let mut errors = Vec::new();
    {
        let _: Vec<_> = Lexer::new(&src, 0, &mut comments, &mut errors).collect();
    }
    comments.first().is_some_and(|c| c.is_doc())
}

// ─────────────────────────── Позитивные тесты (по файлам) ────────────────────

/// Проверяет, что все `.lam`-файлы из директории `valid` лексируются без ошибок.
#[test]
fn valid_files_lex_without_errors() {
    let dir = Path::new("tests/data/lexer/valid");
    let entries: Vec<_> = fs::read_dir(dir)
        .unwrap_or_else(|e| panic!("Не удалось прочитать директорию {:?}: {}", dir, e))
        .filter_map(|e| e.ok())
        .filter(|e| e.path().extension().is_some_and(|ext| ext == "lam"))
        .collect();

    assert!(
        !entries.is_empty(),
        "Директория {:?} не содержит .lam файлов",
        dir
    );

    for entry in entries {
        let path = entry.path();
        let src = fs::read_to_string(&path)
            .unwrap_or_else(|e| panic!("Не удалось прочитать {:?}: {}", path, e));

        let errors = collect_errors(&src);
        assert!(
            errors.is_empty(),
            "Файл {:?} вызвал лексические ошибки: {:?}",
            path,
            errors
        );
    }
}

// ─────────────────── Негативные тесты (контр-примеры по файлам) ──────────────

/// Проверяет, что все `.lam`-файлы из директории `invalid` порождают ошибку лексера.
#[test]
fn invalid_files_produce_lex_errors() {
    let dir = Path::new("tests/data/lexer/invalid");
    let entries: Vec<_> = fs::read_dir(dir)
        .unwrap_or_else(|e| panic!("Не удалось прочитать директорию {:?}: {}", dir, e))
        .filter_map(|e| e.ok())
        .filter(|e| e.path().extension().is_some_and(|ext| ext == "lam"))
        .collect();

    assert!(
        !entries.is_empty(),
        "Директория {:?} не содержит .lam файлов",
        dir
    );

    for entry in entries {
        let path = entry.path();
        let src = fs::read_to_string(&path)
            .unwrap_or_else(|e| panic!("Не удалось прочитать {:?}: {}", path, e));

        let errors = collect_errors(&src);
        assert!(
            !errors.is_empty(),
            "Файл {:?} ожидался с ошибкой, но лексировался без ошибок",
            path
        );
    }
}

// ───────────────────────── Тесты ключевых слов ───────────────────────────────

/// `is_keyword` возвращает `true` для всех ключевых слов Takt, включая `extern`.
#[test]
fn is_keyword_returns_true_for_keywords() {
    use takt_lang::parser::lexer::is_keyword;

    let keywords = [
        "break", "const", "continue", "else", "false", "for", "fn", "if", "import", "loop",
        "return", "string", "true", "type", "as", "assembly", "formula", "in", "out", "model",
        "state", "start", "ref", "template", "cond", "var", "next", "extern", "inout", "address",
    ];
    for kw in keywords {
        assert!(is_keyword(kw), "'{}' должно быть ключевым словом", kw);
    }
}

/// `is_keyword` возвращает `false` для обычных идентификаторов и неключевых слов.
///
/// Контр-примеры: `extern` — ключевое слово, не должен быть в этом списке.
#[test]
fn is_keyword_returns_false_for_identifiers() {
    use takt_lang::parser::lexer::is_keyword;

    let non_keywords = ["MyModel", "", "pragma", "foobar", "_x", "123"];
    for word in non_keywords {
        assert!(
            !is_keyword(word),
            "'{}' не должно быть ключевым словом",
            word
        );
    }
}

/// `extern` распознаётся как `Token::Extern`, а не как идентификатор.
#[test]
fn extern_keyword_produces_extern_token() {
    let src = "extern".to_string();
    let mut comments = Vec::new();
    let mut errors = Vec::new();
    let tokens: Vec<_> = Lexer::new(&src, 0, &mut comments, &mut errors).collect();
    assert_eq!(tokens.len(), 1, "extern должен давать 1 токен");
    assert!(
        matches!(tokens[0].1, Token::Extern),
        "extern должен быть Token::Extern, получено: {:?}",
        tokens[0].1
    );
}

/// `address` распознаётся как `Token::Address`, а не как идентификатор (фича 0020).
#[test]
fn address_keyword_produces_address_token() {
    let src = "address".to_string();
    let mut comments = Vec::new();
    let mut errors = Vec::new();
    let tokens: Vec<_> = Lexer::new(&src, 0, &mut comments, &mut errors).collect();
    assert_eq!(tokens.len(), 1, "address должен давать 1 токен");
    assert!(
        matches!(tokens[0].1, Token::Address),
        "address должен быть Token::Address, получено: {:?}",
        tokens[0].1
    );
}

/// Ключевые слова распознаются как правильные токены.
#[test]
fn keywords_produce_correct_token_count() {
    let pairs: &[(&str, usize)] = &[
        ("model", 1),
        ("state", 1),
        ("start", 1),
        ("ref cond var", 3),
        ("fn if else loop for break continue return", 8),
        ("extern fn", 2),
    ];

    for (src, expected_count) in pairs {
        let count = token_count(src);
        assert_eq!(
            count, *expected_count,
            "Для '{}' ожидалось {} токенов",
            src, expected_count
        );
        let errors = collect_errors(src);
        assert!(
            errors.is_empty(),
            "Ошибки при разборе ключевых слов '{}': {:?}",
            src,
            errors
        );
    }
}

// ─────────────────────────── Тесты числовых литералов ────────────────────────

/// Десятичные числа разбираются корректно.
#[test]
fn decimal_numbers_lex_without_errors() {
    let cases = ["0", "42", "255", "1000", "999999"];
    for src in cases {
        let errors = collect_errors(src);
        assert!(
            errors.is_empty(),
            "Ошибка при разборе числа '{}': {:?}",
            src,
            errors
        );
        assert_eq!(token_count(src), 1, "Число '{}' должно давать 1 токен", src);
    }
}

/// Шестнадцатеричные числа разбираются корректно.
#[test]
fn hex_numbers_lex_without_errors() {
    let cases = ["0xFF", "0x0", "0x1A", "0xDEAD_BEEF"];
    for src in cases {
        let errors = collect_errors(src);
        assert!(
            errors.is_empty(),
            "Ошибка при разборе hex '{}': {:?}",
            src,
            errors
        );
    }
}

/// Отрицательное число разбирается как один токен `Number`.
#[test]
fn negative_number_lexes_as_one_token() {
    let src = "-42";
    let errors = collect_errors(src);
    assert!(errors.is_empty(), "Ошибки при разборе '-42': {:?}", errors);
    assert_eq!(token_count(src), 1, "-42 должно быть одним токеном Number");
}

/// Рациональное число (с точкой) разбирается корректно.
#[test]
fn rational_number_lexes_without_errors() {
    let errors = collect_errors("3.14");
    assert!(errors.is_empty(), "Ошибки при разборе '3.14': {:?}", errors);
    assert_eq!(token_count("3.14"), 1);
}

/// Число с показателем степени разбирается корректно.
#[test]
fn number_with_exponent_lexes_ok() {
    let cases = ["1e5", "2E10", "1e0"];
    for src in cases {
        let errors = collect_errors(src);
        assert!(
            errors.is_empty(),
            "Ошибка при разборе числа с экспонентой '{}': {:?}",
            src,
            errors
        );
        assert_eq!(token_count(src), 1, "'{}' должен давать 1 токен", src);
    }
}

/// Рациональное число с показателем степени разбирается корректно.
#[test]
fn rational_with_exponent_lexes_ok() {
    // 2.5e3 — рациональное число с экспонентой
    let errors = collect_errors("2.5e3");
    assert!(
        errors.is_empty(),
        "Ошибка при разборе '2.5e3': {:?}",
        errors
    );
    assert_eq!(token_count("2.5e3"), 1);
}

/// Десятичные числа с разделителем `_` разбираются корректно.
#[test]
fn decimal_with_underscore_lexes_ok() {
    let cases = ["1_000", "1_000_000"];
    for src in cases {
        let errors = collect_errors(src);
        assert!(
            errors.is_empty(),
            "Ошибка при разборе '{}': {:?}",
            src,
            errors
        );
        assert_eq!(token_count(src), 1);
    }
}

/// Шестнадцатеричный литерал без цифр порождает ошибку.
#[test]
fn hex_without_digits_is_error() {
    let errors = collect_errors("0x");
    assert!(!errors.is_empty(), "0x без цифр должен давать ошибку");
    assert!(
        matches!(
            errors[0],
            LexicalError::EndOfFileInHex(_) | LexicalError::MissingNumber(_)
        ),
        "Ожидалась EndOfFileInHex или MissingNumber, получено: {:?}",
        errors[0]
    );
}

/// Шестнадцатеричный литерал с недопустимым символом сразу после `0x`.
#[test]
fn hex_with_invalid_first_char_is_error() {
    // `0x=` — после 0x сразу неверный символ
    let errors = collect_errors("0x=");
    assert!(
        !errors.is_empty(),
        "0x= без цифр должен давать ошибку MissingNumber"
    );
    assert!(
        matches!(errors[0], LexicalError::MissingNumber(_)),
        "Ожидалась MissingNumber, получено: {:?}",
        errors[0]
    );
}

/// Число с показателем степени без цифр — ошибка.
#[test]
fn number_with_missing_exponent_is_error() {
    let errors = collect_errors("1e");
    assert!(
        !errors.is_empty(),
        "1e без цифр должен давать ошибку MissingExponent"
    );
    assert!(
        matches!(errors[0], LexicalError::MissingExponent(_)),
        "Ожидалась MissingExponent, получено: {:?}",
        errors[0]
    );
}

// ─────────────────────────── Тесты строковых литералов ───────────────────────

/// Строковый литерал в двойных кавычках разбирается корректно.
#[test]
fn double_quoted_string_lexes_ok() {
    let errors = collect_errors(r#""hello world""#);
    assert!(errors.is_empty(), "Ошибки при разборе строки: {:?}", errors);
    assert_eq!(token_count(r#""hello world""#), 1);
}

/// Unicode-строка разбирается корректно.
#[test]
fn unicode_string_lexes_ok() {
    let errors = collect_errors(r#"unicode"text""#);
    assert!(errors.is_empty(), "Ошибки при unicode-строке: {:?}", errors);
    assert_eq!(token_count(r#"unicode"text""#), 1);
}

/// Unicode-строка в одинарных кавычках разбирается корректно.
#[test]
fn unicode_string_single_quote_lexes_ok() {
    let errors = collect_errors("unicode'text'");
    assert!(
        errors.is_empty(),
        "Ошибки при unicode-строке в одинарных кавычках: {:?}",
        errors
    );
    assert_eq!(token_count("unicode'text'"), 1);
}

/// `unicode` как самостоятельный идентификатор (не перед кавычкой) — обычный идентификатор.
#[test]
fn unicode_as_identifier_when_not_before_quote() {
    // Используем Display-строки для сравнения (обход ограничений времени жизни)
    let strings = collect_token_strings("unicode model");
    assert_eq!(strings.len(), 2);
    assert_eq!(strings[0], "unicode", "unicode без кавычки — идентификатор");
    assert_eq!(strings[1], "model");
}

/// Строка в одинарных кавычках разбирается корректно.
#[test]
fn single_quoted_string_lexes_ok() {
    let errors = collect_errors("'hello'");
    assert!(
        errors.is_empty(),
        "Ошибки при строке в одинарных кавычках: {:?}",
        errors
    );
}

/// Незакрытая строка порождает `EndOfFileInString`.
#[test]
fn unclosed_string_produces_eof_in_string_error() {
    let errors = collect_errors("\"unclosed");
    assert!(!errors.is_empty(), "Незакрытая строка должна давать ошибку");
    assert!(
        matches!(errors[0], LexicalError::EndOfFileInString(_)),
        "Ожидалась EndOfFileInString, получено: {:?}",
        errors[0]
    );
}

/// Строка с escape-последовательностями разбирается корректно.
#[test]
fn string_with_escape_sequences_lexes_ok() {
    let errors = collect_errors(r#""line1\nline2\ttab""#);
    assert!(
        errors.is_empty(),
        "Ошибки при строке с escape: {:?}",
        errors
    );
}

/// Незакрытая строка в одинарных кавычках — `EndOfFileInString`.
#[test]
fn unclosed_single_quoted_string_is_error() {
    let errors = collect_errors("'unclosed");
    assert!(!errors.is_empty(), "Незакрытая строка должна давать ошибку");
    assert!(
        matches!(errors[0], LexicalError::EndOfFileInString(_)),
        "Ожидалась EndOfFileInString, получено: {:?}",
        errors[0]
    );
}

// ───────────────────────────── Тесты операторов ──────────────────────────────

/// Все однобайтовые операторы разбираются в ровно 1 токен.
#[test]
fn single_char_operators_lex_ok() {
    let ops = [
        "+", "-", "*", "/", "%", "=", "!", "<", ">", "&", "|", "^", "~", ".", ":", ";", ",", "(",
        ")", "{", "}", "[", "]", "#",
    ];
    for op in ops {
        let errors = collect_errors(op);
        assert!(
            errors.is_empty(),
            "Ошибка при операторе '{}': {:?}",
            op,
            errors
        );
        assert_eq!(
            token_count(op),
            1,
            "Оператор '{}' должен давать 1 токен",
            op
        );
    }
}

/// Двухсимвольные операторы разбираются корректно.
#[test]
fn two_char_operators_lex_ok() {
    let ops = ["**", "==", "!=", "<=", ">=", "<<", ">>", "&&", "||"];
    for op in ops {
        let errors = collect_errors(op);
        assert!(
            errors.is_empty(),
            "Ошибка при операторе '{}': {:?}",
            op,
            errors
        );
        assert_eq!(
            token_count(op),
            1,
            "Оператор '{}' должен давать 1 токен",
            op
        );
    }
}

/// `>>` и `<<` — разные токены.
#[test]
fn shift_operators_are_distinct() {
    assert_eq!(token_count("<<"), 1);
    assert_eq!(token_count(">>"), 1);
    assert_eq!(token_count("<< >>"), 2);
}

/// `ShiftRight` отображается как `>>`, а не `<<`.
#[test]
fn shift_right_display_is_correct() {
    let token = Token::ShiftRight;
    assert_eq!(token.to_string(), ">>");
    assert_ne!(token.to_string(), "<<");
}

/// `->` лексируется как один токен Arrow.
#[test]
fn arrow_lexes_as_single_token() {
    let strings = collect_token_strings("->");
    assert_eq!(strings.len(), 1, "-> должен давать 1 токен");
    assert_eq!(strings[0], "->", "Токен — Arrow (->)");
}

// Вторая половина тестов — в подмодуле (фича 0088, лимит размера модуля).
// `#[path]` обязателен: корень тест-бинарника ищет `mod` в tests/, а не в
// подкаталоге по имени файла.
#[path = "lexer_tests/part2.rs"]
mod part2;
