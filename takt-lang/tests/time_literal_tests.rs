//! Литералы времени: лексика, разбор, печать форматтером (фича 0134, задача 0134-01).
//!
//! # Что здесь проверяется и почему именно так
//!
//! Стадия 0134-01 вводит **синтаксис** времени: литерал длительности (`3s`,
//! `500ms`, `1m30s`), литерал частоты (`1kHz`), объявление `clock`, выдержку
//! `after` и периодическое действие `every`. Семантика (тип `duration`,
//! профили, пересчёт) — подзадача 0134-02, поэтому построение дерева пока даёт
//! явный отказ `SE-066`; тесты это **фиксируют**, а не обходят: молчаливое
//! приведение наносекунд к обычному целому дало бы выдержку, не равную
//! заявленной.
//!
//! Три группы проверок:
//!
//! 1. **лексика** — какие формы принимаются и, что важнее, какие отвергаются
//!    (`1.5s`, `1e3ms`, `0xFFms`, `1s30m`) с нужным кодом, а не «нераспознанным
//!    токеном»;
//! 2. **аддитивность** — запись, которую занимает фича, до неё была ошибкой;
//! 3. **форматтер** — печатает **авторскую** форму (`1m30s`, а не `90s`):
//!    канонизация здесь молча переписала бы исходник пользователя.

use takt_lang::parser::lexer::Lexer;

/// Описания токенов и коды лексических ошибок строки.
///
/// Токены отдаются строками (`Debug`), а не `Token<'_>`: лексема заимствует и
/// входную строку, и буферы лексера, поэтому вернуть её наружу нельзя — тот же
/// приём применён в `lexer_tests.rs`.
fn lex(src: &str) -> (Vec<String>, Vec<String>) {
    let mut comments = Vec::new();
    let mut errors = Vec::new();
    let tokens: Vec<String> = Lexer::new(src, 0, &mut comments, &mut errors)
        .map(|(_, token, _)| format!("{token:?}"))
        .collect();
    let codes = errors.iter().map(|e| e.code().to_string()).collect();
    (tokens, codes)
}

/// Ожидаемое описание токена длительности.
fn duration(ns: i64, text: &str) -> String {
    format!("Duration({ns}, {text:?})")
}

/// Ожидаемое описание токена частоты.
fn frequency(hz: u64, text: &str) -> String {
    format!("Frequency({hz}, {text:?})")
}

/// Единственная лексическая ошибка строки (с её кодом).
fn lex_error(src: &str) -> String {
    let (_, codes) = lex(src);
    assert_eq!(codes.len(), 1, "ожидалась ровно одна ошибка: {src}");
    codes[0].clone()
}

// ── 1. Лексика: принимаемые формы ────────────────────────────────────────────

#[test]
fn simple_durations_are_canonical_nanoseconds() {
    assert_eq!(lex("3s").0, vec![duration(3_000_000_000, "3s")]);
    assert_eq!(lex("500ms").0, vec![duration(500_000_000, "500ms")]);
    assert_eq!(lex("250us").0, vec![duration(250_000, "250us")]);
    assert_eq!(lex("40ns").0, vec![duration(40, "40ns")]);
    assert_eq!(lex("2h").0, vec![duration(7_200_000_000_000, "2h")]);
}

#[test]
fn composite_durations_sum_their_terms() {
    // 1m30s = 90 с.
    assert_eq!(lex("1m30s").0, vec![duration(90_000_000_000, "1m30s")]);
    // 2h30m = 9000 с.
    assert_eq!(lex("2h30m").0, vec![duration(9_000_000_000_000, "2h30m")]);
}

#[test]
fn frequency_literals_are_hertz() {
    assert_eq!(lex("1kHz").0, vec![frequency(1_000, "1kHz")]);
    assert_eq!(lex("8MHz").0, vec![frequency(8_000_000, "8MHz")]);
    assert_eq!(lex("50Hz").0, vec![frequency(50, "50Hz")]);
}

#[test]
fn time_and_frequency_units_do_not_collide() {
    // `h` — час, `Hz` — герц: хвост сопоставляется целиком.
    assert_eq!(lex("3h").0, vec![duration(10_800_000_000_000, "3h")]);
    assert_eq!(lex("3Hz").0, vec![frequency(3, "3Hz")]);
    // `m` — минута, `MHz` — мегагерц.
    assert_eq!(lex("3m").0, vec![duration(180_000_000_000, "3m")]);
    assert_eq!(lex("3MHz").0, vec![frequency(3_000_000, "3MHz")]);
}

