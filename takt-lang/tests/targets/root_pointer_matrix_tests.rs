//! СПЛОШНОЙ сторож признака «нужен ли указатель на корень» (фича 0449).
//!
//! # Зачем перебор, а не отдельные входы
//!
//! Признак `c_needs` (фичи 0396, 0419, 0439) отвечает на вопрос, печатать ли
//! функции под-модели параметр `main`. Проверялся он **входами отдельных
//! фич** — то есть выборкой: каждая фича приносила свой случай и своего
//! сторожа. Здесь вопрос задаётся сплошь: каждый вид обращения к корню × каждая
//! форма реализации состояния, обе функции модели (`_init` и `_tick`).
//!
//! Первый же прогон нашёл дефект, которого выборка не видела: в профиле «часы»
//! выдержка `after Nms` сравнивает метку с `main->now_ms(…)` **в такте**, а
//! признак знал об этом только в `_init` — `cc` отвечал «use of undeclared
//! identifier 'main'» при нулевом коде возврата `taktc`.
//!
//! # Что именно проверяется
//!
//! 1. **Вывод собирается** `cc` флагами гейта цели — ловит ложное «указатель не
//!    нужен» (громкий класс: отказ чужого инструмента).
//! 2. **Признак точен**: там, где обращений нет, параметра в сигнатуре **тоже**
//!    нет — ловит ложное «нужен» (тихий класс: заглушка `(void)main;` делает
//!    вывод валидным, и `cc` молчит).
//!
//! ⚠️ Ожидания таблицы сняты **прогоном** (правило 30), а не выведены из кода
//! признака: сторож, повторяющий реализацию, доказывает лишь сам себя.

use std::path::{Path, PathBuf};
use std::process::Command;

use super::matrix_probes::{Kind, Touch, case_name, cases, source};
use super::target_matrix_tests::refusal;

/// Ожидание: печатается ли `main` функциям `(_init, _tick)`.
///
/// ⚠️ Снято прогоном 2026-08-31, а не выведено из кода признака: сторож,
/// повторяющий реализацию, доказывает лишь сам себя.
fn expects(touch: Touch, _kind: Kind) -> (bool, bool) {
    match touch {
        Touch::None | Touch::ExternCall => (false, false),
        Touch::PortWrite | Touch::SharedRead | Touch::Transitive => (false, true),
        Touch::PortInit | Touch::VarInit => (true, false),
        Touch::ClockAfter => (true, true),
    }
}

/// Уникальный по тесту каталог (инвариант 0190/0429).
fn work_dir(tag: &str) -> PathBuf {
    let thread = std::thread::current()
        .name()
        .unwrap_or("main")
        .replace(':', "_");
    let dir = std::env::temp_dir()
        .join(format!("takt_pid{}", std::process::id()))
        .join(format!("takt_0449_{tag}_{thread}"));
    let _ = std::fs::remove_dir_all(&dir);
    std::fs::create_dir_all(&dir).expect("каталог теста");
    dir
}

fn cc_available() -> bool {
    Command::new("cc")
        .arg("--version")
        .output()
        .map(|o| o.status.success())
        .unwrap_or(false)
}

/// Компилирует случай целью `c`; отдаёт текст порождённого файла.
fn compile(dir: &Path, source: &str) -> String {
    let input = dir.join("probe.takt");
    std::fs::write(&input, source).expect("запись пробы");
    let out = Command::new(env!("CARGO_BIN_EXE_taktc"))
        .arg("compile")
        .args(["-t", "c"])
        .arg(&input)
        .arg("-o")
        .arg(dir.join("out"))
        .output()
        .expect("запуск taktc compile");
    assert!(
        out.status.success(),
        "цель `c` обязана перевести вход:\n{}\n--- исходник ---\n{source}",
        String::from_utf8_lossy(&out.stderr)
    );
    std::fs::read_to_string(dir.join("out").join("probe.c")).expect("порождённый файл читается")
}

