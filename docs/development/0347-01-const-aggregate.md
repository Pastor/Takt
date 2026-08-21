# Разработка 0347-01: константа-агрегат

> Фича: [../features/0347-const-aggregate.md](../features/0347-const-aggregate.md) · ADR: [../adr/0347-const-aggregate.md](../adr/0347-const-aggregate.md)

## Состав правки

| Файл | Что сделано |
|---|---|
| `takt-lang/src/generator/rust/rust_decl.rs` | значение константы печатается через `coerce_to` |
| `takt-lang/src/generator/sv/sv_const.rs` | `localparam` печатается носителем `reset_value`; помощник `enums_of` |
| `takt-lang/src/generator/sv/mod.rs` | типы и перечисления печатаются **до** констант |
| `takt-sim/tests/data/eval/conformance_const_struct.takt` | фикстура: константа-структура, константа-массив, контрольный скаляр |
| `takt-lang/tests/targets/const_aggregate_tests.rs` | `rust` и `sv`: текст, порядок разделов **и** прогон инструментов |

## Проверено

- `verilator` и `rustc -D warnings` принимают вывод.
- Вывод корпуса не изменился; `cargo test` зелёный.
