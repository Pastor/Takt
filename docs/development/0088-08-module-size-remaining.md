# Задача 0088-08: Тест `codegen_tests` — разбиение на подмодуль

> Фича: [../features/0088-module-size-remaining.md](../features/0088-module-size-remaining.md) · ADR: [../adr/0088-module-size-remaining.md](../adr/0088-module-size-remaining.md) · анализ: [../analyze/0088-module-size-remaining.md](../analyze/0088-module-size-remaining.md)

## Что было

`grammar/tests/codegen_tests.rs` — **1393 строки** (нарушитель): helpers
(`tmp_but_file`, `generate_c_content`, `generate_h_content`) + 39 плоских
`#[test]` на корне тест-бинарника (без mod-структуры).

## Что сделано

Вторая половина тестов (с `test_concat_non_last_has_break_inside_if`) вынесена в
подмодуль `grammar/tests/codegen_tests/part2.rs` (приём 0088-06):

- Helpers и импорты — из родителя через `use super::*` (glob). Резал по границе
  **полного `#[test]`** (урок 0088-07), а не по номеру строки.
- `codegen_tests.rs` (crate root) объявляет `#[path = "codegen_tests/part2.rs"]
  mod part2;` — `#[path]` от `tests/` (объявление на корне бинарника).

**Чистое перемещение:** утверждения не менялись. `codegen_tests.rs`: **1393 →
715**; `part2.rs` — 690; оба ≤ 1000 → запись удалена из реестра (**11 → 10**).

Стеки: только `grammar` (тесты). `simulation` — н/п.

## Проверки

- `cargo test --test codegen_tests` — **39 passed, 0 failed** (один бинарник).
- `./scripts/precheck.sh` — зелёный (`check-module-size.sh` −1, все тесты,
  детерминизм-гейт).
