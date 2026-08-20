# Разработка 0341-01: вложенная структура

> Фича: [../features/0341-nested-struct-order.md](../features/0341-nested-struct-order.md) · ADR: [../adr/0341-nested-struct-order.md](../adr/0341-nested-struct-order.md)

## Состав правки

| Файл | Что сделано |
|---|---|
| `takt-lang/src/generator/struct_order.rs` | **новый**: порядок по зависимостям, три юнит-теста (в том числе на цикл) |
| `takt-lang/src/generator/c/c_header.rs` | порядок берётся у носителя |
| `takt-lang/src/generator/st/st_decl.rs` | то же; снята алфавитная сортировка на выходе |
| `takt-lang/src/generator/sv/sv_type.rs` | то же |
| `takt-lang/src/generator/rust/rust_fixed.rs` | `expression_type` различает разряд и поле структуры |
| `takt-lang/tests/targets/rust_printers_tests.rs` | тест, утверждавший «поля структур не транслируются», переписан |
| `takt-sim/tests/data/eval/conformance_nested_struct.takt` | фикстура: вложенная структура плюс контрольная плоская |
| `takt-lang/tests/targets/nested_struct_targets_tests.rs` | четыре цели: порядок **и** прогон настоящих инструментов |

## Проверено

- Вывод корпуса не изменился; `cargo test` зелёный.
- Контроль: разряд `x.7` остался логическим у цели `rust`.
