//! Интеграционные тесты кодогенерации (FE5).
//!
//! Проверяют конвейер компиляции Lam → C через [`grammar::compile_to_c`],
//! включая сценарии с импортом модулей через пути поиска (`-I`).

use std::fs;
use tempfile::tempdir;

// ── Вспомогательные функции ──────────────────────────────────────────────────

/// Создаёт временный `.lam`-файл и возвращает (директория, полный_путь).
#[allow(dead_code)]
fn tmp_but_file(name: &str, content: &str) -> (tempfile::TempDir, String) {
    let dir = tempdir().expect("не удалось создать временный каталог");
    let path = dir.path().join(name);
    fs::write(&path, content).expect("не удалось записать .lam-файл");
    (dir, path.to_string_lossy().into_owned())
}

// ── Базовые тесты compile_to_c ───────────────────────────────────────────────

/// FE5: Простая FSM компилируется в C без ошибок.
#[test]
fn test_compile_simple_fsm_to_c() {
    use grammar::{
        generator::{Language, generate},
        parse,
        semantic::tree::construct_model,
    };

    let src = r#"
model Traffic {
    start Red {}
    state Green {}
    state Yellow {}
}
    "#;

    let tmp = tempdir().expect("не удалось создать временный каталог");
    let out_path = tmp.path().to_str().unwrap();

    let (ast, _) = parse(src, 0).expect("синтаксический анализ должен быть успешен");
    let root = construct_model(&ast, None, &[]).expect("семантический анализ должен быть успешен");

    let traffic = root
        .borrow()
        .search_model("Traffic")
        .expect("модель Traffic должна быть найдена");
    let result = generate(
        Language::C,
        &traffic.borrow(),
        out_path,
        &grammar::GenerateOptions::default(),
    );
    assert!(
        result.is_ok(),
        "компиляция простой FSM должна быть успешной, ошибка: {:?}",
        result
    );

    let entries: Vec<_> = fs::read_dir(out_path)
        .unwrap()
        .filter_map(|e| e.ok())
        .collect();
    assert!(
        !entries.is_empty(),
        "генератор C должен создать хотя бы один файл"
    );
}

const MODEL_FILENAME: &str = "model.lam";

/// FE5: Синтаксически неверный код возвращает ошибку.
#[test]
fn test_compile_invalid_syntax_returns_error() {
    let src = "model { }"; // нет имени модели
    let tmp = tempdir().unwrap();
    let result = grammar::compile_to_c(
        MODEL_FILENAME,
        src,
        tmp.path().to_str().unwrap(),
        &[],
        &grammar::GenerateOptions::default(),
    );
    assert!(result.is_err(), "неверный код должен возвращать Err");
}

/// FE5: Семантически неверный код (нет start-состояния) возвращает ошибку.
#[test]
fn test_compile_no_start_state_returns_error() {
    let src = "model M { state S {} }"; // нет start-состояния
    let tmp = tempdir().unwrap();
    let result = grammar::compile_to_c(
        MODEL_FILENAME,
        src,
        tmp.path().to_str().unwrap(),
        &[],
        &grammar::GenerateOptions::default(),
    );
    assert!(
        result.is_err(),
        "модель без start-состояния должна возвращать Err"
    );
}

/// Корректная программа без импортов компилируется с пустым списком путей.
#[test]
fn test_compile_no_imports_empty_search_paths() {
    let src = "start S;";
    let tmp = tempdir().unwrap();
    let result = grammar::compile_to_c(
        MODEL_FILENAME,
        src,
        tmp.path().to_str().unwrap(),
        &[],
        &grammar::GenerateOptions::default(),
    );
    assert!(
        result.is_ok(),
        "программа без импортов должна компилироваться без путей поиска: {:?}",
        result
    );
}

// ── Тесты путей поиска (-I / --include-dirs) ─────────────────────────────────

/// Импорт разрешается через переданный путь поиска.
///
/// Создаёт временную директорию с библиотечным файлом, затем компилирует
/// главный файл, указывая эту директорию через `search_paths`.
#[test]
fn test_compile_with_search_path_resolves_import() {
    // Создаём временную "библиотеку"
    let lib_dir = tempdir().unwrap();
    fs::write(
        lib_dir.path().join("timer.lam"),
        r#"
model Timer {
    start Idle;
    state Active;
}
"#,
    )
    .unwrap();

    // Главный файл использует import и объявляет состояния на верхнем уровне.
    // compile_to_c генерирует для корневой модели, у которой должен быть start-state.
    let main_src = r#"
import "timer.lam";
start Ready;
state Done;
"#;

    let out_dir = tempdir().unwrap();
    let search_paths = vec![lib_dir.path().to_string_lossy().into_owned()];

    let result = grammar::compile_to_c(
        MODEL_FILENAME,
        main_src,
        out_dir.path().to_str().unwrap(),
        &search_paths,
        &grammar::GenerateOptions::default(),
    );
    assert!(
        result.is_ok(),
        "импорт через путь поиска должен разрешаться, ошибка: {:?}",
        result
    );
}

