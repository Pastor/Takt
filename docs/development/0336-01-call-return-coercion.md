# Разработка 0336-01: приведение в аргументе и возврате

> Фича: [../features/0336-call-return-coercion.md](../features/0336-call-return-coercion.md) · ADR: [../adr/0336-call-return-coercion.md](../adr/0336-call-return-coercion.md)

## Состав правки

| Файл | Что сделано |
|---|---|
| `takt-lang/src/generator/rust/rust_expr.rs` | аргумент приводится по типу параметра; поле `return_type` в `Scope` |
| `takt-lang/src/generator/rust/rust_stmt.rs` | приведение в **обоих** путях возврата — `return x;` и хвостовое выражение |
| `takt-lang/src/generator/rust/rust_func.rs` | тело функции получает `return_type` |
| `takt-lang/src/generator/st/st_func.rs` | аргумент приводится; `collect_hoisted` получает тип возврата |
| `takt-lang/src/generator/st/st_stmt.rs` | `FnContext` = имя POU **и** тип возврата; ветвь `Return` приводит |
| `takt-lang/src/generator/sv/sv_expr.rs` | аргумент приводится (`param_type`); поле `function_ret` в `Scope` |
| `takt-lang/src/generator/sv/sv_stmt.rs`, `sv_fsm.rs` | возврат приводится; тело функции получает тип |
| `takt-lang/tests/targets/call_return_coercion_tests.rs` | три цели: текст **и** прогон `rustc`, `iec2c`, `verilator` |
| `takt-sim/tests/data/eval/conformance_call_coercion.takt` | фикстура: четыре величины, включая контрольную |
| `takt-sim/tests/conformance/conformance_call_coercion_tests.rs` | сверка значений с порождённым Rust |

## Проверено

- Мутация «снять приведение аргумента» валит сверку значений (`E0308`).
- Мутация «снять приведение хвостового выражения» валит тест цели `rust`.
- Вывод корпуса не изменился; `cargo test` — 3196 тестов, ноль провалов.
