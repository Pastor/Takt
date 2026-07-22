# Задача 0076-01: Симулятор не исполняет массивы вовсе

> Фича: [../features/0076-sim-arrays.md](../features/0076-sim-arrays.md) · ADR: [../adr/0076-sim-arrays.md](../adr/0076-sim-arrays.md) · анализ: [../analyze/0076-sim-arrays.md](../analyze/0076-sim-arrays.md)

## Что было

Симулятор не исполнял массивы, хотя `Value::Array` и **чтение** `data[i]` уже
работали. Два пробела: (1) `resolve_place` (`unit/statement.rs`) раскладывал
левую часть в путь **имён полей** (`Vec<String>`) и не разбирал `ArraySubscript`
→ `data[i] := v` давало `SIM-017`; (2) `coerce_initial` (`unit/builder.rs`) не
имел ветки `TypeNode::Array` → список-init клал скаляр, чтение `data[0]` давало
`SIM-010` (обманчиво: массив стал скаляром). Сверка с C была недостижима, сторож
`a9_bit_and_array_conformance_gap` (`conformance_c_tests.rs`) это фиксировал.

## Что сделано

Реализация по [ADR 0076](../adr/0076-sim-arrays.md), Option A — единый механизм
«места» с записью полей структур (0034):

- **`eval/place.rs`:** введён `PlaceSegment{Field(String), Index(usize)}`;
  `update` матчит сегмент — поле идёт в `update_field` (тип поля из реестра),
  индекс в `update_index` (тип элемента из протащенного `ty =
  TypeNode::Array(_, elem)`). Лист приводится `coerce_to_type_with` к типу
  поля/элемента (усечение S9). Новые ошибки `IndexOfNonArray` /
  `ArrayIndexOutOfBounds` — обе под кодом `SIM-010` (как ошибки **чтения**
  массива). Точечность сохранена (`data[0]:=7` не трогает соседей).
- **`eval/error.rs`:** два новых варианта `EvalError` + их тексты и код `SIM-010`.
- **`unit/statement.rs`:** `resolve_place` расширен — разбирает `ArraySubscript`,
  вычисляя индекс **тем же** `eval_expression`, что и чтение (одна переменная для
  `data[i]:=` и `data[i]`); не-целый/отрицательный индекс → `SIM-010`. Сигнатура
  стала `Result<Option<&VarNode>, Diagnostic>` (индекс может не вычислиться).
  Путь передаётся сегментами; тип корня (`ty`) прокидывается в `update_place_via`.
- **`context.rs`:** `update_place_via` принимает `ty: Option<&TypeNode>` и путь
  сегментов — мост к ядру.
- **`unit/builder.rs`:** `coerce_initial` — ветка `TypeNode::Array` →
  `coerce_to_type_with(...).unwrap_or(value)` (список приводится поэлементно + по
  длине; скаляр-init не приводится → остаётся как есть, 0078).

**Статус по функциональности (правило 11):**

| Функциональность | Статус |
|---|---|
| Симулятор (`eval/place`, `error`, `unit/statement`, `builder`, `context`) | ✅ реализовано |
| Структуры 0034 (та же сигнатура `update`) | ✅ путь сохранён, тесты перенастроены |
| Генераторы C/rust/st/sv, LSP, форматтер | н/п — язык не меняется, вывод байт-в-байт неизменен |
| Сверка с C (`conformance_c_tests`) | ✅ сторож заменён положительным тестом |

## Проверки

```sh
cargo test -p simulation -- --test-threads=1        # eval + place + conformance зелены
./scripts/precheck.sh                                # итоговый гейт (fmt/clippy/детерминизм/сценарии)
```

- **R1/A1** — `eval_tests::array_element_write_and_read`, `place::update_array_element_is_pointwise`.
- **R3/A3** — `place::update_array_element_coerced_to_elem_type` (300→44), `arrays.lam` `big[0]`.
- **R4/A4** — `place::{update_array_out_of_bounds_is_error, update_index_on_non_array_is_error}` → `SIM-010`.
- **R5/A5** — список-init `big:[u8;2]:={300,5}` → `[44,5]`.
- **R7/A6** — `conformance_c_tests::array_element_matches_generated_c` (значения = C).
- **R8/A7,A8** — тесты 0034 (`place`/`access`) зелены; `precheck.sh` (детерминизм + сценарии примеров).
- **R6** — скаляр-init массива не приводится (остаётся скаляром) — вне объёма, 0078.