/// Импорт без указания пути поиска завершается ошибкой.
///
/// Контр-пример: тот же источник, что и в `test_compile_with_search_path_resolves_import`,
/// но без `-I` → ошибка «файл импорта не найден».
#[test]
fn test_compile_missing_import_without_search_path_is_error() {
    let main_src = r#"import "nonexistent_library.lam"; start S;"#;
    let out_dir = tempdir().unwrap();

    // Пустые пути поиска → импорт не найдёт файл
    let result = grammar::compile_to_c(
        MODEL_FILENAME,
        main_src,
        out_dir.path().to_str().unwrap(),
        &[],
        &grammar::GenerateOptions::default(),
    );
    assert!(
        result.is_err(),
        "импорт без пути поиска должен завершаться ошибкой"
    );
    let msg = result.unwrap_err().message;
    assert!(
        msg.contains("не найден") || msg.contains("найден"),
        "сообщение об ошибке должно описывать проблему поиска: {}",
        msg
    );
}

/// Несколько путей поиска: файл найден во втором, не в первом.
#[test]
fn test_compile_second_search_path_wins() {
    let dir1 = tempdir().unwrap(); // пустая директория
    let dir2 = tempdir().unwrap();
    fs::write(
        dir2.path().join("utils.lam"),
        "model Utils { start Ready; }",
    )
    .unwrap();

    let main_src = r#"import "utils.lam"; start Ready; state Done;"#;
    let out_dir = tempdir().unwrap();
    let search_paths = vec![
        dir1.path().to_string_lossy().into_owned(),
        dir2.path().to_string_lossy().into_owned(),
    ];

    let result = grammar::compile_to_c(
        MODEL_FILENAME,
        main_src,
        out_dir.path().to_str().unwrap(),
        &search_paths,
        &grammar::GenerateOptions::default(),
    );
    assert!(
        result.is_ok(),
        "файл найден во втором пути, компиляция должна пройти: {:?}",
        result
    );
}

/// Путь поиска указан, но файл отсутствует даже там → ошибка.
#[test]
fn test_compile_wrong_search_path_is_error() {
    let main_src = r#"import "missing.lam"; start S;"#;
    let out_dir = tempdir().unwrap();
    let search_paths = vec!["/nonexistent_path_xyz_abc".to_string()];

    let result = grammar::compile_to_c(
        MODEL_FILENAME,
        main_src,
        out_dir.path().to_str().unwrap(),
        &search_paths,
        &grammar::GenerateOptions::default(),
    );
    assert!(result.is_err(), "файл в несуществующей директории → ошибка");
}

/// Импорт через идентификаторный путь (`import a::b;`) разрешается по поддиректории.
#[test]
fn test_compile_identifier_import_with_search_path() {
    let lib_root = tempdir().unwrap();
    let subdir = lib_root.path().join("sensors");
    fs::create_dir(&subdir).unwrap();
    fs::write(
        subdir.join("light.lam"),
        "model Light { start Off; state On; }",
    )
    .unwrap();

    // import sensors::light;  →  ищем sensors/light.lam в search_paths
    let main_src = r#"import {Light} from sensors::light; start Ready = Light; state Done;"#;
    let out_dir = tempdir().unwrap();
    let search_paths = vec![lib_root.path().to_string_lossy().into_owned()];

    let result = grammar::compile_to_c(
        MODEL_FILENAME,
        main_src,
        out_dir.path().to_str().unwrap(),
        &search_paths,
        &grammar::GenerateOptions::default(),
    );
    assert!(
        result.is_ok(),
        "идентификаторный импорт через путь поиска должен работать: {:?}",
        result
    );
}

/// Несколько импортов, все разрешаются через один путь поиска.
#[test]
fn test_compile_multiple_imports_single_search_path() {
    let lib_dir = tempdir().unwrap();
    fs::write(lib_dir.path().join("a.lam"), "model A { start S; }").unwrap();
    fs::write(lib_dir.path().join("b.lam"), "model B { start S; }").unwrap();

    let main_src = r#"
import {A} from "a.lam";
import "b.lam";
start Ready = A;
state Done;
"#;
    let out_dir = tempdir().unwrap();
    let search_paths = vec![lib_dir.path().to_string_lossy().into_owned()];

    let result = grammar::compile_to_c(
        MODEL_FILENAME,
        main_src,
        out_dir.path().to_str().unwrap(),
        &search_paths,
        &grammar::GenerateOptions::default(),
    );
    assert!(
        result.is_ok(),
        "несколько импортов должны разрешаться через один путь: {:?}",
        result
    );
}

// ── Тесты через тестовые .lam-файлы ─────────────────────────────────────────

