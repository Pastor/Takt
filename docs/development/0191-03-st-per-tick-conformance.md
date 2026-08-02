# Задача 0191-03: инициализатор `[bit;N]` доезжает до цели `st`

> Фича: [../features/0191-st-per-tick-conformance.md](../features/0191-st-per-tick-conformance.md) · ADR: [../adr/0191-st-per-tick-conformance.md](../adr/0191-st-per-tick-conformance.md) · анализ: [../analyze/0191-st-per-tick-conformance.md](../analyze/0191-st-per-tick-conformance.md)

## Что было

`var small: [bit;8] := 255;` объявлялось в ST как `small : USINT;` — **без
значения**, тогда как рядом стоящая `var plain: u8 := 7;` получала
`plain : USINT := 7;`. Эталон и цель `c` давали 255, цель `st` — 0. `taktc`
рапортовал успех, `iec2c` вывод принимал: расхождение молчаливое.

## Что сделано

`st_decl.rs::literal_init` глушил инициализатор у **любого**
`TypeNode::Array`. Для настоящих массивов это верно и обязательно — `iec2c`
отвергает `ARRAY [0..3] OF USINT := 0`, — но `[bit;N≤64]` по фиче
[0078](../features/0078-bit-array-semantics.md) массивом **не является**: это
упакованный скаляр, и `get_st_type` печатает его как `USINT`/`UINT`/…

Гейт уточнён: инициализатор глушится, только если тип не является **упакованным
скаляром**. Признак берётся из того же слоя, которым уже пользуется печать типа
(`semantic::bit_vector::is_bit_vector` + `layout`), — второе правило упаковки
разъехалось бы с первым и дало значение не той ширины.

## Проверки

```sh
cargo test -p takt-lang --lib st_decl -- --test-threads=1
```

| Вход | Результат |
|---|---|
| `var small: [bit;8] := 255;` | `small : USINT := 255;` ✅ |
| `var wide: [bit;128] := 1;` | `wide : ARRAY [0..1] OF ULINT;` — без инициализатора ✅ (широкий вектор в ST действительно массив) |
| `var data: [u8;4] := 0;` | без инициализатора ✅ — существующий тест `test_emit_declarations_array_gets_no_scalar_initializer` зелёный |

Значение доказано **прогоном**, а не текстом: фикстура задачи
[0191-02](0191-02-st-per-tick-conformance.md) переходит по условию
`mask = 255`, и без инициализатора автомат застревает — потактовая сверка
краснеет (мутация проведена).
