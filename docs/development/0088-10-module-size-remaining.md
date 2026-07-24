# Задача 0088-10: Тест `parser_tests` — разбиение на подмодуль

> Фича: [../features/0088-module-size-remaining.md](../features/0088-module-size-remaining.md) · ADR: [../adr/0088-module-size-remaining.md](../adr/0088-module-size-remaining.md) · анализ: [../analyze/0088-module-size-remaining.md](../analyze/0088-module-size-remaining.md)

## Что было

`grammar/tests/parser_tests.rs` — **1892 строки** (нарушитель): helpers
(`must_parse`, `first_named_model`) + 105 плоских `#[test]` на корне
тест-бинарника (без mod-структуры).

## Что сделано

Вторая половина тестов (с `location_methods`) вынесена в подмодуль
`grammar/tests/parser_tests/part2.rs` (приём 0088-06/08):

- Helpers и импорты — из родителя через `use super::*` (glob). Резал по границе
  **полного `#[test]`** (урок 0088-07), захватывая ведущий doc-комментарий.
- `parser_tests.rs` (crate root) объявляет `#[path = "parser_tests/part2.rs"]
  mod part2;` — `#[path]` от `tests/`.

**Чистое перемещение:** утверждения не менялись. `parser_tests.rs`: **1892 →
937**; `part2.rs` — 963; оба ≤ 1000 → запись удалена из реестра (**9 → 8**).

Стеки: только `grammar` (тесты). `simulation` — н/п.

## Проверки

- `cargo test --test parser_tests` — **105 passed, 0 failed** (один бинарник).
- `./scripts/precheck.sh` — зелёный (`check-module-size.sh` −1, все тесты,
  детерминизм-гейт).
