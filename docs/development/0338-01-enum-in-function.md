# Разработка 0338-01: перечисление внутри функции

> Фича: [../features/0338-enum-in-function.md](../features/0338-enum-in-function.md) · ADR: [../adr/0338-enum-in-function.md](../adr/0338-enum-in-function.md)

## Состав правки

| Файл | Что сделано |
|---|---|
| `takt-lang/src/generator/sv/sv_stmt.rs` | инициализатор локальной переменной идёт через `Scope::coerce` |
| `takt-lang/src/generator/st/st_func.rs` | `collect_hoisted` возвращает и текст тела; `enum_constants_used` отбирает упомянутые константы; они печатаются в `VAR CONSTANT` функции |
| `takt-lang/src/generator/st/st_decl.rs` | `visible_enums` открыт для модуля цели |
| `takt-sim/tests/data/eval/conformance_enum_in_function.takt` | фикстура: функция с перечислением плюс контрольная без него |
| `takt-lang/tests/targets/enum_in_function_tests.rs` | две цели: текст **и** прогон `verilator`, `iec2c` |
| `takt-sim/tests/conformance/conformance_st_tests/enum_in_function.rs` | сверка **значений** с порождённым ST |
| `takt-sim/tests/conformance/conformance_st_tests/overflow.rs` | вынесено из общего файла по границе темы (правило размера модуля) |

## Проверено

- Вывод корпуса не изменился; `cargo test` зелёный.
- Контрольная функция (`plain`) лишних объявлений не получает — граница среза в
  тесте кладётся по `END_FUNCTION`, иначе проверка захватывала `FUNCTION_BLOCK`.
