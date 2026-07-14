//! Интеграционные тесты лексического анализатора Lam.
//!
//! Тесты разделены на несколько групп:
//! - **Позитивные по файлам** — файлы из `tests/data/lexer/valid/` лексируются без ошибок.
//! - **Негативные по файлам** — файлы из `tests/data/lexer/invalid/` порождают ошибку.
//! - **Модульные** — проверяют конкретные свойства лексера напрямую.
//!
//! При добавлении нового `.lam`-файла в директорию тест автоматически его подхватит.

use grammar::diagnostics::Location;
use grammar::parser::lexer::{Lexer, LexicalError, Token};
use std::fs;
use std::path::Path;

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
    comments.first().map_or(false, |c| c.is_doc())
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

/// `is_keyword` возвращает `true` для всех ключевых слов Lam, включая `extern`.
#[test]
fn is_keyword_returns_true_for_keywords() {
    use grammar::parser::lexer::is_keyword;

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
    use grammar::parser::lexer::is_keyword;

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

// ─────────────────────────── Тесты Display токенов ───────────────────────────

/// Display токенов ключевых слов совпадает с исходным кодом.
#[test]
fn token_display_keywords() {
    let cases: &[(&str, Token)] = &[
        ("model", Token::Model),
        ("state", Token::State),
        ("start", Token::Start),
        ("ref", Token::Reference),
        ("cond", Token::Condition),
        ("var", Token::Variable),
        ("const", Token::Constant),
        ("in", Token::PortIn),
        ("out", Token::PortOut),
        ("fn", Token::Function),
        ("true", Token::True),
        ("false", Token::False),
        ("next", Token::Next),
        ("extern", Token::Extern),
        ("import", Token::Import),
        ("type", Token::Type),
        ("loop", Token::Loop),
        ("continue", Token::Continue),
        ("break", Token::Break),
        ("return", Token::Return),
        ("string", Token::String),
        ("else", Token::Else),
        ("for", Token::For),
        ("if", Token::If),
        ("as", Token::As),
        ("assembly", Token::Assembly),
        ("formula", Token::Formula),
        ("template", Token::Template),
        ("pragma", Token::Pragma),
        ("X", Token::LtlNext),
        ("F", Token::LtlFinally),
        ("G", Token::LtlGlobally),
        ("U", Token::LtlUntil),
        ("R", Token::LtlRelease),
        ("LTL", Token::TypeLtl),
        ("Guard", Token::TypeGuard),
    ];
    for (expected, token) in cases {
        assert_eq!(
            &token.to_string(),
            expected,
            "Display для {:?} должен быть '{}'",
            token,
            expected
        );
    }
}

/// Display токенов операторов и знаков пунктуации.
#[test]
fn token_display_operators() {
    let cases: &[(&str, Token)] = &[
        (";", Token::Semicolon),
        (",", Token::Comma),
        ("#", Token::Sharp),
        ("(", Token::OpenParenthesis),
        (")", Token::CloseParenthesis),
        ("{", Token::OpenCurlyBrace), // отображается как "{"
        ("}", Token::CloseCurlyBrace),
        ("|", Token::BitwiseOr),
        ("^", Token::BitwiseXor),
        ("||", Token::Or),
        ("&", Token::BitwiseAnd),
        ("~", Token::BitwiseNot),
        ("&&", Token::And),
        ("+", Token::Add),
        ("-", Token::Subtract),
        ("*", Token::Mul),
        ("**", Token::Power),
        ("/", Token::Divide),
        ("%", Token::Modulo),
        ("==", Token::Equal),
        ("=", Token::Assign),
        ("!=", Token::NotEqual),
        ("!", Token::Not),
        ("<<", Token::ShiftLeft),
        (">", Token::More),
        (">=", Token::MoreEqual),
        (".", Token::Member),
        (":", Token::Colon),
        ("[", Token::OpenBracket),
        ("]", Token::CloseBracket),
        (">>", Token::ShiftRight),
        ("<", Token::Less),
        ("<=", Token::LessEqual),
        ("->", Token::Arrow),
        ("-->", Token::PeirceArrow),
    ];
    for (expected, token) in cases {
        assert_eq!(
            &token.to_string(),
            expected,
            "Display для {:?} должен быть '{}'",
            token,
            expected
        );
    }
}

/// Display для литеральных токенов.
#[test]
fn token_display_literals() {
    // Числовой
    assert_eq!(Token::Number(42).to_string(), "42");
    assert_eq!(Token::Number(-1).to_string(), "-1");

    // Рациональный — без знака
    assert_eq!(Token::RationalNumber("3.14", false).to_string(), "3.14");
    // Рациональный — отрицательный
    assert_eq!(Token::RationalNumber("3.14", true).to_string(), "-3.14");

    // Строковый — обычный
    assert_eq!(Token::StringLiteral(false, "hi").to_string(), "\"hi\"");
    // Строковый — unicode
    assert_eq!(
        Token::StringLiteral(true, "hi").to_string(),
        "unicode\"hi\""
    );

    // Адресный
    assert_eq!(Token::AddressLiteral("0x1234").to_string(), "0x1234");

    // Идентификатор
    assert_eq!(Token::Identifier("myVar").to_string(), "myVar");
}

// ─────────────────────────── Тесты ошибок (Display и loc) ────────────────────

/// `LexicalError::loc()` возвращает корректное местоположение для каждого варианта.
#[test]
fn lexical_error_loc_for_all_variants() {
    let loc = Location::Source(0, 5, 10);

    let cases = vec![
        LexicalError::EndOfFileInComment(loc.clone()),
        LexicalError::EndOfFileInString(loc.clone()),
        LexicalError::EndOfFileInHex(loc.clone()),
        LexicalError::MissingNumber(loc.clone()),
        LexicalError::InvalidCharacterInHexLiteral(loc.clone(), 'Z'),
        LexicalError::UnrecognisedToken(loc.clone(), "@".to_string()),
        LexicalError::MissingExponent(loc.clone()),
        LexicalError::ExpectedFrom(loc.clone(), "foo".to_string()),
    ];

    for err in &cases {
        assert_eq!(
            err.loc(),
            Location::Source(0, 5, 10),
            "Неверный loc для {:?}",
            err
        );
    }
}

/// Текстовые сообщения `LexicalError` содержат нужные подстроки.
#[test]
fn lexical_error_display_messages() {
    let loc = Location::Source(0, 0, 1);

    let cases: Vec<(LexicalError, &str)> = vec![
        (
            LexicalError::EndOfFileInComment(loc.clone()),
            "неожиданный конец файла внутри комментария",
        ),
        (
            LexicalError::EndOfFileInString(loc.clone()),
            "неожиданный конец файла внутри строкового литерала",
        ),
        (
            LexicalError::EndOfFileInHex(loc.clone()),
            "неожиданный конец файла внутри шестнадцатеричного литерала",
        ),
        (
            LexicalError::MissingNumber(loc.clone()),
            "отсутствует число после '0x'",
        ),
        (
            LexicalError::InvalidCharacterInHexLiteral(loc.clone(), 'Z'),
            "недопустимый символ 'Z'",
        ),
        (
            LexicalError::UnrecognisedToken(loc.clone(), "@".into()),
            "нераспознанный токен '@'",
        ),
        (
            LexicalError::MissingExponent(loc.clone()),
            "отсутствует показатель степени",
        ),
        (
            LexicalError::ExpectedFrom(loc.clone(), "bar".into()),
            "ожидалось ключевое слово 'from', но найдено 'bar'",
        ),
    ];

    for (err, expected_substr) in &cases {
        let msg = err.to_string();
        assert!(
            msg.contains(expected_substr),
            "Сообщение {:?} не содержит '{}'",
            msg,
            expected_substr
        );
    }
}

// ─────────────────────────── Тесты комментариев ──────────────────────────────

/// Обычный комментарий `//` не производит токены, но собирается.
#[test]
fn line_comment_produces_no_tokens() {
    let src = "// это комментарий\nmodel";
    assert_eq!(token_count(src), 1, "После комментария — только 'model'");
    assert_eq!(comment_count(src), 1, "Ожидался один комментарий");
    assert!(!first_comment_is_doc(src), "// — не документационный");
}

/// Документационный комментарий `///` помечается флагом.
#[test]
fn doc_comment_is_distinguished_from_line_comment() {
    let src = "/// документация\nmodel";
    assert_eq!(token_count(src), 1);
    assert_eq!(comment_count(src), 1);
    assert!(first_comment_is_doc(src), "/// — документационный");
}

/// `////` и далее — обычный (не документационный) комментарий.
#[test]
fn four_slash_comment_is_not_doc() {
    let src = "//// not doc\nmodel";
    assert_eq!(comment_count(src), 1);
    assert!(!first_comment_is_doc(src), "//// не документационный");
}

/// Пустой комментарий `//` (без текста) разбирается без ошибок.
#[test]
fn empty_line_comment_lexes_ok() {
    let errors = collect_errors("//\nmodel");
    assert!(
        errors.is_empty(),
        "Пустой комментарий вызвал ошибку: {:?}",
        errors
    );
}

/// Несколько комментариев — все собраны.
#[test]
fn multiple_comments_are_all_collected() {
    let src = "// первый\n// второй\n/// третий\nmodel";
    assert_eq!(comment_count(src), 3, "Ожидались 3 комментария");
    assert_eq!(token_count(src), 1, "После комментариев — только 'model'");
}

/// Комментарий в конце файла без перевода строки не вызывает ошибок.
#[test]
fn comment_at_eof_without_newline() {
    let src = "model // в конце";
    assert_eq!(token_count(src), 1);
    assert_eq!(comment_count(src), 1);
    let errors = collect_errors(src);
    assert!(errors.is_empty());
}

// ─────────────────────── Тесты блочных комментариев ──────────────────────────

/// Блочный комментарий `/* */` не производит токены, но собирается.
#[test]
fn block_comment_produces_no_tokens() {
    let src = "/* блочный комментарий */\nmodel";
    assert_eq!(
        token_count(src),
        1,
        "После блочного комментария — только 'model'"
    );
    assert_eq!(comment_count(src), 1, "Ожидался один блочный комментарий");
}

/// Блочный комментарий помечается флагом `is_block()`.
#[test]
fn block_comment_is_detected_as_block() {
    use grammar::parser::ast::Comment;
    let src = "/* блок */\nmodel";
    let mut comments = Vec::new();
    let mut errors = Vec::new();
    let _: Vec<_> = Lexer::new(src, 0, &mut comments, &mut errors).collect();
    assert_eq!(comments.len(), 1);
    assert!(
        matches!(&comments[0], Comment::Block(..)),
        "Ожидался Comment::Block, получено: {:?}",
        comments[0]
    );
    assert!(comments[0].is_block(), "is_block() должен возвращать true");
    assert!(!comments[0].is_doc(), "is_doc() должен возвращать false");
    assert!(!comments[0].is_line(), "is_line() должен возвращать false");
}

/// Блочный комментарий может содержать несколько строк.
#[test]
fn multiline_block_comment_lexes_ok() {
    let src = "/*\n строка 1\n строка 2\n*/\nvar x = 1;";
    let errors = collect_errors(src);
    assert!(
        errors.is_empty(),
        "Многострочный блочный комментарий вызвал ошибку: {:?}",
        errors
    );
    assert_eq!(comment_count(src), 1, "Ожидался один комментарий");
    assert_eq!(token_count(src), 5, "Ожидались токены: var, x, =, 1, ;");
}

/// Блочный комментарий может содержать `//` внутри.
#[test]
fn block_comment_containing_line_comment_syntax() {
    let src = "/* содержит // строчный синтаксис */\nmodel";
    assert_eq!(comment_count(src), 1);
    assert_eq!(token_count(src), 1);
    let errors = collect_errors(src);
    assert!(errors.is_empty());
}

/// Несколько блочных комментариев — все собраны.
#[test]
fn multiple_block_comments_collected() {
    let src = "/* первый */ var /* второй */ x = 1;";
    assert_eq!(comment_count(src), 2, "Ожидались 2 блочных комментария");
    assert_eq!(token_count(src), 5, "var, x, =, 1, ;");
    let errors = collect_errors(src);
    assert!(errors.is_empty());
}

/// Смешанные строчные и блочные комментарии — все собраны.
#[test]
fn mixed_line_and_block_comments_collected() {
    let src = "// строчный\n/* блочный */\n/// документация\nmodel";
    assert_eq!(comment_count(src), 3, "Ожидались 3 комментария");
    assert_eq!(token_count(src), 1, "После комментариев — только 'model'");
}

/// Незакрытый блочный комментарий порождает `EndOfFileInComment`.
#[test]
fn unclosed_block_comment_produces_eof_error() {
    let src = "/* незакрытый комментарий";
    let errors = collect_errors(src);
    assert!(
        !errors.is_empty(),
        "Незакрытый блочный комментарий должен давать ошибку"
    );
    assert!(
        matches!(errors[0], LexicalError::EndOfFileInComment(_)),
        "Ожидалась EndOfFileInComment, получено: {:?}",
        errors[0]
    );
}

/// Пустой блочный комментарий `/**/` разбирается без ошибок.
#[test]
fn empty_block_comment_lexes_ok() {
    let src = "/**/ model";
    let errors = collect_errors(src);
    assert!(
        errors.is_empty(),
        "Пустой блочный комментарий вызвал ошибку: {:?}",
        errors
    );
    assert_eq!(comment_count(src), 1);
    assert_eq!(token_count(src), 1);
}

/// Деление `/` после блочного комментария разбирается корректно.
#[test]
fn divide_after_block_comment_is_divide_token() {
    let src = "/* комментарий */ 10 / 2";
    assert_eq!(comment_count(src), 1);
    assert_eq!(token_count(src), 3, "10, /, 2");
    let errors = collect_errors(src);
    assert!(errors.is_empty());
}

/// Значение блочного комментария содержит исходный текст включая `/*` и `*/`.
#[test]
fn block_comment_value_contains_full_text() {
    let src = "/* содержимое */";
    let mut comments = Vec::new();
    let mut errors = Vec::new();
    let _: Vec<_> = Lexer::new(src, 0, &mut comments, &mut errors).collect();
    assert_eq!(comments.len(), 1);
    let val = comments[0].value();
    assert!(
        val.contains("содержимое"),
        "Значение должно содержать текст комментария: {}",
        val
    );
}

// ─────────────────────── Тесты ошибочных конструкций ─────────────────────────

/// Неизвестный символ `@` порождает `UnrecognisedToken`.
#[test]
fn unknown_character_at_produces_error() {
    let errors = collect_errors("@");
    assert!(!errors.is_empty(), "Символ '@' должен давать ошибку");
    assert!(
        matches!(errors[0], LexicalError::UnrecognisedToken(_, _)),
        "Ожидалась UnrecognisedToken, получено: {:?}",
        errors[0]
    );
}

/// Ошибка лексера содержит корректное местоположение.
#[test]
fn lexical_error_has_source_location() {
    let errors = collect_errors("@");
    assert!(!errors.is_empty());
    let loc = errors[0].loc();
    assert_eq!(loc.start(), 0, "Ошибка должна начинаться с позиции 0");
}

/// Несколько неизвестных символов — ошибка для каждого.
#[test]
fn multiple_unknown_chars_produce_multiple_errors() {
    let errors = collect_errors("@ @");
    assert!(
        errors.len() >= 2,
        "Два символа '@' должны дать не менее 2 ошибок"
    );
}

// ─────────────────────── Тесты позиций токенов ───────────────────────────────

/// Позиции токенов соответствуют их расположению в исходной строке.
#[test]
fn token_positions_are_correct() {
    let src = "model M".to_string();
    let mut comments = Vec::new();
    let mut errors = Vec::new();
    let spanned: Vec<_> = Lexer::new(&src, 0, &mut comments, &mut errors).collect();

    assert_eq!(spanned.len(), 2, "Ожидалось 2 токена");
    let (start, _, end) = spanned[0];
    assert_eq!(start, 0, "'model' начинается с 0");
    assert_eq!(end, 5, "'model' заканчивается на 5");

    let (start, _, end) = spanned[1];
    assert_eq!(start, 6, "'M' начинается с 6");
    assert_eq!(end, 7, "'M' заканчивается на 7");
}

// ──────────────────────────── Тесты идентификаторов ──────────────────────────

/// Идентификаторы разбираются без ошибок.
#[test]
fn identifiers_lex_without_errors() {
    let ids = [
        "abc",
        "ABC",
        "_private",
        "$dollar",
        "CamelCase",
        "x1",
        "_123",
    ];
    for id in ids {
        let errors = collect_errors(id);
        assert!(
            errors.is_empty(),
            "Ошибка при идентификаторе '{}': {:?}",
            id,
            errors
        );
        assert_eq!(token_count(id), 1, "'{}' должен давать 1 токен", id);
    }
}

/// Идентификатор, совпадающий с ключевым словом, распознаётся как ключевое слово.
#[test]
fn keyword_is_not_identifier() {
    let src = "model".to_string();
    let errors = collect_errors(&src);
    assert!(errors.is_empty());
    let strings = collect_token_strings(&src);
    assert_eq!(strings.len(), 1);
    assert_eq!(strings[0], "model", "model должен быть Token::Model");
}

/// Пробелы и переносы строк игнорируются.
#[test]
fn whitespace_is_skipped() {
    assert_eq!(token_count("  \t  model  \n  state  "), 2);
    assert_eq!(collect_errors("  \t  model  \n  state  ").len(), 0);
}

/// Пустая строка не даёт токенов и не даёт ошибок.
#[test]
fn empty_input_produces_nothing() {
    assert_eq!(token_count(""), 0);
    assert!(collect_errors("").is_empty());
}

// ──────────────────────────── Тест `pragma` как идентификатора ───────────────

/// `pragma` не является ключевым словом в Lam — распознаётся как идентификатор.
#[test]
fn pragma_is_not_a_keyword_and_lexes_as_identifier() {
    let strings = collect_token_strings("pragma");
    assert_eq!(strings.len(), 1);
    assert_eq!(strings[0], "pragma", "pragma должен быть идентификатором");
    assert_eq!(collect_errors("pragma").len(), 0);
}

// ─────────────────── Тест полного набора Lam-конструкций ─────────────────────

/// Полный пример программы лексируется без ошибок.
#[test]
fn complete_but_program_lexes_without_errors() {
    let src = r#"
/// Пример полной Lam-программы
type u8 = [bit;8];
const MAX: u8 = 0xFF;
out LED: u8 = 00100000;
var counter: u8 = 0;
cond IsMax = counter = MAX;
model Blinker {
    start On {
        ref Off: IsMax;
        enter { LED = 0xFF; }
        exit  { LED = 0x00; }
        always { counter = counter + 1; }
    }
    state Off {
        ref On: true;
    }
}
start Main = Blinker;
    "#;
    let errors = collect_errors(src);
    assert!(
        errors.is_empty(),
        "Полная программа вызвала ошибки: {:?}",
        errors
    );
}

/// `extern fn` лексируется как два токена: `Extern` и `Function`.
#[test]
fn extern_fn_lexes_as_two_tokens() {
    let strings = collect_token_strings("extern fn");
    assert_eq!(strings.len(), 2);
    assert_eq!(strings[0], "extern", "Первый токен — extern");
    assert_eq!(strings[1], "fn", "Второй токен — fn");
}
