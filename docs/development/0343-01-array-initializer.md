# Разработка 0343-01: инициализатор массива

> Фича: [../features/0343-array-initializer.md](../features/0343-array-initializer.md) · ADR: [../adr/0343-array-initializer.md](../adr/0343-array-initializer.md)

## Состав правки

| Файл | Что сделано |
|---|---|
| `takt-lang/src/generator/st/st_decl.rs` | `literal_init` печатает агрегат массива скаляров формой `[…]` |
| `takt-lang/src/generator/st/st_model.rs` | `emit_deferred_inits`: массив структур кладётся операторами первого скана |
| `takt-lang/src/generator/st/st_compose.rs` | агрегат массива в аргументе параметра остаётся `ST-017` — с названной причиной |
| `takt-lang/src/generator/c/c_model_init.rs` | элемент-структура кладётся по полям |
| `takt-lang/src/generator/rust/rust_coerce.rs` | элементы массива структур печатаются литералом структуры |
| `takt-lang/tests/semantic/array_shared_and_index_tests.rs` | совпадение без `;`: у переменной корня появился инициализатор |
| `takt-sim/tests/data/eval/conformance_array_init.takt` | фикстура: массив структур плюс контрольный массив скаляров |
| `takt-sim/tests/conformance/conformance_st_tests/array_init.rs` | сверка **значений** через `iec2c` + `cc` |
| `takt-lang/tests/targets/array_struct_init_tests.rs` | `c`, `rust` и контроль границы `ST-017` |

## Проверено

- Прогон ST до правки: `o = 0`; после: `o = 3` и `o_sum = 9`.
- Вывод корпуса не изменился; `cargo test` зелёный.
