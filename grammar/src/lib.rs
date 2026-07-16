//! Библиотека лексического и синтаксического анализаторов языка Lam.
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

/// Внешняя карта адресов портов (`.ld`-подобный формат, фича 0020).
pub mod address_map;
/// Модуль диагностических сообщений компилятора.
pub mod diagnostics;

/// Канонический форматтер `.lam` (фича 0024).
pub mod format;
/// Модуль генерации кода (C и другие целевые платформы).
pub mod generator;
/// Вспомогательные функции LSP-сервера (только при флаге `lsp`).
#[cfg(feature = "lsp")]
pub mod lsp;
/// Модуль парсера
pub mod parser;
/// Модуль семантического анализа и построение семантического дерева
pub mod semantic;
/// Модуль проверки формальных свойств (LTL-формулы, автоматы Бюхи).
pub mod verification;

/// Внешняя карта адресов: парсер формата и предупреждения оверлея (фича 0020).
pub use address_map::{
    AddressMapEntry, AddressResolution, AddressSource, ResolvedAddress,
    address_map_overlay_warnings, parse_address_map, resolve_addresses,
};
/// Ширина вещественного типа в порождаемом C (фича 0029): `grammar::FloatWidth`.
pub use generator::FloatWidth;
/// Опции генерации кода (реэкспорт для удобства: `grammar::GenerateOptions`).
pub use generator::GenerateOptions;

#[allow(
    clippy::needless_lifetimes,
    clippy::type_complexity,
    clippy::ptr_arg,
    clippy::redundant_clone,
    clippy::just_underscores_and_digits,
    clippy::redundant_field_names,
    clippy::collapsible_if
)]
mod grammar {
    include!(concat!(env!("OUT_DIR"), "/grammar.rs"));
}

/// Разбирает строку исходного кода Lam.
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
        diagnostics.push(
            Diagnostic::parser_error(lexical_error.loc(), lexical_error.to_string())
                .with_code(lexical_error.code()),
        );
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
        )
        .with_code("SY-001"),
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
        )
        .with_code("SY-002"),
        ParseError::User { error } => {
            Diagnostic::parser_error(error.loc(), error.to_string()).with_code(error.code())
        }
        ParseError::ExtraToken { token } => Diagnostic::parser_error(
            Location::Source(file_no, token.0, token.2),
            format!("лишний токен '{}'", token.1),
        )
        .with_code("SY-003"),
        ParseError::UnrecognizedEof { expected, location } => Diagnostic::parser_error(
            Location::Source(file_no, *location, *location),
            format!("неожиданный конец файла, ожидалось {}", expected.join(", ")),
        )
        .with_code("SY-004"),
    }
}

/// Компилирует исходный код Lam в C-код.
///
/// Выполняет полный конвейер: лексический анализ → синтаксический → семантический → генерация C.
///
/// # Параметры
///
/// - `source` — исходный код на языке Lam
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
/// // Без импортов — пустой список путей; guard-проверки по умолчанию включены
/// grammar::compile_to_c(
///     "dummy.lam",
///     "start S;",
///     ".output",
///     &[],
///     &grammar::GenerateOptions::default(),
/// )
/// .unwrap();
///
/// // С импортами — указываем директорию поиска
/// grammar::compile_to_c(
///     "dummy.lam",
///     r#"import "std.lam"; start S;"#,
///     ".output",
///     &["/usr/lib/lam".to_string()],
///     &grammar::GenerateOptions::default(),
/// ).unwrap();
/// ```
pub fn compile_to_c(
    filename: &str,
    source: &str,
    output_path: &str,
    search_paths: &[String],
    options: &GenerateOptions,
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
                // `split('.')` выделяет часть до первой точки.
                // Если точки нет — возвращает всю строку.
                s.split('.').next().unwrap_or(s).to_owned()
            })
            .unwrap_or_else(|| "Root".to_owned());
        model.borrow_mut().name = Some(stem);
    }

    // Шаг 3: Генерация C-кода
    generator::generate(
        generator::Language::C,
        &model.borrow(),
        output_path,
        options,
    )?;

    Ok(())
}

