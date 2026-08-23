//! Сверка симулятора с кодом, порождённым `taktc -t c` (критерий A8 фичи 0025).
//!
//! # Зачем
//!
//! ADR 0025 постановил: для поведения, **определённого** в C, симулятор обязан
//! совпадать с синтезированным C. Иначе проверка модели до синтеза бессмысленна —
//! симулятор показывал бы одно, а прошивка делала другое.
//!
//! До этого теста сверка выполнялась **вручную** (прогоном `cc -std=c11`), то
//! есть держалась на дисциплине. Здесь она автоматизирована: тест порождает C
//! тем же `taktc`, компилирует его настоящим компилятором, запускает и сверяет
//! значения с трассой симулятора.
//!
//! # Область сверки сужена намеренно
//!
//! Изначально (фича 0025) — только `Integer`/`Enum`/`Bool`: для `Rational`,
//! `Bit` и `Array` эталон был **непригоден**, потому что генератор C на них сам
//! был дефектен (`Array(size, elem)` → `uint{size}_t`, где `size` — число
//! элементов; `Bit` → `int`; `Rational` → `float` против f64 в симуляторе).
//! Сверяться с дефектным эталоном — значит узаконить его дефекты.
//!
//! **Фича 0029 сузила это сужение.** Дефекты эталона исправлены, и сверка
//! расширена на **`Rational`** (`float` → `double` = f64 симулятора; тесты
//! `a9_*`). Скалярный `Bit` тоже перестал расходиться (`int` → `uint8_t`).
//!
//! **`Array` теперь сверяется** (фича 0076): симулятор исполняет запись в
//! элемент (`data[i] := v`) и список-инициализатор `{…}`, значения совпадают с
//! порождённым C — тест `array_element_matches_generated_c` (пришёл на смену
//! прежнему сторожу `a9_bit_and_array_conformance_gap`, фиксировавшему
//! препятствие `SIM-017`).
//!
//! **Вне сверки остаётся `[bit;N]`** — уже не из-за эталона: C видит скаляр,
//! симулятор — массив из N значений. Это вопрос **семантики языка** (фича 0078),
//! а не дефект генератора; скалярный инициализатор массива C сам отвергает
//! (CC-017), поэтому эталона у него нет.
//!
//! # Почему фикстура обёрнута в `model`
//!
//! Для **одиночной** корневой модели генератор не эмитит `typedef`, и
//! порождённый C **не компилируется** (`error: must use 'struct' tag`) — дефект
//! генератора, в бэклоге. В обёрнутом виде `typedef` эмитится.
//!
//! # Потактовая сверка трасс (фича 0033)
//!
//! Этот тест при первом прогоне (0025-07) обнаружил, что порождённый C заводил
//! **синтетическое состояние `INIT`** на каждый уровень вложенности и тратил на
//! него по такту: тело модели впервые исполнялось на третьем такте C, тогда как
//! симулятор — на первом. Значения совпадали, расходилась **моментность**,
//! поэтому сверка была вынуждена сравнивать только УСТАНОВИВШИЕСЯ значения.
//!
//! **Фича 0033 (Option B) сдвиг устранила** на стороне генератора C: вход в
//! стартовое состояние больше не расходует такт. Теперь сверяется значение каждой
//! переменной **на каждом такте** (`per_tick_trace_matches_generated_c`), а сдвиг
//! под структурной обёрткой равен нулю (`per_tick_shift_is_zero_under_wrapping`).
//! Тесты `a8_*`/`a9_*` по-прежнему сверяют установившиеся значения — как отдельная,
//! более грубая гарантия; сужение «только установившиеся» снято потактовыми
//! тестами.

use std::path::{Path, PathBuf};
use std::process::Command;
use takt_lang::semantic::tree::construct_model;
use takt_sim::{TickResult, Unit, Value, build_unit};

const FIXTURE: &str = "tests/data/eval/conformance_u8.takt";

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

/// Каталог сборки теста: внутри каталога СВОЕГО процесса (фича 0429).
///
/// Тесты идут параллельно, а помощник начинает с `remove_dir_all` (0190):
/// имя обязано быть уникально и по тесту, и по процессу — прогонов бывает два
/// (предкоммит и запущенный руками `cargo test`), и второй сносил каталог
/// первого прямо во время сборки.
fn work_dir(tag: &str) -> PathBuf {
    let dir = std::env::temp_dir()
        .join(format!("takt_pid{}", std::process::id()))
        .join(tag);
    std::fs::create_dir_all(&dir).expect("каталог сборки");
    dir
}

