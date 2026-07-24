//! Интеграционные тесты `--save-state`/`--load-state` (фича 0032).
//!
//! # Зачем этот слой
//!
//! Дефекты Д1–Д3 (пустой снимок, замороженная после загрузки модель, невидимая
//! запись в `inout`) прожили при зелёных тестах, потому что юнит-тесты
//! `state_io.rs` конструировали `Unit::Node { variables }` **вручную** — то самое
//! второе хранилище, из-за которого дефект и возник. Здесь юниты строятся из
//! `.takt` через `build_unit`, то есть тем же путём, что и в CLI, и сверяются
//! **значения** снимка и кругового рейса.
//!
//! `inout` (Д3/A5) требует впрыска порта через `Context::set_value` (он
//! `pub(crate)`), поэтому проверяется CLI-пробой (см. отчёт 0032); механизм
//! записи тот же, что у восстановления, и покрыт здесь тестом
//! `roundtrip_model_not_frozen`.

use takt_lang::semantic::tree::construct_model;
use takt_sim::state_io::{load_from_file, restore, save_to_file, snapshot};
use takt_sim::{TickResult, Unit, Value, build_unit};

fn unit_from_src(src: &str) -> Unit {
    let (ast, _) = takt_lang::parse(src, 0).unwrap_or_else(|e| panic!("разбор: {e:?}"));
    let model = construct_model(&ast, None, &[]).unwrap_or_else(|e| panic!("семантика: {e:?}"));
    build_unit(model).unwrap_or_else(|e| panic!("построение юнита: {e:?}"))
}

fn run(unit: &mut Unit, steps: usize) {
    for _ in 0..steps {
        if unit.tick() == TickResult::Terminated {
            break;
        }
    }
}

/// Сохраняет юнит в файл и возвращает его содержимое строкой (без зависимости от
/// serde_json в тестовом крейте — проверяем сериализованный текст напрямую).
fn saved_json(unit: &Unit) -> String {
    let dir = tempfile::tempdir().unwrap();
    let path = dir.path().join("state.json");
    save_to_file(unit, &path).expect("сохранение");
    std::fs::read_to_string(&path).expect("чтение файла состояния")
}

/// A2 (Д1): снимок захватывает значения переменных модели, а не пустой объект.
#[test]
fn save_captures_variables() {
    let mut unit = unit_from_src(
        "var n: u8 := 0;\nvar t: u8 := 0;\nstart Idle { always { n := 1; t := 2; } }",
    );
    run(&mut unit, 3);
    let json = saved_json(&unit);
    assert!(
        json.contains("\"n\": 1"),
        "снимок обязан содержать n=1, а не пустой variables:\n{json}"
    );
    assert!(
        json.contains("\"t\": 2"),
        "снимок обязан содержать t=2:\n{json}"
    );
}

/// A3 (Д2): круговой рейс через файл — значения совпадают после загрузки.
#[test]
fn roundtrip_values_match() {
    let src = "var n: u8 := 0;\nstart A { always { n := n + 5; } ref B; }\nstate B { always { n := n + 1; } }";
    let mut unit = unit_from_src(src);
    run(&mut unit, 2);
    let saved = unit.variable("n");

    let dir = tempfile::tempdir().unwrap();
    let path = dir.path().join("state.json");
    save_to_file(&unit, &path).expect("сохранение");

    let mut fresh = unit_from_src(src);
    load_from_file(&mut fresh, &path).expect("загрузка");
    assert_eq!(
        fresh.variable("n"),
        saved,
        "значение n после загрузки обязано совпасть с сохранённым"
    );
}

/// A4 (Д2): после загрузки модель **продолжает вычисляться** — присваивание
/// наблюдаемо меняет значение. Замороженной модели не существует. Это и есть
/// ключевая проверка: точечная починка снимка её бы не прошла.
#[test]
fn roundtrip_model_not_frozen() {
    let src = "var n: u8 := 0;\nstart A { always { n := n + 1; } ref B; }\nstate B { always { n := n + 1; } ref A; }";
    let mut unit = unit_from_src(src);
    run(&mut unit, 3);
    let snap = snapshot(&unit);
    let Some(Value::Number(saved)) = unit.variable("n") else {
        panic!("n обязан быть числом");
    };

    // Восстанавливаем в свежий юнит и тикаем ЕЩЁ раз — значение обязано вырасти.
    let mut fresh = unit_from_src(src);
    restore(&mut fresh, &snap);
    assert_eq!(
        fresh.variable("n"),
        Some(Value::Number(saved)),
        "восстановление обязано вернуть сохранённое n"
    );
    fresh.tick();
    assert_ne!(
        fresh.variable("n"),
        Some(Value::Number(saved)),
        "после загрузки модель обязана СЧИТАТЬ (n изменилось), а не замёрзнуть на {saved}"
    );
}

/// A7 (R7): файл, сохранённый до фичи 0032 (`"variables": {}`), читается без
/// ошибки, и модель после загрузки вычисляется от инициализаторов.
#[test]
fn loads_pre_0032_file() {
    let src = "var n: u8 := 0;\nstart Idle { always { n := n + 1; } ref B; }\nstate B;";
    let dir = tempfile::tempdir().unwrap();
    let path = dir.path().join("old.json");
    std::fs::write(
        &path,
        r#"{"kind":"Node","current_state":"Idle","variables":{}}"#,
    )
    .unwrap();

    let mut unit = unit_from_src(src);
    load_from_file(&mut unit, &path).expect("старый файл обязан читаться без ошибки");
    unit.tick();
    assert_eq!(
        unit.variable("n"),
        Some(Value::Number(1)),
        "модель обязана считать от инициализатора после загрузки пустого снимка"
    );
}

/// A8 (R6): константы (`const`) в снимок не попадают — их значение задано
/// исходником, восстанавливать из файла опасно.
#[test]
fn constants_excluded_from_snapshot() {
    let mut unit =
        unit_from_src("const MAX: u8 := 255;\nvar n: u8 := 0;\nstart Idle { always { n := 1; } }");
    run(&mut unit, 2);
    let json = saved_json(&unit);
    assert!(
        !json.contains("MAX"),
        "константа MAX не должна быть в снимке:\n{json}"
    );
    assert!(
        json.contains("\"n\": 1"),
        "переменная n должна быть в снимке:\n{json}"
    );
}
