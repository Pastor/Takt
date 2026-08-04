//! Проводка `CS-001` в LSP (фича 0226, задача 0226-03).
//!
//! Заказчик назвал **двух** потребителей канона: `taktc fmt` и редактор. Первого
//! сторожит `style_naming_fmt_tests.rs`, второго — этот файл.
//!
//! # Что здесь ловится
//!
//! 1. **Критерий приёмки 3 ADR 0226:** редактор отдаёт то же предупреждение,
//!    уровень — `WARNING` (не ошибка: имя — совет), с кодом в поле протокола и
//!    диапазоном на самом имени.
//! 2. **«Одна реализация на двоих».** Текст предупреждения у `fmt` и у LSP
//!    сверяется **буквально**: две копии правил — класс, который проект закрывал
//!    в 0084, 0193 и 0195, и здесь он был бы особенно тих (кто сверяет вывод
//!    форматтера с подсказкой редактора?).
//! 3. **Асимметрия доставки, зафиксированная как есть.** На файле с
//!    семантическими ошибками `fmt` предупреждение печатает, а редактор —
//!    **нет**: политика `collect_diagnostics_at` «сперва ошибки» старше фичи
//!    0226. Тест это пришпиливает, чтобы поведение было решением, а не
//!    случайностью; расхождение записано кандидатом в `FEATURES.md`.

#![cfg(feature = "lsp")]

use lsp_types::{DiagnosticSeverity, NumberOrString};
use takt_lang::lsp::collect_diagnostics;

/// Каноничный по формату файл с неканоничным именем порта (`BadPort` на строке
/// 2, колонка 9 — та же фикстура, что у тестов `fmt`).
const BAD_NAME: &str = "\
model M {
    out BadPort: bit;
    start S {
        always {
            BadPort := 1;
        }
    }
}
start Main = M;
";

/// То же, но имя каноничное — контр-пример.
const GOOD_NAME: &str = "\
model M {
    out good_port: bit;
    start S {
        always {
            good_port := 1;
        }
    }
}
start Main = M;
";

/// Семантически некорректный файл (`SE-034`: тип не найден) с неканоничным
/// именем переменной.
const SEMANTICALLY_BROKEN: &str = "\
model M {
    var BadVar: NoSuchType := 1;
    start S {
        always {
            BadVar := 1;
        }
    }
}
start Main = M;
";

/// Только диагностики `CS-001`.
fn style_diagnostics(source: &str) -> Vec<lsp_types::Diagnostic> {
    collect_diagnostics(source)
        .into_iter()
        .filter(|d| d.code == Some(NumberOrString::String("CS-001".to_string())))
        .collect()
}

/// **Критерий 3.** Редактор отдаёт `CS-001` предупреждением, с кодом и
/// источником.
///
/// ⚠️ Уровень проверяется явно: ошибкой канон именования быть не должен — иначе
/// редактор объявил бы отказом то, что инструмент считает советом (и что не
/// меняет код возврата `fmt --check`).
#[test]
fn lsp_reports_cs001_as_warning_with_code() {
    let diags = style_diagnostics(BAD_NAME);
    assert_eq!(diags.len(), 1, "ожидалась одна диагностика: {diags:?}");
    let d = &diags[0];

    assert_eq!(
        d.severity,
        Some(DiagnosticSeverity::WARNING),
        "канон именования — предупреждение, а не ошибка: {d:?}"
    );
    assert_eq!(
        d.source.as_deref(),
        Some("takt-lsp"),
        "источник диагностики: {d:?}"
    );
    assert!(
        d.message.contains("порт 'BadPort'") && d.message.contains("snake_case"),
        "текст обязан называть вид объявления и ожидаемую форму: {:?}",
        d.message
    );
}

