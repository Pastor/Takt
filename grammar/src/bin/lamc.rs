//! Компилятор Lam — утилита командной строки.
//!
//! # Использование
//!
//! ```text
//! lamc compile [--target c] [-I dir1:dir2] [--verbose | --quiet] <input.lam> [-o output_dir]
//! lamc compile input.lam           # вывод в ./output
//! lamc --help                      # справка
//! ```
//!
//! # Поиск файлов импорта
//!
//! Флаг `-I` (или `--include-dirs`) задаёт директории, в которых ищутся
//! файлы, указанные в `import`-выражениях. Директории разделяются двоеточием
//! на Unix или точкой с запятой на Windows. Флаг можно указывать несколько раз.
//!
//! ```text
//! # Unix: два пути через двоеточие
//! lamc compile -I /usr/lib/lam:/home/user/lam main.lam -o out
//!
//! # Несколько флагов -I
//! lamc compile -I /usr/lib/lam -I /home/user/lam main.lam
//!
//! # Слитная форма без пробела
//! lamc compile -I/usr/lib/lam main.lam
//! ```
//!
//! # Уровни диагностики
//!
//! - `--verbose` (`-v`): расширенный вывод — показывает все предупреждения
//!   и полные пути к файлам. Взаимоисключается с `--quiet`.
//! - `--quiet` (`-q`): тихий режим — выводит только ошибки компиляции.
//!   Взаимоисключается с `--verbose`.
//!
//! # Целевые платформы
//!
//! - `c` — генерация C-заголовочного файла (по умолчанию)
//! - `c-hal` — C плюс таблица адресов портов и дефолтный HAL (фича 0020)
//! - `plantuml` — генерация диаграммы состояний в формате PlantUML
//! - `st` — генерация Structured Text (IEC 61131-3), язык ПЛК (фича 0041)
//! - `st-at` — ST плюс размещение портов по карте адресов (`AT %…`)

use std::env;
use std::fs;
use std::io;
use std::path::{Path, PathBuf};
use std::process;
use std::rc::Rc;

/// Параметры компиляции, разобранные из аргументов командной строки.
#[derive(Debug, PartialEq)]
pub struct CompileOptions {
    /// Целевой язык генерации (по умолчанию `"c"`).
    pub target: String,
    /// Путь к входному `.lam`-файлу.
    pub input_file: String,
    /// Путь к выходному файлу или директории.
    pub output_path: String,
    /// Список директорий для поиска файлов `import`.
    ///
    /// Заполняется через флаги `-I` / `--include-dirs`.
    /// Порядок элементов соответствует порядку указания флагов —
    /// первый найденный файл используется (приоритет первого пути).
    pub include_dirs: Vec<String>,
    /// Расширенный диагностический вывод.
    ///
    /// При `true` выводятся все предупреждения и полные пути к файлам.
    /// Взаимоисключается с [`quiet`](CompileOptions::quiet).
    pub verbose: bool,
    /// Тихий режим.
    ///
    /// При `true` подавляются все сообщения, кроме ошибок компиляции.
    /// Взаимоисключается с [`verbose`](CompileOptions::verbose).
    pub quiet: bool,
    /// Флаг включения генерации проверок Guard-формул.
    pub guard_enable: bool,
    /// Путь к внешней карте адресов (`.ld`-подобный формат, фича 0020).
    ///
    /// Заполняется флагом `--address-map`. Если задан, карта разбирается и
    /// накладывается на модель оверлеем (с предупреждениями об оверлее/висячих
    /// записях). Понижение адреса в целевой код — задача 0020-05.
    pub address_map: Option<String>,
    /// Ширина вещественного типа в порождаемом C (фича 0029).
    ///
    /// Заполняется флагом `--float-width=32|64`; умолчание — 64 (`double`),
    /// что совпадает с точностью симулятора (f64). Значение 32 (`float`) — для
    /// платформ, где 8-байтное чтение недопустимо.
    pub float_width: grammar::FloatWidth,
}

/// Разбивает строку путей на отдельные директории.
///
/// Платформо-зависимые разделители:
/// - `:`  — на Unix/macOS (стандарт POSIX, аналогично `PATH`).
/// - `;`  — на Windows (стандарт Windows, аналогично `%PATH%`).
///
/// На Unix `:`  всегда является разделителем.
/// На Windows `;` всегда является разделителем.
/// Пустые сегменты и пробелы по краям отбрасываются.
///
/// Для кросс-платформенных сценариев предпочтительнее использовать
/// несколько флагов `-I`: `-I /a -I /b`.
///
/// # Примеры
///
/// ```text
/// # Unix
/// split_include_dirs("/a:/b:/c")  →  ["/a", "/b", "/c"]
/// split_include_dirs("/a::/b")    →  ["/a", "/b"]
/// split_include_dirs("")          →  []
///
/// # Windows
/// split_include_dirs("C:\\a;C:\\b")  →  ["C:\\a", "C:\\b"]
/// ```
pub fn split_include_dirs(s: &str) -> Vec<String> {
    // На Windows путь может начинаться с буквы диска «X:», поэтому разделителем
    // служит точка с запятой. На Unix используется двоеточие (стандарт POSIX).
    #[cfg(windows)]
    let sep = ';';
    #[cfg(not(windows))]
    let sep = ':';

    s.split(sep)
        .map(str::trim)
        .filter(|seg| !seg.is_empty())
        .map(String::from)
        .collect()
}

