//! Панель прогона: порты с родом поля, ручной ввод и значения после такта.

use super::*;
use serde_json::{Value, json};

/// Насос и вентилятор параллельно: по порту каждого рода, двусмысленное имя `on`.
const PLANT: &str = "enum Mode { Off, Slow = 3, Fast }\n\
model Pump {\n in on: bit;\n in armed: bool;\n in level: i16;\n in hold: duration;\n \
in gain: q(8, 8);\n in ratio: float;\n in mode: Mode;\n in mask: [bit;4];\n inout bus: u8;\n \
out speed: u8;\n out flags: [u8;2];\n \
start Run { always { if on { speed := level as u8; } } ref Run; }\n}\n\
model Fan { in on: bit; start Idle { ref Idle; } }\n\
start Main = Pump | Fan;\n";

fn json_of(text: &str) -> Value {
    serde_json::from_str(text).expect("ответ моста — JSON")
}

fn opened() -> (u32, Value) {
    let reply = json_of(&open(PLANT, "", 0, 0, Default::default(), Some(20)));
    assert_eq!(reply["ok"], true, "{reply}");
    (reply["id"].as_u64().expect("id") as u32, reply)
}

fn named(value: Value) -> std::collections::BTreeMap<String, Value> {
    serde_json::from_value(value).expect("словарь")
}

/// P1: порты отдаются с родом поля, границами и вариантами; двусмысленное имя -
/// квалифицированными формами.
#[test]
fn ports_carry_the_field_kind() {
    let (id, reply) = opened();
    let by_name = |name: &str| -> Value {
        reply["ports"]
            .as_array()
            .expect("ports")
            .iter()
            .find(|port| port["name"] == name)
            .cloned()
            .unwrap_or_else(|| panic!("нет порта {name}: {reply}"))
    };
    assert_eq!(
        by_name("Pump::on"),
        json!({"name": "Pump::on", "direction": "in", "type": "bit", "kind": "bit"})
    );
    assert_eq!(by_name("Fan::on")["kind"], "bit");
    assert_eq!(by_name("armed")["kind"], "bool");
    assert_eq!(
        by_name("level"),
        json!({"name": "level", "direction": "in", "type": "i16", "kind": "integer", "min": "-32768", "max": "32767"})
    );
    assert_eq!(by_name("hold")["kind"], "duration");
    assert_eq!(
        by_name("gain"),
        json!({"name": "gain", "direction": "in", "type": "q(8, 8)", "kind": "fixed", "m": 8, "n": 8, "sat": false})
    );
    assert_eq!(by_name("ratio")["kind"], "float");
    assert_eq!(
        by_name("mode")["variants"],
        json!([{"name": "Off", "value": "0"}, {"name": "Slow", "value": "3"}, {"name": "Fast", "value": "4"}])
    );
    assert_eq!(
        by_name("mask")["kind"],
        "composite",
        "бит-вектор - массив разрядов"
    );
    assert_eq!(by_name("bus")["direction"], "inout");
    assert_eq!(by_name("speed")["direction"], "out");
    assert_eq!(by_name("flags")["kind"], "composite");
    assert!(
        !reply["ports"]
            .as_array()
            .unwrap()
            .iter()
            .any(|port| port["name"] == "on"),
        "голого двусмысленного имени нет"
    );
    assert_eq!(reply["values"]["speed"], "0", "значения до первого такта");
    close(id);
}

/// P2: ручной ввод ложится перед следующим тактом, такт отдаёт применённое и значения.
#[test]
fn inputs_reach_the_next_tick() {
    let (id, _) = opened();
    let first = json_of(&tick(id, 1, false, None));
    assert_eq!(first["values"]["speed"], "0");
    assert_eq!(first["manual"], json!([[]]));
    let set = json_of(&inputs(
        id,
        &named(json!({"Pump::on": 1, "level": 42})),
        &named(json!({"bus": 7})),
    ));
    assert_eq!(set, json!({"ok": true, "set": 3}));
    let second = json_of(&tick(id, 2, false, None));
    assert_eq!(second["values"]["speed"], "42", "{second}");
    assert_eq!(second["values"]["bus"], "7", "{second}");
    assert_eq!(
        second["manual"],
        json!([
            [
                {"name": "Pump::on", "field": "in", "value": 1},
                {"name": "level", "field": "in", "value": 42},
                {"name": "bus", "field": "inout", "value": 7}
            ],
            []
        ]),
        "по списку на строку, применённое - на первом такте порции"
    );
    close(id);
}