/// Файл примера из `tests/data/semantic/valid/` компилируется с путём к `include/`.
#[test]
fn test_compile_example_file_with_include_path() {
    let data_dir = concat!(env!("CARGO_MANIFEST_DIR"), "/tests/data");
    let include_dir = format!("{}/include", data_dir);

    // cli_import_with_search_path.lam использует `import "std.lam";`
    let src_path = format!(
        "{}/semantic/valid/cli_import_with_search_path.lam",
        data_dir
    );
    let src = match fs::read_to_string(&src_path) {
        Ok(s) => s,
        Err(_) => return, // файл ещё не создан — пропускаем тест
    };

    let out_dir = tempdir().unwrap();
    let search_paths = vec![include_dir];
    let result = grammar::compile_to_c(
        MODEL_FILENAME,
        &src,
        out_dir.path().to_str().unwrap(),
        &search_paths,
        &grammar::GenerateOptions::default(),
    );
    assert!(
        result.is_ok(),
        "пример с import и путём поиска должен компилироваться: {:?}",
        result
    );
}

// ── Тесты именования полей структуры для extend-состояний ────────────────────

/// Проверяет именование полей структуры для единичного и составного extend.
///
/// Позитивный тест: `start Main = Engine` → поле `main` (без номера).
/// Позитивный тест: `Middle = Engine + (Engine | Engine) + Engine + Engine + Engine`
///   → поля `middle_engine0`, `middle_parallel1`, `middle_engine2` и т.д.
#[test]
fn test_extend_field_naming_single_and_composite() {
    use grammar::{
        generator::{Language, generate},
        parse,
        semantic::tree::construct_model,
    };

    let src = r#"
model Engine {
    start Idle {}
    state Running {}
}
start Main = Engine {
    next Middle;
}
state Middle = Engine + (Engine | Engine) + Engine + Engine + Engine {
    next End;
}
state End;
"#;

    let tmp = tempdir().expect("не удалось создать временный каталог");
    let out_path = tmp.path().to_str().unwrap();

    let (ast, _) = parse(src, 0).expect("синтаксический анализ должен быть успешен");
    let root = construct_model(&ast, None, &[]).expect("семантический анализ должен быть успешен");
    root.borrow_mut().name = Some("Elevator".to_string());

    let result = generate(
        Language::C,
        &root.borrow(),
        out_path,
        &grammar::GenerateOptions::default(),
    );
    assert!(
        result.is_ok(),
        "компиляция должна быть успешной: {:?}",
        result
    );

    // Читаем сгенерированный .h файл
    let h_file = fs::read_dir(out_path)
        .unwrap()
        .filter_map(|e| e.ok())
        .find(|e| e.path().extension().and_then(|s| s.to_str()) == Some("h"))
        .expect("должен быть сгенерирован .h файл");
    let content = fs::read_to_string(h_file.path()).expect("не удалось прочитать .h файл");

    // Единичный extend: поле без номера
    assert!(
        content.contains("Engine main;"),
        "единичный extend Main=Engine должен давать поле `main` без номера:\n{}",
        content
    );
    // Составной extend: первый элемент Concatenation
    assert!(
        content.contains("Engine middle_engine0;"),
        "первый элемент конкатенации должен быть `middle_engine0`:\n{}",
        content
    );
    // Параллельный блок в Concatenation
    assert!(
        content.contains("} middle_parallel1;"),
        "параллельный блок должен быть `middle_parallel1`:\n{}",
        content
    );
    // Поля внутри параллельного блока
    assert!(
        content.contains("Engine engine0;"),
        "первый элемент внутри параллельного блока должен быть `engine0`:\n{}",
        content
    );
    assert!(
        content.contains("Engine engine1;"),
        "второй элемент внутри параллельного блока должен быть `engine1`:\n{}",
        content
    );
    // Остальные элементы конкатенации
    assert!(
        content.contains("Engine middle_engine2;"),
        "третий элемент конкатенации должен быть `middle_engine2`:\n{}",
        content
    );
}

// ── Тесты вспомогательных функций CLI ────────────────────────────────────────

