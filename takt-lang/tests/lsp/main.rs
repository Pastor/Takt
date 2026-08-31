//! Языковой сервер и редакторский слой — ОДНА тестовая цель на все наборы темы (фича 0244, задача 0244-02).
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
//! `cargo test --test lsp <набор>::`.
//!
//! ⚠️ Набор, строящий временный каталог по имени потока (инвариант фичи 0190),
//! обязан вычищать из него `:` — после слияния имя потока несёт префикс модуля.

mod import_binding_kind_tests;
mod imported_symbol_kind_tests;
mod lsp_definition_tests;
mod lsp_document_symbol_tests;
mod lsp_formatting_conformance_tests;
mod lsp_goto_tests;
mod lsp_init_options_tests;
mod lsp_matrix_tests;
mod lsp_port_io_tests;
mod lsp_references_tests;
mod lsp_rename_tests;
mod lsp_tests;
mod lsp_workspace_tests;
mod semantic_tokens_tests;
mod style_naming_lsp_tests;
mod type_highlighting_tests;
