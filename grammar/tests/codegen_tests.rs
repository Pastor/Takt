//! Интеграционные тесты кодогенерации (FE5).
//!
//! Проверяют конвейер компиляции BuT → C через [`grammar::compile_to_c`].

use tempfile::tempdir;

/// FE5: Простая FSM компилируется в C без ошибок.
#[test]
fn test_compile_simple_fsm_to_c() {
    use grammar::{parse, semantic::tree::construct_model, generator::{Language, generate}};

    let src = r#"
model Traffic {
    start Red {}
    state Green {}
    state Yellow {}
}
    "#;

    let tmp = tempdir().expect("не удалось создать временный каталог");
    let out_path = tmp.path().to_str().unwrap();

    // Разбираем и строим семантическое дерево
    let (ast, _) = parse(src, 0).expect("синтаксический анализ должен быть успешен");
    let root = construct_model(&ast, None, &[]).expect("семантический анализ должен быть успешен");

    // Генерируем C-код для именованной модели Traffic
    let traffic = root.borrow().search_model("Traffic")
        .expect("модель Traffic должна быть найдена");
    let result = generate(Language::C, &traffic.borrow(), out_path);
    assert!(
        result.is_ok(),
        "компиляция простой FSM должна быть успешной, ошибка: {:?}",
        result
    );

    // Проверяем что файл был создан
    let entries: Vec<_> = std::fs::read_dir(out_path)
        .unwrap()
        .filter_map(|e| e.ok())
        .collect();
    assert!(
        !entries.is_empty(),
        "генератор C должен создать хотя бы один файл"
    );
}

/// FE5: Синтаксически неверный код возвращает ошибку.
#[test]
fn test_compile_invalid_syntax_returns_error() {
    let src = "model { }";  // нет имени модели
    let tmp = tempdir().unwrap();
    let result = grammar::compile_to_c(src, tmp.path().to_str().unwrap());
    assert!(
        result.is_err(),
        "неверный код должен возвращать Err"
    );
}

/// FE5: Семантически неверный код (нет start-состояния) возвращает ошибку.
#[test]
fn test_compile_no_start_state_returns_error() {
    let src = "model M { state S {} }";  // нет start-состояния
    let tmp = tempdir().unwrap();
    let result = grammar::compile_to_c(src, tmp.path().to_str().unwrap());
    assert!(
        result.is_err(),
        "модель без start-состояния должна возвращать Err"
    );
}
