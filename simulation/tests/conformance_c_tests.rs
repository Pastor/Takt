//! Сверка симулятора с кодом, порождённым `lamc -t c` (критерий A8 фичи 0025).
//!
//! # Зачем
//!
//! ADR 0025 постановил: для поведения, **определённого** в C, симулятор обязан
//! совпадать с синтезированным C. Иначе проверка модели до синтеза бессмысленна —
//! симулятор показывал бы одно, а прошивка делала другое.
//!
//! До этого теста сверка выполнялась **вручную** (прогоном `cc -std=c11`), то
//! есть держалась на дисциплине. Здесь она автоматизирована: тест порождает C
//! тем же `lamc`, компилирует его настоящим компилятором, запускает и сверяет
//! значения с трассой симулятора.
//!
//! # Область сверки сужена намеренно
//!
//! Только `Integer`/`Enum`/`Bool`. Для `Rational`, `Bit` и `Array` эталон
//! **непригоден**: генератор C на них дефектен (`Array(size, elem)` →
//! `uint{size}_t`, где `size` — число элементов; `Bit` → `int`; `Rational` →
//! `float` против f64 в симуляторе). Сверяться с дефектным эталоном — значит
//! узаконить его дефекты. Обоснование — в анализе фичи, раздел «Пригодность
//! эталона C».
//!
//! # Почему фикстура обёрнута в `model`
//!
//! Для **одиночной** корневой модели генератор не эмитит `typedef`, и
//! порождённый C **не компилируется** (`error: must use 'struct' tag`) — дефект
//! генератора, в бэклоге. В обёрнутом виде `typedef` эмитится.
//!
//! # Сверяются финальные значения, а не потактовая трасса
//!
//! Найдено этим тестом при первом же прогоне: порождённый C заводит
//! **синтетическое состояние `INIT`** на каждый уровень вложенности, и первые
//! такты уходят на переходы `INIT → …` без исполнения тела. Тело модели впервые
//! исполняется на **третьем** такте C (корень `INIT→ENTRY`, затем `Conf
//! INIT→IDLE`), тогда как симулятор исполняет его на **первом**.
//!
//! Значения при этом совпадают — расходится **моментность**. Поэтому сверяются
//! установившиеся значения (обе стороны прогоняются до завершения), а сам сдвиг
//! вынесен в бэклог как отдельная находка: потактовая сверка трасс невозможна,
//! пока симулятор не моделирует `INIT`-такты синтезированного кода.

use grammar::semantic::tree::construct_model;
use simulation::{TickResult, Unit, Value, build_unit};
use std::path::{Path, PathBuf};
use std::process::Command;

const FIXTURE: &str = "tests/data/eval/conformance_u8.lam";

/// Тактов на прогон каждой стороны до установившегося состояния.
///
/// С запасом: порождённому C нужно на два такта больше из-за синтетических
/// состояний `INIT` (см. заголовок модуля).
const MAX_TICKS: usize = 8;

/// Переменные, сверяемые с C, и их поля в порождённой структуре.
const CHECKED: &[&str] = &[
    "wrapped",   // S1: 255 + 1 → 0
    "truncated", // S9: 300 → 44
    "divided",   // 7 / 2 → 3
    "shifted",   // S4: 1 << 8 → 0 (продвижение до int, затем усечение)
    "mode",      // S7: вариант enum как целое
    "flag",      // bool → 1
];

fn cc_available() -> bool {
    Command::new("cc")
        .arg("--version")
        .output()
        .map(|o| o.status.success())
        .unwrap_or(false)
}

fn simulate() -> Unit {
    let source = std::fs::read_to_string(FIXTURE).expect("фикстура читается");
    let (ast, _) = grammar::parse(&source, 0).expect("разбор");
    let model = construct_model(&ast, None, &[]).expect("семантика");
    let mut unit = build_unit(model).expect("построение юнита");
    // Прогоняем до завершения — сверяются установившиеся значения (см. заголовок).
    for _ in 0..MAX_TICKS {
        let result = unit.tick();
        assert!(
            !matches!(result, TickResult::Failed(_)),
            "симуляция не должна падать: {result:?}"
        );
        if result == TickResult::Terminated {
            break;
        }
    }
    unit
}

fn sim_value(unit: &Unit, name: &str) -> i64 {
    match unit.variable(name) {
        Some(Value::Number(n)) => n,
        // bool в C печатается как 0/1 — приводим к тому же виду.
        Some(Value::Boolean(b)) => i64::from(b),
        other => panic!("переменная '{name}': неожиданное значение {other:?}"),
    }
}

