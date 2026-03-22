//! Интеграционные тесты лексического анализатора BuT.
//!
//! Тесты разделены на две группы:
//! - **Позитивные** — файлы из `tests/data/lexer/valid/` должны лексироваться без ошибок.
//! - **Негативные** (контр-примеры) — файлы из `tests/data/lexer/invalid/` должны
//!   порождать хотя бы одну лексическую ошибку.
//!
//! При добавлении нового `.but`-файла в директорию тест автоматически его подхватит.

use std::fs;
use std::path::Path;

use grammar::lexer::{LexicalError, Lexer, Token};

// ─────────────────────────────── Вспомогательные макро-утилиты ──────────────

/// Запускает лексер и возвращает вектор лексических ошибок.
///
/// Внутри создаётся локальная копия строки, чтобы время жизни входных данных
/// совпадало с временем жизни буферов `comments` и `errors`.
/// `LexicalError` — полностью владеющий тип, поэтому его можно вернуть.
fn collect_errors(input: &str) -> Vec<LexicalError> {
    // Локальная копия: все three переменные имеют одинаковое время жизни.
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

// ─────────────────────────── Позитивные тесты (по файлам) ───────────────────

/// Проверяет, что все `.but`-файлы из директории `valid` лексируются без ошибок.
#[test]
fn valid_files_lex_without_errors() {
    let dir = Path::new("tests/data/lexer/valid");
    let entries: Vec<_> = fs::read_dir(dir)
        .unwrap_or_else(|e| panic!("Не удалось прочитать директорию {:?}: {}", dir, e))
        .filter_map(|e| e.ok())
        .filter(|e| e.path().extension().map_or(false, |ext| ext == "but"))
        .collect();

    assert!(
        !entries.is_empty(),
        "Директория {:?} не содержит .but файлов",
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

// ─────────────────── Негативные тесты (контр-примеры по файлам) ─────────────

/// Проверяет, что все `.but`-файлы из директории `invalid` порождают ошибку лексера.
#[test]
fn invalid_files_produce_lex_errors() {
    let dir = Path::new("tests/data/lexer/invalid");
    let entries: Vec<_> = fs::read_dir(dir)
        .unwrap_or_else(|e| panic!("Не удалось прочитать директорию {:?}: {}", dir, e))
        .filter_map(|e| e.ok())
        .filter(|e| e.path().extension().map_or(false, |ext| ext == "but"))
        .collect();

    assert!(
        !entries.is_empty(),
        "Директория {:?} не содержит .but файлов",
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

// ───────────────────────── Модульные тесты ключевых слов ────────────────────

/// `is_keyword` возвращает `true` для всех ключевых слов BuT.
#[test]
fn is_keyword_returns_true_for_keywords() {
    use grammar::lexer::is_keyword;

    let keywords = [
        "break", "const", "continue", "do", "else", "false", "for",
        "fn", "if", "import", "return", "string", "true", "type",
        "while", "as", "assembly", "formula", "port", "model",
        "state", "start", "ref", "template", "cond", "var", "next",
    ];
    for kw in keywords {
        assert!(is_keyword(kw), "'{}' должно быть ключевым словом", kw);
    }
}

/// `is_keyword` возвращает `false` для обычных идентификаторов.
#[test]
fn is_keyword_returns_false_for_identifiers() {
    use grammar::lexer::is_keyword;

    let non_keywords = ["MyModel", "extern", "", "pragma", "foobar", "_x", "123"];
    for word in non_keywords {
        assert!(!is_keyword(word), "'{}' не должно быть ключевым словом", word);
    }
}

/// Ключевые слова распознаются как правильные токены.
#[test]
fn keywords_produce_correct_token_count() {
    // Каждое ключевое слово — ровно один токен
    let pairs: &[(&str, usize)] = &[
        ("model", 1),
        ("state", 1),
        ("start", 1),
        ("ref cond var", 3),
        ("fn if else while for do break continue return", 9),
    ];

    for (src, expected_count) in pairs {
        let count = token_count(src);
        assert_eq!(count, *expected_count, "Для '{}' ожидалось {} токенов", src, expected_count);
        let errors = collect_errors(src);
        assert!(errors.is_empty(), "Ошибки при разборе ключевых слов '{}': {:?}", src, errors);
    }
}

// ─────────────────────────── Тесты числовых литералов ───────────────────────

/// Десятичные числа разбираются корректно.
#[test]
fn decimal_numbers_lex_without_errors() {
    let cases = ["0", "42", "255", "1000", "999999"];
    for src in cases {
        let errors = collect_errors(src);
        assert!(errors.is_empty(), "Ошибка при разборе числа '{}': {:?}", src, errors);
        assert_eq!(token_count(src), 1, "Число '{}' должно давать 1 токен", src);
    }
}

/// Шестнадцатеричные числа разбираются корректно.
#[test]
fn hex_numbers_lex_without_errors() {
    let cases = ["0xFF", "0x0", "0x1A", "0xDEAD_BEEF"];
    for src in cases {
        let errors = collect_errors(src);
        assert!(errors.is_empty(), "Ошибка при разборе hex '{}': {:?}", src, errors);
    }
}

/// Отрицательное число разбирается как один токен `Number`.
#[test]
fn negative_number_lexes_as_one_token() {
    // -42 разбирается как Number(-42) когда цифра сразу после -
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
        "Ожидалась EndOfFileInHex или MissingNumber, получено: {:?}", errors[0]
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
        "Ожидалась MissingExponent, получено: {:?}", errors[0]
    );
}

// ─────────────────────────── Тесты строковых литералов ──────────────────────

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

/// Строка в одинарных кавычках разбирается корректно.
#[test]
fn single_quoted_string_lexes_ok() {
    let errors = collect_errors("'hello'");
    assert!(errors.is_empty(), "Ошибки при строке в одинарных кавычках: {:?}", errors);
}

/// Незакрытая строка порождает `EndOfFileInString`.
#[test]
fn unclosed_string_produces_eof_in_string_error() {
    let errors = collect_errors("\"unclosed");
    assert!(!errors.is_empty(), "Незакрытая строка должна давать ошибку");
    assert!(
        matches!(errors[0], LexicalError::EndOfFileInString(_)),
        "Ожидалась EndOfFileInString, получено: {:?}", errors[0]
    );
}

/// Строка с escape-последовательностями разбирается корректно.
#[test]
fn string_with_escape_sequences_lexes_ok() {
    let errors = collect_errors(r#""line1\nline2\ttab""#);
    assert!(errors.is_empty(), "Ошибки при строке с escape: {:?}", errors);
}

// ───────────────────────────── Тесты операторов ─────────────────────────────

/// Все однобайтовые операторы разбираются в ровно 1 токен.
#[test]
fn single_char_operators_lex_ok() {
    let ops = ["+", "-", "*", "/", "%", "=", "!", "<", ">", "&", "|", "^", "~", ".", ":", ";", ",", "(", ")", "{", "}", "[", "]", "#"];
    for op in ops {
        let errors = collect_errors(op);
        assert!(errors.is_empty(), "Ошибка при операторе '{}': {:?}", op, errors);
        assert_eq!(token_count(op), 1, "Оператор '{}' должен давать 1 токен", op);
    }
}

/// Двухсимвольные операторы разбираются корректно.
#[test]
fn two_char_operators_lex_ok() {
    let ops = ["**", "==", "!=", "<=", ">=", "<<", ">>", "&&", "||"];
    for op in ops {
        let errors = collect_errors(op);
        assert!(errors.is_empty(), "Ошибка при операторе '{}': {:?}", op, errors);
        assert_eq!(token_count(op), 1, "Оператор '{}' должен давать 1 токен", op);
    }
}

/// `>>` и `<<` — разные токены.
#[test]
fn shift_operators_are_distinct() {
    // `<<` — ShiftLeft; `>>` — ShiftRight
    assert_eq!(token_count("<<"), 1);
    assert_eq!(token_count(">>"), 1);
    // Оба вместе дают 2 токена
    assert_eq!(token_count("<< >>"), 2);
}

/// `ShiftRight` отображается как `>>` (исправление бага).
#[test]
fn shift_right_display_is_correct() {
    let token = Token::ShiftRight;
    assert_eq!(token.to_string(), ">>", "ShiftRight должен отображаться как '>>'");
    assert_ne!(token.to_string(), "<<", "ShiftRight НЕ должен отображаться как '<<'");
}

/// Display токенов совпадает с исходным кодом.
#[test]
fn token_display_matches_source() {
    // Для каждого ключевого слова/оператора — проверяем Display
    let display_cases: &[(&str, Token)] = &[
        ("model",    Token::Model),
        ("state",    Token::State),
        ("start",    Token::Start),
        ("ref",      Token::Reference),
        ("cond",     Token::Condition),
        ("var",      Token::Variable),
        ("const",    Token::Constant),
        ("port",     Token::Port),
        ("fn",       Token::Function),  // keyword "fn" → Token::Function, Display → "fn"
        ("true",     Token::True),
        ("false",    Token::False),
        ("next",     Token::Next),
    ];
    for (src, expected_token) in display_cases {
        let display = expected_token.to_string();
        assert_eq!(&display, src, "Display для токена {:?} должен быть '{}'", expected_token, src);
    }
}

// ─────────────────────────── Тесты комментариев ─────────────────────────────

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

/// Пустой комментарий `//` (без текста) разбирается без ошибок.
#[test]
fn empty_line_comment_lexes_ok() {
    let errors = collect_errors("//\nmodel");
    assert!(errors.is_empty(), "Пустой комментарий вызвал ошибку: {:?}", errors);
}

/// Несколько комментариев — все собраны.
#[test]
fn multiple_comments_are_all_collected() {
    let src = "// первый\n// второй\n/// третий\nmodel";
    assert_eq!(comment_count(src), 3, "Ожидались 3 комментария");
    assert_eq!(token_count(src), 1, "После комментариев — только 'model'");
}

// ─────────────────────── Тесты ошибочных конструкций ────────────────────────

/// Неизвестный символ `@` порождает `UnrecognisedToken`.
#[test]
fn unknown_character_at_produces_error() {
    let errors = collect_errors("@");
    assert!(!errors.is_empty(), "Символ '@' должен давать ошибку");
    assert!(
        matches!(errors[0], LexicalError::UnrecognisedToken(_, _)),
        "Ожидалась UnrecognisedToken, получено: {:?}", errors[0]
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
    // Каждый '@' — отдельная ошибка (каждый с пробелом = два UnrecognisedToken)
    assert!(errors.len() >= 2, "Два символа '@' должны дать не менее 2 ошибок");
}

// ─────────────────────── Тесты позиций токенов ──────────────────────────────

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

// ──────────────────────────── Тесты идентификаторов ─────────────────────────

/// Идентификаторы разбираются без ошибок.
#[test]
fn identifiers_lex_without_errors() {
    let ids = ["abc", "ABC", "_private", "$dollar", "CamelCase", "x1", "_123"];
    for id in ids {
        let errors = collect_errors(id);
        assert!(errors.is_empty(), "Ошибка при идентификаторе '{}': {:?}", id, errors);
        assert_eq!(token_count(id), 1, "'{}' должен давать 1 токен", id);
    }
}

/// Идентификатор, совпадающий с ключевым словом, распознаётся как ключевое слово.
#[test]
fn keyword_is_not_identifier() {
    // 'model' — ключевое слово, а не идентификатор
    let src = "model".to_string();
    let errors = collect_errors(&src);
    assert!(errors.is_empty());
    // Проверяем через Display что это Model, а не Identifier
    let mut comments = Vec::new();
    let mut errs = Vec::new();
    let tokens: Vec<_> = Lexer::new(&src, 0, &mut comments, &mut errs).collect();
    assert_eq!(tokens.len(), 1);
    assert!(matches!(tokens[0].1, Token::Model), "model должен быть Token::Model");
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

// ─────────────────── Тест полного набора BuT-конструкций ────────────────────

/// Полный пример программы лексируется без ошибок.
#[test]
fn complete_but_program_lexes_without_errors() {
    let src = r#"
/// Пример полной BuT-программы
type u8 = [bit;8];
const MAX: u8 = 0xFF;
port LED: u8 = 00100000;
var counter: u8 = 0;
cond IsMax = counter = 255;
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
    assert!(errors.is_empty(), "Полная программа вызвала ошибки: {:?}", errors);
}