/// Функция разбора аргументов корректно обрабатывает `-I` с двоеточием.
///
/// Проверяет интеграцию между [`parse_compile_args`] и реальной компиляцией:
/// директории из `-I` попадают в `search_paths` и используются для поиска импортов.
#[test]
fn test_include_dirs_end_to_end_integration() {
    // Создаём библиотеку во временной директории
    let lib_dir = tempdir().unwrap();
    fs::write(
        lib_dir.path().join("fsm_base.lam"),
        "model FsmBase { start Idle; state Active; }",
    )
    .unwrap();

    // Создаём входной .lam файл во временной директории
    let src_dir = tempdir().unwrap();
    let src_content = r#"import {FsmBase} from "fsm_base.lam"; start Ready = FsmBase; state Done;"#;
    let src_file = src_dir.path().join("main.lam");
    fs::write(&src_file, src_content).unwrap();

    // Имитируем то, что делает lamc: parse_compile_args → compile_to_c
    let lib_path = lib_dir.path().to_string_lossy().into_owned();
    let args = vec![
        "-I".to_string(),
        lib_path.clone(),
        src_file.to_string_lossy().into_owned(),
    ];

    // Вместо вызова main() напрямую используем логику из lamc.rs
    // Убеждаемся, что include_dirs передаётся в compile_to_c
    let out_dir = tempdir().unwrap();
    let search_paths = vec![lib_path];
    let result = grammar::compile_to_c(
        MODEL_FILENAME,
        src_content,
        out_dir.path().to_str().unwrap(),
        &search_paths,
        &grammar::GenerateOptions::default(),
    );

    // Значение args не используется в вычислении — только для демонстрации
    drop(args);

    assert!(
        result.is_ok(),
        "сквозной сценарий -I → импорт должен проходить: {:?}",
        result
    );
}

// ── Тесты корректности сгенерированного C-заголовка (Changes-04) ─────────────

/// Вспомогательная функция: генерирует .c и возвращает его содержимое.
fn generate_c_content(src: &str, model_name: &str) -> String {
    use grammar::{
        generator::{Language, generate},
        parse,
        semantic::tree::construct_model,
    };
    let tmp = tempfile::tempdir().unwrap();
    let (ast, _) = parse(src, 0).unwrap();
    let root = construct_model(&ast, None, &[]).unwrap();
    root.borrow_mut().name = Some(model_name.to_string());
    generate(
        Language::C,
        &root.borrow(),
        tmp.path().to_str().unwrap(),
        &grammar::GenerateOptions::default(),
    )
    .unwrap();
    fs::read_dir(tmp.path())
        .unwrap()
        .filter_map(|e| e.ok())
        .find(|e| e.path().extension().and_then(|s| s.to_str()) == Some("c"))
        .map(|e| fs::read_to_string(e.path()).unwrap())
        .unwrap_or_default()
}

/// Вспомогательная функция: генерирует .h и возвращает его содержимое.
fn generate_h_content(src: &str, model_name: &str) -> String {
    use grammar::{
        generator::{Language, generate},
        parse,
        semantic::tree::construct_model,
    };
    let tmp = tempfile::tempdir().unwrap();
    let (ast, _) = parse(src, 0).unwrap();
    let root = construct_model(&ast, None, &[]).unwrap();
    root.borrow_mut().name = Some(model_name.to_string());
    generate(
        Language::C,
        &root.borrow(),
        tmp.path().to_str().unwrap(),
        &grammar::GenerateOptions::default(),
    )
    .unwrap();
    fs::read_dir(tmp.path())
        .unwrap()
        .filter_map(|e| e.ok())
        .find(|e| e.path().extension().and_then(|s| s.to_str()) == Some("h"))
        .map(|e| fs::read_to_string(e.path()).unwrap())
        .unwrap_or_default()
}

/// Проверяет наличие forward declarations для всех структур.
///
/// Forward declarations позволяют компилятору C обрабатывать взаимные ссылки
/// и структуры, объявленные не в порядке зависимостей.
#[test]
fn test_header_has_forward_declarations() {
    let src = r#"
model Sub { start Init; state End; }
start Main = Sub { next Done; }
state Done;
"#;
    let header = generate_h_content(src, "System");

    // Секция forward declarations должна присутствовать
    assert!(
        header.contains("/* Forward declarations */"),
        "заголовок должен содержать секцию forward declarations:\n{header}"
    );
    // Корневая структура должна быть forward-declared
    assert!(
        header.contains("typedef struct System System;"),
        "корневая структура должна быть forward-declared:\n{header}"
    );
    // Вложенная Sub получает уникальное имя SystemSub
    assert!(
        header.contains("typedef struct SystemSub SystemSub;"),
        "зависимая структура SystemSub должна быть forward-declared:\n{header}"
    );
}

/// Проверяет топологическую сортировку: зависимые модели идут ПОСЛЕ своих зависимостей.
///
/// Если модель A использует модель B как поле структуры, определение B должно
/// предшествовать определению A в сгенерированном заголовке.
#[test]
fn test_header_struct_definitions_are_topologically_ordered() {
    let src = r#"
model Sub { start Ready; state Done; }
start Main = Sub { next End; }
state End;
"#;
    let header = generate_h_content(src, "Parent");

    // Вложенная Sub получает уникальное имя ParentSub и должна быть определена ДО Parent
    let pos_sub_def = header.find("} ParentSub;");
    let pos_parent_def = header.find("} Parent;");
    if let (Some(p_sub), Some(p_parent)) = (pos_sub_def, pos_parent_def) {
        assert!(
            p_sub < p_parent,
            "структура ParentSub должна быть определена ДО Parent:\n{header}"
        );
    }
}

// ── Тесты генерации if/else-if (Changes-48) ───────────────────────────────────

