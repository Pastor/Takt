//! Отложенная инициализация локальной переменной у цели `rust` (фича 0410).
//!
//! # Что было
//!
//! Замер 2026-08-23 (`scripts/probe.sh`) на `var t: u8; t := 5; o := t;` —
//! записи, которую исполняют эталон и переводят все восемь целей:
//!
//! | Инструмент | Ответ |
//! |---|---|
//! | `cc`, `iec2c`, `verilator`, `yosys` | приняли |
//! | **`rustc -D warnings`** | **ОТВЕРГ:** «variable does not need to be mutable» |
//!
//! Код возврата `taktc` — **ноль**. Причина: объявление без инициализатора
//! печаталось `let mut x: T;` **безусловно**, тогда как первое присваивание —
//! это инициализация, а не изменение.
//!
//! Снятие `mut` открыло **второй** слой: `clippy::needless_late_init` —
//! отложенная форма там, где значение известно следующим оператором. Обе беды
//! лечит приём соседней ветви (мёртвый инициализатор, фича 0216): признак
//! `deferred_needs_mut` и свёртка `fold_assignment`.
//!
//! ⚠️ **Срез из свёртки исключён:** он печатается ПОЭЛЕМЕНТНО (0355), и
//! выражения у него нет — свёрнутый `let part = src[1:3];` дал бы `RS-011`
//! там, где присваивание переводится.

use std::path::PathBuf;
use std::process::Command;
use takt_lang::generator::GenerateOptions;

const SRC: &str = "var o: u8 := 0;\nout probe: u8 at 0;\n\
     start Run {\n    always {\n        var t: u8;\n        t := 5;\n\
     \x20       o := t;\n        probe := o;\n    }\n    ref Run;\n}\n";

fn out_dir(tag: &str) -> PathBuf {
    let thread = std::thread::current()
        .name()
        .unwrap_or("single")
        .replace(':', "_");
    let dir = std::env::temp_dir().join(format!("takt_0410_{tag}_{thread}"));
    let _ = std::fs::remove_dir_all(&dir);
    std::fs::create_dir_all(&dir).expect("каталог вывода");
    dir
}

fn generate(tag: &str, src: &str) -> (PathBuf, String) {
    let dir = out_dir(tag);
    takt_lang::compile_to_rust(
        tag,
        src,
        dir.to_str().expect("путь в UTF-8"),
        &[],
        &GenerateOptions::default(),
    )
    .expect("порождение Rust");
    let text = std::fs::read_to_string(dir.join(format!("{tag}.rs"))).expect("чтение вывода");
    (dir, text)
}

fn tool(name: &str) -> bool {
    Command::new(name)
        .arg("--version")
        .output()
        .map(|o| o.status.success())
        .unwrap_or(false)
}

/// Предмет: отложенное объявление сворачивается и `mut` не печатает.
#[test]
fn deferred_declaration_is_folded_without_mut() {
    let (_, text) = generate("rs0410", SRC);
    assert!(
        text.contains("let t: u8 = 5;"),
        "объявление обязано свернуться в инициализацию:\n{text}"
    );
    assert!(
        !text.contains("let mut t"),
        "первое присваивание — инициализация, `mut` ей не нужен:\n{text}"
    );
}

/// **Контроль:** переменная, которой присваивают дважды, `mut` сохраняет.
///
/// Без него правка читалась бы как «`mut` не печатается никогда», и вывод
/// перестал бы компилироваться на первой же изменяемой переменной.
#[test]
fn twice_assigned_variable_keeps_mut() {
    let src = "var o: u8 := 0;\nout probe: u8 at 0;\n\
         start Run {\n    always {\n        var t: u8;\n        t := 5;\n\
         \x20       t := t + 1;\n        o := t;\n        probe := o;\n    }\n    ref Run;\n}\n";
    let (_, text) = generate("rs0410m", src);
    assert!(
        text.contains("let mut t"),
        "второе присваивание — изменение, `mut` обязателен:\n{text}"
    );
}

/// **Контроль:** срез не сворачивается — он печатается поэлементно (0355).
#[test]
fn slice_assignment_is_not_folded() {
    let src = "var src: [u8; 4] := {5, 6, 7, 8};\nvar o: u8 := 0;\nout probe: u8 at 0;\n\
         start Run {\n    always {\n        var part: [u8; 2] := {0, 0};\n\
         \x20       part := src[1:3];\n        o := part[0];\n        probe := o;\n    }\n\
         \x20   ref Run;\n}\n";
    let (_, text) = generate("rs0410s", src);
    assert!(
        text.contains("part[0]") && text.contains("part[1]"),
        "срез обязан печататься поэлементно:\n{text}"
    );
}

/// Порождённый Rust проходит `rustc` и `clippy` под флагами гейта цели.
#[test]
fn generated_rust_passes_the_gate_tools() {
    if !tool("rustc") || !tool("clippy-driver") {
        eprintln!("[ПРОПУСК] `rustc`/`clippy-driver` не найдены; текст вывода уже проверен");
        return;
    }
    let (dir, _) = generate("rs0410t", SRC);
    for exe in ["rustc", "clippy-driver"] {
        let out = Command::new(exe)
            .args(["--edition", "2021", "--crate-type", "lib", "-D", "warnings"])
            .arg(dir.join("rs0410t.rs"))
            .arg("--out-dir")
            .arg(&dir)
            .output()
            .unwrap_or_else(|e| panic!("запуск {exe}: {e}"));
        assert!(
            out.status.success(),
            "{exe} обязан принять вывод:\n{}",
            String::from_utf8_lossy(&out.stderr)
        );
    }
}
