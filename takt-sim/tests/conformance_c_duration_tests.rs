//! Потактовая сверка **значений** типа `duration` цели `c` с эталоном (фича 0183).
//!
//! Отдельный файл, а не дополнение `conformance_c_time_tests.rs`: тот уже 421
//! строка и посвящён **выдержке** (номеру такта срабатывания), а здесь предмет
//! другой — само значение длительности. Граница честная: представление эталона —
//! наносекунды, представление целей — **миллисекунды** (ADR 0183), и ошибка на
//! этой границе была бы молчаливой (`250us` → 0 мс).
//!
//! Сверяются **значения**, а не факт компиляции: гейт `cc` доказывает, что код
//! собирается, но не что он считает то же (уроки 0045 в `sv` и 0050 в `rust`).

use std::path::Path;
use std::process::Command;

/// Фикстура: `elapsed := pause + 750ms`, `ms := elapsed as u32`, `late := elapsed > 500ms`.
const FIXTURE: &str = "tests/data/eval/conformance_duration_value.takt";

/// Доступен ли компилятор C (иначе сверка мягко пропускается).
fn cc_available() -> bool {
    Command::new("cc")
        .arg("--version")
        .output()
        .map(|o| o.status.success())
        .unwrap_or(false)
}

/// Значения `(ms, late)` у эталона после первого такта.
fn simulator_values() -> (i64, i64) {
    let source = std::fs::read_to_string(FIXTURE).expect("фикстура читается");
    let (ast, _) = takt_lang::parse(&source, 0).expect("разбор фикстуры");
    let model = takt_lang::semantic::tree::construct_model(&ast, None, &[]).expect("семантика");
    let mut unit = takt_sim::build_unit(model).expect("построение Unit");
    let _ = unit.tick();
    let number = |name: &str| match unit.variable(name) {
        Some(takt_sim::Value::Number(v)) => v,
        other => panic!("порт '{name}' обязан быть числом, получено {other:?}"),
    };
    (number("ms"), number("late"))
}

/// Значения `(ms, late)` у порождённого C после первого такта.
fn generated_c_values(dir: &Path) -> (i64, i64) {
    let source = std::fs::read_to_string(FIXTURE).expect("фикстура читается");
    takt_lang::compile_to_c(
        "conformance_duration_value",
        &source,
        dir.to_str().expect("путь в UTF-8"),
        &[],
        &takt_lang::generator::GenerateOptions::default(),
    )
    .expect("порождение C");

    // Харнесс печатает то, что модель записала в порты: значение длительности
    // видно снаружи только через них.
    let harness = r#"#include <stdio.h>
#include "conformance_duration_value.h"

static unsigned long ms_value = 0;
static int late_value = 0;

static void wr_num(ConformanceDurationValue_Out_NumericPort port, int64_t v, void *ud) {
    (void)port;
    (void)ud;
    ms_value = (unsigned long)v;
}

static void wr_bit(ConformanceDurationValue_Out_BitPort port, bool v, void *ud) {
    (void)port;
    (void)ud;
    late_value = v;
}

int main(void) {
    ConformanceDurationValue m = {0};
    m.write_numeric = wr_num;
    m.write_bit = wr_bit;
    ConformanceDurationValue_init(&m);
    ConformanceDurationValue_tick(&m);
    printf("ms=%lu late=%d\n", ms_value, late_value);
    return 0;
}
"#;
    let harness_path = dir.join("harness_duration.c");
    std::fs::write(&harness_path, harness).expect("запись харнесса");

    let bin = dir.join("conformance_duration_bin");
    let compile = Command::new("cc")
        .args(["-std=c11", "-Wall", "-Werror", "-I"])
        .arg(dir)
        .arg(dir.join("conformance_duration_value.c"))
        .arg(&harness_path)
        .arg("-o")
        .arg(&bin)
        .output()
        .expect("запуск cc");
    assert!(
        compile.status.success(),
        "порождённый C со значениями duration не компилируется:\n{}",
        String::from_utf8_lossy(&compile.stderr)
    );
    let run = Command::new(&bin).output().expect("запуск собранного C");
    assert!(run.status.success(), "собранный C завершился с ошибкой");
    let out = String::from_utf8_lossy(&run.stdout);
    let mut ms = None;
    let mut late = None;
    for token in out.split_whitespace() {
        if let Some(v) = token.strip_prefix("ms=") {
            ms = v.parse::<i64>().ok();
        }
        if let Some(v) = token.strip_prefix("late=") {
            late = v.parse::<i64>().ok();
        }
    }
    (
        ms.expect("харнесс печатает ms"),
        late.expect("харнесс печатает late"),
    )
}

/// Арифметика, сравнение и приведение длительности дают у эталона и у цели `c`
/// **одни и те же** значения.
#[test]
fn duration_values_match_simulator_and_generated_c() {
    if !cc_available() {
        eprintln!("[ПРОПУСК] duration_values_match_simulator_and_generated_c: `cc` не найден");
        return;
    }
    let dir = tempfile::tempdir().expect("временный каталог");
    let reference = simulator_values();
    let generated = generated_c_values(dir.path());
    assert_eq!(
        reference, generated,
        "значения обязаны совпадать: эталон {reference:?}, цель C {generated:?}"
    );
    // Числа из модели: 1s + 750ms = 1750 мс, и это больше 500 мс. Проверяются
    // явно — иначе сверка «ноль против нуля» была бы зелёной и бессмысленной.
    assert_eq!(
        reference,
        (1_750, 1),
        "ожидались ms=1750 и late=1, получено {reference:?}"
    );
}

/// Приведение `as` **не порождает арифметики**: единица представления та же
/// (миллисекунды), поэтому в выводе стоит только приведение типа.
///
/// Проверяется текстом, а не поведением: деление на 1000000, вставленное «на
/// всякий случай», прошло бы сверку значений при `elapsed = 0` и провалилось бы на
/// живой модели.
#[test]
fn cast_between_duration_and_number_emits_no_arithmetic() {
    let dir = tempfile::tempdir().expect("временный каталог");
    let source = std::fs::read_to_string(FIXTURE).expect("фикстура читается");
    takt_lang::compile_to_c(
        "conformance_duration_value",
        &source,
        dir.path().to_str().expect("путь в UTF-8"),
        &[],
        &takt_lang::generator::GenerateOptions::default(),
    )
    .expect("порождение C");
    let code = std::fs::read_to_string(dir.path().join("conformance_duration_value.c"))
        .expect("порождённый .c");
    assert!(
        code.contains("(uint32_t)model->elapsed"),
        "приведение обязано быть простым кастом:\n{code}"
    );
    for forbidden in ["1000000", "/ 1000", "* 1000"] {
        assert!(
            !code.contains(forbidden),
            "в выводе не должно быть пересчёта единиц ('{forbidden}'):\n{code}"
        );
    }
}
