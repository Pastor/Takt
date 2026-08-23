//! Двунаправленный порт: эталон против прошивки цели `c` (фича 0421).
//!
//! # Что было
//!
//! Замер 2026-08-23 на `inout line: u8 at 0x200;`: эталон исполняет, `st`,
//! `st-at`, `rust` и `plantuml` переводят и их инструменты принимают, `sv` и
//! `sv-mmio` отказывают `SV-006` (двунаправленного порта у цели нет), а
//! **`c` и `c-hal` рапортовали об успехе с невалидным выводом**:
//!
//! ```text
//! error: redefinition of enumerator 'INOUT_PORT_PORT_LINE'
//! ```
//!
//! Двунаправленный порт попадает в ОБА перечисления (`_In_NumericPort` и
//! `_Out_NumericPort`), а перечислители в C делят одну область видимости —
//! имя обязано их различать. Код возврата `taktc` был **нулевым**.
//!
//! ⚠️ Корпус класс не покрывает: `inout` встречается только в
//! `examples/language/declarations.takt`, а гейты целей смотрят верхний
//! уровень (0403).
//!
//! ⚠️ Сверяются **значения**: перепутанный перечислитель (чтение из выходного
//! слота) даёт валидный C и другую трассу — компиляция этого не видит.

use std::path::{Path, PathBuf};
use std::process::Command;

const FIXTURE: &str = "tests/data/eval/conformance_inout_port.takt";
const UNIT: &str = "conformance_inout_port";
const TICKS: usize = 3;

fn cc_available() -> bool {
    Command::new("cc")
        .arg("--version")
        .output()
        .map(|o| o.status.success())
        .unwrap_or(false)
}

fn build_dir(tag: &str) -> PathBuf {
    let dir = std::env::temp_dir().join(format!(
        "takt_0421_{tag}_{}_{}",
        std::process::id(),
        std::thread::current()
            .name()
            .unwrap_or("t")
            .replace(':', "_")
    ));
    let _ = std::fs::remove_dir_all(&dir);
    std::fs::create_dir_all(&dir).expect("каталог");
    dir
}

/// Потактовые значения двунаправленного порта у эталона.
fn simulator_trace() -> Vec<i128> {
    let source = std::fs::read_to_string(FIXTURE).expect("фикстура читается");
    let (ast, _) = takt_lang::parse(&source, 0).expect("разбор фикстуры");
    let model = takt_lang::semantic::tree::construct_model(&ast, None, &[]).expect("семантика");
    let mut unit = takt_sim::build_unit(model).expect("построение Unit");
    let mut trace = Vec::new();
    for _ in 0..TICKS {
        let _ = unit.tick();
        match unit.variable("line") {
            Some(takt_sim::Value::Number(v)) => trace.push(v),
            other => panic!("порт 'line' обязан быть числом, получено {other:?}"),
        }
    }
    trace
}

