//! Порты под-модели композиции доступны драйверу/дисплею — фича 0079.
//!
//! Дефект: `PortNames` собирались только с корневой модели, поэтому порты
//! под-моделей композиции (`Cabin | Motor`) не перечислялись — их нельзя было
//! подать из sim-файла и они не отображались. Читать симулятор их мог
//! (`Unit::get_value` обходит все ветви), а подать вход было нечем: модель
//! «не реагировала на датчики» (`elevator_mini`). Симптом `SIM-009` замаскирован
//! фичей 0086 (порт без значения → 0), но порт оставался «немым».

use takt_lang::semantic::tree::construct_model;
use takt_sim::runner::PortNames;
use takt_sim::{Value, build_unit};

/// `Cabin | Motor`: Cabin несёт in-порт `Sensor` и out-порт `Alarm`, Motor —
/// in-порт `Limit`. Все три обязаны попасть в перечисление.
const SRC: &str = r#"
model Cabin {
    in Sensor: bit;
    out Alarm: bit;
    var seen: u8 := 0;
    start Watch {
        always { if Sensor { seen := 1; Alarm := true; } }
        ref Done: seen > 0;
    }
    state Done;
}
model Motor {
    in Limit: bit;
    start Run { ref Stop: Limit; }
    state Stop;
}
start Main = Cabin | Motor;
"#;

fn model() -> std::rc::Rc<std::cell::RefCell<takt_lang::semantic::ModelNode>> {
    let (ast, _) = takt_lang::parse(SRC, 0).expect("разбор");
    construct_model(&ast, None, &[]).expect("семантика")
}

/// Порты под-моделей композиции перечисляются (регрессия дефекта 0079).
#[test]
fn composition_submodel_ports_are_enumerated() {
    let names = PortNames::from_model(&model().borrow());
    assert!(
        names.in_ports.contains(&"Sensor".to_string()),
        "in-порт под-модели Cabin должен перечисляться: {:?}",
        names.in_ports
    );
    assert!(
        names.in_ports.contains(&"Limit".to_string()),
        "in-порт под-модели Motor должен перечисляться: {:?}",
        names.in_ports
    );
    assert!(
        names.out_ports.contains(&"Alarm".to_string()),
        "out-порт под-модели Cabin должен перечисляться: {:?}",
        names.out_ports
    );
}

/// Читаемость порта под-модели композиции (страховка): чтение `Sensor`
/// (значение по умолчанию `0` после 0086) не даёт `SIM-009` — порт в среде.
/// Сквозная реакция на поданный вход проверяется driven-сценарием
/// `examples/simulations/elevator_mini_floor2.json` (`FloorSensor_F2_Bottom` →
/// `current_floor := 2`), прогоняемым `scripts/run_simulations.sh`.
#[test]
fn submodel_port_is_readable_in_composition() {
    let unit = build_unit(model()).expect("построение юнита");
    // Порт `Sensor` под-модели читается через корневой юнит (обход всех ветвей).
    assert_eq!(
        unit.variable("Sensor"),
        Some(Value::Number(0)),
        "порт под-модели композиции должен читаться (0 по умолчанию, не SIM-009)"
    );
}