/// Порождает C, собирает с харнессом и возвращает значения, напечатанные C.
fn run_generated_c(dir: &Path) -> Vec<(String, i64)> {
    // Публичный API — тот же путь, которым идёт `lamc compile -t c`.
    let source = std::fs::read_to_string(FIXTURE).expect("фикстура читается");
    grammar::compile_to_c(
        "conformance_u8",
        &source,
        dir.to_str().expect("путь в UTF-8"),
        &[],
        &grammar::generator::GenerateOptions::default(),
    )
    .expect("порождение C");

    // Харнесс: инициализирует модель, делает один такт и печатает переменные.
    let prints = CHECKED
        .iter()
        .map(|name| format!(r#"    printf("{name}=%d\n", (int)m.entry.{name});"#))
        .collect::<Vec<_>>()
        .join("\n");
    let harness = format!(
        r#"#include <stdio.h>
#include "conformance_u8.h"

int main(void) {{
    ConformanceU8 m;
    ConformanceU8_init(&m);
    /* Прогоняем до завершения: первые такты уходят на синтетические INIT. */
    for (int i = 0; i < {MAX_TICKS}; i++) {{
        ConformanceU8_tick(&m);
        if (ConformanceU8_is_done(&m)) break;
    }}
{prints}
    return 0;
}}
"#
    );
    let harness_path = dir.join("harness.c");
    std::fs::write(&harness_path, harness).expect("запись харнесса");

    let bin = dir.join("conformance_bin");
    let compile = Command::new("cc")
        .args(["-std=c11", "-I"])
        .arg(dir)
        .arg(dir.join("conformance_u8.c"))
        .arg(&harness_path)
        .arg("-o")
        .arg(&bin)
        .output()
        .expect("запуск cc");
    assert!(
        compile.status.success(),
        "порождённый C не компилируется:\n{}",
        String::from_utf8_lossy(&compile.stderr)
    );

    let run = Command::new(&bin).output().expect("запуск собранного C");
    assert!(run.status.success(), "собранный C завершился с ошибкой");
    String::from_utf8_lossy(&run.stdout)
        .lines()
        .filter_map(|line| {
            let (name, value) = line.split_once('=')?;
            Some((name.to_string(), value.trim().parse().ok()?))
        })
        .collect()
}

#[test]
fn a8_simulator_matches_generated_c() {
    if !cc_available() {
        // Явный отказ от проверки лучше молчаливого пропуска: без C-компилятора
        // сверить не с чем, и об этом должно быть видно в выводе.
        eprintln!(
            "[ПРОПУСК] a8_simulator_matches_generated_c: компилятор `cc` не найден — \
             сверка симулятора с порождённым C не выполнена"
        );
        return;
    }

    let dir: PathBuf = std::env::temp_dir().join("lam_conformance_0025_07");
    std::fs::create_dir_all(&dir).expect("каталог сборки");

    let unit = simulate();
    let from_c = run_generated_c(&dir);
    assert_eq!(
        from_c.len(),
        CHECKED.len(),
        "C напечатал не все переменные: {from_c:?}"
    );

    for (name, c_value) in &from_c {
        let sim = sim_value(&unit, name);
        assert_eq!(
            sim, *c_value,
            "расхождение по '{name}': симулятор={sim}, порождённый C={c_value}. \
             Симуляция обязана совпадать с синтезированным C (критерий A8)"
        );
    }
}

#[test]
fn a8_expected_values_are_pinned() {
    // Страховка от «сверки пустоты»: если обе стороны сломаются одинаково,
    // предыдущий тест этого не заметит. Значения зафиксированы вручную по
    // семантике C и подтверждены прогоном cc.
    let unit = simulate();
    assert_eq!(sim_value(&unit, "wrapped"), 0, "S1: u8 255 + 1");
    assert_eq!(sim_value(&unit, "truncated"), 44, "S9: u8 <- 300");
    assert_eq!(sim_value(&unit, "divided"), 3, "u8 7 / 2");
    assert_eq!(sim_value(&unit, "shifted"), 0, "S4: u8 1 << 8");
    assert_eq!(sim_value(&unit, "mode"), 1, "S7: enum Manual");
    assert_eq!(sim_value(&unit, "flag"), 1, "bool true");
}
