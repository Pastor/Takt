//! Цели генерации: c, c-hal, st, rust, sv, sv-mmio, адреса — ОДНА тестовая цель на все наборы темы (фича 0244, задача 0244-02).
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
//! `cargo test --test targets <набор>::`.
//!
//! ⚠️ Набор, строящий временный каталог по имени потока (инвариант фичи 0190),
//! обязан вычищать из него `:` — после слияния имя потока несёт префикс модуля.

mod address_export_tests;
mod address_map_tests;
mod anon_port_tests;
mod bit_write_targets_tests;
mod c_diagnostic_code_tests;
mod c_enum_constants_tests;
mod c_redundant_break_tests;
mod c_state_ref_tests;
mod c_stub_tests;
mod cli_version_tests;
mod cli_warning_position_tests;
mod cli_warnings_tests;
mod codegen_tests;
mod duration_targets_tests;
mod generator_warnings_tests;
mod guard_targets_tests;
mod hal_bit_range_tests;
mod port_initial_value_hdl_tests;
mod rust_default_impl_tests;
mod rust_enum_compare_tests;
mod rust_index_cast_tests;
mod rust_printers_tests;
mod st_tests;
mod struct_codegen_tests;
mod sv_apb_adapter_tests;
mod sv_mmio_tests;
mod sv_mmio_write_signals_tests;
mod sv_tick_read_tests;
mod wide_bit_vector_tests;
mod wide_literal_tests;