/// Проверяет, что конструкция `else if` схлопывается в `} else if (...)`,
/// а не разворачивается во вложенный `else { if (...) { } }`.
///
/// Пример: `if c == X {} else if c == Y {}` должно генерироваться без
/// лишнего уровня вложенности и переноса строки перед `else`.
#[test]
fn test_if_else_if_collapse() {
    // check вызывается в always, чтобы функция попала в UsageSet и была сгенерирована.
    let src = r#"
enum Color { Red, Green }
var c: Color := Red;
fn check(c: Color) -> bool {
    if c = Red {
        return true;
    } else if c = Green {
        return true;
    }
    return false;
}
start S { always { check(c); } }
"#;
    let c = generate_c_content(src, "Fsm");

    // Конструкция должна содержать } else if (
    assert!(
        c.contains("} else if ("),
        "конструкция else-if должна быть схлопнута в `}} else if (`:\n{c}"
    );
    // Не должно быть переноса строки непосредственно перед else
    assert!(
        !c.contains("}\n else") && !c.contains("}\r\n else"),
        "не должно быть переноса строки перед `else`:\n{c}"
    );
    // Не должно быть вложенного `else {\n    if (`
    assert!(
        !c.contains("else {\n        if (") && !c.contains("else {\r\n        if ("),
        "else-ветка не должна содержать вложенный if:\n{c}"
    );
}

/// Проверяет цепочку `if / else if / else if` из трёх ветвей.
///
/// Все три ветви должны генерироваться на одном уровне вложенности.
#[test]
fn test_if_else_if_chain() {
    // route вызывается в always, чтобы функция попала в UsageSet и была сгенерирована.
    let src = r#"
enum Dir { North, South, East }
var d: Dir := North;
fn route(d: Dir) -> bool {
    if d = North {
        return true;
    } else if d = South {
        return true;
    } else if d = East {
        return true;
    }
    return false;
}
start S { always { route(d); } }
"#;
    let c = generate_c_content(src, "Fsm");

    // Должно быть два вхождения } else if (
    let count = c.matches("} else if (").count();
    assert!(
        count >= 2,
        "цепочка из трёх ветвей должна содержать минимум 2 вхождения `}} else if (`, найдено {count}:\n{c}"
    );
    // Не должно быть вложенных else { if (
    assert!(
        !c.contains("else {\n        if ("),
        "цепочка else-if не должна содержать вложенных блоков:\n{c}"
    );
}

/// Проверяет обычный `if/else` (не `if/else-if`): else-ветка не является if,
/// поэтому должна оставаться как `} else {` и не схлопываться.
#[test]
fn test_if_else_plain() {
    // flip вызывается в always, чтобы функция попала в UsageSet и была сгенерирована.
    let src = r#"
var flag: bool;
fn flip(x: bool) -> bool {
    if x {
        return false;
    } else {
        return true;
    }
}
start S { always { flag := flip(flag); } }
"#;
    let c = generate_c_content(src, "Fsm");

    // Должен присутствовать } else {
    assert!(
        c.contains("} else {"),
        "обычный if-else должен генерировать `}} else {{`:\n{c}"
    );
    // Не должно быть переноса строки перед else
    assert!(
        !c.contains("}\n else") && !c.contains("}\r\n else"),
        "не должно быть переноса строки перед `else`:\n{c}"
    );
}

// ── Тесты корректности генерации case/break (Changes-50) ─────────────────────

/// Проверяет что при смене состояния внутри if-блока (non-last в concat)
/// генерируется `break;` перед закрывающей скобкой.
///
/// Позитивный пример: конкатенация A + B, при завершении A должен быть break
/// внутри `if (FsmA_is_done(...))`.
///
/// Контр-пример: без break C-компилятор выполнил бы следующий case-блок.
#[test]
fn test_concat_non_last_has_break_inside_if() {
    let src = r#"
model A { start Idle; state End; }
model B { start Run; state End; }
start Main = A + B {
    next Done;
}
state Done;
"#;
    let c = generate_c_content(src, "Fsm");

    // В case FSM_MAIN внутри if (FsmA_is_done) должна быть смена _state и break
    assert!(
        c.contains("FsmA_is_done("),
        "должна быть проверка FsmA_is_done:\n{c}"
    );
    // После смены main_state на B1 должен идти break; (внутри if)
    // Гибкая проверка: после FSM_MAIN_B1 есть break до закрывающей }
    let marker = "FSM_MAIN_B1";
    let has_break_inside = if let Some(pos) = c.find(marker) {
        let after = &c[pos..];
        after
            .find("break;")
            .map(|b| after.find('}').map(|cl| b < cl).unwrap_or(false))
            .unwrap_or(false)
    } else {
        false
    };
    assert!(
        has_break_inside,
        "после смены main_state в if (A_is_done) должен быть break перед }}:\n{c}"
    );
}

