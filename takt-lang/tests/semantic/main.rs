//! Семантика: дерево, типы, диагностики, параметры, порты — ОДНА тестовая цель на все наборы темы (фича 0244, задача 0244-02).
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
//! `cargo test --test semantic <набор>::`.
//!
//! ⚠️ Набор, строящий временный каталог по имени потока (инвариант фичи 0190),
//! обязан вычищать из него `:` — после слияния имя потока несёт префикс модуля.

mod after_const_duration_tests;
mod array_shared_and_index_tests;
mod assignment_place_tests;
mod bare_state_condition_tests;
mod const_eval_tests;
mod const_init_fold_tests;
mod const_int_ops_shared_tests;
mod deep_model_tests;
mod deep_nesting_tests;
mod diagnostic_text_tests;
mod diagnostics_batch_tests;
mod diagnostics_file_tests;
mod diagnostics_tests;
mod empty_enum_tests;
mod empty_struct_tests;
mod fixed_saturation_tests;
mod formula_validation_tests;
mod implemented_model_tests;
mod implicit_bool_delivery_tests;
mod import_adopt_tests;
mod import_enum_match_tests;
mod import_function_tests;
mod import_type_definition_tests;
mod init_forward_reference_tests;
mod init_port_read_tests;
mod library_entry_tests;
mod literal_range_tests;
mod model_always_tests;
mod model_implement_form_tests;
mod model_parameter_apply_tests;
mod model_parameter_args_tests;
mod model_parameter_const_tests;
mod model_parameter_modes_tests;
mod model_parameter_specialize_tests;
mod model_parameter_tests;
mod name_collision_tests;
mod nested_import_hint_tests;
mod nested_statement_resolution_tests;
mod non_ascii_identifier_tests;
mod port_access_contract_tests;
mod port_address_completeness_tests;
mod port_at_syntax_tests;
mod port_direction_tests;
mod port_init_tests;
mod port_initial_value_tests;
mod port_initializer_tests;
mod reference_model_tests;
mod semantic_tests;
mod shared_const_qualified_tests;
mod stage_recovery_tests;
mod type_inference_chain_tests;
mod type_redefinition_tests;
mod unconditional_edge_tests;
mod unused_formula_tests;
mod validate_batch_tests;
mod wider_integer_tests;
