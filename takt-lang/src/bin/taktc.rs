//! Компилятор Takt — утилита командной строки.
//!
//! # Использование
//!
//! ```text
//! taktc compile [--target c] [-I dir1:dir2] [--verbose | --quiet] <input.takt> [-o output_dir]
//! taktc compile input.takt           # вывод в ./output
//! taktc --help                      # справка
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
//! taktc compile -I /usr/lib/lam:/home/user/lam main.takt -o out
//!
//! # Несколько флагов -I
//! taktc compile -I /usr/lib/lam -I /home/user/lam main.takt
//!
//! # Слитная форма без пробела
//! taktc compile -I/usr/lib/lam main.takt
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
use takt_lang::address_map::split_include_dirs;

/// Параметры компиляции, разобранные из аргументов командной строки.
#[derive(Debug, PartialEq)]
pub struct CompileOptions {
    /// Целевой язык генерации (по умолчанию `"c"`).
    pub target: String,
    /// Путь к входному `.takt`-файлу.
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
    /// Символы платформы для выражений адреса: сырые аргументы `--define`
    /// (фича 0042). Разбор в среду — `takt_lang::parse_defines`.
    ///
    /// По умолчанию пусто → без флага поведение `taktc` идентично прежнему.
    pub defines: Vec<String>,
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
    pub float_width: takt_lang::FloatWidth,
    /// Глобальная точность `q(m, n)` для реализации `float` (фича 0096).
    ///
    /// Заполняется флагом `--float-as-q=<m>.<n>` (границы правила 1 ADR 0061:
    /// `m ≥ 1`, `n ≥ 1`, `m + n ≤ 64`). `None` без флага — прежнее поведение.
    pub float_as_q: Option<(u8, u8)>,
    /// Реализовать `float` целочисленным Q-путём в `c`/`rust`/`st` (embedded) —
    /// флаг `--float-embedded` (фича 0096). Действует только с `--float-as-q`.
    pub float_embedded: bool,
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
/// | `--target`, `-t`       | Целевой язык: `c` (по умолчанию), `c-hal`, `plantuml`, `st`, `st-at`, `rust` |
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
///     "main.takt".to_string(),
/// ];
/// let opts = parse_compile_args(&args).unwrap();
/// assert_eq!(opts.include_dirs, vec!["/lib/lam", "/usr/lam"]);
/// assert_eq!(opts.input_file, "main.takt");
/// assert_eq!(opts.target, "c");
/// assert!(!opts.verbose);
/// assert!(!opts.quiet);
/// ```
pub fn parse_compile_args(args: &[String]) -> Result<CompileOptions, String> {
    let mut target = "c".to_string();
    let mut input_file: Option<String> = None;
    let mut output_path: Option<String> = None;
    let mut include_dirs: Vec<String> = Vec::new();
    let mut defines: Vec<String> = Vec::new();
    let mut verbose = false;
    let mut quiet = false;
    let mut guard_enable = true;
    let mut address_map: Option<String> = None;
    let mut float_width = takt_lang::FloatWidth::default();
    let mut float_as_q: Option<(u8, u8)> = None;
    let mut float_embedded = false;

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
            "-D" | "--define" => {
                i += 1;
                match args.get(i) {
                    Some(v) => defines.push(v.clone()),
                    None => return Err(format!("{} требует аргумент", arg)),
                }
            }
            // Слитная форма: -DNAME=VALUE. Ветка стоит ПОСЛЕ раздельной — иначе
            // она перехватывала бы сам `-D` (образец: `-I` ниже).
            s if s.starts_with("-D") && s.len() > 2 => {
                defines.push(s[2..].to_string());
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
            // `--float-as-q=m.n` — глобальная точность реализации float (0096).
            "--float-as-q" => {
                i += 1;
                match args.get(i) {
                    Some(v) => float_as_q = Some(parse_float_as_q(v)?),
                    None => return Err(format!("{} требует аргумент: m.n (напр. 10.22)", arg)),
                }
            }
            a if a.starts_with("--float-as-q=") => {
                float_as_q = Some(parse_float_as_q(&a["--float-as-q=".len()..])?);
            }
            "--float-embedded" => float_embedded = true,
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
        defines,
        verbose,
        quiet,
        guard_enable,
        address_map,
        float_width,
        float_as_q,
        float_embedded,
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
fn print_compile_error(diag: &takt_lang::diagnostics::Diagnostic) {
    eprintln!(
        "{}Ошибка компиляции [{}]: {}",
        takt_lang::diagnostics::position_prefix(diag),
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
fn generate_options(options: &CompileOptions) -> takt_lang::GenerateOptions {
    let mut generate = takt_lang::GenerateOptions::new(options.guard_enable);
    generate.float_width = options.float_width;
    generate.float_as_q = options.float_as_q;
    generate.float_embedded = options.float_embedded;
    generate
}

/// Печатает результат простой цели (`Result<(), Diagnostic>`: `plantuml`/`st`/
/// `rust`/`sv`) и завершает процесс при ошибке. verbose даёт полный путь входа.
/// Цель `c` не пользуется этим: её сообщение перечисляет пути поиска.
fn report_simple_result(
    result: Result<(), takt_lang::diagnostics::Diagnostic>,
    target: &str,
    options: &CompileOptions,
) {
    if let Err(diag) = result {
        print_compile_error(&diag);
        process::exit(1);
    }
    if options.quiet {
        return;
    }
    if options.verbose {
        eprintln!(
            "Скомпилировано: {} → {} ({})",
            fs::canonicalize(&options.input_file)
                .map(|p| p.display().to_string())
                .unwrap_or_else(|_| options.input_file.clone()),
            options.output_path,
            target,
        );
    } else {
        eprintln!(
            "Скомпилировано: {} → {}/ ({})",
            options.input_file, options.output_path, target
        );
    }
}

/// Печатает результат адрес-потребляющей цели (`c-hal`/`st-at`/`sv-mmio`) и
/// завершает процесс при ошибке. Тело идентично (возвращают `Ok(warnings)` либо
/// `Err`), поэтому вынесено сюда.
fn report_hal_result(
    result: Result<Vec<takt_lang::diagnostics::Diagnostic>, takt_lang::diagnostics::Diagnostic>,
    target: &str,
    options: &CompileOptions,
) {
    match result {
        Ok(warnings) => {
            if !options.quiet {
                for w in warnings {
                    eprintln!(
                        "Предупреждение [{}]: {}",
                        w.code.as_deref().unwrap_or("?"),
                        w.message
                    );
                }
                eprintln!(
                    "Скомпилировано: {} → {}/ ({})",
                    options.input_file, options.output_path, target
                );
            }
        }
        Err(diag) => {
            print_compile_error(&diag);
            process::exit(1);
        }
    }
}

/// Разбирает значение флага `--float-width` (фича 0029).
///
/// Допустимы только 32 и 64: иных вещественных типов у цели `c` нет. Прочее —
/// ошибка разбора, а не молчаливое умолчание: `--float-width=16` должен сказать,
/// что такой ширины не бывает, а не собрать код с другой точностью.
fn parse_float_width(value: &str) -> Result<takt_lang::FloatWidth, String> {
    match value {
        "32" => Ok(takt_lang::FloatWidth::W32),
        "64" => Ok(takt_lang::FloatWidth::W64),
        other => Err(format!(
            "--float-width: недопустимое значение '{}' (допустимо 32 или 64)",
            other
        )),
    }
}

/// Разбирает значение флага `--float-as-q=<m>.<n>` в точность `q(m, n)` (0096).
///
/// Границы — те же, что у типа `q` (правило 1 ADR 0061): `m ≥ 1`, `n ≥ 1`,
/// `m + n ≤ 64`. Нарушение или неверный формат — **ошибка CLI**, а не молчаливое
/// умолчание: угадывать точность нельзя (ровно довод, по которому 0045/0061
/// отвергли автоугадывание формата).
fn parse_float_as_q(value: &str) -> Result<(u8, u8), String> {
    let (m_str, n_str) = value.split_once('.').ok_or_else(|| {
        format!("--float-as-q: ожидался формат m.n (напр. 10.22), получено '{value}'")
    })?;
    let parse = |s: &str, what: &str| -> Result<u8, String> {
        s.parse::<u8>()
            .map_err(|_| format!("--float-as-q: {what} '{s}' — не целое 0..255"))
    };
    let (m, n) = (parse(m_str, "m")?, parse(n_str, "n")?);
    if m < 1 || n < 1 || (m as u16) + (n as u16) > 64 {
        return Err(format!(
            "--float-as-q: q({m}, {n}) вне границ (m ≥ 1, n ≥ 1, m + n ≤ 64)"
        ));
    }
    Ok((m, n))
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
    /// Файлы и каталоги (каталоги обходятся рекурсивно по `*.takt`).
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

/// Рекурсивно собирает `*.takt` из файла или каталога.
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
        } else if child.extension().is_some_and(|e| e == "takt") {
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
        return match takt_lang::format::format_source(&source) {
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
        let formatted = match takt_lang::format::format_source(&source) {
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

/// Какой граф верификации выгрузить в Graphviz DOT (фича 0124).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum GraphKind {
    /// Структура Крипке модели (без `--property` — управляющая абстракция 0049).
    Kripke,
    /// Автомат Бюхи для `¬φ` (требует `--property`).
    Buchi,
    /// Произведение `K × A_¬φ` (требует `--property`).
    Product,
}

/// Опции подкоманды `verify`.
#[derive(Debug, Default, PartialEq, Eq)]
pub struct VerifyOptions {
    /// Путь к проверяемому `.takt`-файлу.
    pub input_file: String,
    /// Директории поиска файлов `import`.
    pub include_dirs: Vec<String>,
    /// Свойство из командной строки (`--property "G (Fault -> F Idle)"`).
    ///
    /// `None` — проверяются все формулы `: [LTL] φ;`, объявленные в файле.
    pub property: Option<String>,
    /// Печатать трассу конвейера (Крипке, автомат `¬φ`, произведение).
    pub trace: bool,
    /// Область проверки (фича 0051): `file` (умолчание) — модели своего файла,
    /// `all` — включая импортированные.
    pub scope: takt_lang::VerifyScope,
    /// Выгрузить граф верификации в DOT вместо проверки (фича 0124, `--emit-graph`).
    pub emit_graph: Option<GraphKind>,
}

/// Разбирает значение флага `--emit-graph`.
///
/// Негодное значение — отказ, а не молчаливое умолчание (тот же принцип, что у
/// `--scope`): иначе `--emit-graph kripk` тихо ушёл бы в проверку.
fn parse_graph_kind(value: &str) -> Result<GraphKind, String> {
    match value {
        "kripke" => Ok(GraphKind::Kripke),
        "buchi" => Ok(GraphKind::Buchi),
        "product" => Ok(GraphKind::Product),
        other => Err(format!(
            "неизвестный граф '{other}'; допустимо: kripke (структура Крипке), \
             buchi (автомат ¬φ), product (произведение)"
        )),
    }
}

/// Разбирает значение флага `--scope`.
///
/// Негодное значение — отказ, а не молчаливое умолчание: `--scope al` иначе
/// проверял бы свой файл, отчитавшись «все держатся», и пользователь считал бы,
/// что импорты тоже проверены.
fn parse_scope(value: &str) -> Result<takt_lang::VerifyScope, String> {
    match value {
        "file" => Ok(takt_lang::VerifyScope::File),
        "all" => Ok(takt_lang::VerifyScope::All),
        other => Err(format!(
            "неизвестная область '{other}'; допустимо: file (модели своего файла) \
             или all (включая импортированные)"
        )),
    }
}

/// Задаёт проверяемое свойство, отвергая повтор флага.
///
/// Второй `--property` молча затирал бы первый, и `taktc verify -p "F Done" -p
/// "G Idle" m.takt` отчитался бы «проверено свойств: 1; все держатся» — про
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
            "--emit-graph" => {
                i += 1;
                let value = args.get(i).ok_or_else(|| {
                    format!("флаг '{arg}' требует значение: kripke, buchi или product")
                })?;
                options.emit_graph = Some(parse_graph_kind(value)?);
            }
            "--scope" => {
                i += 1;
                let value = args
                    .get(i)
                    .ok_or_else(|| format!("флаг '{arg}' требует значение: file или all"))?;
                options.scope = parse_scope(value)?;
            }
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
            other if other.starts_with("--scope=") => {
                options.scope = parse_scope(&other["--scope=".len()..])?;
            }
            other if other.starts_with("--emit-graph=") => {
                options.emit_graph = Some(parse_graph_kind(&other["--emit-graph=".len()..])?);
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
        return Err("укажите .takt-файл для проверки".to_string());
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

    let (ast, _) = match takt_lang::parse(&source, 0) {
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

    let model = match takt_lang::semantic::tree::construct_model(&ast, None, &options.include_dirs)
    {
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

    // Экспорт графа верификации в DOT (фича 0124) — вместо проверки.
    if let Some(kind) = options.emit_graph {
        return run_emit_graph(&model.borrow(), kind, options.property.as_deref());
    }

    // Свойства: либо одно из --property, либо все объявленные в файле.
    let outcome = match &options.property {
        Some(text) => {
            let phi = match takt_lang::parse_ltl_property(text) {
                Ok(p) => p,
                Err(d) => {
                    eprintln!("Ошибка разбора свойства: {}", d.message);
                    return 1;
                }
            };
            let (verdict, trace) =
                takt_lang::verification::verify::verify_model_traced(&model.borrow(), &phi);
            // Свойство из строки проверяется против корневой модели файла и
            // области не касается (ADR 0051, A2): пропускать тут нечего.
            takt_lang::VerifyOutcome {
                results: vec![takt_lang::PropertyResult {
                    model: String::new(),
                    loc: takt_lang::diagnostics::Location::Implicit,
                    verdict,
                    formula: phi,
                    trace: options.trace.then_some(trace),
                }],
                skipped: Vec::new(),
            }
        }
        None => takt_lang::verify_all_scoped(Rc::clone(&model), options.trace, options.scope),
    };

    if outcome.results.is_empty() && outcome.skipped.is_empty() {
        eprintln!(
            "В файле '{}' нет LTL-формул. Объявите свойство как `: [LTL] φ;` \
             или задайте его флагом --property \"φ\".",
            options.input_file
        );
        return 1;
    }

    print_verify_results(&outcome)
}

/// Печатает диагностику вердикта-отказа при экспорте графа (фича 0124).
fn report_graph_refusal(verdict: &takt_lang::verification::verify::Verdict) -> i32 {
    use takt_lang::verification::verify::Verdict;
    match verdict {
        Verdict::NoStartState => {
            eprintln!("Экспорт графа невозможен: у модели нет стартового состояния.");
        }
        Verdict::Unsupported(atoms) => {
            eprintln!(
                "Экспорт графа невозможен: атом(ы) {} — не имя состояния и не \
                 отслеживаемый предикат над данными.",
                atoms.join(", ")
            );
        }
        // Holds/Violated здесь не возникают: build_graphs не проверяет пустоту.
        _ => eprintln!("Экспорт графа невозможен."),
    }
    1
}

/// Выгружает запрошенный граф верификации в Graphviz DOT (фича 0124).
///
/// `kripke` без свойства — управляющая структура Крипке; `buchi`/`product`
/// требуют `--property` (строятся по `¬φ`). DOT печатается в stdout
/// (перенаправляется в файл, рендерится `dot -Tsvg`).
fn run_emit_graph(
    model: &takt_lang::semantic::ModelNode,
    kind: GraphKind,
    property: Option<&str>,
) -> i32 {
    use takt_lang::verification::{dot, verify};

    // Крипке без свойства — единственный граф, не требующий формулы.
    if kind == GraphKind::Kripke && property.is_none() {
        return match verify::build_control_kripke(model) {
            Ok(kripke) => {
                print!("{}", dot::kripke_to_dot(&kripke));
                0
            }
            Err(v) => report_graph_refusal(&v),
        };
    }

    // buchi/product (и kripke со свойством) строятся по формуле.
    let Some(text) = property else {
        eprintln!(
            "граф '{}' строится по свойству — задайте его флагом --property \"φ\".",
            match kind {
                GraphKind::Buchi => "buchi",
                GraphKind::Product => "product",
                GraphKind::Kripke => "kripke",
            }
        );
        return 1;
    };
    let phi = match takt_lang::parse_ltl_property(text) {
        Ok(p) => p,
        Err(d) => {
            eprintln!("Ошибка разбора свойства: {}", d.message);
            return 1;
        }
    };
    let graphs = match verify::build_graphs(model, &phi) {
        Ok(g) => g,
        Err(v) => return report_graph_refusal(&v),
    };
    match kind {
        GraphKind::Kripke => print!("{}", dot::kripke_to_dot(&graphs.kripke)),
        GraphKind::Buchi => print!("{}", dot::buchi_to_dot(&graphs.automaton)),
        GraphKind::Product => print!("{}", dot::product_to_dot(&graphs.product, &graphs.kripke)),
    }
    0
}

/// Печатает вердикты и возвращает код возврата процесса.
fn print_verify_results(outcome: &takt_lang::VerifyOutcome) -> i32 {
    use takt_lang::verification::verify::Verdict;

    let results = &outcome.results;
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
                // Предикат над данными теперь отслеживается (фича 0068).
                // `Unsupported` остаётся, когда атом — не имя состояния и не
                // отслеживаемый предикат (причины перечислены в тексте ниже).
                println!(
                    "  атом(ы) {} — не имя состояния и не отслеживаемый предикат над данными.",
                    atoms.join(", ")
                );
                println!(
                    "  В охвате: свойства управления (имя состояния) и предикаты над данными \
                     (`cond`/булев `var` над `bit`/`bool`/целым/`enum`, фича 0068)."
                );
                println!(
                    "  Вне охвата: `float`/`q` в предикате, арифметика/функции/порты в нём, \
                     превышение потолка перечисления, состояния вложенных моделей."
                );
            }
            Verdict::NoStartState => {
                failures += 1;
                println!("СВОЙСТВО НЕ ПРОВЕРЕНО{scope}: {}", result.formula);
                println!("  у модели нет стартового состояния — проверять нечего");
            }
        }
    }

    // Сужение области — не тишина (ADR 0051): пропущенное перечисляется, иначе
    // «все держатся» читалось бы как «проверено всё».
    if !outcome.skipped.is_empty() {
        println!(
            "\nНе проверено (вне области): {} — модели из импортов: {}",
            outcome.skipped.len(),
            outcome.skipped.join(", ")
        );
        println!("  --scope all — проверить их тоже.");
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
    eprintln!("Использование: taktc compile [флаги] <input.takt> [-o <output>]");
    eprintln!("               taktc fmt [--check] [--stdin] <файлы/каталоги>");
    eprintln!(
        "               taktc verify [--property \"φ\"] [--scope file|all] [--trace] <input.takt>"
    );
    eprintln!(
        "               taktc address-map [--emit map|json] [--address-map <файл>] [-D N=V] [-o <out>] <input.takt>"
    );
    eprintln!("               taktc --help");
    eprintln!();
    eprintln!("Флаги:");
    eprintln!("  --target, -t <цель>    Целевой язык (по умолчанию: c) — см. «Целевые платформы»");
    eprintln!("  --output, -o <путь>    Путь к выходному файлу");
    eprintln!("  --include-dirs, -I <dirs>  Пути поиска файлов import, разделённые ':' или ';'");
    eprintln!("                             Можно повторять: -I /a -I /b  или  -I /a:/b");
    eprintln!("  --verbose, -v          Расширенный вывод: все предупреждения и полные пути");
    eprintln!("  --quiet, -q            Тихий режим: только ошибки");
    eprintln!("                         Флаги --verbose и --quiet взаимоисключающие");
    eprintln!("  --guard-enable         Включить генерацию проверок Guard-формул (по умолчанию)");
    eprintln!("  --guard-disable        Выключить генерацию проверок Guard-формул");
    eprintln!("  --address-map <файл>   Внешняя карта адресов портов (.ld-подобный формат)");
    eprintln!("  -D, --define N=VALUE   Символ платформы для выражений адреса (повторяем);");
    eprintln!("                         слитно: -DN=VALUE. Значение — 0x…/десятичное[:бит].");
    eprintln!(
        "                         Виден ТОЛЬКО выражениям адреса, логику автомата не меняет."
    );
    eprintln!("  --float-width=32|64    Ширина вещественного типа в C: float или double");
    eprintln!(
        "  --float-as-q=m.n       Точность q(m,n) для реализации float (0096): sv → q; c/rust/st → q с --float-embedded"
    );
    eprintln!(
        "  --float-embedded       Реализовать float целочисленным q в c/rust/st (embedded без FPU)"
    );
    eprintln!("                         Цель rust всегда даёт f64 и флаг 32 отвергает (RS-015)");
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
    eprintln!("  rust      Генерация no_std Rust (.rs) — прошивка МК (фича 0050)");
    eprintln!("            Порты через трейт Hal; подключается в крейт через mod");
    eprintln!("  sv        Генерация синтезируемого SystemVerilog (.sv) — FPGA/ASIC (фича 0045)");
    eprintln!("            Такт модели ≡ posedge clk; clk/rst_n — служебные порты модуля");
    eprintln!("  sv-mmio   SV + порты с адресом → регистровый файл на шине (фича 0062)");
    eprintln!("            Порт с адресом = бит регистра; интерфейс reg_addr/wdata/wen/rdata");
    eprintln!();
    eprintln!("Примеры:");
    eprintln!("  taktc compile main.takt");
    eprintln!("  taktc compile -I /lib/lam:/home/user/lam main.takt -o build/");
    eprintln!("  taktc compile -I /lib/lam -I /home/user/lam --target c main.takt");
    eprintln!("  taktc compile --verbose main.takt");
    eprintln!("  taktc compile --quiet main.takt -o dist/");
    eprintln!();
    eprintln!("Подкоманда fmt (канонический форматтер):");
    eprintln!("  --check      Не писать файлы; ненулевой код, если нужен формат (для CI)");
    eprintln!("  --stdin      Читать из stdin, писать в stdout");
    eprintln!("  taktc fmt examples/            # отформатировать каталог на месте");
    eprintln!("  taktc fmt --check examples/    # проверить (CI)");
    eprintln!("  cat a.takt | taktc fmt --stdin  # отформатировать поток");
    eprintln!();
    eprintln!("Подкоманда verify (проверка LTL-свойств, model checking — фича 0049):");
    eprintln!("  --property, -p \"φ\"  Проверить одну формулу из командной строки");
    eprintln!("                      Без флага проверяются все `: [LTL] φ;` файла");
    eprintln!("  --scope file|all    Область проверки (по умолчанию: file — модели своего файла)");
    eprintln!("                      all — проверять и модели, пришедшие через import");
    eprintln!("  --trace             Печатать конвейер (Крипке, автомат !φ, произведение)");
    eprintln!("  -I <dirs>           Пути поиска файлов import");
    eprintln!();
    eprintln!("  Атом формулы — ИМЯ СОСТОЯНИЯ: `S` истинно, когда автомат в состоянии S.");
    eprintln!("  Проверяются свойства управления: достижимость, порядок состояний, живость.");
    eprintln!("  Свойства над данными (`G (temp <= 100)`) в этой абстракции не поддержаны.");
    eprintln!("  Код возврата: 0 — все свойства держатся; 1 — нарушение/не проверено.");
    eprintln!();
    eprintln!("  taktc verify model.takt");
    eprintln!("  taktc verify --property \"F Done\" model.takt       # достижимость");
    eprintln!("  taktc verify -p \"G (Fault -> F Idle)\" model.takt  # живость");
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

    if args[1] == "address-map" {
        // Логика — в библиотеке (`bin/taktc.rs` пришпилен к baseline размера).
        process::exit(takt_lang::address_map::run_export_subcommand(&args[2..]));
    }

    if args[1] != "compile" {
        eprintln!(
            "Ошибка: неизвестная команда '{}'. Используйте 'compile', 'fmt', 'verify' или 'address-map'.",
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
    let external_entries: Vec<takt_lang::AddressMapEntry> =
        if let Some(map_path) = &options.address_map {
            let map_src = match fs::read_to_string(map_path) {
                Ok(s) => s,
                Err(e) => {
                    eprintln!("Ошибка чтения карты адресов '{}': {}", map_path, e);
                    process::exit(1);
                }
            };
            match takt_lang::parse_address_map(&map_src, 0) {
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

    // Символы платформы для выражений адреса (фича 0042). Разбор один раз, до
    // компиляции: битый аргумент — ошибка CLI, а не повод собрать не тот адрес.
    let address_env = match takt_lang::parse_defines(&options.defines) {
        Ok(env) => env,
        Err(diags) => {
            for d in diags {
                eprintln!(
                    "Ошибка --define [{}]: {}",
                    d.code.as_deref().unwrap_or("?"),
                    d.message
                );
            }
            process::exit(1);
        }
    };

    // Предупреждения компилятора (фича 0081): единая точка `collect_model_warnings`
    // печатается для **всех** целей (до 0081 CLI не звал ни `SE-036`, ни `SE-037`).
    // Адрес-специфичные (оверлей карты, сломанное выражение адреса) — только у
    // целей, адрес НЕ потребляющих: у `c-hal`/`st-at` это ошибки при генерации.
    let consumes_addresses = matches!(options.target.as_str(), "c-hal" | "st-at" | "sv-mmio");
    if let Some((ast, model)) = takt_lang::parse(&source, 0).ok().and_then(|(ast, _)| {
        takt_lang::semantic::tree::construct_model(&ast, None, &options.include_dirs)
            .ok()
            .map(|m| (ast, m))
    }) {
        let mut warnings = takt_lang::semantic::warnings::collect_model_warnings(&ast, &model);
        if !consumes_addresses {
            if !external_entries.is_empty() {
                warnings.extend(takt_lang::address_map_overlay_warnings(
                    std::rc::Rc::clone(&model),
                    &external_entries,
                ));
            }
            warnings.extend(takt_lang::address_expr_warnings(
                std::rc::Rc::clone(&model),
                &address_env,
            ));
        }
        if !options.quiet {
            // Формат общий с ошибкой (`print_compile_error`): позиция + код + текст.
            for w in &warnings {
                eprintln!(
                    "{}Предупреждение [{}]: {}",
                    takt_lang::diagnostics::position_prefix(w),
                    w.code.as_deref().unwrap_or("?"),
                    w.message
                );
            }
        }
    }

    match options.target.as_str() {
        "c-hal" => {
            report_hal_result(
                takt_lang::compile_to_c_hal(
                    &options.input_file,
                    &source,
                    &options.output_path,
                    &options.include_dirs,
                    &external_entries,
                    &address_env,
                    &generate_options(&options),
                ),
                "c-hal",
                &options,
            );
        }
        "c" => {
            if let Err(diag) = takt_lang::compile_to_c(
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
            report_simple_result(
                takt_lang::compile_to_plantuml(
                    &options.input_file,
                    &source,
                    &options.output_path,
                    &options.include_dirs,
                ),
                "plantuml",
                &options,
            );
        }
        "st" => {
            report_simple_result(
                takt_lang::compile_to_st(
                    &options.input_file,
                    &source,
                    &options.output_path,
                    &options.include_dirs,
                    &generate_options(&options),
                ),
                "st",
                &options,
            );
        }
        "st-at" => {
            report_hal_result(
                takt_lang::compile_to_st_at(
                    &options.input_file,
                    &source,
                    &options.output_path,
                    &options.include_dirs,
                    &external_entries,
                    &address_env,
                    &generate_options(&options),
                ),
                "st-at",
                &options,
            );
        }
        "rust" => {
            report_simple_result(
                takt_lang::compile_to_rust(
                    &options.input_file,
                    &source,
                    &options.output_path,
                    &options.include_dirs,
                    &generate_options(&options),
                ),
                "rust",
                &options,
            );
        }
        "sv" => {
            report_simple_result(
                takt_lang::compile_to_sv(
                    &options.input_file,
                    &source,
                    &options.output_path,
                    &options.include_dirs,
                    &generate_options(&options),
                ),
                "sv",
                &options,
            );
        }
        "sv-mmio" => {
            report_hal_result(
                takt_lang::compile_to_sv_mmio(
                    &options.input_file,
                    &source,
                    &options.output_path,
                    &options.include_dirs,
                    &external_entries,
                    &address_env,
                    &generate_options(&options),
                ),
                "sv-mmio",
                &options,
            );
        }
        t => {
            eprintln!(
                "Ошибка: неизвестная цель '{}'. Поддерживается: c, c-hal, plantuml, st, st-at, rust, sv, sv-mmio",
                t
            );
            process::exit(1);
        }
    }
}

// ─── Тесты ────────────────────────────────────────────────────────────────────

#[cfg(test)]
mod tests {
    const SEP: &str = if cfg!(windows) { ";" } else { ":" }; // разделитель `-I` на платформе; тесты параметризуются им (фича 0037)
    // ── Подкоманда fmt (задача 0024-03) ──────────────────────────────────────

    #[test]
    fn fmt_args_paths() {
        let args = vec!["examples/".to_string(), "a.takt".to_string()];
        let o = parse_fmt_args(&args).unwrap();
        assert!(!o.check);
        assert!(!o.stdin);
        assert_eq!(o.paths, vec!["examples/", "a.takt"]);
    }

    #[test]
    fn fmt_args_check_flag() {
        let args = vec!["--check".to_string(), "a.takt".to_string()];
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
        let args = vec!["--stdin".to_string(), "a.takt".to_string()];
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
        let o = parse_verify_args(&args(&["model.takt"])).unwrap();
        assert_eq!(o.input_file, "model.takt");
        assert_eq!(o.property, None, "без --property проверяются формулы файла");
        assert!(!o.trace);
    }

    #[test]
    fn verify_args_property_flag() {
        let o = parse_verify_args(&args(&["--property", "G (Fault -> F Idle)", "m.takt"])).unwrap();
        assert_eq!(o.property.as_deref(), Some("G (Fault -> F Idle)"));
        assert_eq!(o.input_file, "m.takt");
    }

    /// Короткая форма `-p` и слитная `--property=` — синонимы длинной.
    #[test]
    fn verify_args_property_short_and_joined_forms() {
        let short = parse_verify_args(&args(&["-p", "F Done", "m.takt"])).unwrap();
        let joined = parse_verify_args(&args(&["--property=F Done", "m.takt"])).unwrap();
        assert_eq!(short.property.as_deref(), Some("F Done"));
        assert_eq!(joined.property.as_deref(), Some("F Done"));
    }

    #[test]
    fn verify_args_trace_flag() {
        let o = parse_verify_args(&args(&["--trace", "m.takt"])).unwrap();
        assert!(o.trace);
        assert_eq!(o.input_file, "m.takt");
    }

    #[test]
    fn verify_args_include_dirs() {
        let o = parse_verify_args(&args(&["-I", "/a", "-I/b", "m.takt"])).unwrap();
        assert_eq!(o.include_dirs, vec!["/a", "/b"]);
    }

    #[test]
    fn verify_args_emit_graph() {
        // Фича 0124: --emit-graph в раздельной и слитной форме.
        let k = parse_verify_args(&args(&["--emit-graph", "kripke", "m.takt"])).unwrap();
        assert_eq!(k.emit_graph, Some(GraphKind::Kripke));
        let b = parse_verify_args(&args(&["--emit-graph=buchi", "m.takt"])).unwrap();
        assert_eq!(b.emit_graph, Some(GraphKind::Buchi));
        let p = parse_verify_args(&args(&["--emit-graph", "product", "m.takt"])).unwrap();
        assert_eq!(p.emit_graph, Some(GraphKind::Product));
        // По умолчанию экспорта нет.
        let none = parse_verify_args(&args(&["m.takt"])).unwrap();
        assert_eq!(none.emit_graph, None);
    }

    #[test]
    fn verify_args_emit_graph_bad_value_is_error() {
        // Контрпример: опечатка не должна молча уходить в проверку.
        assert!(parse_verify_args(&args(&["--emit-graph", "kripk", "m.takt"])).is_err());
        assert!(parse_verify_args(&args(&["--emit-graph", "m.takt"])).is_err());
    }

    #[test]
    fn verify_args_without_file_is_error() {
        // Контрпример: проверять нечего — отказ, а не пустой прогон.
        assert!(parse_verify_args(&[]).is_err());
        assert!(parse_verify_args(&args(&["--property", "F Done"])).is_err());
    }

    #[test]
    fn verify_args_property_without_value_is_error() {
        assert!(parse_verify_args(&args(&["m.takt", "--property"])).is_err());
    }

    #[test]
    fn verify_args_duplicate_property_is_error() {
        // Контрпример: второй -p молча затирал бы первый, и отчёт «проверено
        // свойств: 1» умалчивал бы о невыполненной проверке.
        assert!(parse_verify_args(&args(&["-p", "F Done", "-p", "G Idle", "m.takt"])).is_err());
        assert!(
            parse_verify_args(&args(&["-p", "F Done", "--property=G Idle", "m.takt"])).is_err(),
            "смешение форм флага повтором быть не перестаёт"
        );
    }

    #[test]
    fn verify_args_second_file_is_error() {
        // Контрпример: verify работает с одним файлом; второй молча
        // проигнорировать — значит соврать о том, что проверено.
        assert!(parse_verify_args(&args(&["a.takt", "b.takt"])).is_err());
    }

    #[test]
    fn verify_args_unknown_flag_is_error() {
        assert!(parse_verify_args(&args(&["--target", "c", "m.takt"])).is_err());
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
        let args = vec!["main.takt".to_string()];
        let opts = parse_compile_args(&args).unwrap();
        assert_eq!(opts.input_file, "main.takt");
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
            "main.takt".to_string(),
        ];
        let opts = parse_compile_args(&args).unwrap();
        assert_eq!(opts.include_dirs, vec!["/lib/lam"]);
    }

    /// Флаг `-I` с разделителем платформы — два пути.
    #[test]
    fn parse_include_dirs_separator() {
        let args = vec![
            "-I".to_string(),
            format!("/lib/lam{SEP}/usr/lam"),
            "main.takt".to_string(),
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
            "main.takt".to_string(),
        ];
        let opts = parse_compile_args(&args).unwrap();
        assert_eq!(opts.include_dirs, vec!["/a", "/b"]);
    }

    /// Длинный флаг `--include-dirs`.
    #[test]
    fn parse_include_dirs_long_flag() {
        let args = vec![
            "--include-dirs".to_string(),
            format!("/lib{SEP}/usr"),
            "main.takt".to_string(),
        ];
        let opts = parse_compile_args(&args).unwrap();
        assert_eq!(opts.include_dirs, vec!["/lib", "/usr"]);
    }

    /// Слитная форма: `-I/path` без пробела.
    #[test]
    fn parse_include_dir_glued() {
        let args = vec!["-I/lib/lam".to_string(), "main.takt".to_string()];
        let opts = parse_compile_args(&args).unwrap();
        assert_eq!(opts.include_dirs, vec!["/lib/lam"]);
    }

    /// Слитная форма с разделителем платформы: `-I/a:/b` (Unix) / `-I/a;/b` (Windows).
    #[test]
    fn parse_include_dir_glued_separator() {
        let args = vec![format!("-I/a{SEP}/b"), "main.takt".to_string()];
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
            format!("/lib/lam{SEP}/usr/lam"),
            "-I".to_string(),
            "/local/but".to_string(),
            "main.takt".to_string(),
            "-o".to_string(),
            "build/".to_string(),
        ];
        let opts = parse_compile_args(&args).unwrap();
        assert_eq!(opts.target, "c");
        assert_eq!(opts.input_file, "main.takt");
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
            "main.takt".to_string(),
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
            "main.takt".to_string(),
        ];
        let opts = parse_compile_args(&args).unwrap();
        assert_eq!(opts.output_path, "out/");
    }

    /// Короткий флаг целевой платформы `-t`.
    #[test]
    fn parse_target_short_flag() {
        let args = vec!["-t".to_string(), "c".to_string(), "main.takt".to_string()];
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
        let args = vec!["main.takt".to_string(), "-o".to_string()];
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
        let args = vec!["main.takt".to_string(), "-I".to_string()];
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
        let args = vec!["main.takt".to_string(), "--include-dirs".to_string()];
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
        let args = vec!["main.takt".to_string(), "--unknown-flag".to_string()];
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
            format!("/a{SEP}{SEP}/b"),
            "main.takt".to_string(),
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
            "main.takt".to_string(),
        ];
        let opts = parse_compile_args(&args).unwrap();
        assert_eq!(opts.include_dirs, vec!["/first", "/second", "/third"]);
    }

    // ── parse_compile_args: флаги --verbose / --quiet (I6) ───────────────────

    /// Флаг `--verbose` устанавливает `verbose = true`.
    #[test]
    fn parse_verbose_long_flag() {
        let args = vec!["main.takt".to_string(), "--verbose".to_string()];
        let opts = parse_compile_args(&args).unwrap();
        assert!(opts.verbose, "--verbose должен устанавливать verbose=true");
        assert!(!opts.quiet, "--verbose не должен затрагивать quiet");
    }

    /// Короткий флаг `-v` устанавливает `verbose = true`.
    #[test]
    fn parse_verbose_short_flag() {
        let args = vec!["-v".to_string(), "main.takt".to_string()];
        let opts = parse_compile_args(&args).unwrap();
        assert!(opts.verbose, "-v должен устанавливать verbose=true");
    }

    /// Флаг `--quiet` устанавливает `quiet = true`.
    #[test]
    fn parse_quiet_long_flag() {
        let args = vec!["main.takt".to_string(), "--quiet".to_string()];
        let opts = parse_compile_args(&args).unwrap();
        assert!(opts.quiet, "--quiet должен устанавливать quiet=true");
        assert!(!opts.verbose, "--quiet не должен затрагивать verbose");
    }

    /// Короткий флаг `-q` устанавливает `quiet = true`.
    #[test]
    fn parse_quiet_short_flag() {
        let args = vec!["-q".to_string(), "main.takt".to_string()];
        let opts = parse_compile_args(&args).unwrap();
        assert!(opts.quiet, "-q должен устанавливать quiet=true");
    }

    /// По умолчанию ни `verbose`, ни `quiet` не установлены.
    #[test]
    fn parse_defaults_no_verbose_no_quiet() {
        let args = vec!["main.takt".to_string()];
        let opts = parse_compile_args(&args).unwrap();
        assert!(!opts.verbose, "verbose должен быть false по умолчанию");
        assert!(!opts.quiet, "quiet должен быть false по умолчанию");
    }

    /// Одновременное указание `--verbose` и `--quiet` → ошибка.
    #[test]
    fn parse_verbose_and_quiet_is_error() {
        let args = vec![
            "main.takt".to_string(),
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
        let args = vec!["main.takt".to_string(), "-v".to_string(), "-q".to_string()];
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
            "main.takt".to_string(),
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
            "main.takt".to_string(),
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
            "main.takt".to_string(),
            "--address-map".to_string(),
            "stm32.map".to_string(),
        ];
        let opts = parse_compile_args(&args).unwrap();
        assert_eq!(opts.address_map.as_deref(), Some("stm32.map"));
    }

    /// T6: три формы флага `--define` дают одно и то же.
    ///
    /// Слитная форма разбирается веткой, стоящей ПОСЛЕ раздельной, — иначе она
    /// перехватывала бы сам `-D` (образец: `-I`).
    #[test]
    fn define_flag_forms_are_equivalent() {
        let expected = vec!["N=0x1".to_string()];
        for args in [
            vec![
                "m.takt".to_string(),
                "--define".to_string(),
                "N=0x1".to_string(),
            ],
            vec!["m.takt".to_string(), "-D".to_string(), "N=0x1".to_string()],
            vec!["m.takt".to_string(), "-DN=0x1".to_string()],
        ] {
            let opts = parse_compile_args(&args).unwrap();
            assert_eq!(opts.defines, expected, "форма: {args:?}");
        }
    }

    /// T7: флаг повторяем — символы копятся, а не затирают друг друга.
    #[test]
    fn define_flag_is_repeatable() {
        let args = vec![
            "m.takt".to_string(),
            "-D".to_string(),
            "A=0x1".to_string(),
            "-DB=0x2".to_string(),
        ];
        let opts = parse_compile_args(&args).unwrap();
        assert_eq!(opts.defines, vec!["A=0x1".to_string(), "B=0x2".to_string()]);
    }

    /// По умолчанию define'ов нет → поведение `taktc` идентично прежнему.
    #[test]
    fn defines_absent_by_default() {
        let opts = parse_compile_args(&["m.takt".to_string()]).unwrap();
        assert!(opts.defines.is_empty());
    }

    /// `-D` без аргумента — отказ (как у `--address-map`).
    #[test]
    fn define_requires_argument() {
        let args = vec!["m.takt".to_string(), "-D".to_string()];
        assert!(parse_compile_args(&args).is_err());
    }

    /// T21: разбор `-D` не проглатывает чужие флаги.
    #[test]
    fn unknown_flag_is_still_rejected() {
        let args = vec!["m.takt".to_string(), "-Q".to_string(), "foo".to_string()];
        assert!(
            parse_compile_args(&args).is_err(),
            "-Q обязан остаться неизвестным"
        );
    }

    /// По умолчанию карта адресов не задана.
    #[test]
    fn address_map_absent_by_default() {
        let opts = parse_compile_args(&["main.takt".to_string()]).unwrap();
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
        let opts = parse_compile_args(&["main.takt".to_string()]).unwrap();
        assert_eq!(opts.float_width, takt_lang::FloatWidth::W64);
    }

    /// T8 (0029-03): `--float-width=32` → `float`.
    #[test]
    fn parse_float_width_32() {
        let args = vec!["main.takt".to_string(), "--float-width=32".to_string()];
        let opts = parse_compile_args(&args).unwrap();
        assert_eq!(opts.float_width, takt_lang::FloatWidth::W32);
    }

    /// Форма с пробелом — наравне со слитной (как у прочих флагов).
    #[test]
    fn parse_float_width_separate_argument() {
        let args = vec![
            "main.takt".to_string(),
            "--float-width".to_string(),
            "32".to_string(),
        ];
        let opts = parse_compile_args(&args).unwrap();
        assert_eq!(opts.float_width, takt_lang::FloatWidth::W32);
    }

    /// T16 (0029-03): `--float-width=16` — ошибка разбора, а не молчаливое
    /// умолчание: иначе код собрался бы с ДРУГОЙ точностью, чем просил
    /// пользователь.
    #[test]
    fn float_width_rejects_unsupported_value() {
        let args = vec!["main.takt".to_string(), "--float-width=16".to_string()];
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

    // ── Флаги float→q (фича 0096, задача 0096-01) ───────────────────────────────

    /// T2: `--float-as-q=10.22` → точность `(10, 22)`; обе формы флага.
    #[test]
    fn parse_float_as_q_valid() {
        let slit =
            parse_compile_args(&["m.takt".to_string(), "--float-as-q=10.22".to_string()]).unwrap();
        assert_eq!(slit.float_as_q, Some((10, 22)));
        let sep = parse_compile_args(&[
            "m.takt".to_string(),
            "--float-as-q".to_string(),
            "8.8".to_string(),
        ])
        .unwrap();
        assert_eq!(sep.float_as_q, Some((8, 8)));
    }

    /// Умолчание: без флага — `None` (прежнее поведение, T1).
    #[test]
    fn float_as_q_defaults_to_none() {
        let o = parse_compile_args(&["m.takt".to_string()]).unwrap();
        assert_eq!(o.float_as_q, None);
        assert!(!o.float_embedded);
    }

    /// T3: контрпримеры границ (правило 1 ADR 0061) и формата — ошибка CLI.
    #[test]
    fn float_as_q_rejects_out_of_bounds_and_bad_format() {
        for bad in ["40.40", "0.8", "8.0", "abc", "8", "8.x"] {
            let arg = format!("--float-as-q={bad}");
            let err = parse_compile_args(&["m.takt".to_string(), arg]).unwrap_err();
            assert!(err.contains("--float-as-q"), "для '{bad}' сообщение: {err}");
        }
    }

    /// `--float-embedded` — булев флаг.
    #[test]
    fn parse_float_embedded_flag() {
        let o = parse_compile_args(&[
            "m.takt".to_string(),
            "--float-as-q=8.8".to_string(),
            "--float-embedded".to_string(),
        ])
        .unwrap();
        assert!(o.float_embedded);
    }

    // ── Область проверки verify (фича 0051, задача 0051-02) ──────────────────

    #[test]
    fn verify_scope_defaults_to_file() {
        let o = parse_verify_args(&["m.takt".to_string()]).unwrap();
        assert_eq!(o.scope, takt_lang::VerifyScope::File);
    }

    #[test]
    fn verify_scope_is_parsed_in_both_forms() {
        let split = parse_verify_args(&[
            "--scope".to_string(),
            "all".to_string(),
            "m.takt".to_string(),
        ])
        .unwrap();
        assert_eq!(split.scope, takt_lang::VerifyScope::All);
        let joined = parse_verify_args(&["--scope=all".to_string(), "m.takt".to_string()]).unwrap();
        assert_eq!(joined.scope, takt_lang::VerifyScope::All);
    }

    /// A5: негодная область — отказ, а не молчаливое умолчание.
    ///
    /// Иначе `--scope al` проверил бы свой файл и отчитался «все держатся», а
    /// пользователь считал бы, что импорты тоже проверены.
    #[test]
    fn verify_unknown_scope_is_rejected() {
        let err = parse_verify_args(&["--scope=al".to_string(), "m.takt".to_string()]).unwrap_err();
        assert!(err.contains("al"), "сообщение: {err}");
        assert!(
            err.contains("file"),
            "подсказать допустимые значения: {err}"
        );
    }

    #[test]
    fn verify_scope_requires_value() {
        let err = parse_verify_args(&["--scope".to_string()]).unwrap_err();
        assert!(err.contains("--scope"), "сообщение: {err}");
    }
}
