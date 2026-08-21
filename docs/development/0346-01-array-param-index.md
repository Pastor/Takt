# Разработка 0346-01: индексация параметра-массива

> Фича: [../features/0346-array-param-index.md](../features/0346-array-param-index.md) · ADR: [../adr/0346-array-param-index.md](../adr/0346-array-param-index.md)

## Состав правки

| Файл | Что сделано |
|---|---|
| `takt-lang/src/semantic/expression/mod.rs` | `ArraySubscript` и `ArraySlice` ищут имя сперва среди параметров; параметры передаются в разбор индекса; помощник `param_variable` |
| `takt-lang/tests/semantic/array_param_index_tests.rs` | четыре случая: две рабочие формы и два контроля (`SE-003`, `SE-030`) |
| `takt-sim/tests/data/eval/conformance_array_param.takt` | фикстура: индексация параметра, параметр-индекс, контрольная функция |
| `takt-sim/tests/conformance/conformance_array_param_tests.rs` | сверка **значений** с порождённым C |

## Проверено

- Все восемь целей и эталон исполняют вход, который прежде отвергала семантика.
- `cargo test` зелёный; вывод корпуса не изменился.