// ── 2. Лексика: отвергаемые формы ────────────────────────────────────────────

#[test]
fn fractional_and_exponent_and_hex_forms_are_rejected() {
    // Дробная длительность выражается меньшей единицей — `1500ms`.
    assert_eq!(lex_error("1.5s"), "LE-011");
    assert_eq!(lex_error("1e3ms"), "LE-011");
    assert_eq!(lex_error("0xFFms"), "LE-011");
}

#[test]
fn composite_order_is_enforced() {
    // `1s30m` — не «полторы минуты»: порядок единиц значим.
    assert_eq!(lex_error("1s30m"), "LE-011");
    // Повтор единицы.
    assert_eq!(lex_error("1m30m"), "LE-011");
    // Незавершённая форма: за цифрами нет единицы.
    assert_eq!(lex_error("1m30"), "LE-011");
}

#[test]
fn out_of_range_duration_is_reported_not_wrapped() {
    // Обёртка здесь дала бы другую выдержку — молча.
    assert_eq!(lex_error("9223372036854775807h"), "LE-010");
}

#[test]
fn separated_form_and_foreign_identifiers_are_left_alone() {
    // `3 s` — не длительность (единица обязана примыкать).
    assert_eq!(
        lex("3 s").0,
        vec!["Number(3)".to_string(), "Identifier(\"s\")".to_string()]
    );
    // `3msg` — число и имя: хвост сопоставляется целиком, имя не «объедается».
    assert_eq!(
        lex("3msg").0,
        vec!["Number(3)".to_string(), "Identifier(\"msg\")".to_string()]
    );
    assert!(lex("3msg").1.is_empty(), "ошибок быть не должно");
}

// ── 3. Аддитивность и разбор ─────────────────────────────────────────────────

/// Модель, использующая все четыре новые конструкции.
const TIME_SRC: &str = r#"model Doors {
    clock 1kHz;
    const DWELL := 3s;
    var left: duration := 0s;
    start Open {
        every 100ms {
            left := DWELL;
        }
        ref Closing: after 1m30s;
    }
    state Closing { }
}
"#;

#[test]
fn all_new_constructs_parse() {
    match takt_lang::parse(TIME_SRC, 0) {
        Ok((_ast, _comments)) => {}
        Err(errors) => panic!("разбор не должен давать ошибок: {errors:?}"),
    }
}

#[test]
fn semantics_refuses_loudly_until_next_subtask() {
    // Подзадача 0134-02 заменит этот отказ настоящей семантикой. До неё
    // конструкция обязана давать ЯВНУЮ диагностику: молчаливое приведение
    // наносекунд к целому дало бы выдержку, не равную заявленной.
    let diagnostics = takt_lang::collect_compile_diagnostics("doors.takt", TIME_SRC, &[]);
    assert!(
        diagnostics
            .iter()
            .any(|d| d.code.as_deref() == Some("SE-066")),
        "ожидалась диагностика SE-066: {diagnostics:?}"
    );
}

#[test]
fn syntax_was_an_error_before_the_feature() {
    // Аддитивность (правило 11): фича занимает запись, которая до неё
    // отвергалась. Проверяем это со стороны языка — числом с именем.
    let (tokens, errors) = lex("3 s");
    assert!(errors.is_empty());
    assert_eq!(tokens.len(), 2, "до фичи это два токена, и так и осталось");
}

// ── 4. Форматтер печатает авторскую форму ────────────────────────────────────

#[test]
fn formatter_keeps_the_authors_spelling() {
    let formatted = takt_lang::format::format_source(TIME_SRC).expect("форматтер обязан печатать");
    // Канонизация `1m30s` → `90s` молча переписала бы исходник пользователя.
    assert!(formatted.contains("after 1m30s"), "{formatted}");
    assert!(formatted.contains("clock 1kHz;"), "{formatted}");
    assert!(formatted.contains("every 100ms {"), "{formatted}");
    assert!(formatted.contains("3s"), "{formatted}");
    assert!(formatted.contains("duration"), "{formatted}");
}