/// Разбирает аргументы подкоманды `compile` в [`CompileOptions`].
///
/// Принимает слайс без имени программы и без `"compile"` в начале
/// (т.е. аргументы, следующие сразу после `"compile"`).
///
/// # Поддерживаемые флаги
///
/// | Флаг                   | Описание                                  |
/// |------------------------|-------------------------------------------|
/// | `--target`, `-t`       | Целевой язык (`c`, по умолчанию)          |
/// | `--output`, `-o`       | Путь к выходному файлу/директории         |
/// | `--include-dirs`, `-I` | Пути поиска импортов (`:` или `;`)        |
/// | `-I<путь>`             | Слитная форма без пробела                 |
/// | `--verbose`, `-v`      | Расширенный вывод (все предупреждения)    |
/// | `--quiet`, `-q`        | Тихий режим (только ошибки)               |
///
/// Флаги `--verbose` и `--quiet` взаимоисключающие.
/// Флаг `-I` можно повторять; все пути объединяются в один список.
///
/// # Ошибки
///
/// Возвращает строку с описанием ошибки при:
/// - отсутствии входного файла,
/// - флаге без обязательного аргумента,
/// - одновременном указании `--verbose` и `--quiet`,
/// - неизвестном флаге.
///
/// # Примеры
///
/// ```
/// # use grammar_bin::{parse_compile_args, CompileOptions};
/// let args = vec![
///     "-I".to_string(), "/lib/lam:/usr/lam".to_string(),
///     "main.lam".to_string(),
/// ];
/// let opts = parse_compile_args(&args).unwrap();
/// assert_eq!(opts.include_dirs, vec!["/lib/lam", "/usr/lam"]);
/// assert_eq!(opts.input_file, "main.lam");
/// assert_eq!(opts.target, "c");
/// assert!(!opts.verbose);
/// assert!(!opts.quiet);
/// ```
pub fn parse_compile_args(args: &[String]) -> Result<CompileOptions, String> {
    let mut target = "c".to_string();
    let mut input_file: Option<String> = None;
    let mut output_path: Option<String> = None;
    let mut include_dirs: Vec<String> = Vec::new();
    let mut verbose = false;
    let mut quiet = false;
    let mut guard_enable = true;
    let mut address_map: Option<String> = None;
    let mut float_width = grammar::FloatWidth::default();

    let mut i = 0;
    while i < args.len() {
        let arg = args[i].as_str();
        match arg {
            "--target" | "-t" => {
                i += 1;
                match args.get(i) {
                    Some(v) => target = v.clone(),
                    None => return Err(format!("{} требует аргумент", arg)),
                }
            }
            "-o" | "--output" => {
                i += 1;
                match args.get(i) {
                    Some(v) => output_path = Some(v.clone()),
                    None => return Err(format!("{} требует аргумент", arg)),
                }
            }
            "-I" | "--include-dirs" => {
                i += 1;
                match args.get(i) {
                    Some(v) => include_dirs.extend(split_include_dirs(v)),
                    None => return Err(format!("{} требует аргумент", arg)),
                }
            }
            // Слитная форма: -I/path или -I/a:/b
            s if s.starts_with("-I") && s.len() > 2 => {
                include_dirs.extend(split_include_dirs(&s[2..]));
            }
            "--verbose" | "-v" => {
                verbose = true;
            }
            "--quiet" | "-q" => {
                quiet = true;
            }
            "--guard-enable" => {
                guard_enable = true;
            }
            "--guard-disable" => {
                guard_enable = false;
            }
            "--address-map" => {
                i += 1;
                match args.get(i) {
                    Some(v) => address_map = Some(v.clone()),
                    None => return Err(format!("{} требует аргумент", arg)),
                }
            }
            // Фича 0029: ширина вещественного типа. Принимаются обе формы —
            // `--float-width=32` (как в тест-плане) и `--float-width 32` (как у
            // прочих флагов): расходиться с соседями по синтаксису нечего.
            "--float-width" => {
                i += 1;
                match args.get(i) {
                    Some(v) => float_width = parse_float_width(v)?,
                    None => return Err(format!("{} требует аргумент: 32 или 64", arg)),
                }
            }
            a if a.starts_with("--float-width=") => {
                float_width = parse_float_width(&a["--float-width=".len()..])?;
            }
            // Позиционный аргумент — входной файл
            a if !a.starts_with('-') => {
                input_file = Some(a.to_string());
            }
            unknown => {
                return Err(format!("неизвестный флаг '{}'", unknown));
            }
        }
        i += 1;
    }

    // Флаги --verbose и --quiet взаимоисключающие
    if verbose && quiet {
        return Err(
            "флаги --verbose и --quiet взаимоисключающие: нельзя указывать оба одновременно"
                .to_string(),
        );
    }

    let input_file = input_file.ok_or_else(|| "не указан входной файл".to_string())?;
    let output_path = output_path.unwrap_or_else(|| "output".to_string());

    Ok(CompileOptions {
        target,
        input_file,
        output_path,
        include_dirs,
        verbose,
        quiet,
        guard_enable,
        address_map,
        float_width,
    })
}

/// Печатает ошибку компиляции: код, сообщение и приложенные заметки.
///
/// Единая точка на все цели (0028-01). Прежде печать была **расходящейся**:
/// цели `c-hal`/`st-at` показывали код (`Ошибка компиляции [SE-049]: …`), а
/// `c`/`st` — нет, и пользователь одной и той же диагностики видел разное в
/// зависимости от цели. Заметки не печатала **ни одна** ветка, из-за чего
/// приложенная причина (`Diagnostic::error_with_note`) до пользователя не
/// доезжала вовсе. Тот же урок, что в закрытой 0026: расхождение веток —
/// причина того, что одна работает, а другая нет.
fn print_compile_error(diag: &grammar::diagnostics::Diagnostic) {
    eprintln!(
        "Ошибка компиляции [{}]: {}",
        diag.code.as_deref().unwrap_or("?"),
        diag.message
    );
    for note in &diag.notes {
        eprintln!("  примечание: {}", note.message);
    }
}

/// Собирает опции генерации из разобранных аргументов CLI.
///
/// Одна точка сборки на все цели: иначе новая опция доезжает до одних целей и
/// не доезжает до других, а расхождение обнаруживается на выходе генератора.
/// Цели `st`/`st-at` `float_width` не потребляют — у ST своё отображение типов
/// (`generator/st/st_type.rs`).
fn generate_options(options: &CompileOptions) -> grammar::GenerateOptions {
    let mut generate = grammar::GenerateOptions::new(options.guard_enable);
    generate.float_width = options.float_width;
    generate
}

/// Разбирает значение флага `--float-width` (фича 0029).
///
/// Допустимы только 32 и 64: иных вещественных типов у цели `c` нет. Прочее —
/// ошибка разбора, а не молчаливое умолчание: `--float-width=16` должен сказать,
/// что такой ширины не бывает, а не собрать код с другой точностью.
fn parse_float_width(value: &str) -> Result<grammar::FloatWidth, String> {
    match value {
        "32" => Ok(grammar::FloatWidth::W32),
        "64" => Ok(grammar::FloatWidth::W64),
        other => Err(format!(
            "--float-width: недопустимое значение '{}' (допустимо 32 или 64)",
            other
        )),
    }
}

// ─────────────────────────────────────────────────────────────────────────────
// Подкоманда `fmt` — канонический форматтер (фича 0024, задача 0024-03)
// ─────────────────────────────────────────────────────────────────────────────

/// Опции подкоманды `fmt`.
#[derive(Debug, Default, PartialEq, Eq)]
pub struct FmtOptions {
    /// Не писать файлы; вернуть ненулевой код, если ввод отличается от канона.
    ///
    /// Режим для CI: проверяет, отформатирован ли репозиторий.
    pub check: bool,
    /// Читать из stdin, писать в stdout.
    pub stdin: bool,
    /// Файлы и каталоги (каталоги обходятся рекурсивно по `*.lam`).
    pub paths: Vec<String>,
}

/// Разбирает аргументы подкоманды `fmt`.
///
/// Принимает слайс без имени программы и без `"fmt"` в начале.
pub fn parse_fmt_args(args: &[String]) -> Result<FmtOptions, String> {
    let mut options = FmtOptions::default();
    for arg in args {
        match arg.as_str() {
            "--check" => options.check = true,
            "--stdin" => options.stdin = true,
            other if other.starts_with('-') => {
                return Err(format!("неизвестный флаг '{other}'"));
            }
            other => options.paths.push(other.to_string()),
        }
    }
    if options.stdin && !options.paths.is_empty() {
        return Err("--stdin несовместим с указанием файлов".to_string());
    }
    if !options.stdin && options.paths.is_empty() {
        return Err("укажите файлы/каталоги или --stdin".to_string());
    }
    Ok(options)
}

/// Рекурсивно собирает `*.lam` из файла или каталога.
fn collect_lam_files(path: &Path, out: &mut Vec<PathBuf>) -> Result<(), String> {
    if path.is_file() {
        out.push(path.to_path_buf());
        return Ok(());
    }
    if !path.is_dir() {
        return Err(format!("путь не найден: {}", path.display()));
    }
    let entries = fs::read_dir(path).map_err(|e| format!("{}: {e}", path.display()))?;
    for entry in entries.flatten() {
        let child = entry.path();
        if child.is_dir() {
            collect_lam_files(&child, out)?;
        } else if child.extension().is_some_and(|e| e == "lam") {
            out.push(child);
        }
    }
    Ok(())
}

