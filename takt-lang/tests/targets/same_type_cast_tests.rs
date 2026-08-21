//! Приведение к ТОМУ ЖЕ типу опускается (фичи 0361, 0374).
//!
//! # Что было
//!
//! `q := r as u16;` при `r: u16` — законная и осмысленная запись (автор
//! подчёркивает тип) — печаталась буквально:
//!
//! | Цель | Печать | Ответ инструмента |
//! |---|---|---|
//! | `rust` | `self.r as u16` | **`clippy::unnecessary_cast`** — отказ гейта |
//! | `c` | `(uint16_t)model->r` | `cc -Werror` молчит (лишний код) |
//! | `sv` | `16'(samecast_r)` | verilator молчит (лишний код) |
//! | `st` | приведения нет вовсе | — |
//!
//! То есть у одной цели вывод **не собирался** под флагами её же гейта, при
//! нулевом коде возврата `taktc`.
//!
//! # Почему правится у трёх целей, а не у одной
//!
//! Отказ был у `rust`, но правило одно: приведение, ничего не меняющее, в
//! выводе не нужно. Разное поведение трёх целей на одной записи — то, из чего
//! вырастают классы 0084/0193/0195.

use std::process::Command;
use takt_lang::generator::GenerateOptions;

const SAME: &str = "var r: u16 := 300; var q: u16 := 0; out o: u8 at 0x100; \
                    start Run { always { q := r as u16; o := 1; } ref Done: q > 0; } \
                    state Done { }";

/// Приведение, совпадающее ПОСЛЕ отображения типа (фича 0374): типы Takt
/// различны (`duration` и `u32`), а напечатанные совпадают — `duration`
/// отображается в `u32`/`uint32_t`/`logic [31:0]` (правило 0183).
const MAPPED: &str = "var d: duration := 5ms; var ms: u32 := 0; out o: u8 at 0x100; \
                      start Run { always { ms := d as u32; o := 1; } ref Done: ms > 0; } \
                      state Done { }";

/// **Контрпример:** настоящее приведение остаётся на месте.
const REAL: &str = "var b: u8 := 200; var w: u16 := 0; out o: u8 at 0x100; \
                    start Run { always { w := b as u16; o := 1; } ref Done: w > 0; } \
                    state Done { }";

fn generate(tag: &str, target: &str, source: &str) -> (std::path::PathBuf, String) {
    let thread = std::thread::current()
        .name()
        .unwrap_or("single")
        .replace(':', "_");
    let dir = std::env::temp_dir().join(format!("takt_0361_{tag}_{target}_{thread}"));
    let _ = std::fs::remove_dir_all(&dir);
    std::fs::create_dir_all(&dir).expect("каталог");
    let path = dir.to_str().expect("путь");
    let opts = GenerateOptions::default();
    match target {
        "c" => takt_lang::compile_to_c("probe", source, path, &[], &opts).map(|_| ()),
        "rust" => takt_lang::compile_to_rust("probe", source, path, &[], &opts).map(|_| ()),
        _ => takt_lang::compile_to_sv("probe", source, path, &[], &opts).map(|_| ()),
    }
    .unwrap_or_else(|e| panic!("порождение для '{target}': {e:?}"));
    let ext = match target {
        "c" => "c",
        "rust" => "rs",
        _ => "sv",
    };
    let text = std::fs::read_to_string(dir.join(format!("probe.{ext}"))).expect("чтение");
    (dir, text)
}

/// Приведение к тому же типу не печатается ни одной из трёх целей.
#[test]
fn same_type_cast_is_omitted() {
    let (_d, rust) = generate("same", "rust", SAME);
    assert!(
        !rust.contains("as u16"),
        "`r as u16` при `r: u16` — это `clippy::unnecessary_cast`, отказ гейта.\n{rust}"
    );
    let (_d, c) = generate("same", "c", SAME);
    assert!(!c.contains("(uint16_t)"), "правило одно на три цели.\n{c}");
    let (_d, sv) = generate("same", "sv", SAME);
    assert!(!sv.contains("16'("), "то же у цели `sv`.\n{sv}");
}