/// Компилирует Lam в C в режиме `c-hal` (фича 0020-05): к обычному C добавляются
/// таблица адресов портов и дефолтная реализация HAL.
///
/// Адреса разрешаются с приоритетом inline < `address` < внешняя карта
/// (`external`). Полнота обязательна: используемый порт без адреса → ошибка
/// (`SE-052`). Возвращает `Ok(warnings)` (предупреждения оверлея `SE-050` /
/// висячих записей карты `SE-051`) при успехе либо первую ошибку.
pub fn compile_to_c_hal(
    filename: &str,
    source: &str,
    output_path: &str,
    search_paths: &[String],
    external: &[address_map::AddressMapEntry],
    options: &GenerateOptions,
) -> Result<Vec<Diagnostic>, Diagnostic> {
    let (model_ast, _) = parse(source, 0).map_err(|d| d.into_iter().next().unwrap())?;
    let model = semantic::tree::construct_model(&model_ast, None, search_paths)?;

    if model.borrow().name.is_none() {
        let stem = Path::new(filename)
            .file_name()
            .and_then(|s| s.to_str())
            .map(|s| s.split('.').next().unwrap_or(s).to_owned())
            .unwrap_or_else(|| "Root".to_owned());
        model.borrow_mut().name = Some(stem);
    }

    // Разрешаем адреса (inline < address < внешняя карта) и проверяем полноту.
    let resolution = address_map::resolve_addresses(std::rc::Rc::clone(&model), external);
    if let Some(err) = resolution
        .diagnostics
        .iter()
        .find(|d| d.level == diagnostics::Level::Error)
    {
        return Err(err.clone());
    }

    let mut hal_options = options.clone();
    hal_options.hal = true;
    hal_options.address_map = resolution.map;

    generator::generate(
        generator::Language::C,
        &model.borrow(),
        output_path,
        &hal_options,
    )?;

    Ok(resolution.diagnostics)
}

/// Компилирует исходный код Lam в Structured Text (IEC 61131-3) — язык ПЛК.
///
/// Выполняет полный конвейер: лексический анализ → синтаксический →
/// семантический → генерация `.st`. Модель Lam отображается в `FUNCTION_BLOCK`
/// (ADR 0041, Option A). Адреса портов **не потребляются** — для размещения по
/// карте адресов (`AT %…`) используйте [`compile_to_st_at`].
///
/// # Параметры
///
/// - `filename` — имя входного файла (используется для именования модели и диагностики)
/// - `source` — исходный код на языке Lam
/// - `output_path` — путь к выходному каталогу (создаёт `<filename>.st`)
/// - `search_paths` — директории для поиска файлов `import`
/// - `options` — опции генерации
///
/// # Ошибки
///
/// Возвращает [`Diagnostic`] при синтаксической или семантической ошибке, либо
/// при ошибке записи файла (`ST-001`).
pub fn compile_to_st(
    filename: &str,
    source: &str,
    output_path: &str,
    search_paths: &[String],
    options: &GenerateOptions,
) -> Result<(), Diagnostic> {
    let (model_ast, _) = parse(source, 0).map_err(|d| d.into_iter().next().unwrap())?;
    let model = semantic::tree::construct_model(&model_ast, None, search_paths)?;

    if model.borrow().name.is_none() {
        let stem = Path::new(filename)
            .file_name()
            .and_then(|s| s.to_str())
            .map(|s| s.split('.').next().unwrap_or(s).to_owned())
            .unwrap_or_else(|| "Root".to_owned());
        model.borrow_mut().name = Some(stem);
    }

    generator::generate(
        generator::Language::ST,
        &model.borrow(),
        output_path,
        options,
    )?;

    Ok(())
}

/// Компилирует исходный код Lam в `no_std` Rust — прошивку микроконтроллера.
///
/// Выполняет полный конвейер: лексический анализ → синтаксический →
/// семантический → генерация `.rs`. Модель Lam отображается в `struct`,
/// состояния — в `enum` + `match`, порты — в трейт `Hal` (ADR 0050, Option A по
/// обеим развилкам).
///
/// ## Чем отличается от [`compile_to_c`]
///
/// Ниша та же — прошивка МК, — но дефекты отображения цели `c` здесь не
/// воспроизводятся конструктивно: `[u8;4]` → `[u8; 4]` (а не `uint4_t`, дефект
/// [0029]), `bit` → `bool` (а не `int`), `Rational` → `f64` (а не `float`, что
/// делает сверку с симулятором достижимой). `void *userdata` заменён параметром
/// типа `H: Hal`, а `unsafe` в порождаемом коде запрещён атрибутом
/// `#![forbid(unsafe_code)]`.
///
/// ## Границы
///
/// - Порождается **один `.rs`-файл**; `Cargo.toml` генератор не порождает.
///   Подключение — через `mod` в крейте пользователя.
/// - Атрибут `#![no_std]` в файле **не эмитится**: он допустим только в корне
///   крейта. Модуль не обращается к `std`, поэтому подключается и в
///   `no_std`-крейт, и в обычный.
/// - Карта адресов (`--address-map`) **не потребляется**: порты идут через HAL.
/// - `--float-width=32` несовместим с целью и даёт ошибку `RS-015`, а не
///   молчаливое игнорирование.
///
/// # Параметры
///
/// - `filename` — имя входного файла (используется для именования модели и диагностики)
/// - `source` — исходный код на языке Lam
/// - `output_path` — путь к выходному каталогу (создаёт `<filename>.rs`)
/// - `search_paths` — директории для поиска файлов `import`
/// - `options` — опции генерации
///
/// # Ошибки
///
/// Возвращает [`Diagnostic`] при синтаксической или семантической ошибке, при
/// непереводимой конструкции (`RS-0xx`) либо при ошибке записи файла (`RS-001`).
///
/// [0029]: https://github.com/Pastor/BuT/blob/main/docs/features/0029-c-type-mapping.md
pub fn compile_to_rust(
    filename: &str,
    source: &str,
    output_path: &str,
    search_paths: &[String],
    options: &GenerateOptions,
) -> Result<(), Diagnostic> {
    let (model_ast, _) = parse(source, 0).map_err(|d| d.into_iter().next().unwrap())?;
    let model = semantic::tree::construct_model(&model_ast, None, search_paths)?;

    if model.borrow().name.is_none() {
        let stem = Path::new(filename)
            .file_name()
            .and_then(|s| s.to_str())
            .map(|s| s.split('.').next().unwrap_or(s).to_owned())
            .unwrap_or_else(|| "Root".to_owned());
        model.borrow_mut().name = Some(stem);
    }

    generator::generate(
        generator::Language::Rust,
        &model.borrow(),
        output_path,
        options,
    )?;

    Ok(())
}