/// Собирает порождённый файл флагами гейта цели.
fn build(dir: &Path) -> Result<(), String> {
    let cc = Command::new("cc")
        .args([
            "-std=c11",
            "-Wall",
            "-Wextra",
            "-Wno-unused-parameter",
            "-Werror",
            "-c",
        ])
        .arg(dir.join("out").join("probe.c"))
        .arg("-I")
        .arg(dir.join("out"))
        .arg("-o")
        .arg(dir.join("probe.o"))
        .output()
        .expect("запуск cc");
    if cc.status.success() {
        Ok(())
    } else {
        Err(String::from_utf8_lossy(&cc.stderr).into_owned())
    }
}

/// Есть ли у функции параметр-указатель на корень.
///
/// Ищется **прототип**: он печатается первым и в единственном экземпляре.
fn has_root_parameter(text: &str, function: &str) -> bool {
    let needle = format!("static void {function}(");
    let line = text
        .lines()
        .find(|l| l.contains(&needle))
        .unwrap_or_else(|| panic!("в выводе нет прототипа '{function}':\n{text}"));
    line.contains("Probe *main")
}

/// Сплошной перебор: форма реализации × вид обращения × форма объявления.
#[test]
fn root_pointer_is_exact_for_every_shape_and_touch() {
    if !cc_available() {
        eprintln!("cc недоступен — сплошной сторож пропущен");
        return;
    }
    let all = cases();
    let mut failures: Vec<String> = Vec::new();
    for (shape, touch, kind) in &all {
        let tag = case_name(*shape, *touch, *kind);
        // Законную границу цели судит таблица набора 0450 — здесь она означает
        // «сигнатуры проверять не на чем», а не «сторож молчит».
        if let Some(code) = refusal("c", *touch, *kind) {
            let dir = work_dir(&tag);
            let input = dir.join("probe.takt");
            std::fs::write(&input, source(*shape, *touch, *kind)).expect("запись пробы");
            let out = Command::new(env!("CARGO_BIN_EXE_taktc"))
                .arg("compile")
                .args(["-t", "c"])
                .arg(&input)
                .arg("-o")
                .arg(dir.join("out"))
                .output()
                .expect("запуск taktc compile");
            let stderr = String::from_utf8_lossy(&out.stderr);
            if out.status.success() || !stderr.contains(code) {
                failures.push(format!(
                    "{tag}: ожидался отказ {code}, получено: {}",
                    if out.status.success() {
                        "перевод".to_string()
                    } else {
                        stderr.trim().to_string()
                    }
                ));
            }
            continue;
        }
        let dir = work_dir(&tag);
        let text = compile(&dir, &source(*shape, *touch, *kind));
        if let Err(err) = build(&dir) {
            failures.push(format!("{tag}: cc отверг вывод:\n{err}"));
            continue;
        }
        let (init_expected, tick_expected) = expects(*touch, *kind);
        let init_actual = has_root_parameter(&text, "ProbeWrap_init");
        let tick_actual = has_root_parameter(&text, "ProbeWrap_tick");
        if init_actual != init_expected {
            failures.push(format!(
                "{tag}: `_init` {} указатель, ожидалось «{}»",
                if init_actual {
                    "получил"
                } else {
                    "не получил"
                },
                if init_expected {
                    "получит"
                } else {
                    "не получит"
                }
            ));
        }
        if tick_actual != tick_expected {
            failures.push(format!(
                "{tag}: `_tick` {} указатель, ожидалось «{}»",
                if tick_actual {
                    "получил"
                } else {
                    "не получил"
                },
                if tick_expected {
                    "получит"
                } else {
                    "не получит"
                }
            ));
        }
    }
    assert!(
        failures.is_empty(),
        "признак «нужен ли указатель на корень» разошёлся с ожиданием в {} случаях из {}:\n{}",
        failures.len(),
        all.len(),
        failures.join("\n")
    );
}