/// Исполняет подкоманду `fmt`; возвращает код завершения процесса.
///
/// Коды: `0` — всё канонично (или файлы отформатированы); `1` — при `--check`
/// найдены отличия либо произошла ошибка. Ненулевой код при `--check` — это и
/// есть контракт для CI (критерий A4).
fn run_fmt(options: &FmtOptions) -> i32 {
    if options.stdin {
        let mut source = String::new();
        if let Err(e) = io::Read::read_to_string(&mut io::stdin(), &mut source) {
            eprintln!("Ошибка чтения stdin: {e}");
            return 1;
        }
        return match grammar::format::format_source(&source) {
            Ok(formatted) => {
                if options.check {
                    if formatted == source {
                        0
                    } else {
                        eprintln!("stdin: требуется форматирование");
                        1
                    }
                } else {
                    print!("{formatted}");
                    0
                }
            }
            Err(e) => {
                eprintln!("Ошибка форматирования stdin: {e}");
                1
            }
        };
    }

    let mut files = Vec::new();
    for path in &options.paths {
        if let Err(e) = collect_lam_files(Path::new(path), &mut files) {
            eprintln!("Ошибка: {e}");
            return 1;
        }
    }

    let mut need_format = Vec::new();
    let mut failed = 0usize;
    let mut changed = 0usize;

    for file in &files {
        let source = match fs::read_to_string(file) {
            Ok(s) => s,
            Err(e) => {
                eprintln!("Ошибка чтения '{}': {e}", file.display());
                failed += 1;
                continue;
            }
        };
        let formatted = match grammar::format::format_source(&source) {
            Ok(f) => f,
            Err(e) => {
                // Отказ форматтера — не «файл канонический». Сообщаем и считаем
                // ошибкой: молча пропустить значило бы соврать в --check.
                eprintln!("Ошибка форматирования '{}': {e}", file.display());
                failed += 1;
                continue;
            }
        };
        if formatted == source {
            continue;
        }
        if options.check {
            need_format.push(file.clone());
        } else if let Err(e) = fs::write(file, &formatted) {
            eprintln!("Ошибка записи '{}': {e}", file.display());
            failed += 1;
        } else {
            changed += 1;
        }
    }

    if options.check {
        for file in &need_format {
            eprintln!("требуется форматирование: {}", file.display());
        }
        if !need_format.is_empty() {
            eprintln!(
                "\nНе отформатировано файлов: {} из {}",
                need_format.len(),
                files.len()
            );
        }
        if need_format.is_empty() && failed == 0 {
            return 0;
        }
        return 1;
    }

    if changed > 0 {
        eprintln!("Отформатировано файлов: {changed} из {}", files.len());
    }
    if failed > 0 { 1 } else { 0 }
}

// ─────────────────────────────────────────────────────────────────────────────
// Подкоманда `verify` — model checking по LTL (фича 0049, задача 0049-04)
// ─────────────────────────────────────────────────────────────────────────────

/// Опции подкоманды `verify`.
#[derive(Debug, Default, PartialEq, Eq)]
pub struct VerifyOptions {
    /// Путь к проверяемому `.lam`-файлу.
    pub input_file: String,
    /// Директории поиска файлов `import`.
    pub include_dirs: Vec<String>,
    /// Свойство из командной строки (`--property "G (Fault -> F Idle)"`).
    ///
    /// `None` — проверяются все формулы `: [LTL] φ;`, объявленные в файле.
    pub property: Option<String>,
    /// Печатать трассу конвейера (Крипке, автомат `¬φ`, произведение).
    pub trace: bool,
}

/// Задаёт проверяемое свойство, отвергая повтор флага.
///
/// Второй `--property` молча затирал бы первый, и `lamc verify -p "F Done" -p
/// "G Idle" m.lam` отчитался бы «проверено свойств: 1; все держатся» — про
/// первую формулу пользователь узнал бы только из исходников. Отказ по тому же
/// правилу, что и для второго файла.
fn set_property(options: &mut VerifyOptions, value: &str) -> Result<(), String> {
    if let Some(first) = &options.property {
        return Err(format!(
            "свойство задано дважды ('{first}' и '{value}'); \
             verify проверяет одно свойство за вызов"
        ));
    }
    options.property = Some(value.to_string());
    Ok(())
}

/// Разбирает аргументы подкоманды `verify`.
///
/// Принимает слайс без имени программы и без `"verify"` в начале.
pub fn parse_verify_args(args: &[String]) -> Result<VerifyOptions, String> {
    let mut options = VerifyOptions::default();
    let mut i = 0;
    while i < args.len() {
        let arg = args[i].as_str();
        match arg {
            "--property" | "-p" => {
                i += 1;
                let value = args
                    .get(i)
                    .ok_or_else(|| format!("флаг '{arg}' требует значение — LTL-формулу"))?;
                set_property(&mut options, value)?;
            }
            "--trace" => options.trace = true,
            "--include-dirs" | "-I" => {
                i += 1;
                let value = args
                    .get(i)
                    .ok_or_else(|| format!("флаг '{arg}' требует значение"))?;
                options.include_dirs.extend(split_include_dirs(value));
            }
            other if other.starts_with("--property=") => {
                set_property(&mut options, &other["--property=".len()..])?;
            }
            // Слитная форма `-I/путь` — как в подкоманде compile.
            other if other.starts_with("-I") && other.len() > 2 => {
                options.include_dirs.extend(split_include_dirs(&other[2..]));
            }
            other if other.starts_with('-') => {
                return Err(format!("неизвестный флаг '{other}'"));
            }
            other => {
                if !options.input_file.is_empty() {
                    return Err(format!(
                        "verify принимает один файл; лишний аргумент '{other}'"
                    ));
                }
                options.input_file = other.to_string();
            }
        }
        i += 1;
    }
    if options.input_file.is_empty() {
        return Err("укажите .lam-файл для проверки".to_string());
    }
    Ok(options)
}

/// Выполняет подкоманду `verify`; возвращает код возврата процесса.
///
/// Код `0` — все проверенные свойства держатся; `1` — есть нарушение,
/// непроверяемое свойство или ошибка разбора (R8/A1).
fn run_verify(options: &VerifyOptions) -> i32 {
    let source = match fs::read_to_string(&options.input_file) {
        Ok(s) => s,
        Err(e) => {
            eprintln!("Ошибка чтения файла '{}': {e}", options.input_file);
            return 1;
        }
    };

    let (ast, _) = match grammar::parse(&source, 0) {
        Ok(parsed) => parsed,
        Err(diags) => {
            for d in diags {
                eprintln!(
                    "Ошибка разбора [{}]: {}",
                    d.code.as_deref().unwrap_or("?"),
                    d.message
                );
            }
            return 1;
        }
    };

    let model = match grammar::semantic::tree::construct_model(&ast, None, &options.include_dirs) {
        Ok(m) => m,
        Err(d) => {
            eprintln!(
                "Семантическая ошибка [{}]: {}",
                d.code.as_deref().unwrap_or("?"),
                d.message
            );
            return 1;
        }
    };

    // Свойства: либо одно из --property, либо все объявленные в файле.
    let results = match &options.property {
        Some(text) => {
            let phi = match grammar::parse_ltl_property(text) {
                Ok(p) => p,
                Err(d) => {
                    eprintln!("Ошибка разбора свойства: {}", d.message);
                    return 1;
                }
            };
            let (verdict, trace) =
                grammar::verification::verify::verify_model_traced(&model.borrow(), &phi);
            // Область — корневая модель файла; помечать её именем незачем,
            // файл назван в командной строке.
            vec![grammar::PropertyResult {
                model: String::new(),
                loc: grammar::diagnostics::Location::Implicit,
                verdict,
                formula: phi,
                trace: options.trace.then_some(trace),
            }]
        }
        None => grammar::verify_all_traced(Rc::clone(&model), options.trace),
    };

    if results.is_empty() {
        eprintln!(
            "В файле '{}' нет LTL-формул. Объявите свойство как `: [LTL] φ;` \
             или задайте его флагом --property \"φ\".",
            options.input_file
        );
        return 1;
    }

    print_verify_results(&results)
}

