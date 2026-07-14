use crate::eval::value::Value;
use serde::Deserialize;
use std::collections::HashMap;
use std::path::Path;

// ── Структуры шага симуляции ─────────────────────────────────────────────────

/// Один шаг симуляции из JSON-файла.
///
/// ```json
/// { "in": [2.5, 0, true], "inout": [8], "guard": { "out": [0], "vars": {"M::x": 1} } }
/// ```
#[derive(Deserialize, Default, Clone)]
pub struct SimStep {
    #[serde(rename = "in_ports")]
    pub in_ports: Option<Vec<serde_json::Value>>,
    pub inout: Option<Vec<serde_json::Value>>,
    pub guard: Option<Guard>,
}

/// Проверка состояния модели после шага.
#[derive(Deserialize, Clone)]
pub struct Guard {
    pub out: Option<Vec<serde_json::Value>>,
    pub inout: Option<Vec<serde_json::Value>>,
    pub vars: Option<HashMap<String, serde_json::Value>>,
}

// ── Чтение файла ─────────────────────────────────────────────────────────────

/// Читает JSON-файл симуляции и возвращает вектор шагов.
pub fn load_sim_steps(path: &Path) -> Result<Vec<SimStep>, String> {
    let content = std::fs::read_to_string(path)
        .map_err(|e| format!("Ошибка чтения {}: {}", path.display(), e))?;
    serde_json::from_str(&content)
        .map_err(|e| format!("Ошибка парсинга JSON {}: {}", path.display(), e))
}

// ── Конвертация значений ──────────────────────────────────────────────────────

/// Конвертирует JSON-значение в симуляционное Value.
pub fn json_to_value(v: &serde_json::Value) -> Option<Value> {
    match v {
        serde_json::Value::Bool(b) => Some(Value::Boolean(*b)),
        serde_json::Value::Number(n) => {
            if let Some(i) = n.as_i64() {
                Some(Value::Number(i))
            } else if let Some(f) = n.as_f64() {
                Some(Value::Real(f))
            } else {
                None
            }
        }
        serde_json::Value::Array(arr) => {
            let items: Option<Vec<Value>> = arr.iter().map(json_to_value).collect();
            Some(Value::Array(items?))
        }
        serde_json::Value::Null => None,
        _ => None,
    }
}

// ── Тесты ─────────────────────────────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_json_to_value_bool_true() {
        let v = serde_json::Value::Bool(true);
        assert!(matches!(json_to_value(&v), Some(Value::Boolean(true))));
    }

    #[test]
    fn test_json_to_value_bool_false() {
        let v = serde_json::Value::Bool(false);
        assert!(matches!(json_to_value(&v), Some(Value::Boolean(false))));
    }

    #[test]
    fn test_json_to_value_integer() {
        let v = serde_json::json!(42i64);
        assert!(matches!(json_to_value(&v), Some(Value::Number(42))));
    }

    #[test]
    fn test_json_to_value_negative_integer() {
        let v = serde_json::json!(-7i64);
        assert!(matches!(json_to_value(&v), Some(Value::Number(-7))));
    }

    #[test]
    fn test_json_to_value_float() {
        let v = serde_json::json!(2.5f64);
        let result = json_to_value(&v);
        let Some(Value::Real(f)) = result else {
            panic!("ожидался Real");
        };
        assert!((f - 2.5).abs() < 1e-9);
    }

    #[test]
    fn test_json_to_value_null_is_none() {
        let v = serde_json::Value::Null;
        assert!(
            json_to_value(&v).is_none(),
            "null должен означать 'не проверять'"
        );
    }

    #[test]
    fn test_json_to_value_array() {
        let v = serde_json::json!([1, true, 2.5]);
        let Some(Value::Array(arr)) = json_to_value(&v) else {
            panic!("ожидался Array");
        };
        assert_eq!(arr.len(), 3);
        assert!(matches!(arr[0], Value::Number(1)));
        assert!(matches!(arr[1], Value::Boolean(true)));
    }

    #[test]
    fn test_json_to_value_string_is_none() {
        let v = serde_json::Value::String("x".to_string());
        assert!(json_to_value(&v).is_none());
    }

    #[test]
    fn test_load_sim_steps_valid_json() {
        use std::io::Write;
        let mut f = tempfile::NamedTempFile::new().unwrap();
        write!(f, r#"[{{}}, {{"in_ports": [1, 2]}}]"#).unwrap();
        let steps = load_sim_steps(f.path()).unwrap();
        assert_eq!(steps.len(), 2);
        assert!(steps[0].in_ports.is_none());
        assert_eq!(steps[1].in_ports.as_ref().unwrap().len(), 2);
    }

    #[test]
    fn test_load_sim_steps_invalid_json() {
        use std::io::Write;
        let mut f = tempfile::NamedTempFile::new().unwrap();
        write!(f, "not json").unwrap();
        assert!(load_sim_steps(f.path()).is_err());
    }

    #[test]
    fn test_load_sim_steps_missing_file() {
        let path = std::path::Path::new("/no/such/file.json");
        assert!(load_sim_steps(path).is_err());
    }
}
