# Задача 0088-01: Генератор C — вынос init-группы (`c_model.rs`)

> Фича: [../features/0088-module-size-remaining.md](../features/0088-module-size-remaining.md) · ADR: [../adr/0088-module-size-remaining.md](../adr/0088-module-size-remaining.md) · анализ: [../analyze/0088-module-size-remaining.md](../analyze/0088-module-size-remaining.md)

## Что было

`grammar/src/generator/c/c_model.rs` — **1073 строки** (нарушитель лимита, запись
в реестре). Содержит и `_init`-генерацию, и `_tick`/`is_done`/переходы —
разнородные ответственности в одном файле.

## Что сделано

Вынесена **init-группа** (инициализация стартового состояния и вложенных
элементов в `_init`, фича 0033 R6) в новый модуль
`grammar/src/generator/c/c_model_init.rs` (309 строк):

- Функции: `generate_model_init`, `generate_start_state_init`, `is_real_array`,
  `generate_array_init`, `generate_parallel_items_init`, `generate_concat_item_init`.
- Наружу (в остающуюся `_tick`-логику `c_model.rs`) видны `pub(super)`
  `generate_model_init` и `generate_concat_item_init`; прочие — приватные модуля.
- `c_model.rs` импортирует их через `use super::c_model_init::{…}`; лишние
  импорты (`generate_expr`, `TypeNode`, `ExpressionNode`, `VariableNode`,
  `generate_scalar_init`), уехавшие с функциями, удалены.
- `generator/c/mod.rs` — добавлено `mod c_model_init;`.

**Чистое перемещение:** тело функций скопировано дословно, вывод C неизменен.
`c_model.rs`: **1073 → 782** строки (уложился в лимит) — запись **удалена** из
`scripts/module-size-baseline.txt` (реестр 18 → 17). `c_model_init.rs` (309) — не
нарушитель.

Стеки: только `grammar`. `simulation` — н/п.

## Проверки

- `cargo build --bin lamc` — без предупреждений.
- `cargo test --test codegen_tests` — **39 passed** (вывод C байт-в-байт неизменен).
- `./scripts/precheck.sh` — зелёный (детерминизм-гейт, `check-module-size.sh`,
  все тесты, сборка примеров).
