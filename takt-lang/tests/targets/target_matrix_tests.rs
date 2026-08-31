//! Сплошной перебор форм по ВСЕМ целям и их инструментам (фича 0450).
//!
//! # Что доказывает набор
//!
//! Матрица «вид обращения к корню × форма реализации состояния» (носитель —
//! [`matrix_probes`](super::matrix_probes), 40 случаев) прогоняется через
//! **восемь** целей, и для каждой проверяется не код возврата `taktc`, а
//! **вердикт её инструмента**: `cc` для `c`/`c-hal`, `iec2c` + `cc` для
//! `st`/`st-at`, `clippy -D warnings` для `rust`, `verilator` и `yosys` для
//! `sv`/`sv-mmio`.
//!
//! ⚠️ Мера предосторожности не теоретическая: тем же перебором у цели `c`
//! нашёлся вход, дававший невалидный C при нулевом коде возврата (фича 0449).
//! Прочие цели проверялись входами отдельных фич — то есть выборкой.
//!
//! # Границы целей — часть таблицы, а не исключение из неё
//!
//! Там, где цель отказывает **законно**, ожидается именно её код отказа, и
//! молчаливая смена поведения (перевела то, что не умеет; отказала там, где
//! умела) роняет сторож так же, как невалидный вывод:
//!
//! | Цель | Вид | Код | Почему |
//! |---|---|---|---|
//! | `rust` | `var_init` | `RS-017` | функция порождается свободной и состояния модели не видит |
//! | `sv`, `sv-mmio` | `var_init` | `SV-002` | ветвь сброса выражений не вычисляет |
//! | `sv`, `sv-mmio` | `extern_call` | `SV-005` | внешней функции в синтезируемом RTL нет |

use std::path::{Path, PathBuf};
use std::process::Command;

use super::matrix_probes::{Kind, Touch, case_name, cases, source};

/// Ожидание для тройки «цель × вид обращения × форма объявления»: перевод либо
/// отказ **названным** кодом.
///
/// ⚠️ Таблица снята прогоном (правило 30). Границы целей — её часть: цель,
/// которая вдруг перевела то, что не умеет, роняет сторож так же, как
/// невалидный вывод.
pub(crate) fn refusal(target: &str, touch: Touch, kind: Kind) -> Option<&'static str> {
    match (target, touch, kind) {
        // Инициализатор от переменной корня: у `rust` функция порождается
        // свободной и состояния модели не видит, у `sv` ветвь сброса выражений
        // не вычисляет.
        ("rust", Touch::VarInit, _) => Some("RS-017"),
        ("sv" | "sv-mmio", Touch::VarInit, _) => Some("SV-002"),
        // Инициализатор массива значением другой переменной: в C массив не
        // присваивается.
        ("c" | "c-hal", Touch::VarInit, Kind::Array) => Some("CC-017"),
        // Внешней функции в синтезируемом RTL нет.
        ("sv" | "sv-mmio", Touch::ExternCall, _) => Some("SV-005"),
        // Порт перечислимого типа: HAL-трейт `rust` знает бит и число, а
        // размещение `st-at` — только скаляры IEC.
        ("rust", Touch::PortWrite | Touch::PortInit, Kind::Enum) => Some("RS-016"),
        ("st-at", Touch::PortWrite | Touch::PortInit, Kind::Enum) => Some("ST-004"),
        _ => None,
    }
}

fn tool(bin: &str) -> bool {
    Command::new(bin)
        .arg("--version")
        .output()
        .map(|o| o.status.success())
        .unwrap_or(false)
}

/// `(бинарник iec2c, каталог lib MatIEC)` — если оба на месте.
fn iec2c_available() -> Option<(PathBuf, PathBuf)> {
    let prefix = std::env::var("IEC2C_PREFIX")
        .map(PathBuf::from)
        .unwrap_or_else(|_| {
            PathBuf::from(std::env::var("HOME").unwrap_or_default()).join(".local")
        });
    let bin = prefix.join("bin").join("iec2c");
    let lib = prefix.join("share").join("matiec").join("lib");
    (bin.is_file() && lib.join("C").is_dir()).then_some((bin, lib))
}

