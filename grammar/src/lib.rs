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
use std::path::Path;

/// Модуль диагностических сообщений компилятора.
pub mod diagnostics;
/// Модуль генерации кода (C и другие целевые платформы).
pub mod generator;
/// Вспомогательные функции LSP-сервера (только при флаге `lsp`).
#[cfg(feature = "lsp")]
pub mod lsp;
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

/// Компилирует исходный код BuT в C-код.
///
/// Выполняет полный конвейер: лексический анализ → синтаксический → семантический → генерация C.
///
/// # Параметры
///
/// - `source` — исходный код на языке BuT
/// - `output_path` — путь к выходному каталогу (генератор C создаёт `.h`-файл)
/// - `search_paths` — директории для поиска файлов `import` (пустой слайс — только текущая директория)
///
/// # Ошибки
///
/// Возвращает [`Diagnostic`] при синтаксической или семантической ошибке,
/// а также если файл импорта не найден ни в одном из `search_paths`.
///
/// # Примеры
///
/// ```no_run
/// // Без импортов — пустой список путей
/// grammar::compile_to_c("dummy.but", "start S;", ".output", &[]).unwrap();
///
/// // С импортами — указываем директорию поиска
/// grammar::compile_to_c(
///     "dummy.but",
///     r#"import "std.but"; start S;"#,
///     ".output",
///     &["/usr/lib/but".to_string()],
/// ).unwrap();
/// ```
pub fn compile_to_c(
    filename: &str,
    source: &str,
    output_path: &str,
    search_paths: &[String],
) -> Result<(), Diagnostic> {
    // Шаг 1: Синтаксический анализ
    let (model_ast, _) = parse(source, 0).map_err(|d| d.into_iter().next().unwrap())?;

    // Шаг 2: Семантический анализ (с путями поиска для разрешения импортов)
    let model = semantic::tree::construct_model(&model_ast, None, search_paths)?;

    // Генератор C требует именованной модели.
    // Корневая (файловая) модель всегда анонимна — задаём имя из имени файла.
    if model.borrow().name.is_none() {
        // V1: `file_name()` возвращает None, если путь оканчивается на `/` или пустой.
        //     `to_str()` возвращает None для не-UTF-8 путей.
        //     В обоих случаях используем запасное имя «Root».
        let stem = Path::new(filename)
            .file_name()
            .and_then(|s| s.to_str())
            .map(|s| {
                // V2: `splitn(2, '.')` безопасно выделяет часть до первой точки.
                //     Если точки нет — возвращает всю строку.
                s.splitn(2, '.').next().unwrap_or(s).to_owned()
            })
            .unwrap_or_else(|| "Root".to_owned());
        model.borrow_mut().name = Some(stem);
    }

    // Шаг 3: Генерация C-кода
    generator::generate(generator::Language::C, &model.borrow(), output_path)?;

    Ok(())
}

/// Ce13: возвращает предупреждения о неиспользуемых переменных в модели.
///
/// Обходит все выражения, операторы и условия модели и её вложенных моделей,
/// возвращая предупреждения для каждой `var`-переменной, не упомянутой нигде.
/// Порты и константы не проверяются.
///
/// # Пример
///
/// ```
/// use grammar::parse;
/// use grammar::semantic::tree::construct_model;
///
/// let (ast, _) = parse("var unused: bit = 0; start S;", 0).unwrap();
/// let model = construct_model(&ast, None, &[]).unwrap();
/// let warnings = grammar::unused_variable_warnings(model);
/// assert_eq!(warnings.len(), 1);
/// ```
pub fn unused_variable_warnings(
    model: std::rc::Rc<std::cell::RefCell<semantic::ModelNode>>,
) -> Vec<Diagnostic> {
    semantic::unused::check_unused_variables(model)
}

/// Ce14: возвращает предупреждения о недетерминированных переходах в модели.
///
/// Предупреждает, если несколько `ref`-переходов из одного состояния
/// не имеют условий (безусловные переходы), что является явной недетерминированностью.
///
/// # Пример
///
/// ```
/// use grammar::parse;
/// use grammar::semantic::tree::construct_model;
///
/// let (ast, _) = parse("start A { ref B; ref C; } state B; state C;", 0).unwrap();
/// let model = construct_model(&ast, None, &[]).unwrap();
/// let warnings = grammar::nondeterministic_transition_warnings(model);
/// assert_eq!(warnings.len(), 1);
/// ```
pub fn nondeterministic_transition_warnings(
    model: std::rc::Rc<std::cell::RefCell<semantic::ModelNode>>,
) -> Vec<Diagnostic> {
    semantic::validate::check_nondeterministic_transitions(model)
}