/// P3: отказ ввода - текст эталона; закрытый прогон отвечает отказом.
#[test]
fn a_bad_input_is_refused_by_the_reference() {
    let (id, _) = opened();
    let refused = json_of(&inputs(id, &named(json!({"on": 1})), &Default::default()));
    assert_eq!(refused["ok"], false, "{refused}");
    let text = refused["error"]["message"].as_str().expect("текст");
    assert!(
        text.contains("SIM-031") && text.contains("Pump::on"),
        "{text}"
    );
    close(id);
    let closed = json_of(&inputs(
        id,
        &named(json!({"level": 1})),
        &Default::default(),
    ));
    assert_eq!(closed["ok"], false, "{closed}");
}

#[test]
fn recorded_ticks_replay_inputs_and_guards() {
    let scenario = r#"[{"in_ports":{"Pump::on":1,"level":7}},
        {"guard":{"out":{"speed":42}},"time_ms":9},{}]"#;
    let open_run = |scenario| {
        let reply = json_of(&open(PLANT, scenario, 0, 0, Default::default(), Some(3)));
        assert_eq!(reply["ok"], true, "{reply}");
        reply["id"].as_u64().unwrap() as u32
    };
    let id = open_run(scenario);
    assert_eq!(
        json_of(&inputs(
            id,
            &named(json!({"Pump::level":42})),
            &Default::default()
        ))["ok"],
        true
    );
    let run = json_of(&tick(id, 3, true, None));
    let recorded = run["recorded"].as_array().expect("записанные шаги");
    assert_eq!(recorded.len(), 3);
    assert_eq!(
        json_of(recorded[0].as_str().unwrap())["in_ports"]["level"],
        42
    );
    let scenario = format!(
        "[{}]",
        recorded
            .iter()
            .map(|step| step.as_str().unwrap())
            .collect::<Vec<_>>()
            .join(",")
    );
    let replay = open_run(&scenario);
    let again = json_of(&tick(replay, 3, true, None));
    assert_eq!(again["lines"], run["lines"]);
    close(id);
    close(replay);
}

#[test]
fn edits_replace_only_unplayed_ticks() {
    let source = "in x: u8; out y: u8; start Read { always { y := x; } ref Read; }";
    let scenario = r#"[{"in_ports":{"x":3}},{},{}]"#;
    let edited = r#"[{"in_ports":{"x":9}},{},{"in_ports":{"x":8}}]"#;
    let opened = json_of(&open(source, scenario, 0, 0, Default::default(), Some(3)));
    let id = opened["id"].as_u64().unwrap() as u32;
    assert_eq!(json_of(&tick(id, 1, false, None))["values"]["y"], "3");
    assert_eq!(
        json_of(&tick(id, 1, false, Some(edited)))["values"]["y"],
        "3"
    );
    assert_eq!(json_of(&tick(id, 1, false, None))["values"]["y"], "8");
    close(id);

    let opened = json_of(&open(source, edited, 0, 0, Default::default(), Some(3)));
    let id = opened["id"].as_u64().unwrap() as u32;
    assert_eq!(json_of(&tick(id, 1, false, None))["values"]["y"], "9");
    close(id);
}

#[test]
fn recording_keeps_completed_ticks_before_a_guard_failure() {
    let source = "out x: u8; start Read { ref Read; }";
    let scenario = r#"[{},{"guard":{"out":{"x":42}}}]"#;
    let opened = json_of(&open(source, scenario, 0, 0, Default::default(), Some(2)));
    let id = opened["id"].as_u64().unwrap() as u32;
    let reply = json_of(&tick(id, 2, true, None));
    assert_eq!(reply["recorded"], json!(["{}"]));
    assert_eq!(reply["lines"].as_array().unwrap().len(), 1);
    assert!(reply["failure"].is_string());
    close(id);
}

#[test]
fn recorded_ticks_keep_the_external_stub_beyond_the_scenario() {
    let source =
        "extern fn read() -> u8; out x: u8; start Read { always { x := read(); } ref Read; }";
    let scenario = r#"[{"extern":{"read":42}}]"#;
    let opened = json_of(&open(source, scenario, 0, 0, Default::default(), Some(2)));
    let id = opened["id"].as_u64().unwrap() as u32;
    let reply = json_of(&tick(id, 2, true, None));
    assert_eq!(reply["values"]["x"], "42", "{reply}");
    let recorded = reply["recorded"].as_array().unwrap();
    assert_eq!(recorded.len(), 2);
    assert_eq!(recorded[0], recorded[1]);
    assert_eq!(json_of(recorded[1].as_str().unwrap())["extern"]["read"], 42);
    close(id);
}