fn cc_available() -> bool {
    Command::new("cc")
        .arg("--version")
        .output()
        .map(|o| o.status.success())
        .unwrap_or(false)
}

fn simulate() -> Unit {
    simulate_fixture(FIXTURE)
}

fn simulate_fixture(fixture: &str) -> Unit {
    let source = std::fs::read_to_string(fixture).expect("фикстура читается");
    let (ast, _) = takt_lang::parse(&source, 0).expect("разбор");
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

fn sim_value(unit: &Unit, name: &str) -> i128 {
    match unit.variable(name) {
        Some(Value::Number(n)) => n,
        // bool в C печатается как 0/1 — приводим к тому же виду.
        Some(Value::Boolean(b)) => i128::from(b),
        // q(m, n) (0061): наблюдаемое — **представление** (сырые биты `intW`),
        // ровно то же читает C из поля структуры `(int)m.entry.<var>`. Сверка
        // идёт по repr, а не по вещественному приближению — побитово (A4).
        Some(Value::Fixed { repr, .. }) => i128::from(repr),
        other => panic!("переменная '{name}': неожиданное значение {other:?}"),
    }
}

/// Порождает C, собирает с харнессом и возвращает значения, напечатанные C.
fn run_generated_c(dir: &Path) -> Vec<(String, i128)> {
    // Публичный API — тот же путь, которым идёт `taktc compile -t c`.
    let source = std::fs::read_to_string(FIXTURE).expect("фикстура читается");
    takt_lang::compile_to_c(
        "conformance_u8",
        &source,
        dir.to_str().expect("путь в UTF-8"),
        &[],
        &takt_lang::generator::GenerateOptions::default(),
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

    let dir = work_dir("takt_conformance_0025_07");

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

// ─────────────────────────────────────────────────────────────────────────────
// Фикс 0005-01 (Tier 1, фича 0060): знак перечисления в цели `c`.
//
// Гейт `c` (cmake+ninja, без `-Werror`) этот дефект ПРИНИМАЛ: `cc` даёт лишь
// предупреждение `-Wtautological-constant-out-of-range-compare`, а гейт его не
// возводит в ошибку. То есть зелёный гейт про дефект НЕ говорит ничего — сверку
// заводим вместе с правкой (уроки 0045/0050).
// ─────────────────────────────────────────────────────────────────────────────

const NEG_ENUM_FIXTURE: &str = "tests/data/eval/conformance_neg_enum.takt";

/// Переменные, сверяемые с C на модели с отрицательным перечислением.
const NEG_ENUM_CHECKED: &[&str] = &[
    "lv",      // эталон −5; до фикса в C было 251 (uint8_t)
    "reached", // 1 ⇔ переход `lv == Low` сработал; до фикса — 0 (автомат стоял)
];

/// Порождает C для фикстуры с отрицательным перечислением, собирает и
/// возвращает напечатанное. Модель — `ConformanceNegEnum` (из имени файла).
fn run_generated_neg_enum_c(dir: &Path) -> Vec<(String, i128)> {
    let source = std::fs::read_to_string(NEG_ENUM_FIXTURE).expect("фикстура читается");
    takt_lang::compile_to_c(
        "conformance_neg_enum",
        &source,
        dir.to_str().expect("путь в UTF-8"),
        &[],
        &takt_lang::generator::GenerateOptions::default(),
    )
    .expect("порождение C");

    let prints = NEG_ENUM_CHECKED
        .iter()
        .map(|name| format!(r#"    printf("{name}=%d\n", (int)m.entry.{name});"#))
        .collect::<Vec<_>>()
        .join("\n");
    let harness = format!(
        r#"#include <stdio.h>
#include "conformance_neg_enum.h"

int main(void) {{
    ConformanceNegEnum m;
    ConformanceNegEnum_init(&m);
    for (int i = 0; i < {MAX_TICKS}; i++) {{
        ConformanceNegEnum_tick(&m);
        if (ConformanceNegEnum_is_done(&m)) break;
    }}
{prints}
    return 0;
}}
"#
    );
    let harness_path = dir.join("harness.c");
    std::fs::write(&harness_path, harness).expect("запись харнесса");

    let bin = dir.join("neg_enum_bin");
    let compile = Command::new("cc")
        // `-Werror=tautological-constant-out-of-range-compare`: A8/T9 — до фикса
        // `cc` предупреждал ровно об этом, здесь предупреждение возведено в
        // ошибку, поэтому возврат дефекта сорвёт саму СБОРКУ теста, а не только
        // сверку. Двойная страховка к потактовому сравнению ниже.
        .args([
            "-std=c11",
            "-Werror=tautological-constant-out-of-range-compare",
            "-I",
        ])
        .arg(dir)
        .arg(dir.join("conformance_neg_enum.c"))
        .arg(&harness_path)
        .arg("-o")
        .arg(&bin)
        .output()
        .expect("запуск cc");
    assert!(
        compile.status.success(),
        "порождённый C не компилируется (или сработал -Werror=tautological — \
         вернулся дефект 0005-01):\n{}",
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

/// **Сторож фикса [0005-01](../../docs/fixes/0005-01-c-enum-signedness.md), Tier 1.**
///
/// Отрицательное перечисление: симулятор и порождённый C обязаны совпасть.
/// Эталон — `lv = -5` (знак сохранён) и `reached = 1` (переход сработал).
///
/// ⚠️ **Мутация (T14):** вернуть расчёт цели `c` по `max` (беззнаковый) → C
/// даст `lv = 251` и `reached = 0`, сверка **упадёт** по обоим, а гейт `c`
/// (без `-Werror`) остался бы зелёным.
#[test]
fn neg_enum_signedness_matches_generated_c() {
    if !cc_available() {
        eprintln!("[ПРОПУСК] neg_enum_signedness_matches_generated_c: компилятор `cc` не найден");
        return;
    }

    let dir = work_dir("takt_conformance_0060");

    let unit = simulate_fixture(NEG_ENUM_FIXTURE);
    // Эталон: знак сохранён и переход сработал — иначе сверять не с чем.
    assert_eq!(
        sim_value(&unit, "lv"),
        -5,
        "симулятор обязан хранить знак: lv = -5"
    );
    assert_eq!(
        sim_value(&unit, "reached"),
        1,
        "в симуляторе переход `lv == Low` обязан сработать"
    );

    let from_c = run_generated_neg_enum_c(&dir);
    assert_eq!(
        from_c.len(),
        NEG_ENUM_CHECKED.len(),
        "C напечатал не всё: {from_c:?}"
    );
    for (name, c_value) in &from_c {
        let sim = sim_value(&unit, name);
        assert_eq!(
            sim, *c_value,
            "расхождение по '{name}': симулятор={sim}, C={c_value}. Знак перечисления \
             в цели C потерян — фикс 0005-01 (Tier 1)"
        );
    }
}

// ─────────────────────────────────────────────────────────────────────────────
// T17 (фича 0029): сверка по `Rational`
//
// Сужение критерия A8 фичи 0025 («не для Rational/Bit/Array») снято **частично**:
// по `Rational` сверка теперь возможна, потому что 0029-03 сменила отображение
// `float` → `double`, и обе стороны считают в IEEE 754 binary64.
//
// Про `Array` — см. `array_element_matches_generated_c` (сверяется, 0076);
// про `[bit;N]` — шапку модуля (вне сверки, вопрос семантики 0078).
// ─────────────────────────────────────────────────────────────────────────────

const FLOAT_FIXTURE: &str = "tests/data/eval/conformance_float.takt";

/// Вещественные переменные, сверяемые с C.
const CHECKED_FLOAT: &[&str] = &[
    "sum",   // 0.1 + 0.2 — в f32 и f64 РАЗНЫЕ (проверяет точность, а не арифметику)
    "third", // 1.0 / 3.0 — то же
    "exact", // 1.5 + 2.25 = 3.75 — точно в обеих разрядностях (контроль)
];

fn sim_real(unit: &Unit, name: &str) -> f64 {
    match unit.variable(name) {
        Some(Value::Real(x)) => x,
        other => panic!("переменная '{name}': ожидалось Real, получено {other:?}"),
    }
}

/// Порождает C для вещественной фикстуры, собирает и возвращает напечатанное.
///
/// Печать — `%.17g`: 17 значащих цифр восстанавливают binary64 однозначно.
/// Меньшая точность скрыла бы ровно то расхождение, ради которого тест написан.
fn run_generated_float_c(dir: &Path) -> Vec<(String, f64)> {
    let source = std::fs::read_to_string(FLOAT_FIXTURE).expect("фикстура читается");
    takt_lang::compile_to_c(
        "conformance_float",
        &source,
        dir.to_str().expect("путь в UTF-8"),
        &[],
        &takt_lang::GenerateOptions::default(),
    )
    .expect("порождение C");

    let prints = CHECKED_FLOAT
        .iter()
        .map(|name| format!(r#"    printf("{name}=%.17g\n", m.entry.{name});"#))
        .collect::<Vec<_>>()
        .join("\n");
    let harness = format!(
        r#"#include <stdio.h>
#include "conformance_float.h"

int main(void) {{
    ConformanceFloat m;
    ConformanceFloat_init(&m);
    /* Прогоняем до завершения: первые такты уходят на синтетические INIT. */
    for (int i = 0; i < {MAX_TICKS}; i++) {{
        ConformanceFloat_tick(&m);
        if (ConformanceFloat_is_done(&m)) break;
    }}
{prints}
    return 0;
}}
"#
    );
    let harness_path = dir.join("harness.c");
    std::fs::write(&harness_path, harness).expect("запись харнесса");

    let bin = dir.join("conformance_float_bin");
    let compile = Command::new("cc")
        .args(["-std=c11", "-I"])
        .arg(dir)
        .arg(dir.join("conformance_float.c"))
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

/// **T17 (0029).** Значения `Rational`: симулятор (f64) = порождённый C (`double`).
///
/// Сверка **побитовая** (`==` по f64), а не с допуском: обе стороны обязаны
/// исполнять одну и ту же арифметику IEEE 754 binary64. Допуск скрыл бы ровно
/// тот класс расхождения, ради которого тест написан, — разную разрядность.
#[test]
fn a9_simulator_matches_generated_c_rational() {
    if !cc_available() {
        eprintln!(
            "[ПРОПУСК] a9_simulator_matches_generated_c_rational: компилятор `cc` не найден — \
             сверка по Rational не выполнена"
        );
        return;
    }

    let dir = work_dir("takt_conformance_0029_float");

    let unit = simulate_fixture(FLOAT_FIXTURE);
    let from_c = run_generated_float_c(&dir);
    assert_eq!(
        from_c.len(),
        CHECKED_FLOAT.len(),
        "C напечатал не все переменные: {from_c:?}"
    );

    for (name, c_value) in &from_c {
        let sim = sim_real(&unit, name);
        assert_eq!(
            sim, *c_value,
            "расхождение по '{name}': симулятор={sim:.17}, порождённый C={c_value:.17}. \
             Симуляция обязана совпадать с синтезированным C (критерий A9)"
        );
    }
}

/// **T17 (0029).** Значения зафиксированы вручную — страховка от «сверки пустоты».
///
/// Значения **захвачены зондом** (прогон `cc` над порождённым C), а не угаданы.
/// `sum` и `third` — те самые, что отличают f64 от f32: на прежнем отображении
/// (`float` → `float`) C напечатал бы `0.30000001192092896` и
/// `0.33333334326744080`, и сверка стала бы красной. Именно поэтому фикстура
/// построена на них, а не на «круглых» числах.
#[test]
fn a9_rational_expected_values_are_pinned() {
    let unit = simulate_fixture(FLOAT_FIXTURE);
    assert_eq!(
        sim_real(&unit, "sum"),
        0.30000000000000004,
        "0.1 + 0.2 в binary64"
    );
    assert_eq!(
        sim_real(&unit, "third"),
        0.3333333333333333,
        "1.0 / 3.0 в binary64"
    );
    assert_eq!(sim_real(&unit, "exact"), 3.75, "1.5 + 2.25 — точно");
}

// Сверка `Array` с порождённым C вынесена в `conformance_c_arrays_tests.rs`
// (этот модуль упёрся в лимит размера — правило CLAUDE.md). Там —
// `array_element_matches_generated_c` (фича 0076).

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

// ─────────────────────────────────────────────────────────────────────────────
// Потактовая сверка трасс (фича 0033, R4/R5/A6/A7/A13)
//
// До 0033 сверка была вынуждена сравнивать только УСТАНОВИВШИЕСЯ значения:
// порождённый C тратил такт на каждый уровень `INIT`, и трассы были смещены.
// Option B (генератор C не тратит такт на `INIT`) сдвиг устранил, и теперь
// сверяется значение каждой переменной НА КАЖДОМ такте.
// ─────────────────────────────────────────────────────────────────────────────

const TICKS_FIXTURE: &str = "tests/data/eval/conformance_ticks.takt";

/// Максимум тактов для потактовой сверки — с запасом над длиной трассы.
const TRACE_TICKS: usize = 6;

/// Потактовая трасса симулятора: для каждого такта (до терминального включительно)
/// вектор значений `vars` в порядке `vars`.
fn simulate_trace(fixture: &str, vars: &[&str]) -> Vec<Vec<i128>> {
    let source = std::fs::read_to_string(fixture).expect("фикстура читается");
    let (ast, _) = takt_lang::parse(&source, 0).expect("разбор");
    let model = construct_model(&ast, None, &[]).expect("семантика");
    let mut unit = build_unit(model).expect("построение юнита");
    let mut trace = Vec::new();
    for _ in 0..TRACE_TICKS {
        let result = unit.tick();
        assert!(
            !matches!(result, TickResult::Failed(_)),
            "симуляция не должна падать: {result:?}"
        );
        trace.push(vars.iter().map(|v| sim_value(&unit, v)).collect());
        if result == TickResult::Terminated {
            break;
        }
    }
    trace
}

/// Потактовая трасса порождённого C: тем же `taktc`, собирается `cc`, печатает
/// значения `vars` после каждого такта. `basename` — имя файла без расширения
/// (задаёт имя корневой структуры и файлов), `accessor` — путь к вложенной
/// модели в структуре (например `entry`).
fn c_trace(
    dir: &Path,
    fixture: &str,
    basename: &str,
    root: &str,
    accessor: &str,
    vars: &[&str],
) -> Vec<Vec<i128>> {
    c_trace_opts(
        dir,
        fixture,
        basename,
        root,
        accessor,
        vars,
        &takt_lang::generator::GenerateOptions::default(),
    )
}

/// Как [`c_trace`], но с явными опциями генерации (фича 0096: Q-режим `float`
/// через `--float-as-q`/`--float-embedded`).
#[allow(clippy::too_many_arguments)]
fn c_trace_opts(
    dir: &Path,
    fixture: &str,
    basename: &str,
    root: &str,
    accessor: &str,
    vars: &[&str],
    opts: &takt_lang::generator::GenerateOptions,
) -> Vec<Vec<i128>> {
    let source = std::fs::read_to_string(fixture).expect("фикстура читается");
    takt_lang::compile_to_c(
        basename,
        &source,
        dir.to_str().expect("путь в UTF-8"),
        &[],
        opts,
    )
    .expect("порождение C");

    let prints = vars
        .iter()
        .map(|v| format!(r#"        printf("%d:{v}=%d\n", t, (int)m.{accessor}.{v});"#))
        .collect::<Vec<_>>()
        .join("\n");
    let harness = format!(
        r#"#include <stdio.h>
#include "{basename}.h"

int main(void) {{
    {root} m;
    {root}_init(&m);
    for (int t = 0; t < {TRACE_TICKS}; t++) {{
        {root}_tick(&m);
{prints}
        if ({root}_is_done(&m)) break;
    }}
    return 0;
}}
"#
    );
    let harness_path = dir.join("trace_harness.c");
    std::fs::write(&harness_path, harness).expect("запись харнесса");
    let bin = dir.join("trace_bin");
    let compile = Command::new("cc")
        .args(["-std=c11", "-I"])
        .arg(dir)
        .arg(dir.join(format!("{basename}.c")))
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

    // Строки вида "T:var=val" → трасса. Индекс такта `t` начинается с 0 в C.
    let out = String::from_utf8_lossy(&run.stdout).into_owned();
    let mut trace: Vec<Vec<i128>> = Vec::new();
    for line in out.lines() {
        let (t_str, rest) = line.split_once(':').expect("формат T:var=val");
        let t: usize = t_str.parse().expect("индекс такта");
        let (_, val) = rest.split_once('=').expect("формат var=val");
        let val: i128 = val.trim().parse().expect("значение");
        if trace.len() <= t {
            trace.resize(t + 1, Vec::new());
        }
        trace[t].push(val);
    }
    trace
}

/// A6/R4: потактовые трассы симулятора и порождённого C совпадают целиком, а не
/// только в установившемся состоянии. Значения также **пришпилены** (A7): если
/// обе стороны сломать одинаково, тест это заметит.
#[test]
fn per_tick_trace_matches_generated_c() {
    let vars = ["n"];
    let sim = simulate_trace(TICKS_FIXTURE, &vars);
    // Пиннинг: n принимает 1 → 2 → 3, модель завершается за 3 такта. Возврат
    // `break` в INIT-диспетчер сдвинул бы трассу и уронил бы это сравнение.
    assert_eq!(
        sim,
        vec![vec![1], vec![2], vec![3]],
        "ожидаемая потактовая трасса симулятора: n = 1, 2, 3"
    );

    if !cc_available() {
        eprintln!(
            "[ПРОПУСК] per_tick_trace_matches_generated_c: `cc` не найден — \
             потактовая сверка с C не выполнена (трасса симулятора пришпилена выше)"
        );
        return;
    }
    let dir = work_dir("takt_conformance_0033_trace");
    let c = c_trace(
        &dir,
        TICKS_FIXTURE,
        "conformance_ticks",
        "ConformanceTicks",
        "entry",
        &vars,
    );
    assert_eq!(
        sim, c,
        "потактовые трассы симулятора и порождённого C обязаны совпадать НА КАЖДОМ \
         такте (R4/A6), а не только в установившемся состоянии.\nсимулятор={sim:?}\nC={c:?}"
    );
}

// Переполнение беззнакового целого (фича 0127) — в подмодуле: файл упирался в
// лимит размера, а правило требует делить по логике, а не расширять реестр
// долга. `#[path]` — потому что корень тестового крейта ищет подмодули рядом с
// собой, а не в одноимённом каталоге (приём фичи 0088).
#[path = "conformance_c_tests/overflow.rs"]
mod overflow;

// Model-level `always` у модели-композиции (фича 0194) — тем же приёмом
// подмодуля и по той же причине: файл упирается в лимит размера.
#[path = "conformance_c_tests/composition_always.rs"]
mod composition_always;

// Реализация модели `model M = A | B { … }` (фича 0199) — тем же приёмом
// подмодуля: файл упирается в лимит размера.
#[path = "conformance_c_tests/model_implement.rs"]
mod model_implement;

// Перенос q к ширине формата (фикс 0061-01) — тем же приёмом подмодуля:
// файл упирается в лимит размера.
#[path = "conformance_c_tests/fixed_width.rs"]
mod fixed_width;

// Насыщение q(m, n) sat (фича 0170) — тем же приёмом подмодуля.
#[path = "conformance_c_tests/fixed_sat.rs"]
mod fixed_sat;

/// Фича 0083: model-level `always` исполняется КАЖДЫЙ такт и потактово совпадает
/// с C. Прежде блок молча терялся (и в C, и в симуляторе).
#[test]
fn model_level_always_matches_generated_c() {
    let vars = ["n"];
    let f = "tests/data/eval/conformance_model_always.takt";
    let sim = simulate_trace(f, &vars);
    // `always { n := n + 1; }`: 1 → 2 → 3 (переход n > 2) → 4 (такт в `Done`).
    assert_eq!(sim, vec![vec![1], vec![2], vec![3], vec![4]], "n = 1,2,3,4");
    if !cc_available() {
        eprintln!("[ПРОПУСК] model_level_always: `cc` не найден");
        return;
    }
    let dir = work_dir("takt_conf_0083_ma");
    let c = c_trace(&dir, f, "conf_ma", "ConfMa", "entry", &vars);
    assert_eq!(
        sim, c,
        "потактовая сверка model-level `always` (0083):\n{sim:?}\n{c:?}"
    );
}

/// R2/A4: чисто структурная обёртка (`model Mid { start M = Counter; }`) не
/// меняет потактовую трассу — сдвиг остаётся нулевым на любой глубине. Если бы
/// вход в стартовое состояние стоил такта, лишний уровень сдвинул бы трассу.
#[test]
fn per_tick_shift_is_zero_under_wrapping() {
    let vars = ["n"];
    // Симулятор эталон моментности не менял — обёрнутая трасса та же [1,2,3].
    let sim_wrapped = simulate_trace("tests/data/eval/conformance_ticks_wrapped.takt", &vars);
    assert_eq!(
        sim_wrapped,
        vec![vec![1], vec![2], vec![3]],
        "обёртка не должна менять трассу симулятора"
    );

    if !cc_available() {
        eprintln!(
            "[ПРОПУСК] per_tick_shift_is_zero_under_wrapping: `cc` не найден — \
             сверка обёрнутой трассы с C не выполнена"
        );
        return;
    }
    let dir = work_dir("takt_conformance_0033_wrapped");
    // Лишний уровень `Mid` даёт доступ `entry.m` вместо `entry`.
    let c_wrapped = c_trace(
        &dir,
        "tests/data/eval/conformance_ticks_wrapped.takt",
        "conformance_ticks_wrapped",
        "ConformanceTicksWrapped",
        "entry.m",
        &vars,
    );
    assert_eq!(
        c_wrapped,
        vec![vec![1], vec![2], vec![3]],
        "лишний уровень иерархии НЕ должен сдвигать потактовую трассу C (R2/A4).\nC={c_wrapped:?}"
    );
}

// ─────────────────────────────────────────────────────────────────────────────
// Q-арифметика fixed-point (фича 0061, задача 0061-03): T10/T11/T19
//
// Гейт `cc` доказывает лишь, что вывод компилируется; неверная Q-арифметика
// компилируется тоже. Основной критерий — **побитовая** потактовая сверка repr.
// ─────────────────────────────────────────────────────────────────────────────

const FIXED_FIXTURE: &str = "tests/data/eval/conformance_fixed.takt";

/// T10/A4 (цель C): побитовая потактовая сверка Q-арифметики с симулятором —
/// **включая отрицательные** значения и floor к −∞ у `*` (T9: на S2 усечение к
/// нулю дало бы −1 вместо −2).
#[test]
fn fixed_point_arithmetic_matches_generated_c() {
    let vars = ["acc"];
    let sim = simulate_trace(FIXED_FIXTURE, &vars);
    // Пиннинг представлений q(8,8): -3.0, -1.5, -2·2⁻⁸ (floor!), +2.0-ish.
    assert_eq!(
        sim,
        vec![vec![-768], vec![-384], vec![-2], vec![510]],
        "трасса представлений q(8,8) — эталон Q-арифметики симулятора"
    );

    if !cc_available() {
        eprintln!(
            "[ПРОПУСК] fixed_point_arithmetic_matches_generated_c: `cc` не найден \
             (трасса симулятора пришпилена выше)"
        );
        return;
    }
    let dir = work_dir("takt_conformance_fixed");
    let c = c_trace(
        &dir,
        FIXED_FIXTURE,
        "conformance_fixed",
        "ConformanceFixed",
        "entry",
        &vars,
    );
    assert_eq!(
        sim, c,
        "Q-арифметика симулятора и порождённого C обязана совпасть ПОБИТОВО на \
         каждом такте (repr q(8,8)).\nсимулятор={sim:?}\nC={c:?}"
    );
}

/// T19/A4 (цель C): переполнение `+` над q — wraparound (перенос), не насыщение.
#[test]
fn fixed_point_addition_wraps_matches_generated_c() {
    let vars = ["big"];
    let fixture = "tests/data/eval/conformance_fixed_wrap.takt";
    let sim = simulate_trace(fixture, &vars);
    assert_eq!(
        sim,
        vec![vec![-32768]],
        "q(8,8): 32767 + 1 → −32768 (перенос, правило 3 ADR)"
    );

    if !cc_available() {
        eprintln!("[ПРОПУСК] fixed_point_addition_wraps_matches_generated_c: `cc` не найден");
        return;
    }
    let dir = work_dir("takt_conformance_fixed_wrap");
    let c = c_trace(
        &dir,
        fixture,
        "conformance_fixed_wrap",
        "ConformanceFixedWrap",
        "entry",
        &vars,
    );
    assert_eq!(
        sim, c,
        "wraparound `+` q обязан совпасть с C.\nсим={sim:?}\nC={c:?}"
    );
}

/// T11/A5 (ловушка C11 6.5.7p5): порождённый C Q-модели НЕ содержит `>>` — тем
/// более над знаковым отрицательным. Floor идёт floor-делением (`/`/`%`
/// стандартно-определены). Греп по исходнику **и** сверка значений выше — оба
/// обязательны (T12): на **нашем** компиляторе (clang) неверный `>>` дал бы тот
/// же результат, и одна сверка дефект не поймала бы.
#[test]
fn generated_c_fixed_has_no_right_shift() {
    let dir = work_dir("takt_conformance_fixed_shift");
    let source = std::fs::read_to_string(FIXED_FIXTURE).expect("фикстура читается");
    takt_lang::compile_to_c(
        "conformance_fixed",
        &source,
        dir.to_str().expect("путь в UTF-8"),
        &[],
        &takt_lang::generator::GenerateOptions::default(),
    )
    .expect("порождение C");
    let c = std::fs::read_to_string(dir.join("conformance_fixed.c")).expect("читается .c");
    assert!(
        !c.contains(">>"),
        "Q-арифметика C обязана обходиться без `>>` (C11 6.5.7p5: `>>` знакового \
         отрицательного — implementation-defined). Floor — через floor-деление.\n{c}"
    );
}

// ─────────────────────────────────────────────────────────────────────────────
// Прозрачный float → q(m, n), embedded-путь (фича 0096, задача 0096-03): T6/T8/T9
//
// Под `--float-as-q=8.8 --float-embedded` цель `c` реализует `float` целочисленным
// q(8,8) (embedded без FPU). Трасса `acc` обязана совпасть с q-версией
// (`conformance_fixed`) ПОБИТОВО. Без `--float-embedded` — прежний native `double`.
// ─────────────────────────────────────────────────────────────────────────────

const FLOAT_Q_FIXTURE: &str = "tests/data/eval/conformance_float_q.takt";

/// Опции embedded-Q для `float` (фича 0096): точность + Q-путь для c/rust/st.
#[allow(clippy::field_reassign_with_default)] // GenerateOptions — #[non_exhaustive]
fn float_embedded_opts(m: u8, n: u8) -> takt_lang::generator::GenerateOptions {
    let mut o = takt_lang::generator::GenerateOptions::default();
    o.float_as_q = Some((m, n));
    o.float_embedded = true;
    o
}

/// Q-режим эталона: понижает `float → q(m, n)` в модели симулятора тем же
/// проходом, что и цель (ADR 0096, драйвер 2 — сверка ВНУТРИ режима).
fn simulate_trace_float_q(fixture: &str, m: u8, n: u8, vars: &[&str]) -> Vec<Vec<i128>> {
    let source = std::fs::read_to_string(fixture).expect("фикстура читается");
    let (ast, _) = takt_lang::parse(&source, 0).expect("разбор");
    let model = construct_model(&ast, None, &[]).expect("семантика");
    takt_lang::semantic::lower_float::lower_float_to_fixed(model.clone(), m, n)
        .expect("float → q(m, n)");
    let mut unit = build_unit(model).expect("построение юнита");
    let mut trace = Vec::new();
    for _ in 0..TRACE_TICKS {
        let result = unit.tick();
        assert!(
            !matches!(result, TickResult::Failed(_)),
            "симуляция не должна падать: {result:?}"
        );
        trace.push(vars.iter().map(|v| sim_value(&unit, v)).collect());
        if result == TickResult::Terminated {
            break;
        }
    }
    trace
}

/// T6/A4 (цель C, embedded-путь): под `--float-embedded` `float` → q(8,8),
/// **побитовая** потактовая сверка с Q-эталоном симулятора. Трасса — та же, что у
/// явной q-версии `conformance_fixed` (float→q(8,8) ≡ q(8,8)).
#[test]
fn float_embedded_q_matches_generated_c() {
    let vars = ["acc"];
    let sim = simulate_trace_float_q(FLOAT_Q_FIXTURE, 8, 8, &vars);
    assert_eq!(
        sim,
        vec![vec![-768], vec![-384], vec![-2], vec![510]],
        "Q-эталон float→q(8,8) обязан совпасть с трассой явной q-версии (0061)"
    );

    if !cc_available() {
        eprintln!("[ПРОПУСК] float_embedded_q_matches_generated_c: `cc` не найден");
        return;
    }
    let dir = work_dir("takt_conformance_float_q");
    let c = c_trace_opts(
        &dir,
        FLOAT_Q_FIXTURE,
        "conformance_float_q",
        "ConformanceFloatQ",
        "entry",
        &vars,
        &float_embedded_opts(8, 8),
    );
    assert_eq!(
        sim, c,
        "float→q(8,8) в симуляторе и порождённом C обязаны совпасть ПОБИТОВО.\n\
         сим={sim:?}\nC={c:?}"
    );
}

// Сторож двухрежимного эталона (native vs Q) и native-гейты c/rust/st вынесены
// в conformance_float_modes_tests.rs (лимит размера этого файла).
