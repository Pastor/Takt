//! Потактовая сверка табличной формы автомата (фича 0435).
//!
//! # Что доказывает набор
//!
//! Флаг `--fsm=table` меняет **форму** порождённого C, а не поведение: эталон,
//! прошивка формы `switch` и прошивка формы `table` обязаны давать **одну**
//! трассу такт в такт.
//!
//! ⚠️ Гейт цели этого не видит по устройству: таблица, переставившая две
//! строки или потерявшая блок `exit`, собирается тем же `cc -Wall -Wextra
//! -Werror` без единого замечания — вывод валиден, автомат другой. Ровно
//! поэтому сверка заведена вместе с формой (правило «сверку заводить вместе с
//! бэкендом», уроки 0045 и 0050).
//!
//! # Фикстуры
//!
//! - `conformance_fsm_table.takt` — условное ребро, блоки `enter`/`exit`,
//!   **накапливающее** тело (на идемпотентном пропуск и двойное исполнение
//!   неразличимы), самопереход;
//! - `conformance_fsm_table_parallel.takt` — параллельная композиция: страж
//!   строки есть конъюнкция готовностей ветвей, и берётся она у того же
//!   носителя, что печатает их тик.

use std::path::{Path, PathBuf};
use std::process::Command;

use takt_lang::generator::{FsmForm, GenerateOptions};
use takt_lang::semantic::tree::construct_model;
use takt_sim::{TickResult, Value, build_unit};

const SIMPLE_FIXTURE: &str = "tests/data/eval/conformance_fsm_table.takt";
const SIMPLE_UNIT: &str = "conformance_fsm_table";
const SIMPLE_TICKS: usize = 8;

const PARALLEL_FIXTURE: &str = "tests/data/eval/conformance_fsm_table_parallel.takt";
const PARALLEL_UNIT: &str = "conformance_fsm_table_parallel";
const PARALLEL_TICKS: usize = 6;

fn tool(bin: &str) -> bool {
    Command::new(bin)
        .arg("--version")
        .output()
        .map(|o| o.status.success())
        .unwrap_or(false)
}

/// Каталог сборки, уникальный по потоку И процессу (инвариант 0190/0429).
fn build_dir(tag: &str) -> PathBuf {
    let thread = std::thread::current()
        .name()
        .unwrap_or("single")
        .replace(':', "_");
    let dir = std::env::temp_dir()
        .join(format!("takt_pid{}", std::process::id()))
        .join(format!("takt_0435_{tag}_{thread}"));
    let _ = std::fs::remove_dir_all(&dir);
    std::fs::create_dir_all(&dir).expect("каталог сборки");
    dir
}

fn source(fixture: &str) -> String {
    std::fs::read_to_string(fixture).expect("фикстура читается")
}

/// Трасса эталона: значение порта `probe` по тактам.
fn simulator_trace(fixture: &str, ticks: usize) -> Vec<i128> {
    let (ast, _) = takt_lang::parse(&source(fixture), 0).expect("разбор фикстуры");
    let model = construct_model(&ast, None, &[]).expect("семантика");
    let mut unit = build_unit(model).expect("построение Unit");
    let mut trace = Vec::new();
    let mut probe = 0i128;
    for _ in 0..ticks {
        let result = unit.tick();
        assert!(
            !matches!(result, TickResult::Failed(_)),
            "эталон не обрывает прогон: {result:?}"
        );
        if let Some(Value::Number(v)) = unit.variable("probe") {
            probe = v;
        }
        trace.push(probe);
    }
    trace
}