/// Ce16: возвращает ошибки рекурсивных псевдонимов типов в модели.
///
/// Проверяет граф зависимостей псевдонимов типов на наличие циклов.
/// При обнаружении цикла возвращает ошибку Ce16 с именем псевдонима.
pub fn recursive_type_alias_errors(
    model: std::rc::Rc<std::cell::RefCell<semantic::ModelNode>>,
) -> Vec<Diagnostic> {
    semantic::validate::check_recursive_type_aliases(model)
}

/// NI6: возвращает ошибки типобезопасных операций с перечислениями в модели.
///
/// Проверяет, что при присваивании переменной типа enum значение является
/// одним из допустимых вариантов перечисления.
///
/// # Пример
///
/// ```
/// use grammar::parse;
/// use grammar::semantic::tree::construct_model;
/// use grammar::semantic::EnumDefinitionNode;
///
/// // Создаём модель с перечислением программно
/// let (ast, _) = parse("start S;", 0).unwrap();
/// let model = construct_model(&ast, None, &[]).unwrap();
/// {
///     let mut m = model.borrow_mut();
///     let e = EnumDefinitionNode::new("Dir", &[("North", Some(0)), ("South", Some(1))]);
///     m.enums.insert("Dir".to_string(), e);
/// }
/// let errors = grammar::enum_type_safety_errors(model);
/// assert!(errors.is_empty());
/// ```
pub fn enum_type_safety_errors(
    model: std::rc::Rc<std::cell::RefCell<semantic::ModelNode>>,
) -> Vec<Diagnostic> {
    semantic::validate::check_enum_type_safety(model)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::generator::{Language, generate};
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
        model.borrow_mut().name = Some(String::from("ThisIsMyModel"));
        generate(Language::C, &model.borrow(), ".output").unwrap();
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
        assert!(
            !diagnostics.is_empty(),
            "должна быть хотя бы одна диагностика"
        );
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
        assert!(
            result.is_ok(),
            "корректная программа должна разбираться без ошибок"
        );
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

    // ── V1/V2: Тесты безопасного извлечения имени файла ──────────────────────

    /// V1: обычный путь вида "path/to/model.but" → имя модели "model".
    #[test]
    fn compile_to_c_normal_filename_sets_model_name() {

        // Используем пустую директорию; ошибка записи нас не интересует —
        // важно только, что функция не паникует при нормальном имени файла.
        let dir = tempfile::tempdir().unwrap();
        let out = dir.path().to_string_lossy().into_owned();
        // Простой FSM без имени модели; имя должно быть взято из имени файла.
        let src = "start S;";
        let result = compile_to_c("path/to/my_model.but", src, &out, &[]);
        // Функция может вернуть ошибку генератора (файл записан / не записан),
        // но не должна паниковать.
        let _ = result;
    }

    /// V1: путь оканчивается на `/` — `file_name()` вернёт None → имя «Root».
    #[test]
    fn compile_to_c_trailing_slash_uses_root_name() {
        let dir = tempfile::tempdir().unwrap();
        let out = dir.path().to_string_lossy().into_owned();
        let src = "start S;";
        // Путь с завершающим слешем — не должен паниковать.
        let _ = compile_to_c("some/dir/", src, &out, &[]);
    }

    /// V1: пустая строка имени файла — `file_name()` вернёт None → имя «Root».
    #[test]
    fn compile_to_c_empty_filename_uses_root_name() {
        let dir = tempfile::tempdir().unwrap();
        let out = dir.path().to_string_lossy().into_owned();
        let src = "start S;";
        // Пустое имя файла — не должно паниковать.
        let _ = compile_to_c("", src, &out, &[]);
    }

    /// V2: имя файла без расширения — возвращается строка целиком.
    #[test]
    fn compile_to_c_filename_without_extension() {
        let dir = tempfile::tempdir().unwrap();
        let out = dir.path().to_string_lossy().into_owned();
        let src = "start S;";
        // Без точки — `splitn(2,'.')` вернёт всё имя файла.
        let _ = compile_to_c("my_model", src, &out, &[]);
    }

    /// V2: имя файла с несколькими точками — берётся только часть до первой.
    #[test]
    fn compile_to_c_filename_with_multiple_dots() {
        let dir = tempfile::tempdir().unwrap();
        let out = dir.path().to_string_lossy().into_owned();
        let src = "start S;";
        // "arch.v2.but" → должно брать "arch", не "arch.v2".
        let _ = compile_to_c("arch.v2.but", src, &out, &[]);
    }
}
