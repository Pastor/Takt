//! Потактовая сверка **массива** цели `sv` с эталоном (фича 0309).
//!
//! # Что было
//!
//! Тип массива цель печатала с фичи 0076 (`logic [7:0] a [0:2]`), но
//! **агрегатный инициализатор** уходил в ветвь сброса конкатенацией `{…}` —
//! формой, которой у распакованного массива нет. Замер 2026-08-20 на
//! `var arr: [u8; 3] := {1, 2, 3};`: семь потребителей вход исполняли, а
//! `sv` и `sv-mmio` отвечали `SV-002` уже на объявлении.
//!
//! # Почему сверка, а не линт
//!
//! `verilator` и `yosys` принимают модуль, который считает другое (урок 0045):
//! на этом стоял дефект, где `always_comb` читал регистр вместо `_next`.
//! Проверяются **значения** по тактам.

use std::path::{Path, PathBuf};
use std::process::Command;
use takt_lang::semantic::tree::construct_model;
use takt_sim::{Value, build_unit};

const FIXTURE: &str = "tests/data/eval/conformance_sv_array.takt";
const UNIT: &str = "svarray";
const TICKS: usize = 3;

fn verilator_available() -> bool {
    Command::new("verilator")
        .arg("--version")
        .output()
        .map(|o| o.status.success())
        .unwrap_or(false)
}

/// Каталог сборки уникален по тесту (инварианты 0190 и 0244).
fn build_dir(tag: &str) -> PathBuf {
    let thread = std::thread::current()
        .name()
        .unwrap_or("single")
        .replace(':', "_");
    let dir = std::env::temp_dir().join(format!("takt_{tag}_{thread}"));
    let _ = std::fs::remove_dir_all(&dir);
    std::fs::create_dir_all(&dir).expect("каталог сборки");
    dir
}

/// Трасса эталона: `(probe, first)` по тактам.
fn simulator_trace() -> Vec<(i128, i128)> {
    let source = std::fs::read_to_string(FIXTURE).expect("фикстура читается");
    let (ast, _) = takt_lang::parse(&source, 0).expect("разбор фикстуры");
    let model = construct_model(&ast, None, &[]).expect("семантика");
    let mut unit = build_unit(model).expect("построение Unit");
    let mut trace = Vec::new();
    for _ in 0..TICKS {
        let _ = unit.tick();
        let value = |name: &str| match unit.variable(name) {
            Some(Value::Number(v)) => v,
            other => panic!("порт '{name}' обязан быть числом, получено {other:?}"),
        };
        trace.push((value("probe"), value("first")));
    }
    trace
}

/// Трасса порождённого RTL: те же порты по тактам.
fn generated_sv_trace(dir: &Path) -> Vec<(i128, i128)> {
    let source = std::fs::read_to_string(FIXTURE).expect("фикстура читается");
    takt_lang::compile_to_sv(
        UNIT,
        &source,
        dir.to_str().expect("путь в UTF-8"),
        &[],
        &takt_lang::generator::GenerateOptions::default(),
    )
    .expect("порождение SystemVerilog");

    let tb = format!(
        r#"module tb;
    logic clk = 0, rst_n = 0;
    logic [7:0] probe, first;
    logic is_done;
    {UNIT} dut (.clk(clk), .rst_n(rst_n), .probe(probe), .first(first), .is_done(is_done));
    always #5 clk = ~clk;
    initial begin
        @(posedge clk);
        rst_n <= 1'b1;
        for (int i = 0; i < {TICKS}; i++) begin
            @(posedge clk);
            #1 $display("TICK %0d %0d", probe, first);
        end
        $finish;
    end
endmodule
"#
    );
    std::fs::write(dir.join("tb.sv"), tb).expect("тестбенч");

    let build = Command::new("verilator")
        .current_dir(dir)
        .args([
            "--binary",
            // Сборку порождённого C++ verilator ведёт в один поток; `-j 0`
            // отдаёт ей все ядра (фича 0241).
            "-j",
            "0",
            "--timing",
            "-Wno-fatal",
            "--top-module",
            "tb",
            "tb.sv",
            &format!("{UNIT}.sv"),
            "-o",
            "simtb",
        ])
        .output()
        .expect("запуск verilator");
    assert!(
        build.status.success(),
        "verilator не собрал тестбенч массива:\n{}",
        String::from_utf8_lossy(&build.stderr)
    );
    let run = Command::new(dir.join("obj_dir").join("simtb"))
        .current_dir(dir)
        .output()
        .expect("запуск симуляции");
    assert!(run.status.success(), "симуляция RTL массива упала");
    String::from_utf8_lossy(&run.stdout)
        .lines()
        .filter_map(|line| {
            let rest = line.strip_prefix("TICK ")?;
            let mut it = rest.split_whitespace();
            let probe = it.next()?.parse::<i128>().ok()?;
            let first = it.next()?.parse::<i128>().ok()?;
            Some((probe, first))
        })
        .collect()
}

/// Значения массива совпадают у эталона и у RTL.
///
/// ⚠️ Ожидание записано **числами**: `arr[1] = 2` не меняется, поэтому `acc`
/// идёт 2, 4, 6, а `arr[0]` получает то же значение в том же такте — это и
/// проверяет, что инициализатор доехал (иначе `arr[1]` был бы нулём) и что
/// индексная запись работает.
#[test]
fn array_values_match_generated_sv() {
    let sim = simulator_trace();
    assert_eq!(
        sim,
        vec![(2, 2), (4, 4), (6, 6)],
        "эталон обязан читать инициализированный массив: {sim:?}"
    );

    if !verilator_available() {
        eprintln!("[ПРОПУСК] array_values_match_generated_sv: verilator не найден");
        return;
    }
    let dir = build_dir("sv_array");
    let rtl = generated_sv_trace(&dir);
    assert_eq!(rtl.len(), TICKS, "тестбенч обязан напечатать каждый такт");
    assert_eq!(sim, rtl, "трассы разошлись\nsim={sim:?}\nRTL={rtl:?}");
}

/// Число значений в агрегате сверяется с объявленным размером.
///
/// Без проверки лишний элемент уехал бы в шаблон присваивания, и `verilator`
/// ответил бы своей ошибкой о ширине — то есть отказ пришёл бы от **чужого**
/// инструмента на порождённом файле (класс 0184).
#[test]
fn aggregate_size_mismatch_is_refused() {
    let dir = build_dir("sv_array_bad");
    let source = "var arr: [u8; 3] := {1, 2};\nvar i: u8 := 0;\n\
         out probe: u8 at 0;\n\
         start Run { always { i := i + 1; probe := arr[0]; } ref Run: i < 100; }\n";
    let err = takt_lang::compile_to_sv(
        "svarraybad",
        source,
        dir.to_str().expect("путь в UTF-8"),
        &[],
        &takt_lang::generator::GenerateOptions::default(),
    )
    .expect_err("несовпадение размера обязано отвергаться");
    assert_eq!(err.code.as_deref(), Some("SV-002"), "{err:?}");
}