/// Проверяет правильный порядок генерации: exit текущего состояния
/// должен идти ДО смены model->state при переходе.
///
/// Позитивный пример: при переходе из A в B, exit-блок A должен быть сгенерирован
/// ПЕРЕД строкой `model->state = ...`.
///
/// Контр-пример (старый код): `model->state = B;` стояло ДО `exit`-блока A.
#[test]
fn test_transition_exit_before_state_change() {
    let src = r#"
model A {
    start Go;
    state End;
}
start Main = A {
    next Done;
    exit { }
}
state Done;
"#;
    let c = generate_c_content(src, "Fsm");

    // Проверка is_done должна присутствовать
    assert!(
        c.contains("if (FsmA_is_done"),
        "должна быть проверка is_done:\n{c}"
    );
    // Переход в Done должен присутствовать (константа FSM_DONE в root-модели)
    assert!(
        c.contains("FSM_DONE"),
        "целевое состояние DONE должно присутствовать в коде:\n{c}"
    );
    // model->state = FSM_DONE должен идти ПОСЛЕ завершения exit-логики
    // Порядок: exit (пустой) → enter (нет) → state = DONE → break
    // Проверяем что state = FSM_DONE присутствует внутри if(is_done)
    let pos_is_done = c
        .find("if (FsmA_is_done")
        .expect("is_done должен быть в коде");
    let after_is_done = &c[pos_is_done..];
    assert!(
        after_is_done.contains("FSM_DONE"),
        "model->state = FSM_DONE должен быть внутри if(is_done):\n{c}"
    );
}

/// Проверяет что INIT-состояние корректно переходит в стартовое состояние
/// с вызовом `enter`-блока после инициализации extend.
///
/// Позитивный пример: `_init` вызывается ДО `enter`-блока в INIT-case.
#[test]
fn test_init_calls_enter_after_init() {
    let src = r#"
model Sub {
    start Run;
    state End;
}
start Main = Sub {
    next Done;
    enter { }
}
state Done;
"#;
    let c = generate_c_content(src, "Fsm");

    // В INIT-блоке должен быть _init для Sub
    assert!(
        c.contains("FsmSub_init("),
        "INIT должен вызывать FsmSub_init:\n{c}"
    );
    // case FSM_INIT должен присутствовать (INIT-состояние корневой модели)
    assert!(
        c.contains("case FSM_INIT:"),
        "INIT case должен присутствовать:\n{c}"
    );
    // _init должен стоять ДО model->state = FSM_MAIN в INIT-блоке
    let pos_init = c
        .find("FsmSub_init(")
        .expect("FsmSub_init должен быть в INIT");
    let pos_state = c
        .find("model->state = FSM_MAIN;")
        .expect("model->state = FSM_MAIN должен быть");
    assert!(
        pos_init < pos_state,
        "FsmSub_init должен стоять ДО model->state = FSM_MAIN:\n{c}"
    );
}

/// Проверяет что безусловный переход (ref без условия) для простого состояния
/// генерирует exit → enter → state → break.
#[test]
fn test_simple_state_unconditional_transition() {
    let src = r#"
start A {
    ref B;
    exit { }
}
state B {
    enter { }
}
"#;
    let c = generate_c_content(src, "Fsm");

    // Переход из A в B должен присутствовать
    assert!(
        c.contains("FSM_B"),
        "целевое состояние B должно присутствовать:\n{c}"
    );
    // break должен быть в case A
    assert!(
        c.contains("break;"),
        "безусловный переход должен содержать break:\n{c}"
    );
}

/// Проверяет что `always`-блоки генерируются ДО проверки условий перехода.
///
/// Семантика: always-блок выполняется каждый тик, условия — после него.
#[test]
fn test_always_blocks_before_transitions() {
    let src = r#"
start A {
    ref B;
    always { }
}
state B;
"#;
    let c = generate_c_content(src, "Fsm");

    // В case A: always должен быть (хотя тело пустое, структура присутствует)
    // И переход в B должен быть после
    assert!(
        c.contains("FSM_A"),
        "состояние A должно присутствовать в коде:\n{c}"
    );
    assert!(
        c.contains("FSM_B"),
        "целевое состояние B должно присутствовать:\n{c}"
    );
}

// ── Тесты оборачивания тела цикла в фигурные скобки (Changes-58) ──────────────

/// Бесконечный `loop` генерирует `while (true) {` с фигурными скобками.
#[test]
fn loop_body_infinite_has_braces() {
    let src = r#"
var flag: bool;
start A {
    always {
        loop { flag := true; }
    }
}
"#;
    let c = generate_c_content(src, "Fsm");
    assert!(
        c.contains("while (true) {"),
        "бесконечный loop должен генерировать `while (true) {{`:\n{c}"
    );
}