#[test]
fn formatter_is_idempotent_on_time_constructs() {
    let once = takt_lang::format::format_source(TIME_SRC).expect("первый проход");
    let twice = takt_lang::format::format_source(&once).expect("второй проход");
    assert_eq!(once, twice, "fmt(fmt(x)) обязан равняться fmt(x)");
}

// ── 5. Тип `duration` и профили (задача 0134-02) ─────────────────────────────

/// Диагностики компиляции исходника (коды).
fn codes(src: &str) -> Vec<String> {
    takt_lang::collect_compile_diagnostics("probe.takt", src, &[])
        .iter()
        .filter_map(|d| d.code.clone())
        .collect()
}

/// Модель с длительностью в объявлениях (без `after`/`every`).
fn model_with(body: &str) -> String {
    // `start Main = Probe;` обязателен: без него корневая модель файла не имеет
    // стартового состояния и разбор падает раньше проверяемого (SE-011).
    format!(
        "model Probe {{\n    out ready: bit := 0;\n{body}\n    start Idle {{ }}\n}}\n\nstart Main = Probe;\n"
    )
}

#[test]
fn duration_literal_and_type_are_accepted_by_semantics() {
    // Тип `duration` связан по имени, литерал понижается в наносекунды —
    // SE-066 (отказ стадии 0134-01) больше не возникает.
    let src = model_with("    const DWELL := 3s;\n    var left: duration := 0s;");
    assert!(
        !codes(&src).iter().any(|c| c == "SE-066"),
        "семантика обязана принять литерал и тип: {:?}",
        codes(&src)
    );
}

#[test]
fn mixing_duration_with_number_is_se065() {
    // Правило 5 ADR: длительность сочетается только с длительностью.
    let src = model_with(
        "    var left: duration := 0s;\n    var n: u8 := 0;\n    always { left := left + n; }",
    );
    assert!(
        codes(&src).iter().any(|c| c == "SE-065"),
        "ожидалась SE-065: {:?}",
        codes(&src)
    );
}

#[test]
fn duration_plus_duration_is_allowed() {
    let src = model_with(
        "    var a: duration := 1s;\n    var b: duration := 2s;\n    always { a := a + b; }",
    );
    let found = codes(&src);
    assert!(
        !found.iter().any(|c| c == "SE-065"),
        "сложение длительностей запрещать нельзя: {found:?}"
    );
}

#[test]
fn conflicting_clock_declarations_are_se067() {
    // Две разные частоты в одной модели — ошибка автора, а не «победит последняя».
    let src = "model Probe {\n    clock 1kHz;\n    clock 8MHz;\n    out ready: bit := 0;\n    start Idle { }\n}\n\nstart Main = Probe;\n";
    assert!(
        codes(src).iter().any(|c| c == "SE-067"),
        "ожидалась SE-067: {:?}",
        codes(src)
    );
    // Повтор одной и той же частоты безвреден.
    let same = "model Probe {\n    clock 1kHz;\n    clock 1kHz;\n    out ready: bit := 0;\n    start Idle { }\n}\n\nstart Main = Probe;\n";
    assert!(
        !codes(same).iter().any(|c| c == "SE-067"),
        "повтор одной частоты ошибкой не является: {:?}",
        codes(same)
    );
}

#[test]
fn targets_refuse_time_loudly_until_their_own_subtasks() {
    // Эмиссия времени — задачи 0134-04…07. До них цель обязана ОТКАЗАТЬ, а не
    // напечатать наносекунды обычным целым: молча неверный код компилируется.
    // ⚠️ Переменную нужно ИСПОЛЬЗОВАТЬ: неиспользуемая отфильтровывается из
    // структуры цели `c` (известная ловушка, разобранная фичей 0029), и тогда
    // отображение типа не вызывается вовсе — тест зеленел бы впустую.
    let src = model_with("    var left: duration := 0s;\n    always { left := 5s; }");
    let mut out = std::env::temp_dir();
    out.push("takt_0134_02_probe.c");
    let result = takt_lang::compile_to_c(
        "probe.takt",
        &src,
        out.to_str().expect("путь"),
        &[],
        &takt_lang::generator::GenerateOptions::new(false),
    );
    let diagnostic = result.expect_err("цель 'c' обязана отказать");
    assert_eq!(diagnostic.code.as_deref(), Some("CC-020"), "{diagnostic:?}");
}