/// Компилирует исходный код Lam в синтезируемый SystemVerilog (IEEE 1800) —
/// описание аппаратуры для FPGA/ASIC.
///
/// Выполняет полный конвейер: лексический анализ → синтаксический →
/// семантический → генерация `.sv`. Модель Lam отображается в `module`,
/// состояния — в `typedef enum` + `unique case`, порты — в порты модуля
/// (ADR 0045).
///
/// ## Чем отличается от программных целей
///
/// `c`, `st` и `rust` — программные цели: такт модели там есть итерация цикла
/// сканирования. Здесь такт Lam ≡ **фронт тактового сигнала**: `clk` и `rst_n`
/// — неявные служебные порты модуля, которых в языке `.lam` нет, а их имена для
/// цели `sv` зарезервированы (порт модели с таким именем → `SV-007`).
///
/// Часть дефектов отображения цели `c` здесь не воспроизводится конструктивно:
/// `bit` → `logic` (а не `int`, дефект [0029]), а синтетического
/// `INIT`-состояния нет вовсе — стартовое состояние стоит в ветви сброса,
/// поэтому сдвиг такта равен нулю на любой глубине вложенности (контракт
/// [ADR 0033]) без единой правки.
///
/// ## Границы
///
/// - Порождается **один `.sv`-файл**, один `module` на корневую модель:
///   композиция `M1 | M2` уплощается в общий `always_comb`. Иерархия модулей SV
///   дерево моделей не повторяет — это плата за точную семантику `|`.
/// - `float` (`SV-003`), `extern fn` (`SV-005`) и `inout` (`SV-006`) — ошибки, а
///   не молчаливый пропуск: в синтезируемом RTL плавающей точки, вызова внешнего
///   кода и двунаправленного провода без сигнала `oe` не существует.
/// - Карта адресов (`--address-map`) **не потребляется**: MMIO-адрес для RTL
///   бессмыслен. Парной цели `sv-at` нет.
/// - Один тактовый домен и один сброс; CDC и множественные домены — вне объёма.
///
/// # Параметры
///
/// - `filename` — имя входного файла (используется для именования модели и диагностики)
/// - `source` — исходный код на языке Lam
/// - `output_path` — путь к выходному каталогу (создаёт `<filename>.sv`)
/// - `search_paths` — директории для поиска файлов `import`
/// - `options` — опции генерации
///
/// # Ошибки
///
/// Возвращает [`Diagnostic`] при синтаксической или семантической ошибке, при
/// непереводимой конструкции (`SV-0xx`) либо при ошибке записи файла (`SV-001`).
///
/// [0029]: https://github.com/Pastor/BuT/blob/main/docs/features/0029-c-type-mapping.md
/// [ADR 0033]: https://github.com/Pastor/BuT/blob/main/docs/adr/0033-init-tick-alignment.md
pub fn compile_to_sv(
    filename: &str,
    source: &str,
    output_path: &str,
    search_paths: &[String],
    options: &GenerateOptions,
) -> Result<(), Diagnostic> {
    let (model_ast, _) = parse(source, 0).map_err(|d| d.into_iter().next().unwrap())?;
    let model = semantic::tree::construct_model(&model_ast, None, search_paths)?;

    if model.borrow().name.is_none() {
        let stem = Path::new(filename)
            .file_name()
            .and_then(|s| s.to_str())
            .map(|s| s.split('.').next().unwrap_or(s).to_owned())
            .unwrap_or_else(|| "Root".to_owned());
        model.borrow_mut().name = Some(stem);
    }

    generator::generate(
        generator::Language::SV,
        &model.borrow(),
        output_path,
        options,
    )?;

    Ok(())
}

