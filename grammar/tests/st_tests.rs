//! Интеграционные тесты цели Structured Text (IEC 61131-3) — фича 0041.
//!
//! Задача 0041-02: отображение типов и секции объявлений. Тесты идут через
//! публичный API [`grammar::compile_to_st`] — то есть проверяют файл, который
//! реально увидит инженер ПЛК, а не промежуточное представление.
//!
//! Ожидаемые строки **сняты зондом** с фактического вывода компилятора
//! (правило `CLAUDE.md`: сперва зонд, затем assertions против захваченного).
//!
//! ## Чего эти тесты не доказывают
//!
//! Валидность ST по стандарту доказывает только внешний транспилятор MatIEC
//! `iec2c` (задача 0041-06); в CI его нет. На сегодня вывод `iec2c` **отвергает**
//! — у `FUNCTION_BLOCK` нет тела (`CASE state OF` — задача 0041-03). Проверено,
//! что это **единственное** препятствие: те же файлы с подставленным фиктивным
//! телом принимаются по всему корпусу `examples/`.

use grammar::generator::GenerateOptions;
use std::fs;
use std::path::{Path, PathBuf};

/// Компилирует фикстуру в ST и возвращает текст порождённого файла.
fn compile_fixture(name: &str) -> String {
    let src_path = format!("tests/data/st/valid/{}.lam", name);
    let source = fs::read_to_string(&src_path).unwrap_or_else(|e| panic!("{}: {}", src_path, e));
    let out_dir = target_dir(name);
    let _ = fs::remove_dir_all(&out_dir);
    grammar::compile_to_st(
        &src_path,
        &source,
        out_dir.to_str().unwrap(),
        &[],
        &GenerateOptions::default(),
    )
    .unwrap_or_else(|d| panic!("компиляция {} провалилась: {:?}", name, d));
    fs::read_to_string(out_dir.join(format!("{}.st", name)))
        .unwrap_or_else(|e| panic!("нет порождённого .st для {}: {}", name, e))
}

/// Каталог вывода теста. Имя уникально по фикстуре: тесты гоняются в один поток,
/// но общий каталог всё равно связал бы их друг с другом.
fn target_dir(name: &str) -> PathBuf {
    Path::new(env!("CARGO_TARGET_TMPDIR")).join(format!("st_{}", name))
}

/// **A4.1.** Используемая переменная-массив попадает в ST настоящим `ARRAY`.
///
/// Прямой контрпример дефекту Д1b фичи 0029: на этом же входе цель `c` печатает
/// `uint4_t` — размер массива подставлен вместо разрядности, тип не существует.
#[test]
fn test_st_array_variable_is_emitted_as_real_array() {
    let st = compile_fixture("array_var");
    assert!(
        st.contains("data : ARRAY [0..3] OF USINT;"),
        "массив [u8; 4] обязан дать ARRAY [0..3] OF USINT:\n{st}"
    );
    assert!(
        !st.contains("uint4_t"),
        "разрядность не должна подменяться числом элементов:\n{st}"
    );
}

/// **A4.3, A4.4.** Скалярные типы отображаются по нормативной таблице T1..T11.
///
/// Ключевые пункты: `float` → `LREAL` (f64 — как симулятор), а не `REAL` (f32,
/// дефект Д3); `bit` → `BOOL`, а не `int` (дефект Д2). `i16` → `INT` — в IEC
/// `INT` 16-битный, это не опечатка.
#[test]
fn test_st_scalar_types_follow_iec_table() {
    let st = compile_fixture("types_all");
    for expected in [
        "flag : BOOL := TRUE;",
        "ready : BOOL := FALSE;",
        "ratio : LREAL := 0.0;",
        "small : USINT := 0;",
        "signed_small : INT := 0;",
        "wide : UDINT := 0;",
        "signed_wide : LINT := 0;",
    ] {
        assert!(st.contains(expected), "нет объявления '{expected}':\n{st}");
    }
    assert!(
        !st.contains(": REAL "),
        "float обязан давать LREAL (f64), а не REAL (f32):\n{st}"
    );
}

/// **A5.2.** Варианты перечисления становятся именованными константами.
///
/// Не перечислимым типом IEC: MatIEC отвергает явные значения вариантов (проба
/// П4). Значения — `Bottom = 80`, `Top` наследует `81` (снято зондом с
/// `examples/elevator.lam:117`).
#[test]
fn test_st_enum_variants_become_named_constants() {
    let st = compile_fixture("enum_struct");
    assert!(st.contains("VAR CONSTANT"), "нет секции констант:\n{st}");
    assert!(
        st.contains("Floor_Bottom : USINT := 80;"),
        "нет константы Floor_Bottom:\n{st}"
    );
    assert!(
        st.contains("Floor_Top : USINT := 81;"),
        "Top обязан наследовать значение 81:\n{st}"
    );
}

