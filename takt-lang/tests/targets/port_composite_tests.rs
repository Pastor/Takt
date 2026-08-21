//! Порт составного типа (фича 0350).
//!
//! # Что было
//!
//! Замер 2026-08-20 на `out po: Pair at 0;` и `out pa: [u8;2] at 4;`:
//!
//! | Потребитель | Ответ |
//! |---|---|
//! | эталон, `st`, `plantuml` | верно |
//! | **`c`** | `cc`: «passing 'Pair' to parameter of incompatible type 'int64_t'» |
//! | **`sv`** | verilator: «Reference to 'pair_t' before declaration», затем ошибка на массиве |
//! | `c-hal`, `st-at`, `rust`, `sv-mmio` | честный отказ (`CC-015`, `ST-004`, `RS-016`, `SV-002`) |
//!
//! Две цели из восьми порождали невалидный вывод при **нулевом** коде возврата
//! `taktc`, а четыре на том же входе отказывали — то есть согласия не было.
//!
//! # Что сделано
//!
//! - **`c`:** порт составного типа отвергается `CC-015` — как у `c-hal`.
//!   Колбэки HAL принимают скаляр, и структура в них не проходит.
//! - **`sv`:** пользовательские типы объявляются **вне модуля**, поэтому порт
//!   структурного типа стал выразим; порт-**массив** отвергается `SV-002`:
//!   распакованный массив в списке портов **yosys не принимает вовсе**
//!   («syntax error, unexpected '['»), хотя verilator его допускает — форма
//!   выбирается по тому, что принимают **оба** (урок 0235).

use std::path::PathBuf;
use std::process::Command;

const STRUCT_PORT: &str = "struct Pair { a: u8, b: u8 }\nout po: Pair at 0;\n\
     var v: Pair := {1, 2};\nstart Run { always { po := v; } ref Run: v.a = 1; }\n";
const ARRAY_PORT: &str = "out pa: [u8;2] at 4;\nvar v: u8 := 1;\n\
     start Run { always { pa := {3, 4}; } ref Run: v = 1; }\n";

fn build_dir(tag: &str) -> PathBuf {
    let thread = std::thread::current()
        .name()
        .unwrap_or("single")
        .replace(':', "_");
    let dir = std::env::temp_dir().join(format!("takt_0350_{tag}_{thread}"));
    let _ = std::fs::remove_dir_all(&dir);
    std::fs::create_dir_all(&dir).expect("каталог");
    dir
}

/// Цель `c`: порт составного типа — отказ, а не невалидный вывод.
#[test]
fn c_refuses_composite_port() {
    let dir = build_dir("c");
    let err = takt_lang::compile_to_c(
        "cp",
        STRUCT_PORT,
        dir.to_str().expect("путь в UTF-8"),
        &[],
        &takt_lang::generator::GenerateOptions::default(),
    )
    .expect_err("порт структурного типа цель `c` не переводит");
    assert_eq!(err.code.as_deref(), Some("CC-015"));
}

/// Цель `sv`: порт структурного типа переводится, оба инструмента чисты.
///
/// ⚠️ Проверяется **позиция** `typedef`: он обязан стоять до `module`, иначе
/// шапка ссылается на необъявленный тип.
#[test]
fn sv_struct_port_declares_type_before_module() {
    let dir = build_dir("sv");
    takt_lang::compile_to_sv(
        "cp",
        STRUCT_PORT,
        dir.to_str().expect("путь в UTF-8"),
        &[],
        &takt_lang::generator::GenerateOptions::default(),
    )
    .expect("порождение SystemVerilog");
    let text = std::fs::read_to_string(dir.join("cp.sv")).expect("чтение");
    let typedef = text.find("} pair_t;").expect("тип объявлен");
    let module = text.find("module cp").expect("модуль объявлен");
    assert!(
        typedef < module,
        "тип обязан объявляться до шапки модуля:\n{text}"
    );

    let available = Command::new("verilator")
        .arg("--version")
        .output()
        .map(|o| o.status.success())
        .unwrap_or(false);
    if !available {
        eprintln!("[ПРОПУСК] sv_struct_port_declares_type_before_module: verilator не найден");
        return;
    }
    let lint = Command::new("verilator")
        .args(["--lint-only", "-Wall"])
        .arg(dir.join("cp.sv"))
        .output()
        .expect("запуск verilator");
    assert!(
        lint.status.success(),
        "verilator не принял модуль:\n{}",
        String::from_utf8_lossy(&lint.stderr)
    );
}

/// Цель `sv`: порт-массив отвергается с названной причиной.
///
/// ⚠️ Контроль границы: структура выразима, массив — нет, и различает их не
/// вкус, а прогон **yosys**.
#[test]
fn sv_refuses_array_port() {
    let dir = build_dir("sv_arr");
    let err = takt_lang::compile_to_sv(
        "ap",
        ARRAY_PORT,
        dir.to_str().expect("путь в UTF-8"),
        &[],
        &takt_lang::generator::GenerateOptions::default(),
    )
    .expect_err("порт-массив цель `sv` не переводит");
    assert_eq!(err.code.as_deref(), Some("SV-002"));
    assert!(
        err.message.contains("распакованный массив"),
        "отказ обязан называть причину:\n{}",
        err.message
    );
}
