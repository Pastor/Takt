//! Агрегат в локальном объявлении переводят все цели (фича 0345).
//!
//! # Что было
//!
//! Замер 2026-08-20 на `var p: Point := {0, 0};` внутри функции:
//!
//! | Потребитель | Ответ |
//! |---|---|
//! | эталон, `c`, `c-hal`, `rust`, `plantuml` | верно |
//! | **`st`, `st-at`** | `ST-011`: «агрегат в позиции значения» |
//! | **`sv`, `sv-mmio`** | `SV-002`: «инициализатор структуры» |
//!
//! То есть отказ приходил на запись, у которой **есть** верное поведение, а не
//! на невыразимую конструкцию. Обе цели умеют поэлементную форму — её и не
//! хватало.
//!
//! ⚠️ Класс найден так: проба ставилась под «функция возвращает структуру», и
//! отказ был приписан возврату. Проба `iec2c` показала, что функция,
//! возвращающая структуру, инструментом **принимается**, — предмет оказался в
//! инициализаторе локальной переменной.

use std::path::{Path, PathBuf};
use std::process::Command;

const FIXTURE: &str = "../takt-sim/tests/data/eval/conformance_local_aggregate.takt";

fn source() -> String {
    std::fs::read_to_string(FIXTURE).expect("фикстура читается")
}

fn build_dir(tag: &str) -> PathBuf {
    let thread = std::thread::current()
        .name()
        .unwrap_or("single")
        .replace(':', "_");
    let dir = std::env::temp_dir().join(format!("takt_0345_{tag}_{thread}"));
    let _ = std::fs::remove_dir_all(&dir);
    std::fs::create_dir_all(&dir).expect("каталог");
    dir
}

/// Цель `st`: поэлементно, `iec2c` принимает.
#[test]
fn st_local_aggregate_is_accepted() {
    let dir = build_dir("st");
    takt_lang::compile_to_st(
        "la",
        &source(),
        dir.to_str().expect("путь в UTF-8"),
        &[],
        &takt_lang::generator::GenerateOptions::default(),
    )
    .expect("порождение ST");
    let text = std::fs::read_to_string(dir.join("la.st")).expect("чтение");
    assert!(
        text.contains("p.x := 0;") && text.contains("p.y := 0;"),
        "агрегат обязан печататься поэлементно:\n{text}"
    );
    assert!(
        text.contains("acc := 1;"),
        "контроль: обычный инициализатор не изменился:\n{text}"
    );

    let Some(iec2c) = iec2c_path() else {
        eprintln!("[ПРОПУСК] st_local_aggregate_is_accepted: iec2c не найден");
        return;
    };
    let out = dir.join("st_out");
    std::fs::create_dir_all(&out).expect("каталог");
    let lib = iec2c
        .parent()
        .and_then(Path::parent)
        .map_or_else(|| PathBuf::from("/usr/local"), Path::to_path_buf)
        .join("share/matiec/lib");
    let run = Command::new(&iec2c)
        .args(["-I".as_ref(), lib.as_os_str()])
        .arg("-T")
        .arg(&out)
        .arg(dir.join("la.st"))
        .output()
        .expect("запуск iec2c");
    let stderr = String::from_utf8_lossy(&run.stderr);
    assert!(
        !stderr.contains("error"),
        "iec2c отверг порождённый ST:\n{stderr}"
    );
}

fn iec2c_path() -> Option<PathBuf> {
    let home = std::env::var("HOME").ok()?;
    let path = PathBuf::from(home).join(".local/bin/iec2c");
    path.is_file().then_some(path)
}

/// Цель `sv`: то же, линт чист.
#[test]
fn sv_local_aggregate_is_accepted() {
    let dir = build_dir("sv");
    takt_lang::compile_to_sv(
        "la",
        &source(),
        dir.to_str().expect("путь в UTF-8"),
        &[],
        &takt_lang::generator::GenerateOptions::default(),
    )
    .expect("порождение SystemVerilog");
    let text = std::fs::read_to_string(dir.join("la.sv")).expect("чтение");
    assert!(
        text.contains("p.x = 0;") && text.contains("p.y = 0;"),
        "агрегат обязан печататься поэлементно:\n{text}"
    );

    let available = Command::new("verilator")
        .arg("--version")
        .output()
        .map(|o| o.status.success())
        .unwrap_or(false);
    if !available {
        eprintln!("[ПРОПУСК] sv_local_aggregate_is_accepted: verilator не найден");
        return;
    }
    let lint = Command::new("verilator")
        .args(["--lint-only", "-Wall"])
        .arg(dir.join("la.sv"))
        .output()
        .expect("запуск verilator");
    assert!(
        lint.status.success(),
        "verilator не принял модуль:\n{}",
        String::from_utf8_lossy(&lint.stderr)
    );
}
