//! Модельное время внутри модели, реализующей состояние (`state Work = Heater`).
//!
//! Реализация - дочерний юнит со своими часами, и время обязано доходить до него
//! так же, как до ветвей композиции: иначе `after` по времени не истекает, а
//! `every` не исполняется ни разу. Выдержка в тактах (`after 3t`) идёт другим
//! счётчиком и служит контролем: спуск времени не вправе её сдвинуть.

use takt_lang::semantic::tree::construct_model;
use takt_sim::{TickResult, Unit, Value, build_unit};

/// Состояние дерева после такта: активные состояния и значения названных имён.
struct Snapshot {
    states: Vec<String>,
    values: Vec<Option<i128>>,
}

fn unit_of(source: &str) -> Unit {
    let (ast, _) = takt_lang::parse(source, 0).expect("разбор");
    let model = construct_model(&ast, None, &[]).expect("семантика");
    build_unit(model).expect("построение юнита")
}

/// Прогон с модельным временем 1 мс на такт, как у `SimulationRunner` без `clock`.
fn trace(source: &str, ticks: usize, names: &[&str]) -> Vec<Snapshot> {
    let mut unit = unit_of(source);
    let mut out = Vec::new();
    for step in 0..ticks {
        unit.set_time_ns(i64::try_from(step).expect("такт") * 1_000_000);
        let result = unit.tick();
        assert!(
            !matches!(result, TickResult::Failed(_)),
            "падение: {result:?}"
        );
        out.push(Snapshot {
            states: unit.active_states(),
            values: names
                .iter()
                .map(|name| match unit.variable(name) {
                    Some(Value::Number(n)) => Some(n),
                    _ => None,
                })
                .collect(),
        });
        if result != TickResult::Processing {
            break;
        }
    }
    out
}

fn fixture(name: &str) -> String {
    let path = format!("tests/data/eval/{name}");
    std::fs::read_to_string(&path).unwrap_or_else(|e| panic!("фикстура {path}: {e}"))
}

fn states(snapshots: &[Snapshot]) -> Vec<String> {
    snapshots.iter().map(|s| s.states.join(", ")).collect()
}

/// `after 5ms` внутри реализации истекает на 6 мс - такт 7, как в корневой модели.
#[test]
fn after_by_time_fires_inside_state_implementation() {
    let snapshots = trace(&fixture("time_in_implementation.takt"), 9, &["h"]);
    let path = states(&snapshots);
    assert_eq!(
        path[..8],
        [
            "Work, W", "Work, W", "Work, W", "Work, W", "Work, W", "Work, W", "Work, D", "Done"
        ],
        "путь состояний: {path:?}"
    );
    assert_eq!(snapshots[5].values[0], Some(1), "до выдержки h = 1");
    assert_eq!(snapshots[6].values[0], Some(2), "выдержка истекла: h = 2");
}

/// `every 2ms` внутри реализации исполняется на 2, 4, 6 и 8 мс от входа.
#[test]
fn every_fires_inside_state_implementation() {
    let snapshots = trace(&fixture("every_in_implementation.takt"), 12, &["n"]);
    let counts: Vec<Option<i128>> = snapshots[..10].iter().map(|s| s.values[0]).collect();
    assert_eq!(
        counts,
        [0, 0, 0, 1, 1, 2, 2, 3, 3, 4].map(Some),
        "счётчик every по тактам"
    );
    let path = states(&snapshots);
    assert_eq!(
        path[9], "Work, Stop",
        "четвёртое срабатывание уводит в Stop"
    );
    assert_eq!(path[10], "Done", "next уводит модель в Done: {path:?}");
}

/// Контроль: выдержка в тактах внутри реализации шла и до правки часов, и спуск
/// времени не должен задеть счётчик тактов (двойной счёт сдвинул бы срабатывание).
#[test]
fn after_in_ticks_inside_state_implementation_is_unchanged() {
    let source = "model Heater {\n    out h: u8;\n    start W {\n        enter { h := 1; }\n        ref D: after 3t;\n    }\n    state D {\n        enter { h := 2; }\n    }\n}\nmodel Host {\n    out o: u8;\n    start Idle {\n        enter { o := 1; }\n        ref Work;\n    }\n    state Work = Heater {\n        next Done;\n    }\n    state Done {\n        enter { o := 3; }\n    }\n}\nstart Main = Host;\n";
    let snapshots = trace(source, 8, &["h"]);
    let path = states(&snapshots);
    let fired = path
        .iter()
        .position(|p| p == "Work, D")
        .unwrap_or_else(|| panic!("after 3t не сработал: {path:?}"));
    assert_eq!(fired + 1, 5, "after 3t срабатывает на такте 5: {path:?}");
}
