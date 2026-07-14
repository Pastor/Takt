//! Сохранение и загрузка состояния симуляции в JSON-файл.
//!
//! Снимок описывает дерево [`Unit`]: текущее состояние каждого узла
//! и значения всех переменных/портов.

use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::path::Path;

use crate::eval::value::Value;
use crate::json_input::json_to_value;
use crate::unit::Unit;

// ── Структуры снимка ──────────────────────────────────────────────────────────

/// Рекурсивный снимок дерева Unit.
#[derive(Serialize, Deserialize, Debug)]
#[serde(tag = "kind")]
pub enum UnitSnapshot {
    None,
    Node {
        current_state: Option<String>,
        variables: HashMap<String, serde_json::Value>,
    },
    Parallel {
        children: Vec<UnitSnapshot>,
    },
    Sequential {
        index: usize,
        children: Vec<UnitSnapshot>,
    },
}

// ── Конвертация Value ←→ JSON ─────────────────────────────────────────────────

fn value_to_json(v: &Value) -> serde_json::Value {
    match v {
        Value::Number(n) => serde_json::Value::Number((*n).into()),
        Value::Real(f) => {
            serde_json::Value::Number(serde_json::Number::from_f64(*f).unwrap_or(0.into()))
        }
        Value::Boolean(b) => serde_json::Value::Bool(*b),
        Value::Array(arr) => serde_json::Value::Array(arr.iter().map(value_to_json).collect()),
    }
}

// ── Снимок Unit ───────────────────────────────────────────────────────────────

/// Создаёт снимок текущего состояния Unit-дерева.
pub fn snapshot(unit: &Unit) -> UnitSnapshot {
    match unit {
        Unit::None => UnitSnapshot::None,
        Unit::Node {
            state, variables, ..
        } => UnitSnapshot::Node {
            current_state: state.clone(),
            variables: variables
                .iter()
                .map(|(k, v)| (k.clone(), value_to_json(v)))
                .collect(),
        },
        Unit::Parallel { units, .. } => UnitSnapshot::Parallel {
            children: units.iter().map(|u| snapshot(&u.borrow())).collect(),
        },
        Unit::Sequential { units, index, .. } => UnitSnapshot::Sequential {
            index: *index,
            children: units.iter().map(|u| snapshot(&u.borrow())).collect(),
        },
    }
}

/// Восстанавливает состояние Unit-дерева из снимка.
///
/// Несовпадения структуры (например, снимок Parallel для Node) игнорируются.
pub fn restore(unit: &mut Unit, snap: &UnitSnapshot) {
    match (unit, snap) {
        (
            Unit::Node {
                state,
                variables,
                entered_initial,
                ..
            },
            UnitSnapshot::Node {
                current_state,
                variables: vars,
            },
        ) => {
            *state = current_state.clone();
            // Возобновление: модель уже находится в этом состоянии, поэтому
            // `enter` повторять нельзя — иначе он затрёт загруженные значения (Д5).
            *entered_initial = true;
            for (k, v) in vars {
                if let Some(val) = json_to_value(v) {
                    variables.insert(k.clone(), val);
                }
            }
        }
        (Unit::Parallel { units, .. }, UnitSnapshot::Parallel { children }) => {
            for (u, snap_child) in units.iter().zip(children.iter()) {
                restore(&mut u.borrow_mut(), snap_child);
            }
        }
        (
            Unit::Sequential { units, index, .. },
            UnitSnapshot::Sequential {
                index: snap_idx,
                children,
            },
        ) => {
            *index = *snap_idx;
            for (u, snap_child) in units.iter().zip(children.iter()) {
                restore(&mut u.borrow_mut(), snap_child);
            }
        }
        _ => {} // структура не совпадает — молча пропускаем
    }
}

// ── Файловые операции ─────────────────────────────────────────────────────────

/// Сохраняет снимок Unit в JSON-файл.
pub fn save_to_file(unit: &Unit, path: &Path) -> Result<(), String> {
    let snap = snapshot(unit);
    let json = serde_json::to_string_pretty(&snap)
        .map_err(|e| format!("Ошибка сериализации состояния: {e}"))?;
    std::fs::write(path, json)
        .map_err(|e| format!("Не удалось записать файл состояния {}: {e}", path.display()))
}

/// Загружает снимок из JSON-файла и восстанавливает состояние Unit.
pub fn load_from_file(unit: &mut Unit, path: &Path) -> Result<(), String> {
    let json = std::fs::read_to_string(path).map_err(|e| {
        format!(
            "Не удалось прочитать файл состояния {}: {e}",
            path.display()
        )
    })?;
    let snap: UnitSnapshot = serde_json::from_str(&json)
        .map_err(|e| format!("Ошибка разбора файла состояния {}: {e}", path.display()))?;
    restore(unit, &snap);
    Ok(())
}

