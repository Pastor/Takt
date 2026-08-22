# Задача 0388-01: Разделение печати объявлений цели `st`

> Фича: [../features/0388-st-decl-split.md](../features/0388-st-decl-split.md) · ADR: [../adr/0388-st-decl-split.md](../adr/0388-st-decl-split.md) · анализ: [../analyze/0388-st-decl-split.md](../analyze/0388-st-decl-split.md)

## Что было

`st_decl.rs` — 1000 строк при лимите 1000. Файл держал оба пласта вывода
цели `st`: объявления типов файла и секции `VAR…` конкретной модели.

## Что сделано

Пласт типов вынесен в `st_decl_types.rs`: `shared_array_names`,
`emit_struct_types`, `shared_array_types`, `function_array_forms`,
`function_array_form_names`. Вызовы в `st/mod.rs` переведены на новый путь.

⚠️ **`Extras` возвращена в `st_decl`**: структура описывает дополнения к
**секциям** (`state`, `is_done`, `VAR_IN_OUT`, разделяемые переменные), а не к
типам; механический перенос увёл её вместе с соседним кодом, и это назвал
компилятор.

⚠️ **Тесты печати объявлений возвращены к своему предмету** — они проверяют
`emit_declarations`, оставшийся в `st_decl`.

Итог: `st_decl.rs` — 806 строк, `st_decl_types.rs` — 216.

## Проверки

```sh
cargo test --all-features
./scripts/check-module-size.sh
./scripts/precheck.sh
git diff --exit-code examples/generated/
```

- **Вывод** (T1): дифф корпуса пуст — код перенесён, не изменён.
- **Тесты** (T2): зелено.
- **Размер** (T3): оба модуля в лимите; реестр долга не менялся.