/// Трасса прошивки заданной формы: тот же порт, те же такты.
fn c_trace(dir: &Path, fixture: &str, unit: &str, ticks: usize, fsm: FsmForm) -> Vec<i128> {
    let mut options = GenerateOptions::default();
    options.fsm = fsm;
    takt_lang::compile_to_c(
        unit,
        &source(fixture),
        dir.to_str().expect("путь в UTF-8"),
        &[],
        &options,
    )
    .expect("порождение C");
    let camel = camel(unit);
    let harness = format!(
        r#"#include <stdio.h>
#include "{unit}.h"
static long long probe;
static void on_num({camel}_Out_NumericPort port, int64_t value, void *userdata) {{
    (void)userdata;
    if (port == {upper}_PORT_PROBE) {{ probe = (long long)value; }}
}}
int main(void) {{
    {camel} m;
    {camel}_init(&m);
    m.write_numeric = on_num;
    m.userdata = 0;
    for (int i = 0; i < {ticks}; i++) {{
        {camel}_tick(&m);
        printf("%lld\n", probe);
    }}
    return 0;
}}
"#,
        upper = probe_port_owner(unit),
    );
    std::fs::write(dir.join("harness.c"), harness).expect("харнесс");
    let bin = dir.join("bin");
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
        .arg(dir.join(format!("{unit}.c")))
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
    String::from_utf8_lossy(&run.stdout)
        .lines()
        .filter_map(|l| l.trim().parse::<i128>().ok())
        .collect()
}

/// Имя структуры корня: `conformance_fsm_table` → `ConformanceFsmTable`.
fn camel(unit: &str) -> String {
    unit.split('_')
        .map(|part| {
            let mut chars = part.chars();
            match chars.next() {
                Some(first) => first.to_uppercase().collect::<String>() + chars.as_str(),
                None => String::new(),
            }
        })
        .collect()
}

/// Владелец перечислителя порта `probe` в порождённом C.
///
/// Имя перечислителя квалифицировано моделью-владельцем (фича 0195), и у двух
/// фикстур владельцы разные: `Worker` и `Pair`.
fn probe_port_owner(unit: &str) -> String {
    let owner = if unit == PARALLEL_UNIT {
        "PAIR"
    } else {
        "WORKER"
    };
    format!("{}_{owner}", unit.to_uppercase())
}

/// Обе формы и эталон дают одну трассу: условное ребро, `enter`/`exit`,
/// накопление, самопереход.
#[test]
fn table_form_matches_switch_and_simulator() {
    if !tool("cc") {
        eprintln!("cc недоступен — сверка пропущена");
        return;
    }
    let expected = simulator_trace(SIMPLE_FIXTURE, SIMPLE_TICKS);
    let switch_dir = build_dir("simple_switch");
    let table_dir = build_dir("simple_table");
    let switch = c_trace(
        &switch_dir,
        SIMPLE_FIXTURE,
        SIMPLE_UNIT,
        SIMPLE_TICKS,
        FsmForm::Switch,
    );
    let table = c_trace(
        &table_dir,
        SIMPLE_FIXTURE,
        SIMPLE_UNIT,
        SIMPLE_TICKS,
        FsmForm::Table,
    );
    assert_eq!(switch, expected, "форма switch разошлась с эталоном");
    assert_eq!(table, expected, "форма table разошлась с эталоном");
    // Контроль осмысленности трассы: в ней есть и вход (`enter` даёт 100), и
    // выход (`exit` даёт 200). На постоянной трассе сверка ничего не значит.
    assert!(
        expected.contains(&100) && expected.contains(&200),
        "трасса не наблюдает блоки enter/exit: {expected:?}"
    );
}

/// Параллельная композиция: страж строки — конъюнкция готовностей ветвей.
#[test]
fn table_form_matches_switch_on_parallel_composition() {
    if !tool("cc") {
        eprintln!("cc недоступен — сверка пропущена");
        return;
    }
    let expected = simulator_trace(PARALLEL_FIXTURE, PARALLEL_TICKS);
    let switch_dir = build_dir("parallel_switch");
    let table_dir = build_dir("parallel_table");
    let switch = c_trace(
        &switch_dir,
        PARALLEL_FIXTURE,
        PARALLEL_UNIT,
        PARALLEL_TICKS,
        FsmForm::Switch,
    );
    let table = c_trace(
        &table_dir,
        PARALLEL_FIXTURE,
        PARALLEL_UNIT,
        PARALLEL_TICKS,
        FsmForm::Table,
    );
    assert_eq!(switch, expected, "форма switch разошлась с эталоном");
    assert_eq!(table, expected, "форма table разошлась с эталоном");
    // Контроль: трасса меняется по тактам — иначе перестановка строк таблицы
    // была бы незаметна.
    assert!(
        expected.first() != expected.last(),
        "трасса постоянна и сверкой ничего не доказывает: {expected:?}"
    );
}