// ── Тесты ─────────────────────────────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use super::*;
    use crate::unit::Predicate;
    use std::collections::HashMap;

    fn make_node(state: &str) -> Unit {
        let mut st = HashMap::new();
        st.insert(state.to_string(), vec![]);
        Unit::Node {
            entered_initial: false,
            context: None,
            variables: HashMap::new(),
            executions: HashMap::new(),
            state: Some(state.to_string()),
            state_transitions: st,
            state_executions: HashMap::new(),
            last_transition: None,
        }
    }

    fn make_node_with_var(state: &str, var: &str, val: Value) -> Unit {
        let mut st = HashMap::new();
        st.insert(state.to_string(), vec![]);
        let mut vars = HashMap::new();
        vars.insert(var.to_string(), val);
        Unit::Node {
            entered_initial: false,
            context: None,
            variables: vars,
            executions: HashMap::new(),
            state: Some(state.to_string()),
            state_transitions: st,
            state_executions: HashMap::new(),
            last_transition: None,
        }
    }

    fn make_transitioning_node(from: &str, to: &str) -> Unit {
        let pred = Predicate::new("go", |_| Ok(true));
        let mut st = HashMap::new();
        st.insert(from.to_string(), vec![(to.to_string(), pred)]);
        st.insert(to.to_string(), vec![]);
        Unit::Node {
            entered_initial: false,
            context: None,
            variables: HashMap::new(),
            executions: HashMap::new(),
            state: Some(from.to_string()),
            state_transitions: st,
            state_executions: HashMap::new(),
            last_transition: None,
        }
    }

    #[test]
    fn test_snapshot_none_is_none() {
        let snap = snapshot(&Unit::None);
        assert!(matches!(snap, UnitSnapshot::None));
    }

    #[test]
    fn test_snapshot_node_captures_state() {
        let unit = make_node("Active");
        let UnitSnapshot::Node { current_state, .. } = snapshot(&unit) else {
            panic!("ожидался UnitSnapshot::Node");
        };
        assert_eq!(current_state, Some("Active".to_string()));
    }

    #[test]
    fn test_snapshot_node_captures_variables() {
        let unit = make_node_with_var("S", "counter", Value::Number(42));
        let UnitSnapshot::Node { variables, .. } = snapshot(&unit) else {
            panic!("ожидался UnitSnapshot::Node");
        };
        assert_eq!(variables["counter"], serde_json::json!(42));
    }

    #[test]
    fn test_restore_node_state() {
        let mut unit = make_transitioning_node("Idle", "Active");
        unit.tick();
        // После тика — в Active
        assert_eq!(unit.current_state(), Some("Active"));

        // Восстанавливаем обратно в Idle
        let snap = UnitSnapshot::Node {
            current_state: Some("Idle".to_string()),
            variables: HashMap::new(),
        };
        restore(&mut unit, &snap);
        assert_eq!(unit.current_state(), Some("Idle"));
    }

    #[test]
    fn test_restore_node_variables() {
        let mut unit = make_node_with_var("S", "x", Value::Number(0));
        let snap = UnitSnapshot::Node {
            current_state: Some("S".to_string()),
            variables: [("x".to_string(), serde_json::json!(99))].into(),
        };
        restore(&mut unit, &snap);
        use crate::context::Context;
        let val = unit.get_value("x");
        assert!(matches!(val, Some(Value::Number(99))));
    }

    #[test]
    fn test_roundtrip_through_file() {
        let unit = make_node_with_var("Running", "count", Value::Number(7));
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("state.json");

        save_to_file(&unit, &path).expect("сохранение должно работать");

        let mut unit2 = make_node("Running");
        load_from_file(&mut unit2, &path).expect("загрузка должна работать");

        use crate::context::Context;
        assert_eq!(unit2.current_state(), Some("Running"));
        assert!(matches!(unit2.get_value("count"), Some(Value::Number(7))));
    }

    #[test]
    fn test_save_invalid_path_returns_error() {
        let unit = make_node("S");
        let result = save_to_file(&unit, Path::new("/no/such/dir/state.json"));
        assert!(result.is_err());
    }

    #[test]
    fn test_load_missing_file_returns_error() {
        let mut unit = make_node("S");
        let result = load_from_file(&mut unit, Path::new("/no/such/file.json"));
        assert!(result.is_err());
    }
}