/// Печатает вердикты и возвращает код возврата процесса.
fn print_verify_results(results: &[grammar::PropertyResult]) -> i32 {
    use grammar::verification::verify::Verdict;

    let mut failures = 0usize;
    for result in results {
        if let Some(trace) = &result.trace {
            print!("{trace}");
        }
        let scope = if result.model.is_empty() {
            String::new()
        } else {
            format!(" [модель {}]", result.model)
        };
        match &result.verdict {
            Verdict::Holds => {
                println!("СВОЙСТВО ДЕРЖИТСЯ{scope}: {}", result.formula);
            }
            Verdict::Violated(cex) => {
                failures += 1;
                println!("СВОЙСТВО НАРУШЕНО{scope}: {}", result.formula);
                println!("  контрпример: {}", cex.trace());
                println!(
                    "  (абстракция управления: условия переходов не учитываются — \
                     контрпример может быть недостижим по данным)"
                );
            }
            Verdict::Unsupported(atoms) => {
                failures += 1;
                println!("СВОЙСТВО НЕ ПРОВЕРЕНО{scope}: {}", result.formula);
                // Причин у неизвестного атома три, и путать их нельзя: это может
                // быть переменная (данные вне абстракции), состояние ВЛОЖЕННОЙ
                // модели (её граф отдельный) или опечатка.
                println!(
                    "  атом(ы) {} — не имена состояний проверяемой модели.",
                    atoms.join(", ")
                );
                println!(
                    "  Атом обязан быть именем состояния: проверяются свойства управления \
                     (достижимость, порядок, живость)."
                );
                println!(
                    "  Вне охвата: свойства над данными (`G (temp <= 100)`) и состояния \
                     вложенных моделей — их формулы объявляйте внутри самой модели."
                );
            }
            Verdict::NoStartState => {
                failures += 1;
                println!("СВОЙСТВО НЕ ПРОВЕРЕНО{scope}: {}", result.formula);
                println!("  у модели нет стартового состояния — проверять нечего");
            }
        }
    }

    // Итог печатается в stdout вместе с вердиктами: разведи их по потокам —
    // и в терминале итог всплывёт выше вердиктов из-за буферизации.
    if failures > 0 {
        println!(
            "\nПроверено свойств: {}; не держится/не проверено: {failures}",
            results.len()
        );
        return 1;
    }
    println!("\nПроверено свойств: {}; все держатся", results.len());
    0
}

/// Выводит справку по использованию утилиты в stderr.
fn print_usage() {
    eprintln!("Использование: lamc compile [флаги] <input.lam> [-o <output>]");
    eprintln!("               lamc fmt [--check] [--stdin] <файлы/каталоги>");
    eprintln!("               lamc verify [--property \"φ\"] [--trace] <input.lam>");
    eprintln!("               lamc --help");
    eprintln!();
    eprintln!("Флаги:");
    eprintln!("  --target, -t <c>       Целевой язык (по умолчанию: c)");
    eprintln!("  --output, -o <путь>    Путь к выходному файлу");
    eprintln!("  --include-dirs, -I <dirs>  Пути поиска файлов import, разделённые ':' или ';'");
    eprintln!("                             Можно повторять: -I /a -I /b  или  -I /a:/b");
    eprintln!("  --verbose, -v          Расширенный вывод: все предупреждения и полные пути");
    eprintln!("  --quiet, -q            Тихий режим: только ошибки");
    eprintln!("                         Флаги --verbose и --quiet взаимоисключающие");
    eprintln!("  --guard-enable         Включить генерацию проверок Guard-формул (по умолчанию)");
    eprintln!("  --guard-disable        Выключить генерацию проверок Guard-формул");
    eprintln!("  --address-map <файл>   Внешняя карта адресов портов (.ld-подобный формат)");
    eprintln!("  --float-width=32|64    Ширина вещественного типа в C: float или double");
    eprintln!(
        "                         По умолчанию 64 (double) — совпадает с точностью симулятора"
    );
    eprintln!();
    eprintln!("Целевые платформы:");
    eprintln!("  c         Генерация C-заголовочного файла");
    eprintln!("  c-hal     C + таблица адресов портов и дефолтный HAL (фича 0020)");
    eprintln!("  plantuml  Генерация диаграммы состояний PlantUML (.puml)");
    eprintln!("  st        Генерация Structured Text IEC 61131-3 (.st), язык ПЛК (фича 0041)");
    eprintln!("  st-at     ST + размещение портов по карте адресов (AT %...)");
    eprintln!();
    eprintln!("Примеры:");
    eprintln!("  lamc compile main.lam");
    eprintln!("  lamc compile -I /lib/lam:/home/user/lam main.lam -o build/");
    eprintln!("  lamc compile -I /lib/lam -I /home/user/lam --target c main.lam");
    eprintln!("  lamc compile --verbose main.lam");
    eprintln!("  lamc compile --quiet main.lam -o dist/");
    eprintln!();
    eprintln!("Подкоманда fmt (канонический форматтер):");
    eprintln!("  --check      Не писать файлы; ненулевой код, если нужен формат (для CI)");
    eprintln!("  --stdin      Читать из stdin, писать в stdout");
    eprintln!("  lamc fmt examples/            # отформатировать каталог на месте");
    eprintln!("  lamc fmt --check examples/    # проверить (CI)");
    eprintln!("  cat a.lam | lamc fmt --stdin  # отформатировать поток");
    eprintln!();
    eprintln!("Подкоманда verify (проверка LTL-свойств, model checking — фича 0049):");
    eprintln!("  --property, -p \"φ\"  Проверить одну формулу из командной строки");
    eprintln!("                      Без флага проверяются все `: [LTL] φ;` файла");
    eprintln!("  --trace             Печатать конвейер (Крипке, автомат !φ, произведение)");
    eprintln!("  -I <dirs>           Пути поиска файлов import");
    eprintln!();
    eprintln!("  Атом формулы — ИМЯ СОСТОЯНИЯ: `S` истинно, когда автомат в состоянии S.");
    eprintln!("  Проверяются свойства управления: достижимость, порядок состояний, живость.");
    eprintln!("  Свойства над данными (`G (temp <= 100)`) в этой абстракции не поддержаны.");
    eprintln!("  Код возврата: 0 — все свойства держатся; 1 — нарушение/не проверено.");
    eprintln!();
    eprintln!("  lamc verify model.lam");
    eprintln!("  lamc verify --property \"F Done\" model.lam       # достижимость");
    eprintln!("  lamc verify -p \"G (Fault -> F Idle)\" model.lam  # живость");
}

