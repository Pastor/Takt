//! Цель `c` ГОВОРИТ о выброшенном вызове встроенной функции — `CC-024`
//! (фича 0314).
//!
//! # Что было
//!
//! Замер 2026-08-20 на `debug("такт");` в теле состояния:
//!
//! | Потребитель | Ответ |
//! |---|---|
//! | эталон | печатает `debug: такт` каждый такт |
//! | **`c`, `c-hal`** | вызова в выводе **нет вовсе** — ни строки, ни комментария, ни предупреждения |
//! | `st`, `st-at` | `ST-011` |
//! | `rust` | `RS-011` |
//! | `sv`, `sv-mmio` | `SV-002` |
//!
//! Поведение цели `c` осмысленно (печать из прошивки не подразумевается), а вот
//! **молчание** — нет: три цели отвечали на один вход тремя разными способами.
//!
//! Канал предупреждений у цели существует с фичи 0168 и был пуст; `CC-024` —
//! первое, что по нему поехало.

use std::path::PathBuf;
use takt_lang::generator::GenerateOptions;

const SRC: &str = "var i: u8 := 0;\nout probe: u8 at 0;\n\
     start Run { always { i := i + 1; debug(\"такт\"); probe := i; } ref Run: i < 100; }\n";

/// Каталог сборки уникален по тесту (инварианты 0190 и 0244).
fn out_dir(tag: &str) -> PathBuf {
    let thread = std::thread::current()
        .name()
        .unwrap_or("single")
        .replace(':', "_");
    let dir = std::env::temp_dir()
        .join(format!("takt_pid{}", std::process::id()))
        .join(format!("takt_cc024_{tag}_{thread}"));
    let _ = std::fs::remove_dir_all(&dir);
    std::fs::create_dir_all(&dir).expect("каталог вывода");
    dir
}

fn compile(tag: &str, src: &str) -> (Vec<takt_lang::diagnostics::Diagnostic>, String) {
    let dir = out_dir(tag);
    let warnings = takt_lang::compile_to_c(
        tag,
        src,
        dir.to_str().expect("путь в UTF-8"),
        &[],
        &GenerateOptions::default(),
    )
    .expect("порождение C");
    let text = std::fs::read_to_string(dir.join(format!("{tag}.c"))).expect("чтение вывода");
    (warnings, text)
}

/// Предмет: вызов выброшен — и об этом сказано.
#[test]
fn dropped_debug_call_is_reported() {
    let (warnings, text) = compile("dbg", SRC);
    let found: Vec<_> = warnings
        .iter()
        .filter(|d| d.code.as_deref() == Some("CC-024"))
        .collect();
    assert_eq!(
        found.len(),
        1,
        "ожидалось одно предупреждение: {warnings:?}"
    );
    assert!(
        found[0].message.contains("debug"),
        "предупреждение обязано назвать функцию:\n{}",
        found[0].message
    );
    // Поведение НЕ изменилось: вызова в выводе по-прежнему нет.
    assert!(
        !text.contains("debug"),
        "печать из прошивки не подразумевается — вызов в вывод попасть не должен:\n{text}"
    );
}

/// Предупреждение на **каждый** выброшенный вызов, а не одно на модель.
///
/// ⚠️ Иначе второй выброшенный вызов терялся бы молча — тот самый класс,
/// ради которого фича и делалась (счёт сторожит его, как `ST-022` в 0235).
#[test]
fn every_dropped_call_is_reported() {
    let src = SRC.replace("debug(\"такт\");", "debug(\"раз\"); debug(\"два\");");
    let (warnings, _) = compile("dbg2", &src);
    let count = warnings
        .iter()
        .filter(|d| d.code.as_deref() == Some("CC-024"))
        .count();
    assert_eq!(count, 2, "ожидалось два предупреждения: {warnings:?}");
}

/// **Контроль:** модель без встроенных вызовов предупреждений не даёт.
///
/// Без него «предупреждение появилось» означало бы «появляется всегда».
#[test]
fn model_without_builtin_calls_is_silent() {
    let src = SRC.replace("debug(\"такт\"); ", "");
    let (warnings, _) = compile("dbg0", &src);
    assert!(
        warnings.is_empty(),
        "чистая модель обязана молчать: {warnings:?}"
    );
}
