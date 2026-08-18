//! Текст `SIM-017` называет то, за что он остался отвечать (фича 0249).
//!
//! ## Зачем сторож
//!
//! Прежний текст — «присваивание не в переменную, поле или элемент массива
//! пока не поддерживается симулятором» — перечислял **три** законных места и
//! умалчивал о бите, срезе, порте и ячейке `#АДРЕС`. Автор, написавший
//! `led := 1;` или `#0x200.5 := 1;`, читал, что симулятор этого «не умеет», —
//! хотя умеет оба. То есть отказ называл не ту причину: класс, правленный в
//! проекте трижды (0202, 0229, 0231) и каждый раз найденный **глазами**.
//!
//! После фичи 0249 форму левой части судит семантика (`SE-111`, `SE-112`), и
//! за `SIM-017` остаются ровно **две** записи — бита и среза. Текст обязан
//! называть обе; проверяется он **прогоном**, а не чтением кода.
//!
//! ⚠️ Проверять надо именно **текст**: и запись бита, и запись среза
//! останавливают прогон одинаково, поэтому «отказ случился» ничего не говорит
//! о том, верную ли причину прочёл автор.

use std::path::PathBuf;
use std::process::Command;

/// Уникальный по тесту каталог (фича 0190: тесты идут параллельно; фича 0244
/// добавила `::` в имя потока).
fn work_dir(tag: &str) -> PathBuf {
    let thread = std::thread::current()
        .name()
        .unwrap_or("main")
        .replace(':', "_");
    let dir = std::env::temp_dir().join(format!("takt_sim_0249_{tag}_{thread}"));
    let _ = std::fs::remove_dir_all(&dir);
    std::fs::create_dir_all(&dir).expect("каталог теста");
    dir
}

/// Гоняет модель эталоном, возвращая (код возврата, stderr + stdout).
fn run(tag: &str, source: &str) -> (Option<i32>, String) {
    let dir = work_dir(tag);
    let path = dir.join("probe.takt");
    std::fs::write(&path, source).expect("запись пробы");
    let result = Command::new(env!("CARGO_BIN_EXE_takt-sim"))
        .arg(&path)
        .arg("-n")
        .arg("3")
        .output()
        .expect("запуск takt-sim");
    let text = format!(
        "{}{}",
        String::from_utf8_lossy(&result.stderr),
        String::from_utf8_lossy(&result.stdout)
    );
    (result.status.code(), text)
}

/// Обе оставшиеся формы обязаны быть названы в тексте отказа.
const MUST_NAME: &[&str] = &["x.N", "x[a:b]"];

/// **T10: запись бита — `SIM-017`, и текст называет обе оставшиеся формы.**
#[test]
fn bit_write_refusal_names_both_remaining_forms() {
    let (_, text) = run(
        "bit",
        "var b: u8 := 0;\nstart Run { always { b.2 := 1; } }\n",
    );
    assert!(text.contains("SIM-017"), "ожидался SIM-017: {text}");
    let missing: Vec<&str> = MUST_NAME
        .iter()
        .filter(|form| !text.contains(**form))
        .copied()
        .collect();
    assert!(
        missing.is_empty(),
        "текст отказа не называет эти формы: {missing:?}; текст: {text}"
    );
}

/// **T10б: запись среза — тот же отказ и тот же текст.**
#[test]
fn slice_write_refusal_names_both_remaining_forms() {
    let (_, text) = run(
        "slice",
        "var arr: [u8; 4] := { 0, 0, 0, 0 };\nstart Run { always { arr[0:2] := 1; } }\n",
    );
    assert!(text.contains("SIM-017"), "ожидался SIM-017: {text}");
    let missing: Vec<&str> = MUST_NAME
        .iter()
        .filter(|form| !text.contains(**form))
        .copied()
        .collect();
    assert!(
        missing.is_empty(),
        "текст отказа не называет эти формы: {missing:?}; текст: {text}"
    );
}

/// **Текст не обещает отказа там, где эталон умеет.**
///
/// Прежняя формулировка перечисляла три места и тем утверждала, что порт и
/// ячейка `#АДРЕС` не поддержаны. Сторож ловит возврат к такому обещанию:
/// прогон, где записаны порт, поле, элемент и переменная, обязан пройти.
#[test]
fn places_the_reference_does_execute_are_not_refused() {
    let (code, text) = run(
        "ok",
        "struct Pt { x: u8, y: u8 }\n\
         var n: u8 := 0;\n\
         var p: Pt := { 1, 2 };\n\
         var arr: [u8; 4] := { 0, 0, 0, 0 };\n\
         out led: bit at 0x100:3;\n\
         start Run { always { n := 1; p.x := 2; arr[1] := 3; led := 1; } }\n",
    );
    assert!(
        !text.contains("SIM-017"),
        "эти места эталон исполняет: {text}"
    );
    assert_eq!(code, Some(0), "прогон обязан завершиться успехом: {text}");
}
