//! Сдвиг на ПЕРЕМЕННУЮ величину: эталон против цели `c` (фича 0416).
//!
//! # Что было
//!
//! Правило насыщения (0326 у `rust`, 0392 у `c`) действовало только для
//! **литеральной** величины. Замер 2026-08-23 прогоном порождённой прошивки
//! (`u32`, значение `0xFFFFFFFF`, величина растёт 16 → 32 → 48):
//!
//! | Такт | эталон | прошивка `c` |
//! |---|---|---|
//! | `n = 16` | 65535 | 65535 |
//! | `n = 32` | **0** | **4294967295** |
//! | `n = 48` | **0** | **65535** (сдвиг по модулю 32) |
//!
//! Результат одинаков на `-O0` и `-O2`, `cc -Wall -Wextra -Werror` **молчит**
//! (величина при компиляции неизвестна), код возврата `taktc` — ноль. Цели
//! `rust` (`checked_shr`), `st` (`SHR`) и `sv` (`>>`) дают ответ эталона: цель
//! `c` расходилась со всеми.
//!
//! ⚠️ Это Tier 1 — инструменты противоречат друг другу **молча**. Ни гейт цели,
//! ни линт такого не видят: вердикт даёт только прогон значений.
//!
//! ⚠️ Контроль обязателен: у **узкого** типа (`u8`) продвижение до `int` уже
//! даёт ответ эталона, и вывод для него меняться не должен — иначе правка
//! раздула бы весь корпус.

use std::path::{Path, PathBuf};
use std::process::Command;

const FIXTURE: &str = "tests/data/eval/conformance_c_variable_shift.takt";
const UNIT: &str = "conformance_c_variable_shift";
/// Тактов в трассе: величина достигает ширины на втором и превышает на третьем.
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
        "takt_0416_{tag}_{}_{}",
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

/// Потактовые значения четырёх наблюдаемых у эталона.
fn simulator_trace() -> Vec<(i128, i128, i128, i128)> {
    let source = std::fs::read_to_string(FIXTURE).expect("фикстура читается");
    let (ast, _) = takt_lang::parse(&source, 0).expect("разбор фикстуры");
    let model = takt_lang::semantic::tree::construct_model(&ast, None, &[]).expect("семантика");
    let mut unit = takt_sim::build_unit(model).expect("построение Unit");
    let mut trace = Vec::new();
    for _ in 0..TICKS {
        let _ = unit.tick();
        let number = |name: &str| match unit.variable(name) {
            Some(takt_sim::Value::Number(v)) => v,
            other => panic!("переменная '{name}' обязана быть числом, получено {other:?}"),
        };
        trace.push((
            number("o_unsigned"),
            number("o_signed"),
            number("o_left"),
            number("o_narrow"),
        ));
    }
    trace
}

/// Те же значения у порождённой прошивки — прогоном харнесса.
fn generated_c_trace(dir: &Path) -> Vec<(i128, i128, i128, i128)> {
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

int main(void) {{
    ConformanceCVariableShift m;
    ConformanceCVariableShift_init(&m);
    for (int i = 0; i < {TICKS}; i++) {{
        ConformanceCVariableShift_tick(&m);
        printf("%lld %lld %lld %lld\n", (long long)m.o_unsigned, (long long)m.o_signed,
               (long long)m.o_left, (long long)m.o_narrow);
    }}
    return 0;
}}
"#
    );
    let harness_path = dir.join("harness.c");
    std::fs::write(&harness_path, &harness).expect("запись харнесса");

    let bin = dir.join("shift_bin");
    // Флаги гейта цели: класс тем и коварен, что они его НЕ ловят.
    let compile = Command::new("cc")
        .args([
            "-std=c11",
            "-Wall",
            "-Wextra",
            "-Wno-unused-parameter",
            "-Werror",
            "-O2",
            "-o",
        ])
        .arg(&bin)
        .arg(&harness_path)
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
        .map(|line| {
            let mut parts = line
                .split_whitespace()
                .map(|p| p.parse::<i128>().expect("число в выводе харнесса"));
            (
                parts.next().expect("беззнаковый"),
                parts.next().expect("знаковый"),
                parts.next().expect("левый"),
                parts.next().expect("контрольный"),
            )
        })
        .collect()
}

/// Эталон и прошивка дают одни значения на каждом такте.
#[test]
fn variable_shift_matches_the_reference() {
    let sim = simulator_trace();
    // Значения названы явно: «трассы совпали» ничего не стоит, если обе стороны
    // ошибаются одинаково (урок 0300).
    assert_eq!(
        sim,
        vec![(65535, -2, 4294901760, 0), (0, -1, 0, 0), (0, -1, 0, 0)],
        "эталон: за шириной беззнаковый даёт 0, знаковый — знак"
    );

    if !cc_available() {
        eprintln!("[ПРОПУСК] variable_shift_matches_the_reference: cc не найден");
        return;
    }
    let dir = build_dir("trace");
    let c = generated_c_trace(&dir);
    assert_eq!(
        sim, c,
        "потактовые значения эталона и прошивки обязаны совпадать"
    );
    let _ = std::fs::remove_dir_all(&dir);
}

/// **Контроль:** ЛИТЕРАЛЬНАЯ величина идёт прежним путём (правило 0392).
///
/// ⚠️ Ожидание пришлось снять замером: первая редакция теста требовала, чтобы
/// узкий тип (`u8`) печатался обычным сдвигом. Прогон показал обратное —
/// продвижение до `int` спасает лишь до 31 включительно, а `narrow << 31`
/// переполняет `int` (UB). Хелпер считает в `uint64_t`, то есть для узких типов
/// он не избыточен, а **необходим**; значения при этом совпадают с эталоном —
/// это видно по `o_narrow` в трассе.
#[test]
fn literal_amount_keeps_the_previous_path() {
    let dir = build_dir("literal");
    let source = "var w: u32 := 200;\n\
                  var far: u32 := 0;\n\
                  var near: u32 := 0;\n\
                  start Run {\n\
                      always {\n\
                          far := w >> 32;\n\
                          near := w >> 4;\n\
                      }\n\
                      ref Run;\n\
                  }\n";
    takt_lang::compile_to_c(
        "literal_shift",
        source,
        dir.to_str().expect("путь в UTF-8"),
        &[],
        &takt_lang::generator::GenerateOptions::default(),
    )
    .expect("порождение C");
    let text = std::fs::read_to_string(dir.join("literal_shift.c")).expect("чтение вывода");
    assert!(
        text.contains("->far = 0;"),
        "литерал за шириной насыщается на месте (0392), без хелпера:\n{text}"
    );
    assert!(
        text.contains(">> 4;"),
        "литерал в пределах ширины печатается обычным сдвигом:\n{text}"
    );
    assert!(
        !text.contains("takt_shr_u("),
        "хелпер при литеральной величине не нужен:\n{text}"
    );
    let _ = std::fs::remove_dir_all(&dir);
}
