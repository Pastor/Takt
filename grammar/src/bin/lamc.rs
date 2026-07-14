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
//! - `plantuml` — генерация диаграммы состояний в формате PlantUML

use std::env;
use std::fs;
use std::io;
use std::path::{Path, PathBuf};
use std::process;

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
    })
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

/// Выводит справку по использованию утилиты в stderr.
fn print_usage() {
    eprintln!("Использование: lamc compile [флаги] <input.lam> [-o <output>]");
    eprintln!("               lamc fmt [--check] [--stdin] <файлы/каталоги>");
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
    eprintln!();
    eprintln!("Целевые платформы:");
    eprintln!("  c         Генерация C-заголовочного файла");
    eprintln!("  c-hal     C + таблица адресов портов и дефолтный HAL (фича 0020)");
    eprintln!("  plantuml  Генерация диаграммы состояний PlantUML (.puml)");
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

    if args[1] != "compile" {
        eprintln!(
            "Ошибка: неизвестная команда '{}'. Используйте 'compile' или 'fmt'.",
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

    if !external_entries.is_empty() && options.target != "c-hal" {
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
                &grammar::GenerateOptions::new(options.guard_enable),
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
                    eprintln!(
                        "Ошибка компиляции [{}]: {}",
                        diag.code.as_deref().unwrap_or("?"),
                        diag.message
                    );
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
                &grammar::GenerateOptions::new(options.guard_enable),
            ) {
                eprintln!("Ошибка компиляции: {}", diag.message);
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
                eprintln!("Ошибка компиляции: {}", diag.message);
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
        t => {
            eprintln!(
                "Ошибка: неизвестная цель '{}'. Поддерживается: c, plantuml",
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
}
