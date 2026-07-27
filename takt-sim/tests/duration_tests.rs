//! Модельное время и выдержка `after` в симуляторе (фича 0134, задача 0134-03).
//!
//! # Почему тесты на значения и на такт срабатывания
//!
//! Симулятор — **эталон** проекта: сверки целей сравнивают их трассу с его
//! трассой. Поэтому проверяется не «переход случился», а **на каком такте** он
//! случился и **при каком модельном времени** — расхождение на один такт
//! компилируется молча (класс дефекта фичи 0033 и главного капкана цели `sv`).
//!
//! Эталонное число взято не из головы: проба стадии архитектуры прогнала штатный
//! `TON` цели `st` под инжектированным временем (1 с на такт) и получила
//! срабатывание ровно через 3 с после входа. Симулятор обязан давать то же.

use takt_lang::semantic::tree::construct_model;
use takt_sim::{TickResult, Unit, Value, build_unit};

/// Модель с выдержкой: дверь открыта, через 3 с закрывается.
const DOORS: &str = r#"
model Doors {
    out open: bit := 0;
    start Opening {
        enter { open := 1; }
        ref Closing: after 3s;
    }
    state Closing {
        enter { open := 0; }
    }
}

start Main = Doors;
"#;

/// Строит `Unit` из исходника.
fn unit_of(source: &str) -> Unit {
    let (ast, _) = takt_lang::parse(source, 0).expect("исходник обязан разбираться");
    let model = construct_model(&ast, None, &[]).expect("семантика обязана строиться");
    build_unit(model).expect("Unit обязан строиться")
}

/// Прогоняет `ticks` тактов с шагом `step_ms`, возвращая `(такт, состояние)` переходов.
fn run(unit: &mut Unit, ticks: usize, step_ms: i64) -> Vec<(usize, String)> {
    let mut trace = Vec::new();
    for step in 0..ticks {
        // Часы двигаются перед тактом, кроме первого — как в `runner`.
        unit.set_time_ns(i64::try_from(step).unwrap_or(0) * step_ms * 1_000_000);
        let result = unit.tick();
        assert!(
            !matches!(result, TickResult::Failed(_)),
            "такт {} провалился: {result:?}",
            step + 1
        );
        if let Some(Value::Number(open)) = unit.variable("open") {
            trace.push((step + 1, format!("open={open}")));
        }
    }
    trace
}

#[test]
fn after_fires_exactly_three_seconds_from_state_entry() {
    let mut unit = unit_of(DOORS);
    // 1 с на такт: вход в стартовое состояние на такте 1 при t = 0,
    // выдержка истекает при t = 3 с — то есть на такте 4.
    let trace = run(&mut unit, 6, 1_000);
    let opened: Vec<usize> = trace
        .iter()
        .filter(|(_, v)| v == "open=1")
        .map(|(t, _)| *t)
        .collect();
    let closed: Vec<usize> = trace
        .iter()
        .filter(|(_, v)| v == "open=0")
        .map(|(t, _)| *t)
        .collect();
    assert_eq!(
        opened,
        vec![1, 2, 3],
        "дверь обязана быть открыта 3 такта: {trace:?}"
    );
    assert_eq!(
        closed,
        vec![4, 5, 6],
        "закрытие обязано быть на такте 4: {trace:?}"
    );
}

#[test]
fn faster_clock_needs_more_ticks_for_the_same_delay() {
    // Та же модель, вдвое более частые такты: та же ФИЗИЧЕСКАЯ выдержка, вдвое
    // больше тактов. Ровно ради этого фича и заводится: длительность переносится
    // между частотами, а число тактов — нет.
    let mut unit = unit_of(DOORS);
    let trace = run(&mut unit, 10, 500);
    let closed_at = trace
        .iter()
        .find(|(_, v)| v == "open=0")
        .map(|(t, _)| *t)
        .expect("дверь обязана закрыться");
    assert_eq!(
        closed_at, 7,
        "при 500 мс на такт 3 с истекают на такте 7: {trace:?}"
    );
}

#[test]
fn without_time_the_delay_never_expires() {
    // Часы стоят (все такты при t = 0) — выдержка не истекает никогда. Это
    // честнее, чем «истекла сразу»: модель, которой не дали времени, не имеет
    // права делать вид, будто время прошло.
    let mut unit = unit_of(DOORS);
    let trace = run(&mut unit, 5, 0);
    assert!(
        trace.iter().all(|(_, v)| v == "open=1"),
        "без хода времени выдержка истекать не должна: {trace:?}"
    );
}

#[test]
fn duration_variables_hold_nanoseconds() {
    // Длительность — значение, а не число: хранится в наносекундах (канон языка).
    let source = r#"
model Probe {
    out ready: bit := 0;
    var left: duration := 1m30s;
    start Idle {
        enter { ready := 1; }
    }
}

start Main = Probe;
"#;
    let mut unit = unit_of(source);
    let _ = unit.tick();
    assert_eq!(
        unit.variable("left"),
        Some(Value::Duration(90_000_000_000)),
        "1m30s обязаны храниться как 90 с в наносекундах"
    );
}

#[test]
fn composition_branches_share_one_clock() {
    // Ветви композиции живут в одном времени: иначе выдержка в одной ветви шла
    // бы по своим часам, и трасса перестала бы быть воспроизводимой.
    let source = r#"
model Left {
    out a: bit := 0;
    start L1 { ref L2: after 2s; }
    state L2 { enter { a := 1; } }
}

model Right {
    out b: bit := 0;
    start R1 { ref R2: after 2s; }
    state R2 { enter { b := 1; } }
}

start Main = Left | Right;
"#;
    let mut unit = unit_of(source);
    let mut first_a = None;
    let mut first_b = None;
    for step in 0..6i32 {
        unit.set_time_ns(i64::from(step) * 1_000_000_000);
        let _ = unit.tick();
        if first_a.is_none() && unit.variable("a") == Some(Value::Number(1)) {
            first_a = Some(step + 1);
        }
        if first_b.is_none() && unit.variable("b") == Some(Value::Number(1)) {
            first_b = Some(step + 1);
        }
    }
    assert_eq!(first_a, first_b, "ветви обязаны сработать на одном такте");
    assert_eq!(first_a, Some(3), "2 с при 1 с на такт истекают на такте 3");
}

#[test]
fn snapshot_round_trip_keeps_duration_exact() {
    // Наносекунды — канон языка, поэтому круговой рейс через снимок точен
    // (тот же приём, что у representation `q(m, n)`).
    let mut unit = unit_of(
        r#"
model Probe {
    out ready: bit := 0;
    var left: duration := 250ms;
    start Idle { enter { ready := 1; } }
}

start Main = Probe;
"#,
    );
    let _ = unit.tick();
    let before = unit.variable("left");
    let snapshot = takt_sim::state_io::snapshot(&unit);
    let json = serde_json::to_string(&snapshot).expect("снимок обязан сериализоваться");
    assert!(
        json.contains("250000000"),
        "снимок обязан нести наносекунды: {json}"
    );
    assert_eq!(before, Some(Value::Duration(250_000_000)));
}