/// Уникальный по тесту каталог (инвариант 0190/0429).
fn work_dir(tag: &str) -> PathBuf {
    let thread = std::thread::current()
        .name()
        .unwrap_or("main")
        .replace(':', "_");
    let dir = std::env::temp_dir()
        .join(format!("takt_pid{}", std::process::id()))
        .join(format!("takt_0450_{tag}_{thread}"));
    let _ = std::fs::remove_dir_all(&dir);
    std::fs::create_dir_all(&dir).expect("каталог теста");
    dir
}

/// Итог компиляции: путь к каталогу вывода либо код отказа цели.
enum Emitted {
    Ok(PathBuf),
    Refused(String),
}

/// Компилирует случай заданной целью.
fn emit(dir: &Path, target: &str, text: &str) -> Emitted {
    let input = dir.join("probe.takt");
    std::fs::write(&input, text).expect("запись пробы");
    let out_dir = dir.join("out");
    let out = Command::new(env!("CARGO_BIN_EXE_taktc"))
        .arg("compile")
        .args(["-t", target])
        .arg(&input)
        .arg("-o")
        .arg(&out_dir)
        .output()
        .expect("запуск taktc compile");
    if out.status.success() {
        return Emitted::Ok(out_dir);
    }
    let stderr = String::from_utf8_lossy(&out.stderr);
    let code = stderr
        .split_once('[')
        .and_then(|(_, rest)| rest.split_once(']'))
        .map(|(code, _)| code.to_string())
        .unwrap_or_else(|| stderr.trim().to_string());
    Emitted::Refused(code)
}

/// Прогоняет матрицу через одну цель; `check` судит порождённый каталог.
fn sweep(target: &str, check: &dyn Fn(&Path, &Path) -> Result<(), String>) -> Vec<String> {
    let mut failures = Vec::new();
    for (shape, touch, kind) in cases() {
        let name = case_name(shape, touch, kind);
        let tag = format!("{target}_{name}");
        let dir = work_dir(&tag);
        let text = source(shape, touch, kind);
        match (emit(&dir, target, &text), refusal(target, touch, kind)) {
            (Emitted::Ok(out), None) => {
                if let Err(err) = check(&dir, &out) {
                    failures.push(format!("{name}: {err}"));
                }
            }
            (Emitted::Ok(_), Some(code)) => failures.push(format!(
                "{name}: цель перевела вход, а ожидался отказ {code} — граница исчезла молча"
            )),
            (Emitted::Refused(code), Some(expected)) if code == expected => {}
            (Emitted::Refused(code), Some(expected)) => {
                failures.push(format!("{name}: отказ {code}, а ожидался {expected}"))
            }
            (Emitted::Refused(code), None) => {
                failures.push(format!("{name}: цель отказала кодом {code}"))
            }
        }
    }
    failures
}

/// Итог перебора: список расхождений обязан быть пуст.
fn verdict(target: &str, failures: Vec<String>) {
    assert!(
        failures.is_empty(),
        "цель `{target}`: {} случаев из {} разошлись с ожиданием:\n{}",
        failures.len(),
        cases().len(),
        failures.join("\n")
    );
}

/// Собирает порождённый C флагами гейта цели.
fn cc_builds(dir: &Path, out: &Path) -> Result<(), String> {
    let cc = Command::new("cc")
        .args([
            "-std=c11",
            "-Wall",
            "-Wextra",
            "-Wno-unused-parameter",
            "-Werror",
            "-c",
        ])
        .arg(out.join("probe.c"))
        .arg("-I")
        .arg(out)
        .arg("-o")
        .arg(dir.join("probe.o"))
        .output()
        .expect("запуск cc");
    if cc.status.success() {
        Ok(())
    } else {
        Err(format!(
            "cc отверг вывод:\n{}",
            String::from_utf8_lossy(&cc.stderr)
        ))
    }
}

#[test]
fn target_c_accepts_every_shape() {
    if !tool("cc") {
        eprintln!("cc недоступен — перебор пропущен");
        return;
    }
    verdict("c", sweep("c", &cc_builds));
}

#[test]
fn target_c_hal_accepts_every_shape() {
    if !tool("cc") {
        eprintln!("cc недоступен — перебор пропущен");
        return;
    }
    verdict("c-hal", sweep("c-hal", &cc_builds));
}