/// Те же значения у прошивки: подставное железо держит один регистр — чтение и
/// запись идут по одному адресу, как на плате.
fn generated_c_trace(dir: &Path) -> Vec<i128> {
    let source = std::fs::read_to_string(FIXTURE).expect("фикстура читается");
    takt_lang::compile_to_c(
        UNIT,
        &source,
        dir.to_str().expect("путь в UTF-8"),
        &[],
        &takt_lang::generator::GenerateOptions::default(),
    )
    .expect("порождение C");

    let harness = format!(
        r#"#include <stdio.h>
#include "{UNIT}.h"

static long long reg = 0;

static int64_t on_read(ConformanceInoutPort_In_NumericPort port, void *ud) {{
    (void)ud;
    (void)port;
    return (int64_t)reg;
}}

static void on_write(ConformanceInoutPort_Out_NumericPort port, int64_t value, void *ud) {{
    (void)ud;
    (void)port;
    reg = (long long)value;
}}

int main(void) {{
    ConformanceInoutPort m;
    ConformanceInoutPort_init(&m);
    m.read_numeric = on_read;
    m.write_numeric = on_write;
    m.userdata = 0;
    for (int i = 0; i < {TICKS}; i++) {{
        ConformanceInoutPort_tick(&m);
        printf("%lld\n", reg);
    }}
    return 0;
}}
"#
    );
    std::fs::write(dir.join("harness.c"), &harness).expect("запись харнесса");

    let bin = dir.join("inout_bin");
    let compile = Command::new("cc")
        .args([
            "-std=c11",
            "-Wall",
            "-Wextra",
            "-Wno-unused-parameter",
            "-Werror",
            "-o",
        ])
        .arg(&bin)
        .arg(dir.join("harness.c"))
        .arg(dir.join(format!("{UNIT}.c")))
        .arg("-I")
        .arg(dir)
        .output()
        .expect("запуск cc");
    assert!(
        compile.status.success(),
        "cc не собрал харнесс флагами гейта цели:\n{}",
        String::from_utf8_lossy(&compile.stderr)
    );

    let run = Command::new(&bin).output().expect("запуск харнесса");
    assert!(run.status.success(), "харнесс завершился с ошибкой");
    String::from_utf8_lossy(&run.stdout)
        .lines()
        .map(|line| line.trim().parse::<i128>().expect("число в выводе"))
        .collect()
}

/// Предмет: вывод собирается, и значения совпадают с эталоном.
#[test]
fn inout_port_values_match_the_reference() {
    let sim = simulator_trace();
    assert_eq!(
        sim,
        vec![1, 3, 6],
        "эталон: значение накапливается через чтение того же порта"
    );

    if !cc_available() {
        eprintln!("[ПРОПУСК] inout_port_values_match_the_reference: cc не найден");
        return;
    }
    let dir = build_dir("trace");
    let c = generated_c_trace(&dir);
    assert_eq!(sim, c, "значения двунаправленного порта обязаны совпадать");
    let _ = std::fs::remove_dir_all(&dir);
}

/// Имена перечислителей двунаправленного порта РАЗЛИЧАЮТСЯ по стороне.
///
/// ⚠️ Сегмент печатается только двунаправленному порту: имена портов видны
/// пользователю (сигнатура HAL-колбэка), и смена формы у однонаправленных была
/// бы ломающей без нужды (урок 0195). На это стоит контрольная проверка.
#[test]
fn inout_enumerators_differ_by_side() {
    let dir = build_dir("names");
    let source = std::fs::read_to_string(FIXTURE).expect("фикстура читается");
    takt_lang::compile_to_c(
        UNIT,
        &source,
        dir.to_str().expect("путь в UTF-8"),
        &[],
        &takt_lang::generator::GenerateOptions::default(),
    )
    .expect("порождение C");
    let header = std::fs::read_to_string(dir.join(format!("{UNIT}.h"))).expect("чтение заголовка");
    assert!(
        header.contains("CONFORMANCE_INOUT_PORT_PORT_LINE_IN")
            && header.contains("CONFORMANCE_INOUT_PORT_PORT_LINE_OUT"),
        "двунаправленный порт обязан дать два разных перечислителя:\n{header}"
    );

    // Контроль: однонаправленный порт имя НЕ меняет.
    let plain = "out probe: u8 at 0x100;\nvar e: u8 := 0;\n\
                 start Run { always { e := e + 1; probe := e; } ref Run; }\n";
    let dir2 = build_dir("plain");
    takt_lang::compile_to_c(
        "plain_port",
        plain,
        dir2.to_str().expect("путь в UTF-8"),
        &[],
        &takt_lang::generator::GenerateOptions::default(),
    )
    .expect("порождение C");
    let header2 = std::fs::read_to_string(dir2.join("plain_port.h")).expect("чтение заголовка");
    assert!(
        header2.contains("PLAIN_PORT_PORT_PROBE =") && !header2.contains("PORT_PROBE_OUT"),
        "у однонаправленного порта имя обязано остаться прежним:\n{header2}"
    );
    let _ = std::fs::remove_dir_all(&dir);
    let _ = std::fs::remove_dir_all(&dir2);
}