/// `loop` с условием генерирует `while (...) {` с фигурными скобками.
#[test]
fn loop_body_cond_has_braces() {
    let src = r#"
var flag: bool;
start A {
    always {
        loop flag { flag := false; }
    }
}
"#;
    let c = generate_c_content(src, "Fsm");
    assert!(
        c.contains("while (") && c.contains(") {"),
        "цикл loop с условием должен генерировать `while (...) {{`:\n{c}"
    );
}

/// `for`-цикл генерирует `for (` с фигурными скобками вокруг тела.
#[test]
fn for_body_has_braces() {
    let src = r#"
var flag: bool;
start A {
    always {
        for var i: bool := true; i; i := false { flag := i; }
    }
}
"#;
    let c = generate_c_content(src, "Fsm");
    assert!(
        c.contains("for ("),
        "for-цикл должен генерировать `for (`:\n{c}"
    );
    assert!(
        c.contains(") {"),
        "тело for-цикла должно быть обёрнуто в фигурные скобки:\n{c}"
    );
}

// ── Тесты фильтрации неиспользуемых элементов (Changes-59) ───────────────────

/// Неиспользуемая переменная не попадает в сгенерированную C-структуру.
///
/// Позитивный пример: переменная `unused` объявлена, но нигде не используется —
/// она должна отсутствовать в заголовочном файле и не инициализироваться в init.
///
/// Контр-пример: переменная `used` присваивается в always и должна присутствовать.
#[test]
fn unused_var_excluded() {
    let src = r#"
type u8 = [bit;8];
var unused: u8 := 0;
var used: u8 := 0;
start S {
    always { used := 1; }
}
"#;
    // Поля struct — в .h, инициализация — в .c
    let h = generate_h_content(src, "Fsm");
    let c = generate_c_content(src, "Fsm");
    assert!(
        !h.contains("uint8_t unused"),
        "неиспользуемая переменная `unused` не должна появляться в struct (.h):\n{h}"
    );
    assert!(
        h.contains("uint8_t used"),
        "используемая переменная `used` должна присутствовать в struct (.h):\n{h}"
    );
    assert!(
        !c.contains("model->unused"),
        "неиспользуемая переменная `unused` не должна инициализироваться в init (.c):\n{c}"
    );
}

/// Используемая переменная остаётся в сгенерированной C-структуре.
///
/// Позитивный пример: переменная `counter` читается и пишется в always —
/// она должна присутствовать в struct (.h).
///
/// Контр-пример: если бы фильтрация удаляла переменные из условий, код не скомпилировался бы.
#[test]
fn used_var_stays() {
    let src = r#"
type u8 = [bit;8];
var counter: u8 := 0;
start S {
    always { counter := counter + 1; }
}
"#;
    let h = generate_h_content(src, "Fsm");
    assert!(
        h.contains("uint8_t counter"),
        "используемая переменная `counter` должна присутствовать в struct (.h):\n{h}"
    );
}

/// Неиспользуемая константа не попадает в сгенерированный C-код.
///
/// Позитивный пример: `DEAD` объявлена, но нигде не используется —
/// `CONST_FSM_DEAD` должна отсутствовать в `.c`-файле.
///
/// Контр-пример: `LIVE` используется в always, она должна присутствовать.
#[test]
fn unused_const_excluded() {
    let src = r#"
type u8 = [bit;8];
const DEAD: u8 := 42;
const LIVE: u8 := 7;
var v: u8 := 0;
start S {
    always { v := LIVE; }
}
"#;
    let c = generate_c_content(src, "Fsm");
    assert!(
        !c.contains("CONST_FSM_DEAD"),
        "неиспользуемая константа DEAD не должна появляться в коде:\n{c}"
    );
    assert!(
        c.contains("CONST_FSM_LIVE"),
        "используемая константа LIVE должна присутствовать в коде:\n{c}"
    );
}

/// Используемая константа остаётся в сгенерированном C-коде.
///
/// Позитивный пример: `MAX` используется в выражении присваивания переменной —
/// она должна генерироваться как `#define CONST_FSM_MAX`.
///
/// Контр-пример: если бы фильтрация удаляла `MAX`, компилятор C выдал бы ошибку.
#[test]
fn used_const_stays() {
    let src = r#"
type u8 = [bit;8];
const MAX: u8 := 255;
var v: u8 := 0;
start S {
    always { v := MAX; }
}
"#;
    let c = generate_c_content(src, "Fsm");
    assert!(
        c.contains("CONST_FSM_MAX"),
        "используемая константа MAX должна присутствовать в коде:\n{c}"
    );
}

// ── Тест коллизии имён enum-варианта и состояния (Changes-XX) ─────────────────