fn main() {
    let args: Vec<String> = env::args().collect();

    if args.len() < 2 || args.iter().any(|a| a == "--help" || a == "-h") {
        print_usage();
        process::exit(0);
    }

    if args[1] == "fmt" {
        let options = match parse_fmt_args(&args[2..]) {
            Ok(o) => o,
            Err(e) => {
                eprintln!("Ошибка разбора аргументов: {e}");
                print_usage();
                process::exit(1);
            }
        };
        process::exit(run_fmt(&options));
    }

    if args[1] == "verify" {
        let options = match parse_verify_args(&args[2..]) {
            Ok(o) => o,
            Err(e) => {
                eprintln!("Ошибка разбора аргументов: {e}");
                print_usage();
                process::exit(1);
            }
        };
        process::exit(run_verify(&options));
    }

    if args[1] != "compile" {
        eprintln!(
            "Ошибка: неизвестная команда '{}'. Используйте 'compile', 'fmt' или 'verify'.",
            args[1]
        );
        print_usage();
        process::exit(1);
    }

    // Разбираем аргументы подкоманды (всё после "compile")
    let compile_args = &args[2..];
    let options = match parse_compile_args(compile_args) {
        Ok(o) => o,
        Err(e) => {
            eprintln!("Ошибка разбора аргументов: {}", e);
            print_usage();
            process::exit(1);
        }
    };

    // Читаем исходный файл
    let source = match fs::read_to_string(&options.input_file) {
        Ok(s) => s,
        Err(e) => {
            eprintln!("Ошибка чтения файла '{}': {}", options.input_file, e);
            process::exit(1);
        }
    };

    // Внешняя карта адресов (фича 0020): разбор один раз. В режиме `c-hal` карта
    // участвует в разрешении адресов (compile_to_c_hal); для остальных целей —
    // только информационные предупреждения об оверлее/висячих записях (0020-03).
    let external_entries: Vec<grammar::AddressMapEntry> =
        if let Some(map_path) = &options.address_map {
            let map_src = match fs::read_to_string(map_path) {
                Ok(s) => s,
                Err(e) => {
                    eprintln!("Ошибка чтения карты адресов '{}': {}", map_path, e);
                    process::exit(1);
                }
            };
            match grammar::parse_address_map(&map_src, 0) {
                Ok(entries) => entries,
                Err(diags) => {
                    for d in diags {
                        eprintln!(
                            "Ошибка карты адресов [{}]: {}",
                            d.code.as_deref().unwrap_or("?"),
                            d.message
                        );
                    }
                    process::exit(1);
                }
            }
        } else {
            Vec::new()
        };

    // Адрес-потребляющие цели: только они читают карту адресов. Для остальных
    // непустая внешняя карта — повод предупредить об оверлее «в никуда».
    let consumes_addresses = matches!(options.target.as_str(), "c-hal" | "st-at");
    if !external_entries.is_empty() && !consumes_addresses {
        let warnings = grammar::parse(&source, 0)
            .ok()
            .and_then(|(ast, _)| {
                grammar::semantic::tree::construct_model(&ast, None, &options.include_dirs).ok()
            })
            .map(|model| grammar::address_map_overlay_warnings(model, &external_entries))
            .unwrap_or_default();
        for w in warnings {
            if !options.quiet {
                eprintln!(
                    "Предупреждение [{}]: {}",
                    w.code.as_deref().unwrap_or("?"),
                    w.message
                );
            }
        }
    }

    match options.target.as_str() {
        "c-hal" => {
            match grammar::compile_to_c_hal(
                &options.input_file,
                &source,
                &options.output_path,
                &options.include_dirs,
                &external_entries,
                &generate_options(&options),
            ) {
                Ok(warnings) => {
                    for w in warnings {
                        if !options.quiet {
                            eprintln!(
                                "Предупреждение [{}]: {}",
                                w.code.as_deref().unwrap_or("?"),
                                w.message
                            );
                        }
                    }
                    if !options.quiet {
                        eprintln!(
                            "Скомпилировано: {} → {}/ (c-hal)",
                            options.input_file, options.output_path
                        );
                    }
                }
                Err(diag) => {
                    print_compile_error(&diag);
                    process::exit(1);
                }
            }
        }
        "c" => {
            if let Err(diag) = grammar::compile_to_c(
                &options.input_file,
                &source,
                &options.output_path,
                &options.include_dirs,
                &generate_options(&options),
            ) {
                print_compile_error(&diag);
                process::exit(1);
            }
            // В тихом режиме не выводим информационные сообщения
            if !options.quiet {
                if options.verbose {
                    // Расширенный вывод: полный путь к файлу и список директорий поиска
                    eprintln!(
                        "Скомпилировано: {} → {} (путей поиска: {}: {:?})",
                        fs::canonicalize(&options.input_file)
                            .map(|p| p.display().to_string())
                            .unwrap_or_else(|_| options.input_file.clone()),
                        options.output_path,
                        options.include_dirs.len(),
                        options.include_dirs,
                    );
                } else {
                    eprintln!(
                        "Скомпилировано: {} → {}/ (путей поиска: {})",
                        options.input_file,
                        options.output_path,
                        options.include_dirs.len()
                    );
                }
            }
        }
        "plantuml" => {
            if let Err(diag) = grammar::compile_to_plantuml(
                &options.input_file,
                &source,
                &options.output_path,
                &options.include_dirs,
            ) {
                print_compile_error(&diag);
                process::exit(1);
            }
            if !options.quiet {
                if options.verbose {
                    eprintln!(
                        "Скомпилировано: {} → {} (plantuml)",
                        fs::canonicalize(&options.input_file)
                            .map(|p| p.display().to_string())
                            .unwrap_or_else(|_| options.input_file.clone()),
                        options.output_path,
                    );
                } else {
                    eprintln!(
                        "Скомпилировано: {} → {}/ (plantuml)",
                        options.input_file, options.output_path,
                    );
                }
            }
        }
        "st" => {
            if let Err(diag) = grammar::compile_to_st(
                &options.input_file,
                &source,
                &options.output_path,
                &options.include_dirs,
                &generate_options(&options),
            ) {
                print_compile_error(&diag);
                process::exit(1);
            }
            if !options.quiet {
                if options.verbose {
                    eprintln!(
                        "Скомпилировано: {} → {} (st)",
                        fs::canonicalize(&options.input_file)
                            .map(|p| p.display().to_string())
                            .unwrap_or_else(|_| options.input_file.clone()),
                        options.output_path,
                    );
                } else {
                    eprintln!(
                        "Скомпилировано: {} → {}/ (st)",
                        options.input_file, options.output_path,
                    );
                }
            }
        }
        "st-at" => {
            match grammar::compile_to_st_at(
                &options.input_file,
                &source,
                &options.output_path,
                &options.include_dirs,
                &external_entries,
                &generate_options(&options),
            ) {
                Ok(warnings) => {
                    for w in warnings {
                        if !options.quiet {
                            eprintln!(
                                "Предупреждение [{}]: {}",
                                w.code.as_deref().unwrap_or("?"),
                                w.message
                            );
                        }
                    }
                    if !options.quiet {
                        eprintln!(
                            "Скомпилировано: {} → {}/ (st-at)",
                            options.input_file, options.output_path
                        );
                    }
                }
                Err(diag) => {
                    print_compile_error(&diag);
                    process::exit(1);
                }
            }
        }
        t => {
            eprintln!(
                "Ошибка: неизвестная цель '{}'. Поддерживается: c, c-hal, plantuml, st, st-at",
                t
            );
            process::exit(1);
        }
    }
}