/// Приведение, совпадающее после ОТОБРАЖЕНИЯ типа, не печатается (фича 0374).
///
/// ⚠️ Признак 0361 сравнивал типы **Takt**, и `duration as u32` под него не
/// подпадал: типы разные. Замер 2026-08-21 — `clippy -D warnings` отвечал
/// «casting to the same type is unnecessary (`u32` -> `u32`)», то есть вывод
/// цели `rust` не собирался под флагами её же гейта при нулевом коде возврата
/// `taktc`.
#[test]
fn cast_matching_after_type_mapping_is_omitted() {
    let (_d, rust) = generate("mapped", "rust", MAPPED);
    assert!(
        !rust.contains("as u32"),
        "`d as u32` при `d: duration` — это `clippy::unnecessary_cast`.\n{rust}"
    );
    let (_d, c) = generate("mapped", "c", MAPPED);
    assert!(
        !c.contains("(uint32_t)model->d"),
        "правило одно на три цели.\n{c}"
    );
    let (_d, sv) = generate("mapped", "sv", MAPPED);
    assert!(!sv.contains("32'("), "то же у цели `sv`.\n{sv}");
}

/// **Контрпример:** приведение, меняющее тип, остаётся.
///
/// Без него правка читается как «приведения не печатаем вовсе», и `u8 → u16`
/// потерялось бы вместе с расширением.
#[test]
fn real_cast_is_kept() {
    let (_d, rust) = generate("real", "rust", REAL);
    assert!(
        rust.contains("as u16"),
        "приведение, меняющее тип, обязано остаться.\n{rust}"
    );
    let (_d, c) = generate("real", "c", REAL);
    assert!(c.contains("(uint16_t)"), "то же у цели `c`.\n{c}");
    let (_d, sv) = generate("real", "sv", REAL);
    assert!(sv.contains("16'("), "то же у цели `sv`.\n{sv}");
}

/// **Контрпример к 0374:** `duration → u8` меняет ширину и остаётся.
///
/// Без него правка читается как «приведения от длительности не печатаем».
#[test]
fn narrowing_cast_from_duration_is_kept() {
    const NARROW: &str = "var d: duration := 5ms; var tiny: u8 := 0; out o: u8 at 0x100; \
                          start Run { always { tiny := d as u8; o := 1; } ref Done: tiny > 0; } \
                          state Done { }";
    let (_d, rust) = generate("narrow", "rust", NARROW);
    assert!(rust.contains("as u8"), "сужение обязано остаться.\n{rust}");
    let (_d, c) = generate("narrow", "c", NARROW);
    assert!(c.contains("(uint8_t)"), "то же у цели `c`.\n{c}");
    let (_d, sv) = generate("narrow", "sv", NARROW);
    assert!(sv.contains("8'("), "то же у цели `sv`.\n{sv}");
}

/// Вывод цели `rust` принимается `clippy -D warnings` — тем же гейтом.
#[test]
fn rust_output_passes_clippy() {
    let available = Command::new("clippy-driver")
        .arg("--version")
        .output()
        .map(|o| o.status.success())
        .unwrap_or(false);
    if !available {
        eprintln!("[ПРОПУСК] rust_output_passes_clippy: `clippy-driver` не найден");
        return;
    }
    // Оба входа: совпадение типов Takt (0361) и совпадение после отображения
    // (0374) — второй и был отказом гейта.
    for (tag, source) in [("gate", SAME), ("gate_mapped", MAPPED)] {
        let (dir, _) = generate(tag, "rust", source);
        let out = Command::new("clippy-driver")
            .args(["--edition", "2021", "--crate-type=lib", "-D", "warnings"])
            .arg(dir.join("probe.rs"))
            .arg("--out-dir")
            .arg(dir.join("out"))
            .output()
            .expect("запуск clippy-driver");
        assert!(
            out.status.success(),
            "вывод обязан приниматься гейтом цели ('{tag}'):\n{}",
            String::from_utf8_lossy(&out.stderr)
        );
    }
}
