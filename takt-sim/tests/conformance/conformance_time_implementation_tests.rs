//! Потактовая сверка времени внутри модели, реализующей состояние.
//! В профиле «часы» первый такт начинается в 0 мс, шаг равен 1 мс.
//! После каждого такта наблюдаем два выходных порта. Порт, в который
//! не записали новое значение, сохраняет предыдущее.

use std::path::Path;
use std::process::Command;

use takt_lang::semantic::tree::construct_model;
use takt_sim::{TickResult, Value, build_unit};

const TICKS: usize = 12;

/// Фикстура, имя модуля C, выходные порты по порядку наблюдения: имя в эталоне и
/// перечислитель порта в порождённом C (без префикса модуля).
struct Case {
    fixture: &'static str,
    module: &'static str,
    ports: [(&'static str, &'static str); 2],
}

const AFTER: Case = Case {
    fixture: "tests/data/eval/time_in_implementation.takt",
    module: "impl_after",
    ports: [("h", "HEATER_PORT_H"), ("o", "HOST_PORT_O")],
};

const EVERY: Case = Case {
    fixture: "tests/data/eval/every_in_implementation.takt",
    module: "impl_every",
    ports: [("t", "TICKER_PORT_T"), ("o", "HOST_PORT_O")],
};

fn cc_available() -> bool {
    Command::new("cc")
        .arg("--version")
        .output()
        .map(|o| o.status.success())
        .unwrap_or(false)
}

/// Трасса эталона: значения портов после каждого такта. Порт неактивной
/// под-модели держит последнее значение - как выход порождённого C.
fn simulate(case: &Case) -> Vec<[i128; 2]> {
    let source = std::fs::read_to_string(case.fixture).expect("фикстура");
    let (ast, _) = takt_lang::parse(&source, 0).expect("разбор");
    let model = construct_model(&ast, None, &[]).expect("семантика");
    let mut unit = build_unit(model).expect("построение юнита");
    let mut held = [0i128; 2];
    let mut trace = Vec::new();
    for step in 0..TICKS {
        unit.set_time_ns(i64::try_from(step).expect("такт") * 1_000_000);
        let result = unit.tick();
        assert!(
            !matches!(result, TickResult::Failed(_)),
            "падение: {result:?}"
        );
        for (slot, (name, _)) in held.iter_mut().zip(case.ports) {
            if let Some(Value::Number(n)) = unit.variable(name) {
                *slot = n;
            }
        }
        trace.push(held);
    }
    trace
}

/// Трасса порождённого C при том же ходе времени.
fn generated_c(case: &Case, dir: &Path) -> Vec<[i128; 2]> {
    let source = std::fs::read_to_string(case.fixture).expect("фикстура");
    takt_lang::compile_to_c(
        case.module,
        &source,
        dir.to_str().expect("путь"),
        &[],
        &takt_lang::generator::GenerateOptions::default(),
    )
    .expect("порождение C");

    let camel: String = case
        .module
        .split('_')
        .map(|w| {
            let mut c = w.chars();
            c.next()
                .map(|f| f.to_ascii_uppercase().to_string() + c.as_str())
                .unwrap_or_default()
        })
        .collect();
    let upper = case.module.to_ascii_uppercase();
    let [(_, first), (_, second)] = case.ports;
    let module = case.module;
    let harness = format!(
        r#"#include <stdio.h>
#include "{module}.h"

static uint64_t fake_now = 0;
static uint64_t clk(void *ud) {{ (void)ud; return fake_now; }}
static int64_t first = 0, second = 0;
static void wr({camel}_Out_NumericPort port, uint8_t index, int64_t v, void *ud) {{
    (void)index; (void)ud;
    if (port == {upper}_{first}) first = v;
    if (port == {upper}_{second}) second = v;
}}

int main(void) {{
    {camel} m = {{0}};
    m.now_ms = clk;
    m.write_numeric = wr;
    fake_now = 0;
    {camel}_init(&m);
    for (int tick = 1; tick <= {TICKS}; tick++) {{
        fake_now = (uint64_t)(tick - 1);
        {camel}_tick(&m);
        printf("TICK %lld %lld\n", (long long)first, (long long)second);
    }}
    return 0;
}}
"#
    );
    let harness_path = dir.join("harness.c");
    std::fs::write(&harness_path, harness).expect("харнесс");
    let bin = dir.join("run");
    let compile = Command::new("cc")
        .args(["-std=c11", "-Wall", "-Werror", "-I"])
        .arg(dir)
        .arg(dir.join(format!("{module}.c")))
        .arg(&harness_path)
        .arg("-o")
        .arg(&bin)
        .output()
        .expect("cc");
    assert!(
        compile.status.success(),
        "порождённый C не компилируется:\n{}",
        String::from_utf8_lossy(&compile.stderr)
    );
    let run = Command::new(&bin).output().expect("запуск");
    assert!(run.status.success(), "собранный C упал");
    parse_trace(&run.stdout)
}

