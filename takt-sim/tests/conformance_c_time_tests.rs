//! Потактовая сверка выдержки `after` цели `c` с эталоном (фича 0134, 0134-04).
//!
//! Отдельный файл, а не дополнение `conformance_c_tests.rs`: тот вместе с этой
//! сверкой давал 1072 строки при лимите 1000, и гейт размера отказал. Заодно
//! граница честная — время самостоятельная тема.
//!
//! Сверяется **номер такта** срабатывания, а не факт перехода: сдвиг на один
//! такт компилируется молча (уроки 0033 и главного капкана цели `sv`). Эталон и
//! цель заданы одной частотой (1 кГц) — сверка идёт **внутри** профиля времени
//! (правило 9 ADR 0134).

use std::path::Path;
use std::process::Command;

/// Доступен ли компилятор C (иначе сверка мягко пропускается — как у 0065-03).
fn cc_available() -> bool {
    Command::new("cc")
        .arg("--version")
        .output()
        .map(|o| o.status.success())
        .unwrap_or(false)
}

// Сверяется **номер такта** срабатывания выдержки, а не факт перехода: сдвиг на
// один такт — тот класс дефекта, который компилируется молча (уроки 0033 и
// главного капкана цели `sv`). Модельное время эталона и счётчик тактов цели
// заданы одной частотой (1 кГц), поэтому сравнение осмысленно: сверка идёт
// **внутри** профиля (правило 9 ADR 0134).

/// Фикстура: выход из состояния через 5 мс при 1 кГц (5 тактов от входа).
const AFTER_FIXTURE: &str = "tests/data/eval/conformance_after.takt";

/// На каком такте выходной порт `done` впервые стал единицей — у эталона.
fn simulator_dwell_tick() -> usize {
    let source = std::fs::read_to_string(AFTER_FIXTURE).expect("фикстура читается");
    let (ast, _) = takt_lang::parse(&source, 0).expect("разбор фикстуры");
    let model = takt_lang::semantic::tree::construct_model(&ast, None, &[]).expect("семантика");
    let mut unit = takt_sim::build_unit(model).expect("построение Unit");
    for step in 0..12i32 {
        // 1 мс на такт — та же частота, что объявлена моделью.
        unit.set_time_ns(i64::from(step) * 1_000_000);
        let _ = unit.tick();
        if unit.variable("done") == Some(takt_sim::Value::Number(1)) {
            return usize::try_from(step + 1).expect("номер такта");
        }
    }
    panic!("эталон обязан снять выдержку за 12 тактов");
}

/// На каком такте `done` впервые стал единицей — у порождённого C.
fn generated_c_dwell_tick(dir: &Path) -> usize {
    let source = std::fs::read_to_string(AFTER_FIXTURE).expect("фикстура читается");
    takt_lang::compile_to_c(
        "conformance_after",
        &source,
        dir.to_str().expect("путь в UTF-8"),
        &[],
        &takt_lang::generator::GenerateOptions::default(),
    )
    .expect("порождение C");

    let harness = r#"#include <stdio.h>
#include "conformance_after.h"

static int done = 0;
static void wr(ConformanceAfter_Out_BitPort port, bool v, void *ud) {
    (void)port;
    (void)ud;
    done = v;
}

int main(void) {
    /* Имя корневой структуры берётся из ИМЕНИ ФАЙЛА: корневая модель анонимна. */
    ConformanceAfter m = {0};
    m.write_bit = wr;
    ConformanceAfter_init(&m);
    for (int tick = 1; tick <= 12; tick++) {
        ConformanceAfter_tick(&m);
        if (done) { printf("tick=%d\n", tick); return 0; }
    }
    printf("tick=0\n");
    return 0;
}
"#;
    let harness_path = dir.join("harness_after.c");
    std::fs::write(&harness_path, harness).expect("запись харнесса");

    let bin = dir.join("conformance_after_bin");
    let compile = Command::new("cc")
        .args(["-std=c11", "-Wall", "-Werror", "-I"])
        .arg(dir)
        .arg(dir.join("conformance_after.c"))
        .arg(&harness_path)
        .arg("-o")
        .arg(&bin)
        .output()
        .expect("запуск cc");
    assert!(
        compile.status.success(),
        "порождённый C с выдержкой не компилируется:\n{}",
        String::from_utf8_lossy(&compile.stderr)
    );
    let run = Command::new(&bin).output().expect("запуск собранного C");
    assert!(run.status.success(), "собранный C завершился с ошибкой");
    String::from_utf8_lossy(&run.stdout)
        .lines()
        .find_map(|line| line.strip_prefix("tick=")?.trim().parse::<usize>().ok())
        .expect("харнесс печатает номер такта")
}

#[test]
fn after_fires_on_the_same_tick_in_simulator_and_generated_c() {
    if !cc_available() {
        eprintln!(
            "[ПРОПУСК] after_fires_on_the_same_tick_in_simulator_and_generated_c: \
             компилятор `cc` не найден"
        );
        return;
    }
    let dir = tempfile::tempdir().expect("временный каталог");
    let reference = simulator_dwell_tick();
    let generated = generated_c_dwell_tick(dir.path());
    assert_eq!(
        reference, generated,
        "выдержка обязана срабатывать на одном такте: эталон {reference}, цель C {generated}"
    );
    // Число из модели: 5 мс при 1 кГц — пять тактов от входа в стартовое
    // состояние, то есть такт 6 (вход занимает такт 1, время на нём — ноль).
    assert_eq!(reference, 6, "ожидался такт 6, получено {reference}");
}
