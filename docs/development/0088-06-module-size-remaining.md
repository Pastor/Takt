# Задача 0088-06: Тест `lexer_tests` — разбиение на подмодуль

> Фича: [../features/0088-module-size-remaining.md](../features/0088-module-size-remaining.md) · ADR: [../adr/0088-module-size-remaining.md](../adr/0088-module-size-remaining.md) · анализ: [../analyze/0088-module-size-remaining.md](../analyze/0088-module-size-remaining.md)

## Что было

`grammar/tests/lexer_tests.rs` — **1081 строка** (нарушитель): helpers + 62
плоских `#[test]` без модульной структуры.

## Что сделано

**Первый вынос тестового файла** — приём «директория-подмодуль» (ADR 0088):

- Вторая половина тестов (секция «Тесты Display токенов» и далее) вынесена в
  `grammar/tests/lexer_tests/part2.rs`. Helpers и импорты берутся из родителя
  через `use super::*` (glob — без пер-элементных `unused_imports`).
- `lexer_tests.rs` объявляет подмодуль **`#[path = "lexer_tests/part2.rs"] mod
  part2;`** — `#[path]` обязателен: корень тест-бинарника ищет `mod` в `tests/`,
  а **не** в подкаталоге по имени файла (иначе `E0583 file not found`).
- **Один тест-бинарник** сохранён (подмодуль, а не второй файл `tests/*.rs`) —
  время компиляции тестов не растёт кратно.

**Чистое перемещение:** утверждения тестов не менялись, только адрес.
`lexer_tests.rs`: **1081 → 529**; `part2.rs` — 561; оба ≤ 1000 → запись удалена
из реестра (**13 → 12**).

Стеки: только `grammar` (тесты). `simulation` — н/п.

## Проверки

- `cargo test --test lexer_tests` — **62 passed, 0 failed** (36 в корне + 26 в
  `part2`, один бинарник).
- `./scripts/precheck.sh` — зелёный (`check-module-size.sh` −1 запись, все тесты).