/// Диапазон лежит **на имени**, а не в начале файла и не на всём объявлении.
///
/// По нему редактор подчёркивает слово; нулевой диапазон (`0,0`–`0,0`), которым
/// `grammar_diagnostic_to_lsp` спасает диагностики без координат, здесь означал
/// бы подчёркивание не там.
#[test]
fn lsp_range_covers_the_name_itself() {
    let diags = style_diagnostics(BAD_NAME);
    let range = diags[0].range;

    // `    out BadPort: bit;` — вторая строка (индекс 1), имя с 9-й колонки
    // (индекс 8), длина 7.
    assert_eq!(range.start.line, 1, "строка имени: {range:?}");
    assert_eq!(range.start.character, 8, "колонка имени: {range:?}");
    assert_eq!(range.end.line, 1, "имя не переносится: {range:?}");
    assert_eq!(
        range.end.character - range.start.character,
        "BadPort".len() as u32,
        "диапазон обязан покрывать имя целиком: {range:?}"
    );
}

/// **Одна реализация на двоих.** Текст предупреждения у LSP и у `fmt`
/// совпадает буквально.
///
/// Сравнивается не «оба сработали», а сама строка: разъехавшиеся формулировки
/// означали бы две копии правил — то, чего фича избегала выбором Option A.
#[test]
fn lsp_message_is_identical_to_fmt_message() {
    let (_, fmt_warnings) =
        takt_lang::format::format_source_with_warnings(BAD_NAME).expect("фикстура форматируется");
    assert_eq!(fmt_warnings.len(), 1, "у fmt тоже одно: {fmt_warnings:?}");

    let lsp = style_diagnostics(BAD_NAME);
    assert_eq!(
        lsp[0].message, fmt_warnings[0].message,
        "текст предупреждения обязан быть один на обоих потребителей"
    );
    assert_eq!(
        fmt_warnings[0].code.as_deref(),
        Some("CS-001"),
        "код у fmt: {:?}",
        fmt_warnings[0]
    );
}

/// **Контр-пример.** Каноничное имя не даёт ни ошибок, ни предупреждений о
/// стиле.
#[test]
fn lsp_is_silent_on_canonical_name() {
    assert!(
        style_diagnostics(GOOD_NAME).is_empty(),
        "каноничное имя не должно давать CS-001"
    );
    let errors: Vec<_> = collect_diagnostics(GOOD_NAME)
        .into_iter()
        .filter(|d| d.severity == Some(DiagnosticSeverity::ERROR))
        .collect();
    assert!(
        errors.is_empty(),
        "фикстура обязана быть корректной: {errors:?}"
    );
}

/// **Асимметрия доставки — пришпилена как есть.**
///
/// На файле с семантической ошибкой редактор `CS-001` **не показывает**:
/// `collect_diagnostics_at` возвращается сразу после ошибок («сперва ошибки» —
/// политика старше фичи 0226), а проверка стиля стоит за этим возвратом. `fmt`
/// на том же файле предупреждение печатает — он семантику не запускает вовсе
/// (сторож: `style_naming_fmt_tests::semantically_broken_file_is_formatted_and_warned`).
///
/// ⚠️ То есть автор, правящий имя в файле, который ещё не собирается, видит
/// замечание в консоли и не видит в редакторе — при том, что проверке
/// семантика не нужна. Расхождение записано кандидатом в `FEATURES.md`; тест
/// фиксирует **сегодняшнее** поведение, чтобы его изменение было заметным.
#[test]
fn lsp_hides_style_warning_while_errors_present() {
    let all = collect_diagnostics(SEMANTICALLY_BROKEN);
    let errors: Vec<_> = all
        .iter()
        .filter(|d| d.severity == Some(DiagnosticSeverity::ERROR))
        .collect();
    assert!(
        !errors.is_empty(),
        "фикстура обязана быть семантически некорректной: {all:?}"
    );
    assert!(
        style_diagnostics(SEMANTICALLY_BROKEN).is_empty(),
        "сегодня редактор при ошибках предупреждения не показывает: {all:?}"
    );

    // А `fmt` на том же входе — показывает: асимметрия наблюдаема здесь же.
    let (_, fmt_warnings) = takt_lang::format::format_source_with_warnings(SEMANTICALLY_BROKEN)
        .expect("форматтер семантику не запускает");
    assert_eq!(
        fmt_warnings.len(),
        1,
        "у fmt предупреждение обязано быть: {fmt_warnings:?}"
    );
}
