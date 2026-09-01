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
mod address_map_roundtrip_tests;
mod address_map_tests;
mod aggregate_assign_targets_tests;
mod anon_port_tests;
mod array_slice_tests;
mod array_struct_init_tests;
mod bit_value_targets_tests;
mod bit_write_targets_tests;
mod build_matrix_tests;
mod build_probes;
mod c_builtin_dropped_tests;
mod c_default_init_tests;
mod c_diagnostic_code_tests;
mod c_enum_constants_tests;
mod c_param_on_demand_tests;
mod c_redundant_break_tests;
mod c_refusal_position_tests;
mod c_shift_width_tests;
mod c_state_ref_tests;
mod c_stub_tests;
mod call_return_coercion_tests;
mod cli_report_result_tests;
mod cli_version_tests;
mod cli_warning_position_tests;
mod cli_warnings_tests;
mod codegen_tests;
mod const_aggregate_tests;
mod duration_field_tests;
mod duration_targets_tests;
mod enum_in_function_tests;
mod format_matrix_tests;
mod formula_in_body_tests;
mod fsm_table_sv_tests;
mod fsm_table_targets_tests;
mod fsm_table_tests;
mod function_inline_tests;
mod generator_warnings_tests;
mod guard_targets_tests;
mod hal_bit_range_tests;
mod local_aggregate_tests;
mod ltl_site_matrix_tests;
mod matrix_corpus_export_tests;
mod matrix_probes;
mod mixed_arith_tests;
mod mixed_sign_tests;
mod nested_struct_targets_tests;
mod operand_type_carrier_tests;
mod port_composite_tests;
mod port_initial_value_hdl_tests;
mod root_pointer_implementations_tests;
mod root_pointer_matrix_tests;
mod rust_default_impl_tests;
mod rust_default_value_tests;
mod rust_deferred_init_tests;
mod rust_enum_compare_tests;
mod rust_index_cast_tests;
mod rust_live_tests;
mod rust_non_ascii_name_tests;
mod rust_power_operands_tests;
mod rust_printers_tests;
mod rust_shift_width_tests;
mod same_type_cast_tests;
mod slice_argument_tests;
mod st_call_order_tests;
mod st_global_name_clash_tests;
mod st_helper_order_tests;
mod st_local_array_argument_tests;
mod st_local_name_clash_tests;
mod st_reserved_names_tests;
mod st_tests;
mod st_type_clash_tests;
mod st_unused_function_tests;
mod statement_site_tests;
mod struct_codegen_tests;
mod sv_apb_adapter_tests;
mod sv_loop_variable_tests;
mod sv_mmio_tests;
mod sv_mmio_write_signals_tests;
mod sv_refusal_text_tests;
mod sv_terminal_branch_tests;
mod sv_tick_read_tests;
mod target_matrix_tests;
mod unused_local_tests;
mod unused_param_targets_tests;
mod verification_matrix_tests;
mod wide_bit_vector_tests;
mod wide_literal_tests;
