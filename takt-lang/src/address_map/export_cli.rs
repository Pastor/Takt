//! CLI-обвязка подкоманды `taktc address-map`.
//!
//! Подкоманда живёт в библиотеке, а бинарник держит лишь диспетчер
//! `args[1] == "address-map" -> run_export_subcommand`.
//!
//! Подкоманда разрешает адреса портов (тем же [`resolve_addresses`], что потребляет `-t
//! c-hal`) и выгружает **фактически разрешённую** карту в формат `map` (`.ld`-подобный,
//! замыкается через `--address-map`) или `json` (машиночитаемый: тип, направление,
//! источник).

use super::export::{export_address_map, export_address_map_json};
use super::{parse_address_map, parse_defines, resolve_addresses};
use crate::AddressMapEntry;
use std::fs;
use std::io::Write as _;
use std::rc::Rc;

use crate::diagnostics::lang::keys;
use crate::msg;

/// Разбивает список путей поиска импортов по платформенному разделителю (`:` на Unix,
/// `;` на Windows - там путь может начинаться с буквы диска).
///
/// ```
/// # use takt_lang::address_map::split_include_dirs;
/// assert_eq!(split_include_dirs("/a:/b:/c"), vec!["/a", "/b", "/c"]);
/// assert_eq!(split_include_dirs("/a::/b"), vec!["/a", "/b"]);
/// assert!(split_include_dirs("").is_empty());
/// ```
pub fn split_include_dirs(s: &str) -> Vec<String> {
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

/// Формат выгрузки карты адресов (`--emit`).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum EmitFormat {
    /// `.ld`-подобный формат - замыкается через `--address-map` (круговой рейс).
    Map,
    /// Машиночитаемый JSON: тип, направление, источник адреса, версия формата.
    Json,
}

/// Опции подкоманды `address-map`.
#[derive(Debug)]
pub struct AddressMapOptions {
    /// Входной `.takt`.
    pub input_file: String,
    /// Путь выходного файла; `None` - печать в stdout.
    pub output_path: Option<String>,
    /// Пути поиска импортов (`-I`).
    pub include_dirs: Vec<String>,
    /// Символы платформы для выражений адреса (`-D`).
    pub defines: Vec<String>,
    /// Внешняя карта адресов (`--address-map`) - источник высшего приоритета.
    pub address_map: Option<String>,
    /// Формат выгрузки (`--emit`, по умолчанию `map`).
    pub emit: EmitFormat,
    /// Тихий режим: подавить предупреждения (SE-050/051/...), но не ошибки.
    pub quiet: bool,
}

/// Разбирает значение флага `--emit`. Неизвестный формат - **ошибка**, а не умолчание:
/// `--emit svd` обязан внятно сказать, что SVD не поставляется (решение ), а не выдать
/// пустой/неверный файл.
fn parse_emit_format(s: &str) -> Result<EmitFormat, String> {
    match s {
        "map" => Ok(EmitFormat::Map),
        "json" => Ok(EmitFormat::Json),
        other => Err(msg!(keys::CLI_ADDRESS_MAP_EMIT_UNKNOWN, value = other)),
    }
}

