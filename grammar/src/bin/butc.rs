//! Компилятор BuT — утилита командной строки.
//!
//! # Использование
//!
//! ```text
//! butc compile [--target c] [-I dir1:dir2] [--verbose | --quiet] <input.but> [-o output_dir]
//! butc compile input.but           # вывод в ./output
//! butc --help                      # справка
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
//! butc compile -I /usr/lib/but:/home/user/but main.but -o out
//!
//! # Несколько флагов -I
//! butc compile -I /usr/lib/but -I /home/user/but main.but
//!
//! # Слитная форма без пробела
//! butc compile -I/usr/lib/but main.but
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

use std::env;
use std::fs;
use std::process;

/// Параметры компиляции, разобранные из аргументов командной строки.
#[derive(Debug, PartialEq)]
pub struct CompileOptions {
    /// Целевой язык генерации (по умолчанию `"c"`).
    pub target: String,
    /// Путь к входному `.but`-файлу.
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
///     "-I".to_string(), "/lib/but:/usr/but".to_string(),
///     "main.but".to_string(),
/// ];
/// let opts = parse_compile_args(&args).unwrap();
/// assert_eq!(opts.include_dirs, vec!["/lib/but", "/usr/but"]);
/// assert_eq!(opts.input_file, "main.but");
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
    })
}

/// Выводит справку по использованию утилиты в stderr.
fn print_usage() {
    eprintln!("Использование: butc compile [флаги] <input.but> [-o <output>]");
    eprintln!("               butc --help");
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
    eprintln!();
    eprintln!("Целевые платформы:");
    eprintln!("  c    Генерация C-заголовочного файла");
    eprintln!();
    eprintln!("Примеры:");
    eprintln!("  butc compile main.but");
    eprintln!("  butc compile -I /lib/but:/home/user/but main.but -o build/");
    eprintln!("  butc compile -I /lib/but -I /home/user/but --target c main.but");
    eprintln!("  butc compile --verbose main.but");
    eprintln!("  butc compile --quiet main.but -o dist/");
}