// ─── Тесты ────────────────────────────────────────────────────────────────────

#[cfg(test)]
mod tests {

    // ── Подкоманда fmt (задача 0024-03) ──────────────────────────────────────

    #[test]
    fn fmt_args_paths() {
        let args = vec!["examples/".to_string(), "a.lam".to_string()];
        let o = parse_fmt_args(&args).unwrap();
        assert!(!o.check);
        assert!(!o.stdin);
        assert_eq!(o.paths, vec!["examples/", "a.lam"]);
    }

    #[test]
    fn fmt_args_check_flag() {
        let args = vec!["--check".to_string(), "a.lam".to_string()];
        let o = parse_fmt_args(&args).unwrap();
        assert!(o.check);
    }

    #[test]
    fn fmt_args_stdin_flag() {
        let args = vec!["--stdin".to_string()];
        let o = parse_fmt_args(&args).unwrap();
        assert!(o.stdin);
        assert!(o.paths.is_empty());
    }

    #[test]
    fn fmt_args_stdin_with_files_is_error() {
        // Контрпример: `--stdin` и файлы одновременно бессмысленны — лучше
        // отказать, чем молча проигнорировать одно из двух.
        let args = vec!["--stdin".to_string(), "a.lam".to_string()];
        assert!(parse_fmt_args(&args).is_err());
    }

    #[test]
    fn fmt_args_without_input_is_error() {
        assert!(parse_fmt_args(&[]).is_err());
    }

    // ── Подкоманда verify (задача 0049-04) ───────────────────────────────────

    fn args(list: &[&str]) -> Vec<String> {
        list.iter().map(|s| s.to_string()).collect()
    }

    #[test]
    fn verify_args_file_only() {
        let o = parse_verify_args(&args(&["model.lam"])).unwrap();
        assert_eq!(o.input_file, "model.lam");
        assert_eq!(o.property, None, "без --property проверяются формулы файла");
        assert!(!o.trace);
    }

    #[test]
    fn verify_args_property_flag() {
        let o = parse_verify_args(&args(&["--property", "G (Fault -> F Idle)", "m.lam"])).unwrap();
        assert_eq!(o.property.as_deref(), Some("G (Fault -> F Idle)"));
        assert_eq!(o.input_file, "m.lam");
    }

    /// Короткая форма `-p` и слитная `--property=` — синонимы длинной.
    #[test]
    fn verify_args_property_short_and_joined_forms() {
        let short = parse_verify_args(&args(&["-p", "F Done", "m.lam"])).unwrap();
        let joined = parse_verify_args(&args(&["--property=F Done", "m.lam"])).unwrap();
        assert_eq!(short.property.as_deref(), Some("F Done"));
        assert_eq!(joined.property.as_deref(), Some("F Done"));
    }

    #[test]
    fn verify_args_trace_flag() {
        let o = parse_verify_args(&args(&["--trace", "m.lam"])).unwrap();
        assert!(o.trace);
        assert_eq!(o.input_file, "m.lam");
    }

    #[test]
    fn verify_args_include_dirs() {
        let o = parse_verify_args(&args(&["-I", "/a", "-I/b", "m.lam"])).unwrap();
        assert_eq!(o.include_dirs, vec!["/a", "/b"]);
    }

    #[test]
    fn verify_args_without_file_is_error() {
        // Контрпример: проверять нечего — отказ, а не пустой прогон.
        assert!(parse_verify_args(&[]).is_err());
        assert!(parse_verify_args(&args(&["--property", "F Done"])).is_err());
    }

    #[test]
    fn verify_args_property_without_value_is_error() {
        assert!(parse_verify_args(&args(&["m.lam", "--property"])).is_err());
    }

    #[test]
    fn verify_args_duplicate_property_is_error() {
        // Контрпример: второй -p молча затирал бы первый, и отчёт «проверено
        // свойств: 1» умалчивал бы о невыполненной проверке.
        assert!(parse_verify_args(&args(&["-p", "F Done", "-p", "G Idle", "m.lam"])).is_err());
        assert!(
            parse_verify_args(&args(&["-p", "F Done", "--property=G Idle", "m.lam"])).is_err(),
            "смешение форм флага повтором быть не перестаёт"
        );
    }

    #[test]
    fn verify_args_second_file_is_error() {
        // Контрпример: verify работает с одним файлом; второй молча
        // проигнорировать — значит соврать о том, что проверено.
        assert!(parse_verify_args(&args(&["a.lam", "b.lam"])).is_err());
    }

    #[test]
    fn verify_args_unknown_flag_is_error() {
        assert!(parse_verify_args(&args(&["--target", "c", "m.lam"])).is_err());
    }

    #[test]
    fn fmt_args_unknown_flag_is_error() {
        let args = vec!["--verbose".to_string()];
        assert!(parse_fmt_args(&args).is_err());
    }
    use super::*;

    // ── split_include_dirs ────────────────────────────────────────────────────

    /// На Unix: разделитель `:` (POSIX-стиль аналогично PATH).
    #[cfg(not(windows))]
    #[test]
    fn split_colon_unix() {
        assert_eq!(split_include_dirs("/a:/b:/c"), vec!["/a", "/b", "/c"]);
    }

    /// На Windows: разделитель `;` (пути вида `C:\dir`).
    #[cfg(windows)]
    #[test]
    fn split_semicolon_windows() {
        assert_eq!(
            split_include_dirs("C:\\lib;C:\\usr"),
            vec!["C:\\lib", "C:\\usr"]
        );
    }

    /// Пустые сегменты пропускаются.
    #[cfg(not(windows))]
    #[test]
    fn split_empty_segments_skipped() {
        assert_eq!(split_include_dirs("/a::/b"), vec!["/a", "/b"]);
    }

    /// Ведущие и завершающие пробелы обрезаются (Unix).
    #[cfg(not(windows))]
    #[test]
    fn split_trims_whitespace() {
        assert_eq!(split_include_dirs("  /x  :  /y  "), vec!["/x", "/y"]);
    }

    /// Ведущие и завершающие пробелы обрезаются (Windows).
    #[cfg(windows)]
    #[test]
    fn split_trims_whitespace_windows() {
        assert_eq!(
            split_include_dirs("  C:\\x  ;  C:\\y  "),
            vec!["C:\\x", "C:\\y"]
        );
    }

    /// Пустая строка → пустой вектор (работает на всех платформах).
    #[test]
    fn split_empty_string() {
        let result: Vec<String> = split_include_dirs("");
        assert!(result.is_empty());
    }

    /// Один путь без разделителей (работает на всех платформах).
    #[test]
    fn split_single_path() {
        assert_eq!(split_include_dirs("/only/one"), vec!["/only/one"]);
    }

    /// Только пробелы → пустой вектор (работает на всех платформах).
    #[test]
    fn split_only_whitespace() {
        let result: Vec<String> = split_include_dirs("   ");
        assert!(result.is_empty());
    }