/// Разбирает аргументы подкоманды `address-map` (без `"address-map"` в начале).
pub fn parse_address_map_args(args: &[String]) -> Result<AddressMapOptions, String> {
    let mut input_file: Option<String> = None;
    let mut output_path: Option<String> = None;
    let mut include_dirs: Vec<String> = Vec::new();
    let mut defines: Vec<String> = Vec::new();
    let mut address_map: Option<String> = None;
    let mut emit = EmitFormat::Map;
    let mut quiet = false;

    let mut i = 0;
    while i < args.len() {
        let a = &args[i];
        match a.as_str() {
            "--emit" => {
                i += 1;
                let v = args
                    .get(i)
                    .ok_or_else(|| msg!(keys::CLI_ADDRESS_MAP_EMIT_NEEDS_VALUE))?;
                emit = parse_emit_format(v)?;
            }
            s if s.starts_with("--emit=") => {
                emit = parse_emit_format(&s["--emit=".len()..])?;
            }
            "-o" | "--output" => {
                i += 1;
                output_path = Some(
                    args.get(i)
                        .ok_or_else(|| msg!(keys::CLI_ADDRESS_MAP_OUTPUT_NEEDS_PATH))?
                        .clone(),
                );
            }
            "-I" | "--include-dirs" => {
                i += 1;
                let v = args
                    .get(i)
                    .ok_or_else(|| msg!(keys::CLI_ADDRESS_MAP_INCLUDE_NEEDS_PATH))?;
                include_dirs.extend(split_include_dirs(v));
            }
            s if s.starts_with("-I") && s.len() > 2 => {
                include_dirs.extend(split_include_dirs(&s[2..]));
            }
            "-D" | "--define" => {
                i += 1;
                defines.push(
                    args.get(i)
                        .ok_or_else(|| msg!(keys::CLI_ADDRESS_MAP_DEFINE_NEEDS_VALUE))?
                        .clone(),
                );
            }
            s if s.starts_with("-D") && s.len() > 2 => {
                defines.push(s[2..].to_string());
            }
            "--address-map" => {
                i += 1;
                address_map = Some(
                    args.get(i)
                        .ok_or_else(|| msg!(keys::CLI_ADDRESS_MAP_MAP_NEEDS_FILE))?
                        .clone(),
                );
            }
            "--quiet" | "-q" => quiet = true,
            unknown if unknown.starts_with('-') => {
                return Err(msg!(keys::CLI_UNKNOWN_FLAG, flag = unknown));
            }
            positional => {
                if input_file.is_some() {
                    return Err(msg!(keys::CLI_ADDRESS_MAP_MANY_INPUTS));
                }
                input_file = Some(positional.to_string());
            }
        }
        i += 1;
    }

    let input_file = input_file.ok_or_else(|| msg!(keys::CLI_NO_INPUT_FILE))?;
    Ok(AddressMapOptions {
        input_file,
        output_path,
        include_dirs,
        defines,
        address_map,
        emit,
        quiet,
    })
}

/// Разбирает аргументы и исполняет подкоманду; возвращает код завершения. Точка входа
/// для тонкого диспетчера в `bin/taktc.rs`.
pub fn run_export_subcommand(args: &[String]) -> i32 {
    match parse_address_map_args(args) {
        Ok(options) => run(&options),
        Err(e) => {
            eprintln!("{}", msg!(keys::CLI_ARGS_ERROR, error = e));
            eprintln!("{}", msg!(keys::CLI_ADDRESS_MAP_USAGE));
            1
        }
    }
}