fn main() {
    let args: Vec<String> = env::args().collect();

    if args.len() < 2 || args.iter().any(|a| a == "--help" || a == "-h") {
        print_usage();
        process::exit(0);
    }

    if args[1] != "compile" {
        eprintln!(
            "Ошибка: неизвестная команда '{}'. Используйте 'compile'.",
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

    match options.target.as_str() {
        "c" => {
            if let Err(diag) = grammar::compile_to_c(
                &options.input_file,
                &source,
                &options.output_path,
                &options.include_dirs,
                options.guard_enable,
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
        t => {
            eprintln!("Ошибка: неизвестная цель '{}'. Поддерживается: c", t);
            process::exit(1);
        }
    }
}

// ─── Тесты ────────────────────────────────────────────────────────────────────

#[cfg(test)]
mod tests {
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
        let args = vec!["main.but".to_string()];
        let opts = parse_compile_args(&args).unwrap();
        assert_eq!(opts.input_file, "main.but");
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
            "/lib/but".to_string(),
            "main.but".to_string(),
        ];
        let opts = parse_compile_args(&args).unwrap();
        assert_eq!(opts.include_dirs, vec!["/lib/but"]);
    }

    /// Флаг `-I` с двоеточием — два пути.
    #[test]
    fn parse_include_dirs_colon() {
        let args = vec![
            "-I".to_string(),
            "/lib/but:/usr/but".to_string(),
            "main.but".to_string(),
        ];
        let opts = parse_compile_args(&args).unwrap();
        assert_eq!(opts.include_dirs, vec!["/lib/but", "/usr/but"]);
    }

    /// Флаг `-I` повторяется дважды — пути объединяются.
    #[test]
    fn parse_multiple_include_flags() {
        let args = vec![
            "-I".to_string(),
            "/a".to_string(),
            "-I".to_string(),
            "/b".to_string(),
            "main.but".to_string(),
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
            "main.but".to_string(),
        ];
        let opts = parse_compile_args(&args).unwrap();
        assert_eq!(opts.include_dirs, vec!["/lib", "/usr"]);
    }

    /// Слитная форма: `-I/path` без пробела.
    #[test]
    fn parse_include_dir_glued() {
        let args = vec!["-I/lib/but".to_string(), "main.but".to_string()];
        let opts = parse_compile_args(&args).unwrap();
        assert_eq!(opts.include_dirs, vec!["/lib/but"]);
    }

    /// Слитная форма с двоеточием: `-I/a:/b`.
    #[test]
    fn parse_include_dir_glued_colon() {
        let args = vec!["-I/a:/b".to_string(), "main.but".to_string()];
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
            "/lib/but:/usr/but".to_string(),
            "-I".to_string(),
            "/local/but".to_string(),
            "main.but".to_string(),
            "-o".to_string(),
            "build/".to_string(),
        ];
        let opts = parse_compile_args(&args).unwrap();
        assert_eq!(opts.target, "c");
        assert_eq!(opts.input_file, "main.but");
        assert_eq!(opts.output_path, "build/");
        assert_eq!(
            opts.include_dirs,
            vec!["/lib/but", "/usr/but", "/local/but"]
        );
    }

    /// Флаг `-o` задаёт выходной путь.
    #[test]
    fn parse_output_flag() {
        let args = vec![
            "main.but".to_string(),
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
            "main.but".to_string(),
        ];
        let opts = parse_compile_args(&args).unwrap();
        assert_eq!(opts.output_path, "out/");
    }

    /// Короткий флаг целевой платформы `-t`.
    #[test]
    fn parse_target_short_flag() {
        let args = vec!["-t".to_string(), "c".to_string(), "main.but".to_string()];
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
        let args = vec!["main.but".to_string(), "-o".to_string()];
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
        let args = vec!["main.but".to_string(), "-I".to_string()];
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
        let args = vec!["main.but".to_string(), "--include-dirs".to_string()];
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
        let args = vec!["main.but".to_string(), "--unknown-flag".to_string()];
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
            "main.but".to_string(),
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
            "main.but".to_string(),
        ];
        let opts = parse_compile_args(&args).unwrap();
        assert_eq!(opts.include_dirs, vec!["/first", "/second", "/third"]);
    }

    // ── parse_compile_args: флаги --verbose / --quiet (I6) ───────────────────

    /// Флаг `--verbose` устанавливает `verbose = true`.
    #[test]
    fn parse_verbose_long_flag() {
        let args = vec!["main.but".to_string(), "--verbose".to_string()];
        let opts = parse_compile_args(&args).unwrap();
        assert!(opts.verbose, "--verbose должен устанавливать verbose=true");
        assert!(!opts.quiet, "--verbose не должен затрагивать quiet");
    }

    /// Короткий флаг `-v` устанавливает `verbose = true`.
    #[test]
    fn parse_verbose_short_flag() {
        let args = vec!["-v".to_string(), "main.but".to_string()];
        let opts = parse_compile_args(&args).unwrap();
        assert!(opts.verbose, "-v должен устанавливать verbose=true");
    }

    /// Флаг `--quiet` устанавливает `quiet = true`.
    #[test]
    fn parse_quiet_long_flag() {
        let args = vec!["main.but".to_string(), "--quiet".to_string()];
        let opts = parse_compile_args(&args).unwrap();
        assert!(opts.quiet, "--quiet должен устанавливать quiet=true");
        assert!(!opts.verbose, "--quiet не должен затрагивать verbose");
    }

    /// Короткий флаг `-q` устанавливает `quiet = true`.
    #[test]
    fn parse_quiet_short_flag() {
        let args = vec!["-q".to_string(), "main.but".to_string()];
        let opts = parse_compile_args(&args).unwrap();
        assert!(opts.quiet, "-q должен устанавливать quiet=true");
    }

    /// По умолчанию ни `verbose`, ни `quiet` не установлены.
    #[test]
    fn parse_defaults_no_verbose_no_quiet() {
        let args = vec!["main.but".to_string()];
        let opts = parse_compile_args(&args).unwrap();
        assert!(!opts.verbose, "verbose должен быть false по умолчанию");
        assert!(!opts.quiet, "quiet должен быть false по умолчанию");
    }

    /// Одновременное указание `--verbose` и `--quiet` → ошибка.
    #[test]
    fn parse_verbose_and_quiet_is_error() {
        let args = vec![
            "main.but".to_string(),
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
        let args = vec!["main.but".to_string(), "-v".to_string(), "-q".to_string()];
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
            "main.but".to_string(),
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
            "main.but".to_string(),
            "-o".to_string(),
            "dist/".to_string(),
        ];
        let opts = parse_compile_args(&args).unwrap();
        assert!(opts.quiet);
        assert!(!opts.verbose);
        assert_eq!(opts.output_path, "dist/");
    }
}
