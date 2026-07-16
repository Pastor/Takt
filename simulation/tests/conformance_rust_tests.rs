//! Сверка симулятора с прошивкой, порождённой `lamc -t rust` (фича 0050).
//!
//! # Зачем, если гейт `rustc`/`clippy` уже зелёный
//!
//! **Гейт доказывает, что вывод компилируется, но не что он ведёт себя как
//! модель.** Молча неверная трансляция (перепутанный приоритет, потерянный
//! переход, не тот операнд) компилируется тоже — и `-D warnings` её пропустит.
//!
//! Урок не теоретический. Фича 0045 (цель `sv`) поймала ровно такой дефект:
//! чтение внутри такта давало значение предыдущего такта, и **оба** её гейта
//! (verilator и yosys) модуль принимали — он был валиден и синтезируем, просто
//! считал не то. Вывод, записанный в отчёте 0045 и в `CLAUDE.md`: потактовую
//! сверку с симулятором нужно заводить **вместе** с бэкендом. Здесь она
//! заводится — и делает цель `rust` третьей после `c` и `sv`, чьё поведение
//! сверено с эталоном, а не только синтаксис.
//!
//! # Что здесь наблюдается и почему через порты
//!
//! Через **выходные порты**, то есть через HAL. Поля модели в порождённом Rust
//! **приватны** (`n: u8`, не `pub n: u8`), и это правильно: инкапсуляция — часть
//! того, что цель даёт сверх `c`, где структура открыта нараспашку. Делать поля
//! публичными ради теста значило бы менять **продукт** ради **проверки**.
//!
//! У цели `sv` наблюдение идёт иерархической ссылкой (`dut.<сигнал>`) — там это
//! законный отладочный механизм языка. В Rust аналога нет, и порт — честный
//! способ: ровно так прошивку наблюдает и реальная плата.
//!
//! # Мягкая деградация
//!
//! `rustc` — зависимость проекта (это Rust-репозиторий), поэтому пропуск здесь
//! теоретический: проверка оставлена по образцу `cc_available()`, чтобы тест
//! падал по существу, а не по отсутствию инструмента.

use grammar::semantic::tree::construct_model;
use simulation::{TickResult, Unit, Value, build_unit};
use std::path::{Path, PathBuf};
use std::process::Command;

/// Тактов в трассе — с запасом над её длиной.
const TRACE_TICKS: usize = 6;

fn rustc_available() -> bool {
    Command::new("rustc")
        .arg("--version")
        .output()
        .map(|o| o.status.success())
        .unwrap_or(false)
}

fn sim_value(unit: &Unit, name: &str) -> i64 {
    match unit.variable(name) {
        Some(Value::Number(n)) => n,
        Some(Value::Boolean(b)) => i64::from(b),
        other => panic!("порт '{name}': неожиданное значение {other:?}"),
    }
}

/// Потактовая трасса симулятора: значение `port` после каждого такта.
fn simulate_trace(fixture: &Path, port: &str) -> Vec<i64> {
    let source = std::fs::read_to_string(fixture).expect("фикстура читается");
    let (ast, _) = grammar::parse(&source, 0).expect("разбор");
    let model = construct_model(&ast, None, &[]).expect("семантика");
    let mut unit = build_unit(model).expect("построение юнита");
    let mut trace = Vec::new();
    for _ in 0..TRACE_TICKS {
        let result = unit.tick();
        assert!(
            !matches!(result, TickResult::Failed(_)),
            "симуляция не должна падать: {result:?}"
        );
        trace.push(sim_value(&unit, port));
        if result == TickResult::Terminated {
            break;
        }
    }
    trace
}

/// Потактовая трасса порождённой прошивки.
///
/// Порождает `.rs` тем же `lamc`, пишет драйвер, компилирует его настоящим
/// `rustc` и запускает. Драйвер реализует `Hal`, запоминая последнее записанное
/// в порт значение, и печатает его после каждого такта — то есть наблюдает
/// прошивку ровно так, как наблюдала бы плата.
///
/// **Драйвер пишется здесь, а не порождается `lamc`.** Он принадлежность
/// проверки, а не продукта (то же решение, что и у тестбенча цели `sv`).
///
/// `root` — имя корневой структуры (CamelCase от имени файла), `variant` — имя
/// варианта порта в `OutU8Port`.
fn rust_trace(
    dir: &Path,
    fixture: &Path,
    basename: &str,
    root: &str,
    variant: &str,
    ticks: usize,
) -> Vec<i64> {
    let source = std::fs::read_to_string(fixture).expect("фикстура читается");
    grammar::compile_to_rust(
        basename,
        &source,
        dir.to_str().expect("путь в UTF-8"),
        &[],
        &grammar::generator::GenerateOptions::default(),
    )
    .expect("порождение Rust");

    let module = dir.join(format!("{basename}.rs"));
    // Наблюдение выносится из HAL общей ячейкой: поле `hal` модели приватно и
    // аксессора не имеет — как и должно быть у прошивки. Ровно так же плата
    // наблюдает не «внутренности прошивки», а состояние регистра, в который та
    // пишет.
    let driver = format!(
        r#"#[path = "{module}"]
mod generated;
use generated::{{Hal, OutU8Port, {root}}};
use std::cell::RefCell;
use std::rc::Rc;

/// Подставное железо: запоминает последнее записанное в порт значение.
struct Probe {{ reg: Rc<RefCell<u8>> }}

impl Hal for Probe {{
    fn write_u8(&mut self, port: OutU8Port, value: u8) {{
        assert!(matches!(port, OutU8Port::{variant}), "неожиданный порт");
        *self.reg.borrow_mut() = value;
    }}
}}

fn main() {{
    let reg = Rc::new(RefCell::new(0u8));
    let mut model = {root}::new(Probe {{ reg: Rc::clone(&reg) }});
    model.init();
    for _ in 0..{ticks} {{
        model.tick();
        println!("TICK {{}}", reg.borrow());
    }}
}}
"#,
        module = module.display(),
    );
    let driver_path = dir.join("driver.rs");
    std::fs::write(&driver_path, driver).expect("запись драйвера");

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
        .current_dir(dir)
        .output()
        .expect("запуск драйвера");
    String::from_utf8_lossy(&run.stdout)
        .lines()
        .filter_map(|line| line.strip_prefix("TICK "))
        .map(|v| v.trim().parse::<i64>().expect("значение — целое"))
        .collect()
}