    /// На Unix: два пути через `;` не разделяются (`;` не является разделителем на Unix).
    #[cfg(not(windows))]
    #[test]
    fn split_semicolon_is_not_unix_separator() {
        // На Unix ';' не является разделителем — весь путь воспринимается как один
        let result = split_include_dirs("/a;/b");
        assert_eq!(result, vec!["/a;/b"], "на Unix ';' не разделяет пути");
    }

    // ── parse_compile_args: позитивные случаи ────────────────────────────────

    /// Минимальный вызов: только входной файл.
    #[test]
    fn parse_minimal() {
        let args = vec!["main.lam".to_string()];
        let opts = parse_compile_args(&args).unwrap();
        assert_eq!(opts.input_file, "main.lam");
        assert_eq!(opts.target, "c");
        assert_eq!(opts.output_path, "output");
        assert!(opts.include_dirs.is_empty());
        assert!(!opts.verbose);
        assert!(!opts.quiet);
    }

    /// Флаг `-I` с единственным путём.
    #[test]
    fn parse_single_include_dir() {
        let args = vec![
            "-I".to_string(),
            "/lib/lam".to_string(),
            "main.lam".to_string(),
        ];
        let opts = parse_compile_args(&args).unwrap();
        assert_eq!(opts.include_dirs, vec!["/lib/lam"]);
    }

    /// Флаг `-I` с двоеточием — два пути.
    #[test]
    fn parse_include_dirs_colon() {
        let args = vec![
            "-I".to_string(),
            "/lib/lam:/usr/lam".to_string(),
            "main.lam".to_string(),
        ];
        let opts = parse_compile_args(&args).unwrap();
        assert_eq!(opts.include_dirs, vec!["/lib/lam", "/usr/lam"]);
    }

    /// Флаг `-I` повторяется дважды — пути объединяются.
    #[test]
    fn parse_multiple_include_flags() {
        let args = vec![
            "-I".to_string(),
            "/a".to_string(),
            "-I".to_string(),
            "/b".to_string(),
            "main.lam".to_string(),
        ];
        let opts = parse_compile_args(&args).unwrap();
        assert_eq!(opts.include_dirs, vec!["/a", "/b"]);
    }

    /// Длинный флаг `--include-dirs`.
    #[test]
    fn parse_include_dirs_long_flag() {
        let args = vec![
            "--include-dirs".to_string(),
            "/lib:/usr".to_string(),
            "main.lam".to_string(),
        ];
        let opts = parse_compile_args(&args).unwrap();
        assert_eq!(opts.include_dirs, vec!["/lib", "/usr"]);
    }

    /// Слитная форма: `-I/path` без пробела.
    #[test]
    fn parse_include_dir_glued() {
        let args = vec!["-I/lib/lam".to_string(), "main.lam".to_string()];
        let opts = parse_compile_args(&args).unwrap();
        assert_eq!(opts.include_dirs, vec!["/lib/lam"]);
    }

    /// Слитная форма с двоеточием: `-I/a:/b`.
    #[test]
    fn parse_include_dir_glued_colon() {
        let args = vec!["-I/a:/b".to_string(), "main.lam".to_string()];
        let opts = parse_compile_args(&args).unwrap();
        assert_eq!(opts.include_dirs, vec!["/a", "/b"]);
    }

    /// Флаги `-t`, `-o` и `-I` все вместе.
    #[test]
    fn parse_full_args() {
        let args = vec![
            "--target".to_string(),
            "c".to_string(),
            "-I".to_string(),
            "/lib/lam:/usr/lam".to_string(),
            "-I".to_string(),
            "/local/but".to_string(),
            "main.lam".to_string(),
            "-o".to_string(),
            "build/".to_string(),
        ];
        let opts = parse_compile_args(&args).unwrap();
        assert_eq!(opts.target, "c");
        assert_eq!(opts.input_file, "main.lam");
        assert_eq!(opts.output_path, "build/");
        assert_eq!(
            opts.include_dirs,
            vec!["/lib/lam", "/usr/lam", "/local/but"]
        );
    }

    /// Флаг `-o` задаёт выходной путь.
    #[test]
    fn parse_output_flag() {
        let args = vec![
            "main.lam".to_string(),
            "-o".to_string(),
            "dist/".to_string(),
        ];
        let opts = parse_compile_args(&args).unwrap();
        assert_eq!(opts.output_path, "dist/");
    }

    /// Длинная форма флага `--output`.
    #[test]
    fn parse_output_long_flag() {
        let args = vec![
            "--output".to_string(),
            "out/".to_string(),
            "main.lam".to_string(),
        ];
        let opts = parse_compile_args(&args).unwrap();
        assert_eq!(opts.output_path, "out/");
    }

    /// Короткий флаг целевой платформы `-t`.
    #[test]
    fn parse_target_short_flag() {
        let args = vec!["-t".to_string(), "c".to_string(), "main.lam".to_string()];
        let opts = parse_compile_args(&args).unwrap();
        assert_eq!(opts.target, "c");
    }

    // ── parse_compile_args: контр-примеры (ошибки) ───────────────────────────

    /// Нет входного файла → ошибка.
    #[test]
    fn parse_missing_input_file_is_error() {
        let args: Vec<String> = vec![];
        assert!(parse_compile_args(&args).is_err());
    }

    /// Нет аргумента после `--target` → ошибка.
    #[test]
    fn parse_target_missing_value_is_error() {
        let args = vec!["--target".to_string()];
        let err = parse_compile_args(&args).unwrap_err();
        assert!(
            err.contains("--target"),
            "сообщение должно упоминать флаг: {}",
            err
        );
    }

    /// Нет аргумента после `-o` → ошибка.
    #[test]
    fn parse_output_missing_value_is_error() {
        let args = vec!["main.lam".to_string(), "-o".to_string()];
        let err = parse_compile_args(&args).unwrap_err();
        assert!(
            err.contains("-o"),
            "сообщение должно упоминать флаг: {}",
            err
        );
    }

    /// Нет аргумента после `-I` → ошибка.
    #[test]
    fn parse_include_missing_value_is_error() {
        let args = vec!["main.lam".to_string(), "-I".to_string()];
        let err = parse_compile_args(&args).unwrap_err();
        assert!(
            err.contains("-I"),
            "сообщение должно упоминать флаг: {}",
            err
        );
    }

    /// Нет аргумента после `--include-dirs` → ошибка.
    #[test]
    fn parse_include_dirs_missing_value_is_error() {
        let args = vec!["main.lam".to_string(), "--include-dirs".to_string()];
        let err = parse_compile_args(&args).unwrap_err();
        assert!(
            err.contains("--include-dirs"),
            "сообщение должно упоминать флаг: {}",
            err
        );
    }

    /// Неизвестный флаг → ошибка с его именем.
    #[test]
    fn parse_unknown_flag_is_error() {
        let args = vec!["main.lam".to_string(), "--unknown-flag".to_string()];
        let err = parse_compile_args(&args).unwrap_err();
        assert!(
            err.contains("--unknown-flag"),
            "сообщение должно упоминать неизвестный флаг: {}",
            err
        );
    }

    /// Несколько флагов `-I` с пустыми сегментами — итоговый список без пустышек.
    #[test]
    fn parse_include_filters_empty_segments() {
        let args = vec![
            "-I".to_string(),
            "/a::/b".to_string(),
            "main.lam".to_string(),
        ];
        let opts = parse_compile_args(&args).unwrap();
        assert_eq!(opts.include_dirs, vec!["/a", "/b"]);
    }