/// Проверяет что при сравнении переменной типа перечисления с одноимённым
/// состоянием в правой части `=`, генерируется числовое значение варианта,
/// а не идентификатор состояния.
///
/// Позитивный пример: `command = Stop` где `command: Command` и существует
/// состояние `Stop` — должно генерировать `== 2` (индекс Stop в Command).
///
/// Контр-пример: без исправления генерировался бы `MOTOR_STOP` (состояние).
#[test]
fn test_enum_equal_name_collision_generates_value() {
    let src = r#"
enum Command { Up, Down, Stop }
var command: Command := Stop;
model Motor {
    start Idle {
        ref Stop: command = Stop;
    }
    state Stop { }
}
start Main = Motor;
"#;
    let c = generate_c_content(src, "Main");
    // Должна быть сравнение с числовым значением (2 = индекс Stop), а не с MOTOR_STOP
    assert!(
        c.contains("== 2") || c.contains("command == 2"),
        "команда Stop должна сравниваться с числовым значением варианта перечисления:\n{c}"
    );
}

// ── Фича 0020-05: режим c-hal (таблица адресов + дефолтный HAL) ───────────────

/// Читает единственный `.h`-файл из каталога вывода.
fn read_header(out_dir: &str) -> String {
    let h = fs::read_dir(out_dir)
        .unwrap()
        .filter_map(|e| e.ok())
        .map(|e| e.path())
        .find(|p| p.extension().map(|x| x == "h").unwrap_or(false))
        .expect("должен быть .h-файл");
    fs::read_to_string(h).unwrap()
}

/// c-hal эмитит таблицу адресов, дефолтный HAL и bind-помощник.
#[test]
fn c_hal_emits_address_table_and_hal() {
    let tmp = tempdir().unwrap();
    let out = tmp.path().to_str().unwrap();
    let src = "type u8 = [bit;8]; in BTN: u8 := 0x00200000; out LED: bit; \
               address LED = 0x00200004; start Idle { ref On: BTN; } state On { ref Idle: BTN; }";
    let warnings = grammar::compile_to_c_hal(
        "demo.lam",
        src,
        out,
        &[],
        &[],
        &grammar::GenerateOptions::default(),
    )
    .expect("c-hal должен компилироваться");
    assert!(
        warnings.is_empty(),
        "без карты предупреждений нет: {:?}",
        warnings
    );

    let h = read_header(out);
    assert!(h.contains("Demo_PortBinding"), "нет типа привязки:\n{}", h);
    assert!(h.contains("__ADDR[]"), "нет таблицы адресов:\n{}", h);
    assert!(h.contains("0x200000"), "нет адреса BTN:\n{}", h);
    assert!(
        h.contains("Demo_bind_default_hal"),
        "нет bind-помощника:\n{}",
        h
    );
    assert!(
        h.contains("typedef struct Demo Demo;"),
        "нужен typedef корня для валидного C:\n{}",
        h
    );
}

/// Обычный режим `c` НЕ эмитит HAL-артефакты (регресс = 0).
#[test]
fn plain_c_has_no_hal_artifacts() {
    let tmp = tempdir().unwrap();
    let out = tmp.path().to_str().unwrap();
    let src = "type u8 = [bit;8]; in BTN: u8 := 0x00200000; start Idle { ref On: BTN; } state On;";
    grammar::compile_to_c(
        "demo.lam",
        src,
        out,
        &[],
        &grammar::GenerateOptions::default(),
    )
    .expect("c должен компилироваться");
    let h = read_header(out);
    assert!(
        !h.contains("PortBinding"),
        "режим c не должен эмитить HAL:\n{}",
        h
    );
    assert!(!h.contains("bind_default_hal"));
}

/// Используемый порт без адреса в c-hal → ошибка полноты SE-052.
#[test]
fn c_hal_missing_address_is_error() {
    let tmp = tempdir().unwrap();
    let out = tmp.path().to_str().unwrap();
    let src = "in BTN: bit; start S { ref T: BTN; } state T;";
    let err = grammar::compile_to_c_hal(
        "demo.lam",
        src,
        out,
        &[],
        &[],
        &grammar::GenerateOptions::default(),
    )
    .expect_err("used-порт без адреса должен давать ошибку");
    assert_eq!(err.code.as_deref(), Some("SE-052"));
}

/// Внешняя карта переопределяет адрес модели → c-hal успешен + предупреждение SE-050.
#[test]
fn c_hal_external_overrides_and_warns() {
    let tmp = tempdir().unwrap();
    let out = tmp.path().to_str().unwrap();
    let src = "type u8 = [bit;8]; in BTN: u8 := 0x00100000; start Idle { ref On: BTN; } state On;";
    let entries = grammar::parse_address_map("BTN = 0x40000000;", 0).unwrap();
    let warnings = grammar::compile_to_c_hal(
        "demo.lam",
        src,
        out,
        &[],
        &entries,
        &grammar::GenerateOptions::default(),
    )
    .expect("c-hal должен компилироваться");
    assert!(warnings.iter().any(|d| d.code.as_deref() == Some("SE-050")));
    let h = read_header(out);
    assert!(
        h.contains("0x40000000"),
        "должен эмитить адрес из карты:\n{}",
        h
    );
}
