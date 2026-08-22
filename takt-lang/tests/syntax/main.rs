//! Лексика, разбор, форматтер и канон стиля — ОДНА тестовая цель на все наборы темы (фича 0244, задача 0244-02).
//!
//! # Зачем агрегатор
//!
//! Замер ADR 0244: первый запуск свежесобранного тестового бинарника стоит
//! 0.60 с при загрузке ЦП 2 % (проверка кода ядром, чтение с диска, работа
//! динамического загрузчика), повторный — 0.033 с. Бинарников было 147, и в
//! гейте каждый запускается впервые: ≈ 88 с из 152.9 с стадии `cargo test`
//! уходило только на это.
//!
//! ⚠️ **Файлы не слиты** — каждый набор остаётся своим файлом и своим модулем:
//! правило размера модуля (`docs/CODE.md`) не задевается, имя теста получает
//! префикс модуля и остаётся различимым:
//! `cargo test --test syntax <набор>::`.
//!
//! ⚠️ Набор, строящий временный каталог по имени потока (инвариант фичи 0190),
//! обязан вычищать из него `:` — после слияния имя потока несёт префикс модуля.

mod ast_tests;
mod contextual_from_tests;
mod dead_lexeme_tests;
mod feature_0021_operators;
mod fmt_diagnostic_tests;
mod format_comment_binding_tests;
mod format_comment_position_tests;
mod format_style_canon_tests;
mod format_tests;
mod format_unsupported_tests;
mod language_coverage_tests;
mod lexer_tests;
mod parse_depth_tests;
mod parser_tests;
mod style_naming_fmt_tests;
mod time_literal_tests;
