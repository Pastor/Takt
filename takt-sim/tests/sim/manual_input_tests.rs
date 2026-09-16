//! Ручной ввод значений портов посреди прогона.
//!
//! Предмет - место ручного значения относительно шага сценария и правило удержания:
//! значение ложится перед следующим тактом после шага сценария, побеждает его на
//! своём такте и держится, пока его не сменят. Проверки идут на значениях, а не на
//! факте перехода.

use std::collections::BTreeMap;

use takt_lang::semantic::tree::construct_model;
use takt_sim::json_input::SimStep;
use takt_sim::runner::{ManualInput, PortNames, SimulationRunner};
use takt_sim::{Value, build_unit};

/// Счётчик прибавляет `step` каждый такт; `step` - вход. По приращениям видно, какое
/// значение входа действовало на каждом такте.
const ADDER: &str = r#"
model Adder {
    in step: u8;
    inout mode: u8;
    var total: u16 := 0;
    start Run {
        always { total := total + step; }
        ref Run;
    }
}
start Root = Adder;
"#;

fn runner(scenario: &str, steps: Option<usize>) -> SimulationRunner {
    let (ast, _) = takt_lang::parse(ADDER, 0).expect("разбор модели");
    let model = construct_model(&ast, None, &[]).expect("семантика");
    let unit = build_unit(model.clone()).expect("построение Unit");
    let names = PortNames::from_model(&model.borrow());
    let steps_json: Vec<SimStep> = serde_json::from_str(scenario).expect("разбор сценария");
    SimulationRunner::new(unit, steps_json, steps, names)
}

fn total(runner: &SimulationRunner) -> i128 {
    match runner.unit().variable("total") {
        Some(Value::Number(n)) => n,
        other => panic!("total — число, получено {other:?}"),
    }
}

fn named(pairs: &[(&str, serde_json::Value)]) -> BTreeMap<String, serde_json::Value> {
    pairs
        .iter()
        .map(|(name, value)| (name.to_string(), value.clone()))
        .collect()
}

/// M1: ручное значение действует со следующего такта и удерживается.
#[test]
fn manual_value_applies_on_the_next_tick_and_holds() {
    let mut run = runner("[]", Some(10));
    run.step().expect("такт 1");
    assert_eq!(total(&run), 0, "без входа прибавлять нечего");
    run.set_manual_inputs(&named(&[("step", 2.into())]), &BTreeMap::new())
        .expect("постановка");
    assert_eq!(total(&run), 0, "постановка такта не делает");
    let step = run.step().expect("такт 2");
    assert_eq!(total(&run), 2, "значение действует на следующем такте");
    assert_eq!(
        step.manual,
        vec![ManualInput {
            name: "step".into(),
            field: "in",
            value: 2.into()
        }],
        "такт отдаёт применённое"
    );
    let step = run.step().expect("такт 3");
    assert_eq!(total(&run), 4, "значение удерживается");
    assert!(step.manual.is_empty(), "применённое отдаётся один раз");
}

/// M2: ручное значение побеждает шаг сценария на своём такте, следующий шаг сценария
/// ставит своё.
#[test]
fn manual_value_beats_the_scenario_step_of_its_tick() {
    let mut run = runner(
        r#"[{"in_ports": {"step": 1}}, {"in_ports": {"step": 1}}, {"in_ports": {"step": 10}}]"#,
        None,
    );
    run.step().expect("такт 1");
    assert_eq!(total(&run), 1);
    run.set_manual_inputs(&named(&[("step", 5.into())]), &BTreeMap::new())
        .expect("постановка");
    run.step().expect("такт 2");
    assert_eq!(
        total(&run),
        6,
        "на такте 2 действует ручное 5, а не 1 сценария"
    );
    run.step().expect("такт 3");
    assert_eq!(total(&run), 16, "такт 3: шаг сценария снова ставит своё");
}

/// M3: повторная постановка до такта заменяет значение; двунаправленный порт идёт своим
/// полем.
#[test]
fn a_second_setting_before_the_tick_replaces_the_first() {
    let mut run = runner("[]", Some(5));
    run.set_manual_inputs(&named(&[("step", 3.into())]), &BTreeMap::new())
        .expect("первая");
    run.set_manual_inputs(&named(&[("step", 7.into())]), &named(&[("mode", 1.into())]))
        .expect("вторая");
    let step = run.step().expect("такт 1");
    assert_eq!(total(&run), 7, "действует последнее поставленное");
    let fields: Vec<(&str, &str)> = step
        .manual
        .iter()
        .map(|input| (input.name.as_str(), input.field))
        .collect();
    assert_eq!(fields, vec![("step", "in"), ("mode", "inout")]);
}

/// M4: неизвестное имя, чужое направление и негодное значение отвергаются при
/// постановке, и ничего не ставится.
#[test]
fn a_bad_setting_is_refused_at_once_and_sets_nothing() {
    let mut run = runner("[]", Some(5));
    let error = run
        .set_manual_inputs(&named(&[("nope", 1.into())]), &BTreeMap::new())
        .expect_err("имени нет");
    assert!(
        error.contains("SIM-030") && error.contains("nope"),
        "{error}"
    );
    let error = run
        .set_manual_inputs(&named(&[("step", 1.into())]), &named(&[("step", 1.into())]))
        .expect_err("вход - не двунаправленный");
    assert!(error.contains("step"), "{error}");
    let error = run
        .set_manual_inputs(&named(&[("step", "много".into())]), &BTreeMap::new())
        .expect_err("строка");
    assert!(error.contains("step") && error.contains("много"), "{error}");
    let step = run.step().expect("такт 1");
    assert!(step.manual.is_empty(), "отказ ничего не поставил");
    assert_eq!(total(&run), 0);
}

/// M5: пустой ручной ввод трассу не меняет: она совпадает строка в строку.
#[test]
fn without_manual_input_the_trace_is_unchanged() {
    let scenario = r#"[{"in_ports": {"step": 1}}, {"in_ports": {"step": 3}}, {}]"#;
    let mut plain = runner(scenario, None);
    let mut touched = runner(scenario, None);
    touched
        .set_manual_inputs(&BTreeMap::new(), &BTreeMap::new())
        .expect("пустая постановка");
    for tick in 1..=3 {
        let a = plain.step().expect("такт").line;
        let b = touched.step().expect("такт").line;
        assert_eq!(a, b, "такт {tick}");
    }
}