/// Цели `st`/`st-at`: вывод транслируется `iec2c`, результат собирается `cc`.
fn st_sweep(target: &str) {
    let Some((iec2c, lib)) = iec2c_available() else {
        eprintln!("iec2c недоступен — перебор пропущен");
        return;
    };
    let check = move |dir: &Path, out: &Path| -> Result<(), String> {
        let work = dir.join("iec");
        std::fs::create_dir_all(&work).expect("рабочий каталог iec2c");
        let run = Command::new(&iec2c)
            .arg("-I")
            .arg(&lib)
            .arg(out.join("probe.st"))
            .current_dir(&work)
            .output()
            .expect("запуск iec2c");
        if !run.status.success() || !work.join("POUS.c").is_file() {
            return Err(format!(
                "iec2c отверг вывод:\n{}",
                String::from_utf8_lossy(&run.stderr)
            ));
        }
        Ok(())
    };
    verdict(target, sweep(target, &check));
}

#[test]
fn target_st_accepts_every_shape() {
    st_sweep("st");
}

#[test]
fn target_st_at_accepts_every_shape() {
    st_sweep("st-at");
}

/// Цель `rust`: вердикт даёт `clippy` — флаги гейта цели, а не `rustc`.
#[test]
fn target_rust_accepts_every_shape() {
    if !tool("clippy-driver") {
        eprintln!("clippy-driver недоступен — перебор пропущен");
        return;
    }
    let check = |dir: &Path, out: &Path| -> Result<(), String> {
        // ⚠️ Рабочий каталог — каталог случая: без него `clippy-driver` кладёт
        // `libprobe.rlib` в текущий, то есть в дерево репозитория (гейт 0377
        // ловит такие артефакты, но лучше их не порождать).
        let run = Command::new("clippy-driver")
            .current_dir(dir)
            .args(["--edition", "2021", "--crate-type", "lib", "-D", "warnings"])
            .arg(out.join("probe.rs"))
            .output()
            .expect("запуск clippy-driver");
        if run.status.success() {
            Ok(())
        } else {
            Err(format!(
                "clippy отверг вывод:\n{}",
                String::from_utf8_lossy(&run.stderr)
            ))
        }
    };
    verdict("rust", sweep("rust", &check));
}

/// Цели `sv`/`sv-mmio`: **оба** инструмента — они видят разные половины
/// картины (урок 0045).
fn sv_sweep(target: &str) {
    if !tool("verilator") {
        eprintln!("verilator недоступен — перебор пропущен");
        return;
    }
    let with_yosys = tool("yosys");
    let check = move |_dir: &Path, out: &Path| -> Result<(), String> {
        let module = out.join("probe.sv");
        let lint = Command::new("verilator")
            .args(["--lint-only", "-Wall"])
            .arg(&module)
            .output()
            .expect("запуск verilator");
        if !lint.status.success() {
            return Err(format!(
                "verilator отверг модуль:\n{}",
                String::from_utf8_lossy(&lint.stderr)
            ));
        }
        if with_yosys {
            let script = format!("read_verilog -sv {}; synth -top probe", module.display());
            let synth = Command::new("yosys")
                .args(["-q", "-p", &script])
                .output()
                .expect("запуск yosys");
            if !synth.status.success() {
                return Err(format!(
                    "yosys не синтезировал модуль:\n{}",
                    String::from_utf8_lossy(&synth.stderr)
                ));
            }
        }
        Ok(())
    };
    verdict(target, sweep(target, &check));
}

#[test]
fn target_sv_accepts_every_shape() {
    sv_sweep("sv");
}

#[test]
fn target_sv_mmio_accepts_every_shape() {
    sv_sweep("sv-mmio");
}

/// Цель `plantuml`: инструмента нет, вердикт — непустая диаграмма.
#[test]
fn target_plantuml_accepts_every_shape() {
    let check = |_dir: &Path, out: &Path| -> Result<(), String> {
        let text = std::fs::read_to_string(out.join("probe.puml"))
            .map_err(|e| format!("диаграмма не читается: {e}"))?;
        if text.contains("@startuml") && text.contains("@enduml") {
            Ok(())
        } else {
            Err(format!("диаграмма пуста либо неполна:\n{text}"))
        }
    };
    verdict("plantuml", sweep("plantuml", &check));
}