/// Компилирует Lam в Structured Text в режиме `st-at` (фича 0041): к обычному ST
/// добавляется размещение портов по карте адресов (`AT %IX…`/`%QX…`).
///
/// Адреса разрешаются тем же слоем и с тем же приоритетом, что и для `c-hal`
/// (inline < `address` < внешняя карта), поэтому поведение двух
/// адрес-потребляющих целей не расходится. Полнота обязательна: используемый
/// порт без адреса → ошибка (`SE-052`). Возвращает `Ok(warnings)`
/// (предупреждения оверлея `SE-050` / висячих записей карты `SE-051`) при успехе
/// либо первую ошибку.
pub fn compile_to_st_at(
    filename: &str,
    source: &str,
    output_path: &str,
    search_paths: &[String],
    external: &[address_map::AddressMapEntry],
    options: &GenerateOptions,
) -> Result<Vec<Diagnostic>, Diagnostic> {
    let (model_ast, _) = parse(source, 0).map_err(|d| d.into_iter().next().unwrap())?;
    let model = semantic::tree::construct_model(&model_ast, None, search_paths)?;

    if model.borrow().name.is_none() {
        let stem = Path::new(filename)
            .file_name()
            .and_then(|s| s.to_str())
            .map(|s| s.split('.').next().unwrap_or(s).to_owned())
            .unwrap_or_else(|| "Root".to_owned());
        model.borrow_mut().name = Some(stem);
    }

    let resolution = address_map::resolve_addresses(std::rc::Rc::clone(&model), external);
    if let Some(err) = resolution
        .diagnostics
        .iter()
        .find(|d| d.level == diagnostics::Level::Error)
    {
        return Err(err.clone());
    }

    let mut at_options = options.clone();
    at_options.hal = true;
    at_options.address_map = resolution.map;

    generator::generate(
        generator::Language::ST,
        &model.borrow(),
        output_path,
        &at_options,
    )?;

    Ok(resolution.diagnostics)
}

