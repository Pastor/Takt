# Разработка 0335-01: разряд в позиции числового значения

> Фича: [../features/0335-bit-value-in-targets.md](../features/0335-bit-value-in-targets.md) · ADR: [../adr/0335-bit-value-in-targets.md](../adr/0335-bit-value-in-targets.md)

## Состав правки

| Файл | Что сделано |
|---|---|
| `takt-lang/src/generator/rust/rust_expr.rs` | ветвь `coerce_to`: числовой приёмник получает `Тип::from(…)`; внешние скобки снимаются (`unused_parens` — отказ под `-D warnings`); список целочисленных типов закрыт константой |
| `takt-lang/src/generator/rust/rust_coerce.rs` | **новый модуль**: приведение к типу приёмника выделено из `rust_expr` по границе ответственности (печать выражения и приведение — разные вопросы); имя реэкспортировано, семь потребителей не правились |
| `takt-lang/src/generator/st/st_expr.rs` | ветвь `coerce_to`: `BOOL_TO_<тип IEC>(…)`; форма проверена пробой `iec2c` |
| `takt-lang/src/generator/sv/sv_expr.rs` | `Scope::coerce`: размерная форма `W'(…)` при приёмнике шире одного бита |
| `takt-lang/tests/targets/bit_value_targets_tests.rs` | три цели: текст вывода **и** прогон настоящих `rustc`, `iec2c`, `verilator`; контрольные проверки на битовый приёмник |
| `takt-sim/tests/data/eval/conformance_bit_value.takt` | фикстура: два разряда (старший и младший) плюс контрольный битовый приёмник |
| `takt-sim/tests/conformance/conformance_st_tests/bit_value.rs` | сверка **значений** с порождённым ST через `iec2c` + `cc` |

## Проверено

- Мутация «снять `BOOL_TO_`» валит сверку ST.
- Контрольный вход (`flag: bit := src.3`) не изменился у всех трёх целей.
- Вывод корпуса не изменился.
