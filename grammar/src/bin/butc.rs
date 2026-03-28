//! Компилятор BuT — утилита командной строки.
//!
//! # Использование
//!
//! ```text
//! butc compile --target c input.but -o output_dir
//! butc compile input.but           # вывод в ./output
//! butc --help                      # справка
//! ```
//!
//! # Целевые платформы
//!
//! - `c` — генерация C-заголовочного файла (по умолчанию)

use std::env;
use std::fs;
use std::process;

/// Выводит справку по использованию утилиты.
fn print_usage() {
    eprintln!("Использование: butc compile [--target <c>] <input.but> [-o <output_dir>]");
    eprintln!("               butc --help");
    eprintln!();
    eprintln!("Целевые платформы:");
    eprintln!("  c    Генерация C-заголовочного файла (по умолчанию)");
}

fn main() {
    let args: Vec<String> = env::args().collect();

    if args.len() < 2 || args.iter().any(|a| a == "--help" || a == "-h") {
        print_usage();
        process::exit(0);
    }

    if args[1] != "compile" {
        eprintln!("Ошибка: неизвестная команда '{}'. Используйте 'compile'.", args[1]);
        print_usage();
        process::exit(1);
    }

    // Разбираем аргументы
    let mut target = "c".to_string();
    let mut input_file: Option<String> = None;
    let mut output_dir: Option<String> = None;

    let mut i = 2;
    while i < args.len() {
        match args[i].as_str() {
            "--target" | "-t" => {
                i += 1;
                if i < args.len() {
                    target = args[i].clone();
                } else {
                    eprintln!("Ошибка: --target требует аргумент");
                    process::exit(1);
                }
            }
            "-o" | "--output" => {
                i += 1;
                if i < args.len() {
                    output_dir = Some(args[i].clone());
                } else {
                    eprintln!("Ошибка: -o требует аргумент");
                    process::exit(1);
                }
            }
            arg if !arg.starts_with('-') => {
                input_file = Some(arg.to_string());
            }
            arg => {
                eprintln!("Ошибка: неизвестный флаг '{}'", arg);
                print_usage();
                process::exit(1);
            }
        }
        i += 1;
    }

    let input_path = match input_file {
        Some(p) => p,
        None => {
            eprintln!("Ошибка: не указан входной файл");
            print_usage();
            process::exit(1);
        }
    };

    let source = match fs::read_to_string(&input_path) {
        Ok(s) => s,
        Err(e) => {
            eprintln!("Ошибка чтения файла '{}': {}", input_path, e);
            process::exit(1);
        }
    };

    let out_path = output_dir.as_deref().unwrap_or("output");

    match target.as_str() {
        "c" => {
            if let Err(diag) = grammar::compile_to_c(&source, out_path) {
                eprintln!("Ошибка компиляции: {}", diag.message);
                process::exit(1);
            }
            eprintln!("Скомпилировано: {} → {}/", input_path, out_path);
        }
        t => {
            eprintln!("Ошибка: неизвестная цель '{}'. Поддерживается: c", t);
            process::exit(1);
        }
    }
}