    /// Порядок путей поиска сохраняется (первый победитель).
    #[test]
    fn parse_include_dirs_order_preserved() {
        let args = vec![
            "-I".to_string(),
            "/first".to_string(),
            "-I".to_string(),
            "/second".to_string(),
            "-I".to_string(),
            "/third".to_string(),
            "main.lam".to_string(),
        ];
        let opts = parse_compile_args(&args).unwrap();
        assert_eq!(opts.include_dirs, vec!["/first", "/second", "/third"]);
    }

    // ── parse_compile_args: флаги --verbose / --quiet (I6) ───────────────────

    /// Флаг `--verbose` устанавливает `verbose = true`.
    #[test]
    fn parse_verbose_long_flag() {
        let args = vec!["main.lam".to_string(), "--verbose".to_string()];
        let opts = parse_compile_args(&args).unwrap();
        assert!(opts.verbose, "--verbose должен устанавливать verbose=true");
        assert!(!opts.quiet, "--verbose не должен затрагивать quiet");
    }

    /// Короткий флаг `-v` устанавливает `verbose = true`.
    #[test]
    fn parse_verbose_short_flag() {
        let args = vec!["-v".to_string(), "main.lam".to_string()];
        let opts = parse_compile_args(&args).unwrap();
        assert!(opts.verbose, "-v должен устанавливать verbose=true");
    }

    /// Флаг `--quiet` устанавливает `quiet = true`.
    #[test]
    fn parse_quiet_long_flag() {
        let args = vec!["main.lam".to_string(), "--quiet".to_string()];
        let opts = parse_compile_args(&args).unwrap();
        assert!(opts.quiet, "--quiet должен устанавливать quiet=true");
        assert!(!opts.verbose, "--quiet не должен затрагивать verbose");
    }

    /// Короткий флаг `-q` устанавливает `quiet = true`.
    #[test]
    fn parse_quiet_short_flag() {
        let args = vec!["-q".to_string(), "main.lam".to_string()];
        let opts = parse_compile_args(&args).unwrap();
        assert!(opts.quiet, "-q должен устанавливать quiet=true");
    }

    /// По умолчанию ни `verbose`, ни `quiet` не установлены.
    #[test]
    fn parse_defaults_no_verbose_no_quiet() {
        let args = vec!["main.lam".to_string()];
        let opts = parse_compile_args(&args).unwrap();
        assert!(!opts.verbose, "verbose должен быть false по умолчанию");
        assert!(!opts.quiet, "quiet должен быть false по умолчанию");
    }

    /// Одновременное указание `--verbose` и `--quiet` → ошибка.
    #[test]
    fn parse_verbose_and_quiet_is_error() {
        let args = vec![
            "main.lam".to_string(),
            "--verbose".to_string(),
            "--quiet".to_string(),
        ];
        let err = parse_compile_args(&args).unwrap_err();
        assert!(
            err.contains("взаимоисключающие") || err.contains("verbose") && err.contains("quiet"),
            "должна быть ошибка о несовместимости флагов: {}",
            err
        );
    }

    /// Одновременное указание `-v` и `-q` → ошибка.
    #[test]
    fn parse_v_and_q_is_error() {
        let args = vec!["main.lam".to_string(), "-v".to_string(), "-q".to_string()];
        let err = parse_compile_args(&args).unwrap_err();
        assert!(
            !err.is_empty(),
            "должна быть ошибка при -v и -q одновременно"
        );
    }

    /// `--verbose` совместим с другими флагами.
    #[test]
    fn parse_verbose_with_other_flags() {
        let args = vec![
            "--verbose".to_string(),
            "-I".to_string(),
            "/lib".to_string(),
            "--target".to_string(),
            "c".to_string(),
            "main.lam".to_string(),
            "-o".to_string(),
            "out/".to_string(),
        ];
        let opts = parse_compile_args(&args).unwrap();
        assert!(opts.verbose);
        assert!(!opts.quiet);
        assert_eq!(opts.include_dirs, vec!["/lib"]);
        assert_eq!(opts.target, "c");
        assert_eq!(opts.output_path, "out/");
    }

    /// `--quiet` совместим с другими флагами.
    #[test]
    fn parse_quiet_with_other_flags() {
        let args = vec![
            "-q".to_string(),
            "main.lam".to_string(),
            "-o".to_string(),
            "dist/".to_string(),
        ];
        let opts = parse_compile_args(&args).unwrap();
        assert!(opts.quiet);
        assert!(!opts.verbose);
        assert_eq!(opts.output_path, "dist/");
    }

    /// Флаг `--address-map` задаёт путь к внешней карте адресов (фича 0020-03).
    #[test]
    fn parse_address_map_flag() {
        let args = vec![
            "main.lam".to_string(),
            "--address-map".to_string(),
            "stm32.map".to_string(),
        ];
        let opts = parse_compile_args(&args).unwrap();
        assert_eq!(opts.address_map.as_deref(), Some("stm32.map"));
    }

    /// По умолчанию карта адресов не задана.
    #[test]
    fn address_map_absent_by_default() {
        let opts = parse_compile_args(&["main.lam".to_string()]).unwrap();
        assert!(opts.address_map.is_none());
    }

    /// `--address-map` без аргумента — ошибка.
    #[test]
    fn address_map_requires_argument() {
        let err = parse_compile_args(&["--address-map".to_string()]).unwrap_err();
        assert!(err.contains("--address-map"), "сообщение: {}", err);
    }

    /// T7 (0029-03): без флага — `W64`, то есть `double`: эталон C совпадает с
    /// точностью симулятора (f64) по умолчанию, а не по особой просьбе.
    #[test]
    fn float_width_defaults_to_64() {
        let opts = parse_compile_args(&["main.lam".to_string()]).unwrap();
        assert_eq!(opts.float_width, grammar::FloatWidth::W64);
    }

    /// T8 (0029-03): `--float-width=32` → `float`.
    #[test]
    fn parse_float_width_32() {
        let args = vec!["main.lam".to_string(), "--float-width=32".to_string()];
        let opts = parse_compile_args(&args).unwrap();
        assert_eq!(opts.float_width, grammar::FloatWidth::W32);
    }

    /// Форма с пробелом — наравне со слитной (как у прочих флагов).
    #[test]
    fn parse_float_width_separate_argument() {
        let args = vec![
            "main.lam".to_string(),
            "--float-width".to_string(),
            "32".to_string(),
        ];
        let opts = parse_compile_args(&args).unwrap();
        assert_eq!(opts.float_width, grammar::FloatWidth::W32);
    }

    /// T16 (0029-03): `--float-width=16` — ошибка разбора, а не молчаливое
    /// умолчание: иначе код собрался бы с ДРУГОЙ точностью, чем просил
    /// пользователь.
    #[test]
    fn float_width_rejects_unsupported_value() {
        let args = vec!["main.lam".to_string(), "--float-width=16".to_string()];
        let err = parse_compile_args(&args).unwrap_err();
        assert!(
            err.contains("16") && err.contains("32") && err.contains("64"),
            "сообщение обязано назвать и отвергнутое значение, и допустимые: {}",
            err
        );
    }

    /// `--float-width` без аргумента — ошибка.
    #[test]
    fn float_width_requires_argument() {
        let err = parse_compile_args(&["--float-width".to_string()]).unwrap_err();
        assert!(err.contains("--float-width"), "сообщение: {}", err);
    }
}