fn build_dir(tag: &str) -> PathBuf {
    let dir = std::env::temp_dir().join(format!("lam_conformance_rust_{tag}"));
    let _ = std::fs::remove_dir_all(&dir);
    std::fs::create_dir_all(&dir).expect("каталог сборки");
    dir
}

fn fixture(dir: &Path, name: &str, source: &str) -> PathBuf {
    let path = dir.join(format!("{name}.lam"));
    std::fs::write(&path, source).expect("запись фикстуры");
    path
}

/// Общая проверка: трассы совпадают потактово и тело старта — на такте 1.
fn check(tag: &str, root: &str, source: &str) {
    let dir = build_dir(tag);
    let path = fixture(&dir, tag, source);
    let sim = simulate_trace(&path, "n");
    assert_eq!(
        sim.first(),
        Some(&1),
        "симулятор: тело стартового состояния обязано исполниться на такте 1 \
         (контракт ADR 0033), трасса={sim:?}"
    );

    if !rustc_available() {
        eprintln!("[ПРОПУСК] {tag}: rustc не найден — сверка с прошивкой не выполнена");
        return;
    }
    let rs = rust_trace(&dir, &path, tag, root, "N", sim.len());
    assert_eq!(
        sim, rs,
        "потактовые трассы симулятора и порождённого Rust обязаны совпадать НА \
         КАЖДОМ такте: гейт rustc/clippy доказывает лишь, что вывод \
         компилируется.\nсимулятор={sim:?}\nRust={rs:?}"
    );
}

/// **T23 (контракт 0033), глубина 1:** тело стартового состояния — на такте 1.
#[test]
fn shift_is_zero_at_depth_1() {
    check(
        "rsdepth1",
        "Rsdepth1",
        "out n: u8 := 0; \
         start S0 { always { n := 1; } ref S1; } \
         state S1 { always { n := 2; } }",
    );
}

/// **T23, глубина 2** (`start E = M;`): лишний уровень трассу не сдвигает.
#[test]
fn shift_is_zero_at_depth_2() {
    check(
        "rsdepth2",
        "Rsdepth2",
        "model M { out n: u8 := 0; start S0 { always { n := 1; } ref S1; } \
         state S1 { always { n := 2; } } } \
         start E = M;",
    );
}

/// **T23, глубина 3** — где сдвиг цели `c` был максимальным (3 такта до 0033).
#[test]
fn shift_is_zero_at_depth_3() {
    check(
        "rsdepth3",
        "Rsdepth3",
        "model Inner { out n: u8 := 0; start S0 { always { n := 1; } ref S1; } \
         state S1 { always { n := 2; } } } \
         model Mid { start M = Inner; } \
         start E = Mid;",
    );
}

/// Потактовая трасса на модели, эволюционирующей несколько тактов.
///
/// Сдвиг на такт (если бы он вернулся) сместил бы **всю** трассу, а не только
/// первое значение, — и был бы пойман здесь.
#[test]
fn per_tick_trace_matches_generated_rust() {
    let dir = build_dir("rstrace");
    let source = "model Counter { out n: u8 := 0; \
                  start S0 { always { n := 1; } ref S1; } \
                  state S1 { always { n := 2; } ref S2; } \
                  state S2 { always { n := 3; } } } \
                  start Entry = Counter;";
    let path = fixture(&dir, "rstrace", source);
    let sim = simulate_trace(&path, "n");
    // Пиннинг: если трасса симулятора изменится, тест упадёт здесь, а не
    // «подстроится» под прошивку.
    assert_eq!(
        sim,
        vec![1, 2, 3],
        "ожидаемая трасса симулятора: n = 1, 2, 3"
    );

    if !rustc_available() {
        eprintln!(
            "[ПРОПУСК] per_tick_trace_matches_generated_rust: rustc не найден \
             (трасса симулятора пришпилена выше)"
        );
        return;
    }
    let rs = rust_trace(&dir, &path, "rstrace", "Rstrace", "N", sim.len());
    assert_eq!(
        sim, rs,
        "потактовые трассы обязаны совпадать.\nсимулятор={sim:?}\nRust={rs:?}"
    );
}