/// Исполняет подкоманду `address-map`: разрешает адреса портов (как `-t c-hal`) и
/// выгружает фактически разрешённую карту в формат `map`/`json`.
///
/// Коды: `0` - выгрузка удалась; `1` - ошибка (чтение/разбор/`SE-052` и пр.).
/// `SE-052` (достижимый порт без адреса) обрывает экспорт - как и сборку `c-hal`:
/// неполную карту наружу не отдаём. Предупреждения (`SE-050`/`051`/`053`) идут в
/// **stderr**, чтобы stdout оставался чистой выгрузкой (её парсит `--address-map`).
fn run(options: &AddressMapOptions) -> i32 {
    let source = match fs::read_to_string(&options.input_file) {
        Ok(s) => s,
        Err(e) => {
            eprintln!(
                "{}",
                msg!(
                    keys::CLI_READ_FILE_ERROR,
                    path = options.input_file,
                    error = e
                )
            );
            return 1;
        }
    };

    let external: Vec<AddressMapEntry> = match &options.address_map {
        Some(path) => {
            let map_src = match fs::read_to_string(path) {
                Ok(s) => s,
                Err(e) => {
                    eprintln!(
                        "{}",
                        msg!(keys::CLI_ADDRESS_MAP_READ_ERROR, path = path, error = e)
                    );
                    return 1;
                }
            };
            match parse_address_map(&map_src, 0) {
                Ok(entries) => entries,
                Err(diags) => {
                    for d in diags {
                        eprintln!(
                            "{}",
                            msg!(
                                keys::CLI_ADDRESS_MAP_ERROR,
                                code = d.code.as_deref().unwrap_or("?"),
                                message = d.message
                            )
                        );
                    }
                    return 1;
                }
            }
        }
        None => Vec::new(),
    };

    let address_env = match parse_defines(&options.defines) {
        Ok(env) => env,
        Err(diags) => {
            for d in diags {
                eprintln!(
                    "{}",
                    msg!(
                        keys::CLI_DEFINE_ERROR,
                        code = d.code.as_deref().unwrap_or("?"),
                        message = d.message
                    )
                );
            }
            return 1;
        }
    };

    let (ast, _) = match crate::parse(&source, 0) {
        Ok(parsed) => parsed,
        Err(diags) => {
            for d in diags {
                eprintln!(
                    "{}",
                    msg!(
                        keys::CLI_PARSE_ERROR,
                        code = d.code.as_deref().unwrap_or("?"),
                        message = d.message
                    )
                );
            }
            return 1;
        }
    };
    let model = match crate::semantic::tree::construct_model(&ast, None, &options.include_dirs) {
        Ok(m) => m,
        Err(d) => {
            eprintln!(
                "{}",
                msg!(
                    keys::CLI_SEMANTIC_ERROR,
                    code = d.code.as_deref().unwrap_or("?"),
                    message = d.message
                )
            );
            return 1;
        }
    };

    let resolution = resolve_addresses(Rc::clone(&model), &external, &address_env);

    // Диагностики в stderr. Ошибка (SE-052/054/055) обрывает экспорт: неполную карту
    // наружу не отдаём (симметрия с проверкой полноты `c-hal`).
    let mut has_error = false;
    for d in &resolution.diagnostics {
        let is_error = matches!(d.level, crate::diagnostics::Level::Error);
        if is_error {
            has_error = true;
        } else if options.quiet {
            continue;
        }
        let code = d.code.as_deref().unwrap_or("?");
        let line = if is_error {
            msg!(
                keys::CLI_ADDRESS_MAP_DIAG_ERROR,
                code = code,
                message = d.message
            )
        } else {
            msg!(keys::DIAG_WARNING, code = code, message = d.message)
        };
        eprintln!("{}{line}", crate::diagnostics::position_prefix(d));
    }
    if has_error {
        return 1;
    }

    let out = match options.emit {
        EmitFormat::Map => export_address_map(&resolution),
        EmitFormat::Json => export_address_map_json(&resolution),
    };

    match &options.output_path {
        Some(path) => {
            if let Err(e) = fs::write(path, &out) {
                eprintln!("{}", msg!(keys::CLI_WRITE_ERROR, path = path, error = e));
                return 1;
            }
        }
        None => {
            // Печать в stdout без паники на закрытом канале (напр.
            let _ = std::io::stdout().write_all(out.as_bytes());
        }
    }
    0
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn split_basic() {
        assert_eq!(split_include_dirs("/a:/b:/c"), vec!["/a", "/b", "/c"]);
    }

    #[test]
    fn split_skips_empty_segments() {
        assert_eq!(split_include_dirs("/a::/b"), vec!["/a", "/b"]);
    }

    #[test]
    fn split_trims_whitespace() {
        assert_eq!(split_include_dirs("  /x  :  /y  "), vec!["/x", "/y"]);
    }

    #[test]
    fn split_empty_is_empty() {
        assert!(split_include_dirs("").is_empty());
        assert!(split_include_dirs("   ").is_empty());
    }

    #[test]
    fn split_single() {
        assert_eq!(split_include_dirs("/only/one"), vec!["/only/one"]);
    }

    #[test]
    fn emit_format_parses_map_and_json() {
        assert_eq!(parse_emit_format("map"), Ok(EmitFormat::Map));
        assert_eq!(parse_emit_format("json"), Ok(EmitFormat::Json));
    }

    #[test]
    fn emit_format_rejects_svd_with_mention() {
        let err = parse_emit_format("svd").unwrap_err();
        assert!(err.contains("SVD"), "ошибка обязана назвать SVD: {err}");
    }

    #[test]
    fn args_default_emit_is_map() {
        let opts = parse_address_map_args(&["m.takt".to_string()]).unwrap();
        assert_eq!(opts.emit, EmitFormat::Map);
        assert_eq!(opts.input_file, "m.takt");
    }

    #[test]
    fn args_parse_full() {
        let args: Vec<String> = [
            "--emit",
            "json",
            "-I",
            "/a",
            "--address-map",
            "p.map",
            "m.takt",
        ]
        .iter()
        .map(|s| s.to_string())
        .collect();
        let opts = parse_address_map_args(&args).unwrap();
        assert_eq!(opts.emit, EmitFormat::Json);
        assert_eq!(opts.include_dirs, vec!["/a"]);
        assert_eq!(opts.address_map.as_deref(), Some("p.map"));
        assert_eq!(opts.input_file, "m.takt");
    }
}
