//! Позиции в диагностиках симулятора (фича 0054).
//!
//! Тесты сквозные — гоняют сам бинарник: печать живёт в `bin/simulation.rs`, и
//! проверять надо ровно то, что увидит пользователь. Прежде симулятор печатал
//! только текст сообщения — терялись и позиция, и код (`SE-002`), из-за чего
//! ошибка в своём файле была неотличима от ошибки внутри импортированной
//! библиотеки.

use std::process::Command;

const DIR: &str = "tests/data/diag54";

/// Запускает симулятор на фикстуре и возвращает stderr (фикстуры ошибочны).
fn stderr_of(fixture: &str) -> String {
    let out = Command::new(env!("CARGO_BIN_EXE_simulation"))
        .args([&format!("{DIR}/{fixture}"), "-I", DIR, "--steps", "3"])
        .output()
        .expect("запуск симулятора");
    assert!(!out.status.success(), "{fixture}: фикстура обязана падать");
    String::from_utf8_lossy(&out.stderr).into_owned()
}

/// A1: позиция и код печатаются.
#[test]
fn semantic_error_prints_position_and_code() {
    let err = stderr_of("lib_bad.lam");
    assert!(
        err.contains("lib_bad.lam:2:18:"),
        "ожидались путь:строка:колонка, получено: {err}"
    );
    assert!(err.contains("[SE-002]"), "код диагностики потерян: {err}");
}

/// A2: ошибка ВНУТРИ импорта названа библиотекой, а не импортёром.
///
/// Это и есть суть фичи: прежде оба случая давали дословно одинаковый вывод.
#[test]
fn error_inside_import_names_the_library() {
    let err = stderr_of("importer.lam");
    assert!(
        err.contains("lib_bad.lam:"),
        "виновник — библиотека, а не импортёр: {err}"
    );
    assert!(
        !err.contains("importer.lam:"),
        "импортёр ошибок не содержит и назван быть не должен: {err}"
    );
}

/// A3: ошибки разбора печатаются ВСЕ, каждая со своей позицией.
///
/// Симулятор показывает их все (в отличие от `lamc`, показывающего первую) —
/// поведение сохранено осознанно: каждая ошибка своя подсказка.
#[test]
fn all_parse_errors_are_printed_with_positions() {
    let err = stderr_of("syntax_bad.lam");
    let positioned = err
        .lines()
        .filter(|l| l.contains("syntax_bad.lam:"))
        .count();
    assert!(
        positioned >= 2,
        "ожидалось несколько ошибок разбора, каждая с позицией; получено {positioned}: {err}"
    );
}

/// A4: формат позиции совпадает с `lamc` — потому что функция одна и та же
/// (`grammar::diagnostics::position_prefix`).
///
/// Сторож против расхождения печати: копия формата в каждом бинарнике разошлась
/// бы (доказанный класс дефекта — задача 0028-01).
#[test]
fn position_format_matches_the_shared_layer() {
    let err = stderr_of("lib_bad.lam");
    let source = std::fs::read_to_string(format!("{DIR}/lib_bad.lam")).expect("чтение");
    let offset = source.find("Nowhere").expect("ссылка есть");
    let (line, column) = grammar::diagnostics::line_column(&source, offset);
    assert!(
        err.contains(&format!("lib_bad.lam:{line}:{column}:")),
        "позиция обязана совпадать с расчётом общего слоя ({line}:{column}): {err}"
    );
}