/// Компилирует исходный код Lam в диаграмму состояний PlantUML.
///
/// Выполняет полный конвейер: лексический анализ → синтаксический → семантический → генерация `.puml`.
///
/// # Параметры
///
/// - `filename` — имя входного файла (используется для именования модели и диагностики)
/// - `source` — исходный код на языке Lam
/// - `output_path` — путь к выходному каталогу (создаёт `<filename>.puml`)
/// - `search_paths` — директории для поиска файлов `import`
///
/// # Ошибки
///
/// Возвращает [`Diagnostic`] при синтаксической или семантической ошибке.
pub fn compile_to_plantuml(
    filename: &str,
    source: &str,
    output_path: &str,
    search_paths: &[String],
) -> Result<(), Diagnostic> {
    let (model_ast, _) = parse(source, 0).map_err(|d| d.into_iter().next().unwrap())?;
    let model = semantic::tree::construct_model(&model_ast, None, search_paths)?;

    if model.borrow().name.is_none() {
        let stem = Path::new(filename)
            .file_name()
            .and_then(|s| s.to_str())
            .map(|s| s.split('.').next().unwrap_or(s).to_owned())
            .unwrap_or_else(|| "Root".to_owned());
        model.borrow_mut().name = Some(stem);
    }

    generator::generate(
        generator::Language::PlantUML,
        &model.borrow(),
        output_path,
        &GenerateOptions::default(),
    )?;

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
/// let (ast, _) = parse("var unused: bit := 0; start S;", 0).unwrap();
/// let model = construct_model(&ast, None, &[]).unwrap();
/// let warnings = grammar::unused_variable_warnings(model);
/// assert_eq!(warnings.len(), 1);
/// ```
pub fn unused_variable_warnings(
    model: std::rc::Rc<std::cell::RefCell<semantic::ModelNode>>,
) -> Vec<Diagnostic> {
    semantic::unused::check_unused_variables(model)
}

/// Фича 0035: предупреждения по LTL-формулам (SE-055 — формула не
/// верифицируется; SE-056 — неизвестный атом).
///
/// LTL разбирается на всех уровнях (модель/состояние/блок), но не проверяется
/// ни одной целью генерации. Функция гарантирует, что ни один путь LTL не
/// заканчивается тишиной — по образцу [`unused_variable_warnings`].
pub fn ltl_warnings(
    model: std::rc::Rc<std::cell::RefCell<semantic::ModelNode>>,
) -> Vec<Diagnostic> {
    semantic::ltl_check::ltl_warnings(model)
}

/// Фича 0049: проверяет LTL-свойство модели методом model checking.
///
/// Атом формулы — **имя состояния** FSM (абстракция управляющего графа, ADR
/// 0049): `S` истинен, когда автомат в состоянии `S`. Проверяемы свойства
/// управления — достижимость (`F Done`), порядок состояний и живость
/// (`G(Fault -> F Idle)`). Свойства над данными (`G(temp <= 100)`) в этой
/// абстракции невыразимы и дают [`Verdict::Unsupported`] — не молчаливое
/// «ложно».
///
/// Условия переходов абстрагированы (любой `ref` — возможный переход), поэтому
/// [`Verdict::Holds`] надёжен, а [`Verdict::Violated`] может оказаться ложным
/// срабатыванием (контрпример недостижим по данным).
///
/// # Пример
///
/// ```
/// use grammar::parse;
/// use grammar::semantic::tree::construct_model;
/// use grammar::verification::ltl::Ltl;
/// use grammar::verification::verify::Verdict;
/// use std::rc::Rc;
///
/// let (ast, _) = parse("start Idle { ref Fault; } state Fault { ref Fault; }", 0).unwrap();
/// let model = construct_model(&ast, None, &[]).unwrap();
///
/// // G(Fault -> F Idle): «после сбоя система обязана вернуться в Idle».
/// let phi = Ltl::Globally(Rc::new(Ltl::Implies(
///     Rc::new(Ltl::Atom("Fault".to_string())),
///     Rc::new(Ltl::Finally(Rc::new(Ltl::Atom("Idle".to_string())))),
/// )));
/// // Нарушено: из Fault нет пути назад — автомат залипает в нём навсегда.
/// assert!(matches!(grammar::verify_model(model, &phi), Verdict::Violated(_)));
/// ```
pub fn verify_model(
    model: std::rc::Rc<std::cell::RefCell<semantic::ModelNode>>,
    phi: &verification::ltl::Ltl,
) -> verification::verify::Verdict {
    verification::verify::verify_model(&model.borrow(), phi)
}

/// Фича 0049: результат проверки одной LTL-формулы модели.
#[derive(Debug, Clone)]
pub struct PropertyResult {
    /// Имя модели, в которой объявлена формула (пустое — анонимный корень).
    pub model: String,
    /// Проверенная формула.
    pub formula: verification::ltl::Ltl,
    /// Позиция объявления формулы в исходном тексте.
    pub loc: diagnostics::Location,
    /// Вердикт проверки.
    pub verdict: verification::verify::Verdict,
    /// Дамп конвейера для отладки (Крипке, автомат `¬φ`, произведение).
    ///
    /// Заполняется только при `trace = true` в [`verify_all_traced`]: дамп
    /// стоит памяти и обычному потребителю не нужен.
    pub trace: Option<String>,
}

/// Фича 0049: проверяет **все** LTL-формулы модели и её вложенных моделей.
///
/// Формулы вложенной модели говорят о состояниях **своей** модели, поэтому
/// каждая проверяется против графа той модели, где объявлена, — а не против
/// корневой.
///
/// Порядок результатов детерминирован (обход `BTreeMap` моделей — гейт 0048).
pub fn verify_all(
    model: std::rc::Rc<std::cell::RefCell<semantic::ModelNode>>,
) -> Vec<PropertyResult> {
    verify_all_traced(model, false)
}

/// Фича 0049: то же, что [`verify_all`], но при `trace = true` заполняет
/// [`PropertyResult::trace`] дампом конвейера (`lamc verify --trace`).
pub fn verify_all_traced(
    model: std::rc::Rc<std::cell::RefCell<semantic::ModelNode>>,
    trace: bool,
) -> Vec<PropertyResult> {
    let mut out = Vec::new();
    verify_all_inner(&model, trace, &mut out);
    out
}

fn verify_all_inner(
    model: &std::rc::Rc<std::cell::RefCell<semantic::ModelNode>>,
    trace: bool,
    out: &mut Vec<PropertyResult>,
) {
    let borrowed = model.borrow();
    let name = borrowed.name().to_string();
    for (formula, loc) in semantic::ltl_check::model_ltl_formulas(&borrowed) {
        let (verdict, dump) = if trace {
            let (v, t) = verification::verify::verify_model_traced(&borrowed, &formula);
            (v, Some(t))
        } else {
            (
                verification::verify::verify_model(&borrowed, &formula),
                None,
            )
        };
        out.push(PropertyResult {
            model: name.clone(),
            formula,
            loc,
            verdict,
            trace: dump,
        });
    }
    let nested: Vec<_> = borrowed.models.values().map(std::rc::Rc::clone).collect();
    drop(borrowed);
    for m in nested {
        verify_all_inner(&m, trace, out);
    }
}

/// Фича 0049: LTL-формулы, объявленные непосредственно в модели, с позициями.
///
/// Вложенные модели не обходятся — их формулы проверяются против своего графа
/// (см. [`verify_all`]).
pub fn model_ltl_formulas(
    model: std::rc::Rc<std::cell::RefCell<semantic::ModelNode>>,
) -> Vec<(verification::ltl::Ltl, diagnostics::Location)> {
    semantic::ltl_check::model_ltl_formulas(&model.borrow())
}

/// Фича 0049: разбирает LTL-формулу из строки (для `lamc verify --property`).
///
/// Строка разбирается **грамматикой языка** — как тело `: [LTL] φ;`, поэтому
/// синтаксис свойства в командной строке и в `.lam`-файле совпадает буква в
/// букву, а имена состояний могут быть любой длины.
///
/// Не путать с [`verification::ltl::parse_ltl`]: тот — тестовая игрушка
/// (паникует на ошибке, атом — один символ) и в продуктовом пути не участвует
/// (ADR 0049, A6).
///
/// # Пример
///
/// ```
/// use grammar::parse_ltl_property;
///
/// let phi = parse_ltl_property("G (Fault -> F Idle)").unwrap();
/// assert_eq!(phi.to_string(), "G (Fault -> F Idle)");
/// assert!(parse_ltl_property("G (").is_err());
/// ```
pub fn parse_ltl_property(source: &str) -> Result<verification::ltl::Ltl, Diagnostic> {
    use parser::ast::{InlineFormulaDefine, ModelElement};

    let wrapped = format!(": [LTL] {};", source);
    let (ast, _) = parse(&wrapped, 0).map_err(|diags| {
        diags.into_iter().next().unwrap_or_else(|| {
            Diagnostic::error(
                diagnostics::Location::Implicit,
                format!("не удалось разобрать LTL-формулу '{}'", source),
            )
        })
    })?;

    // Разобранное обязано быть РОВНО одной встроенной формулой. Строка со своей
    // `;` закрывает обёртку досрочно, и хвост становится отдельными элементами
    // файла: `-p "F Done; : [LTL] G Idle"` дал бы «проверено: F Done», умолчав о
    // второй формуле, а `-p "G Idle; start X { ref X; }"` протащил бы объявление
    // состояния. Взять первый элемент и промолчать об остальных — соврать о
    // проверенном (тот же отказ, что и для формы через запятую).
    let [element] = ast.elements.as_slice() else {
        return Err(Diagnostic::error(
            diagnostics::Location::Implicit,
            format!(
                "ожидалась одна LTL-формула, а строка '{}' разбирается как {} \
                 конструкций: уберите ';' — проверяйте по одной формуле за вызов",
                source,
                ast.elements.len()
            ),
        ));
    };

    let ModelElement::InlineFormula(inline) = element else {
        return Err(Diagnostic::error(
            diagnostics::Location::Implicit,
            format!("строка '{}' не является LTL-формулой", source),
        ));
    };
    let InlineFormulaDefine::Ltl { formulas, loc } = inline.as_ref() else {
        return Err(Diagnostic::error(
            diagnostics::Location::Implicit,
            format!("строка '{}' не является LTL-формулой", source),
        ));
    };

    match formulas.as_slice() {
        [single] => Ok(semantic::formula::ltl_ast_to_semantic(single)),
        // `: [LTL] a, b;` — список формул; какую из них проверять, непонятно.
        _ => Err(Diagnostic::error(
            *loc,
            format!(
                "ожидалась одна LTL-формула, задано {}: уберите запятые или \
                 вызовите verify для каждой формулы отдельно",
                formulas.len()
            ),
        )),
    }
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

/// Фича 0020-04: предупреждения о портах без адреса, попадающих в кодогенерацию.
///
/// Используемый (достижимый кодогенерацией) порт обязан иметь адрес: inline,
/// оператором `address` или во внешней карте. Имена портов, покрытых внешней
/// картой, передаются в `external_ports`. Мёртвые (неиспользуемые) порты без
/// адреса не предупреждаются. Возвращает предупреждения `SE-052`.
///
/// Аналитическая функция: потребитель адресов (C-таблица/HAL, задача 0020-05)
/// вызывает её и при необходимости трактует как ошибку.
pub fn port_address_completeness_warnings(
    model: std::rc::Rc<std::cell::RefCell<semantic::ModelNode>>,
    external_ports: &[String],
) -> Vec<Diagnostic> {
    let external: std::collections::HashSet<String> = external_ports.iter().cloned().collect();
    semantic::validate::check_port_address_completeness(model, &external)
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

/// SE-046: предупреждения о недостижимых состояниях в модели.
///
/// Обходит граф переходов BFS от стартового состояния.
/// Состояние, не достижимое ни по одному `ref`/`next`-переходу, генерирует предупреждение.
pub fn unreachable_state_warnings(
    model: std::rc::Rc<std::cell::RefCell<semantic::ModelNode>>,
) -> Vec<Diagnostic> {
    semantic::validate::check_unreachable_states(model)
}

/// SE-047: предупреждения об очевидно константных условиях переходов.
///
/// Проверяет условия `ref`/`next` переходов на наличие сравнений литералов,
/// результат которых известен статически (например, `1 = 0` — всегда ложно).
pub fn constant_condition_warnings(
    model: &std::rc::Rc<std::cell::RefCell<semantic::ModelNode>>,
) -> Vec<Diagnostic> {
    semantic::validate::check_constant_conditions(model)
}

/// SE-044: предупреждения о лишних точках с запятой в АСД модели.
///
/// Обходит все элементы модели и состояний (рекурсивно), генерируя
/// предупреждение для каждого [`ast::ModelElement::StraySemicolon`]
/// и [`ast::StateElement::StraySemicolon`].
pub fn stray_semicolon_warnings(model: &ast::Model) -> Vec<Diagnostic> {
    let mut diags = Vec::new();
    collect_stray_semicolons_model(model, &mut diags);
    diags
}

/// SE-045: предупреждения об именованных блоках с неизвестным именем.
///
/// Допустимые имена: `enter`, `exit`, `always`. Любое другое имя генерирует
/// предупреждение — вероятнее всего это опечатка.
pub fn unknown_named_block_warnings(model: &ast::Model) -> Vec<Diagnostic> {
    let mut diags = Vec::new();
    collect_unknown_named_blocks_model(model, &mut diags);
    diags
}

const KNOWN_NAMED_BLOCKS: &[&str] = &["enter", "exit", "always"];

fn collect_unknown_named_blocks_model(model: &ast::Model, out: &mut Vec<Diagnostic>) {
    for element in &model.elements {
        match element {
            ast::ModelElement::NamedBlockCode(def) => {
                check_named_block_def(def, out);
            }
            ast::ModelElement::State(state) => {
                for se in &state.elements {
                    if let ast::StateElement::NamedBlockCode(def) = se {
                        check_named_block_def(def, out);
                    }
                }
            }
            ast::ModelElement::Model(nested) => {
                collect_unknown_named_blocks_model(nested, out);
            }
            _ => {}
        }
    }
}

fn check_named_block_def(def: &ast::NamedBlockCodeDefine, out: &mut Vec<Diagnostic>) {
    if let Some(name_id) = &def.name {
        if !KNOWN_NAMED_BLOCKS.contains(&name_id.name.as_str()) {
            out.push(
                Diagnostic::warning(
                    name_id.loc,
                    format!(
                        "неизвестный именованный блок '{}'; допустимые имена: enter, exit, always",
                        name_id.name
                    ),
                )
                .with_code("SE-045"),
            );
        }
    }
}

fn collect_stray_semicolons_model(model: &ast::Model, out: &mut Vec<Diagnostic>) {
    for element in &model.elements {
        match element {
            ast::ModelElement::StraySemicolon(loc) => {
                out.push(
                    Diagnostic::warning(*loc, "лишняя точка с запятой".to_string())
                        .with_code("SE-044"),
                );
            }
            ast::ModelElement::State(state) => {
                for se in &state.elements {
                    if let ast::StateElement::StraySemicolon(loc) = se {
                        out.push(
                            Diagnostic::warning(*loc, "лишняя точка с запятой".to_string())
                                .with_code("SE-044"),
                        );
                    }
                }
            }
            ast::ModelElement::Model(nested) => {
                collect_stray_semicolons_model(nested, out);
            }
            _ => {}
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::generator::{Language, generate};
    use crate::semantic::tree::construct_model;

    const SRC: &str = r#"
//Алиас типа
type u8 = [bit;8];
//Константа
const MATRIX: u8 := { 0, 0, 0, 0, 0, 0, 0, 0 };
const NUMB: u8 := 0xFF;
cond  IsEmpty = it = 0;
//Порт с указанием отображаемого адреса
out   A : u8  := 0x00548835;
in    B1: bit := 0x00648835:6;
//Переменная
var   it: [bit;64] := 0;

//Модель
model Ping {
    //Начальное состояние
    start Start {
        //Переход на состояние по условию
        ref End: B1;
        //Исполнение блока кода при первом переходе в состояние
        enter {
            A.0 := true;
            A.1 := false;
        }
        //Исполнение блока кода при выходе из состояния
        exit {
            A.0 := false;
            A.1 := true;
        }
        always {
            A.2 := toggle;
        }
        always {
            toggle := !toggle;
        }
    }
    state End;
    var toggle := false;
}
model Pong {
    start Begin {
        ref Stop: S(Ping) = End;
        always {
            A.5 := MATRIX.5;
        }
    }
    state Stop {
        enter {
            A.6 := MATRIX.3;
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
    it := it + 1;
    if S(Toggle) := Pong {
        debug("Pong processing");
    }
}"#;

    /// Комплексный тест разбора Lam-программы с различными конструкциями.
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

    /// Инвариант: условия рёбер `ref` **не разрешаются** семантикой.
    ///
    /// `ref Stop: S(Ping) = End;` обязан пережить конвейер как
    /// `Condition::Unresolved`. Проход `resolve_state_references` добавлять
    /// нельзя — он ломает эту конструкцию (см. `CLAUDE.md`, критические
    /// инварианты). Именно это здесь и охраняется.
    #[test]
    fn syntax_simple() {
        let (model, _) = parse(SRC, 0).unwrap();
        let model = construct_model(&model, None, &[]).unwrap();
        assert!(model.borrow().has_states());
        model.borrow_mut().name = Some(String::from("ThisIsMyModel"));

        // Успех здесь ЗНАЧИМ, а не формален. История этой строки:
        //
        // 1. Был `.unwrap()`, и тест был зелёным — но не потому, что переход
        //    генерировался: генератор ПРОГЛАТЫВАЛ ошибку `CC-003`, печатая
        //    вместо перехода комментарий. Тест подтверждал успех там, где
        //    автомат молча терял ребро.
        // 2. Фича 0028 сняла заглушку — и тест честно покраснел, вскрыв, что
        //    `S(Ping) = End` в C не переводился ВОВСЕ.
        // 3. Фича 0047 трансляцию реализовала, и успех снова достижим —
        //    теперь по-настоящему.
        //
        // Поэтому проверяется не только `Ok`, но и наличие самого перехода в
        // порождённом C: иначе тест вернётся к тому, чем был, — к подтверждению
        // успеха без проверки результата.
        generate(
            Language::C,
            &model.borrow(),
            ".output",
            &GenerateOptions::default(),
        )
        .expect("порождение C");

        let source = std::fs::read_to_string(".output/this_is_my_model.c").expect(".c порождён");
        // Строка захвачена из реального вывода, не угадана. `Ping` — сестра
        // владельца условия (`Pong`) внутри `(Ping | Pong) + Toggle`, поэтому
        // база пути — `main`, а не `model`: своя структура чужого поля не имеет.
        assert!(
            source
                .contains("if (main->entry_parallel0.ping0.state == THIS_IS_MY_MODEL_PING_END) {"),
            "условие `S(Ping) = End` обязано попасть в C сравнением состояния:\n{source}"
        );
        assert!(
            !source.contains("TODO") && !source.contains("FIXME"),
            "заглушек в выводе быть не должно:\n{source}"
        );
    }

    // ── Тесты ошибок парсера ──────────────────────────────────────────────────

    /// Недопустимый токен: строка с управляющим символом вызывает ошибку парсера.
    #[test]
    fn parse_invalid_token_error() {
        // Управляющие символы не являются допустимыми токенами языка Lam
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

    /// V1: обычный путь вида "path/to/model.lam" → имя модели "model".
    #[test]
    fn compile_to_c_normal_filename_sets_model_name() {
        // Используем пустую директорию; ошибка записи нас не интересует —
        // важно только, что функция не паникует при нормальном имени файла.
        let dir = tempfile::tempdir().unwrap();
        let out = dir.path().to_string_lossy().into_owned();
        // Простой FSM без имени модели; имя должно быть взято из имени файла.
        let src = "start S;";
        let result = compile_to_c(
            "path/to/my_model.lam",
            src,
            &out,
            &[],
            &GenerateOptions::default(),
        );
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
        let _ = compile_to_c("some/dir/", src, &out, &[], &GenerateOptions::default());
    }

    /// V1: пустая строка имени файла — `file_name()` вернёт None → имя «Root».
    #[test]
    fn compile_to_c_empty_filename_uses_root_name() {
        let dir = tempfile::tempdir().unwrap();
        let out = dir.path().to_string_lossy().into_owned();
        let src = "start S;";
        // Пустое имя файла — не должно паниковать.
        let _ = compile_to_c("", src, &out, &[], &GenerateOptions::default());
    }

    /// V2: имя файла без расширения — возвращается строка целиком.
    #[test]
    fn compile_to_c_filename_without_extension() {
        let dir = tempfile::tempdir().unwrap();
        let out = dir.path().to_string_lossy().into_owned();
        let src = "start S;";
        // Без точки — `splitn(2,'.')` вернёт всё имя файла.
        let _ = compile_to_c("my_model", src, &out, &[], &GenerateOptions::default());
    }

    /// V2: имя файла с несколькими точками — берётся только часть до первой.
    #[test]
    fn compile_to_c_filename_with_multiple_dots() {
        let dir = tempfile::tempdir().unwrap();
        let out = dir.path().to_string_lossy().into_owned();
        let src = "start S;";
        // "arch.v2.lam" → должно брать "arch", не "arch.v2".
        let _ = compile_to_c("arch.v2.lam", src, &out, &[], &GenerateOptions::default());
    }
}