fn generated_rust(case: &Case, dir: &Path) -> Vec<[i128; 2]> {
    let source = std::fs::read_to_string(case.fixture).expect("фикстура");
    takt_lang::compile_to_rust(
        "impl_time",
        &source,
        dir.to_str().expect("путь"),
        &[],
        &takt_lang::generator::GenerateOptions::default(),
    )
    .expect("порождение Rust");

    let [first, second] = case.ports.map(|(name, _)| name.to_ascii_uppercase());
    let driver = format!(
        r#"#[path = "impl_time.rs"]
mod generated;
use generated::{{Hal, OutU8Port, ImplTime}};
use std::cell::Cell;

#[derive(Default)]
struct Probe {{
    now: Cell<u64>,
    first: Cell<u8>,
    second: Cell<u8>,
}}

impl Hal for &Probe {{
    fn write_u8(&mut self, port: OutU8Port, value: u8) {{
        match port {{
            OutU8Port::{first} => self.first.set(value),
            OutU8Port::{second} => self.second.set(value),
        }}
    }}

    fn now_ms(&mut self) -> u64 {{ self.now.get() }}
}}

fn main() {{
    let probe = Probe::default();
    let mut model = ImplTime::new(&probe);
    model.init();
    for step in 0..{TICKS} {{
        probe.now.set(step);
        model.tick();
        println!("TICK {{}} {{}}", probe.first.get(), probe.second.get());
    }}
}}
"#
    );
    std::fs::write(dir.join("driver.rs"), driver).expect("драйвер");
    let build = Command::new("rustc")
        .current_dir(dir)
        .args(["--edition", "2021", "driver.rs", "-o", "driver"])
        .output()
        .expect("запуск rustc");
    assert!(
        build.status.success(),
        "rustc не собрал драйвер:\n{}",
        String::from_utf8_lossy(&build.stderr)
    );
    let run = Command::new(dir.join("driver"))
        .output()
        .expect("запуск драйвера");
    assert!(run.status.success(), "драйвер Rust завершился с ошибкой");
    parse_trace(&run.stdout)
}

fn parse_trace(stdout: &[u8]) -> Vec<[i128; 2]> {
    String::from_utf8_lossy(stdout)
        .lines()
        .map(|line| {
            let mut values = line
                .strip_prefix("TICK ")
                .expect("строка трассы")
                .split_whitespace();
            let pair = std::array::from_fn(|_| {
                values
                    .next()
                    .expect("значение порта")
                    .parse()
                    .expect("целое число")
            });
            assert!(values.next().is_none(), "лишнее значение: {line}");
            pair
        })
        .collect()
}

fn check(case: &Case, expected: &[[i128; 2]]) {
    let sim = simulate(case);
    assert_eq!(sim, expected, "эталон, {}: {sim:?}", case.fixture);
    if !cc_available() {
        eprintln!("[ПРОПУСК] {}: `cc` не найден", case.fixture);
        return;
    }
    let dir = tempfile::tempdir().expect("временный каталог");
    let c = generated_c(case, dir.path());
    assert_eq!(
        sim, c,
        "трассы эталона и C обязаны совпадать, {}\nэталон={sim:?}\nC={c:?}",
        case.fixture
    );
}

/// `after 5ms` в реализации: `h = 2` на такте 7, `o = 3` на такте 8 - у обоих.
#[test]
fn after_in_state_implementation_matches_generated_c() {
    let mut expected = vec![[0, 1]];
    expected.extend([[1, 1]; 5]);
    expected.push([2, 1]);
    expected.extend([[2, 3]; TICKS - 7]);
    check(&AFTER, &expected);
}

/// `every 2ms` в реализации: `t` = 1, 2, 3, 4 на тактах 4, 6, 8, 10, `o = 3` на 11.
#[test]
fn every_in_state_implementation_matches_generated_c() {
    let t = [0, 0, 0, 1, 1, 2, 2, 3, 3, 4, 4, 4];
    let expected: Vec<[i128; 2]> = t
        .iter()
        .enumerate()
        .map(|(i, &t)| [t, if i >= 10 { 3 } else { 1 }])
        .collect();
    check(&EVERY, &expected);
}

fn check_rust(case: &Case) {
    if !Command::new("rustc")
        .arg("--version")
        .output()
        .is_ok_and(|o| o.status.success())
    {
        eprintln!("[ПРОПУСК] {}: `rustc` не найден", case.fixture);
        return;
    }
    let dir = tempfile::tempdir().expect("временный каталог");
    let expected = simulate(case);
    let actual = generated_rust(case, dir.path());
    assert_eq!(actual, expected, "трасса Rust, {}", case.fixture);
}

#[test]
fn after_in_state_implementation_matches_generated_rust() {
    check_rust(&AFTER);
}

#[test]
fn every_in_state_implementation_matches_generated_rust() {
    check_rust(&EVERY);
}
