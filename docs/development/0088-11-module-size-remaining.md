# Задача 0088-11: Тест `semantic_tests` — разбиение на 5 подмодулей

> Фича: [../features/0088-module-size-remaining.md](../features/0088-module-size-remaining.md) · ADR: [../adr/0088-module-size-remaining.md](../adr/0088-module-size-remaining.md) · анализ: [../analyze/0088-module-size-remaining.md](../analyze/0088-module-size-remaining.md)

## Что было

`grammar/tests/semantic_tests.rs` — **5015 строк**, **крупнейший файл проекта**
(нарушитель №1): helpers (`build`, `build_err`) + **294** плоских `#[test]` на
корне тест-бинарника (без mod-структуры). Одним подмодулем (приём 0088-06) не
уложиться — нужно **6 файлов** ≤ 1000.

## Что сделано

Тесты разбиты на **5** подмодулей `grammar/tests/semantic_tests/part{2..6}.rs`
(родитель держит helpers + первую группу):

- Резал по границе **полного элемента** — от начала ведущего doc-комментария до
  конца `#[test] fn` (урок 0088-07/08; иначе висячий `///` на границе, как фикс
  для 0088-08). Точки раскола — ближайший `#[test]` к 950/1880/2810/3740/4670.
- Helpers и импорты — из родителя через `use super::*` (glob).
- `semantic_tests.rs` (crate root) объявляет `#[path = "semantic_tests/partN.rs"]
  mod partN;` для каждой части.

⚠️ **Ловушка:** два top-level `use` (`StatementNode`, `type_node::TypeNode`)
жили **в середине** оригинального файла (не в шапке). При расколе они попали в
`part2`, и `part5` перестала видеть `TypeNode` (`use super::*` реэкспортит
только items **родителя**, не сиблингов). Исправлено **подъёмом обоих `use` в
шапку родителя** — теперь `super::*` раздаёт их всем частям.

**Чистое перемещение:** ни одно утверждение не менялось. `semantic_tests.rs`:
**5015 → 948**; части — 940/934/933/941/355; все ≤ 1000 → запись удалена из
реестра (**8 → 7**).

Стеки: только `grammar` (тесты). `simulation` — н/п.

## Проверки

- `cargo test --test semantic_tests` — **294 passed, 0 failed** (один бинарник).
- `./scripts/precheck.sh` — зелёный (`check-module-size.sh` −1, все тесты,
  детерминизм-гейт).