/// **R5.4.** Структура объявляется через `TYPE … END_TYPE` **до** блока.
///
/// В IEC 61131-3 порядок объявлений значим: тип обязан быть известен к моменту
/// использования.
#[test]
fn test_st_struct_type_is_declared_before_function_block() {
    let st = compile_fixture("enum_struct");
    let type_decl = st.find("TYPE").expect("нет объявления TYPE");
    let fb = st.find("FUNCTION_BLOCK").expect("нет FUNCTION_BLOCK");
    assert!(
        type_decl < fb,
        "TYPE обязан предшествовать FUNCTION_BLOCK:\n{st}"
    );
    assert!(st.contains("END_STRUCT;"), "структура не закрыта:\n{st}");
    assert!(
        st.contains("origin : Coord;"),
        "переменная структурного типа ссылается на объявленный тип:\n{st}"
    );
}

/// **A5.1.** Массив нулевого размера → `ST-007`; файл **не создаётся**.
///
/// Проверяется и код диагностики, и отсутствие вывода: половинчатый файл был бы
/// хуже отказа — его бы отдали в среду ПЛК.
#[test]
fn test_st_zero_sized_array_fails_with_st007_and_writes_nothing() {
    let src_path = "tests/data/st/invalid/array_zero.lam";
    let source = fs::read_to_string(src_path).unwrap();
    let out_dir = target_dir("array_zero");
    let _ = fs::remove_dir_all(&out_dir);
    let err = grammar::compile_to_st(
        src_path,
        &source,
        out_dir.to_str().unwrap(),
        &[],
        &GenerateOptions::default(),
    )
    .expect_err("массив нулевого размера обязан отвергаться");
    assert_eq!(
        err.code.as_deref(),
        Some("ST-007"),
        "диагностика: {:?}",
        err
    );
    assert!(
        !out_dir.join("array_zero.st").exists(),
        "при отказе файл ST не должен создаваться"
    );
}

/// **Закрывает РИ4.** Внешняя карта переопределяет МК-адреса на ПЛК-локации.
///
/// `elevator.lam` написан под МК-адресацию: `in sensors_1: u8 := 268435456;`
/// (`0x10000000`) → `AT %IB268435456`, чего не имеет ни один ПЛК. Модель при
/// этом **корректна** (для целей `c`/`c-hal`) и не правится: ПЛК-локации живут
/// во внешней карте, а генератор берёт их по приоритету (inline < `address` <
/// карта).
#[test]
fn test_external_map_overrides_mcu_addresses_with_plc_locations() {
    let source = fs::read_to_string("../examples/elevator.lam").unwrap();
    let map_src = fs::read_to_string("../examples/elevator.plc.map").unwrap();
    let entries = grammar::parse_address_map(&map_src, 0).expect("карта должна разбираться");
    let out_dir = target_dir("elevator_at");
    let _ = fs::remove_dir_all(&out_dir);

    grammar::compile_to_st_at(
        "../examples/elevator.lam",
        &source,
        out_dir.to_str().unwrap(),
        &[],
        &entries,
        &GenerateOptions::default(),
    )
    .expect("st-at с картой должен компилироваться");

    let st = fs::read_to_string(out_dir.join("elevator.st")).unwrap();
    assert!(
        st.contains("sensors_1 AT %IB0"),
        "карта обязана переопределить адрес на ПЛК-локацию:\n{}",
        &st[..st.len().min(2000)]
    );
    assert!(
        !st.contains("%IB268435456"),
        "МК-адрес не должен доезжать до ПЛК-локации"
    );
    assert!(
        st.contains("CONFIGURATION"),
        "st-at обязана эмитить обёртку: VAR_GLOBAL вне CONFIGURATION недопустим"
    );
}

/// Цель `st` карту адресов **не** потребляет: она порождает библиотеку блоков.
#[test]
fn test_plain_st_target_ignores_addresses_and_emits_no_configuration() {
    let st = compile_fixture("types_all");
    assert!(!st.contains("AT %"), "цель st адреса не эмитит:\n{st}");
    assert!(
        !st.contains("CONFIGURATION"),
        "цель st обёртки не требует — это библиотека блоков:\n{st}"
    );
}

/// Комментарии порождённого файла — только в форме IEC `(* … *)`.
///
/// Проверка не косметическая: `//` и `/* */` для компилятора ST — синтаксическая
/// ошибка, то есть файл не приняла бы ни одна среда ПЛК.
#[test]
fn test_st_output_uses_iec_comments_only() {
    let st = compile_fixture("types_all");
    assert!(!st.contains("//"), "C-комментарий недопустим в ST:\n{st}");
    assert!(!st.contains("/*"), "C-комментарий недопустим в ST:\n{st}");
}
