# Задача 0088-07: Тест `lsp_tests` — вынос mod-групп в подмодуль

> Фича: [../features/0088-module-size-remaining.md](../features/0088-module-size-remaining.md) · ADR: [../adr/0088-module-size-remaining.md](../adr/0088-module-size-remaining.md) · анализ: [../analyze/0088-module-size-remaining.md](../analyze/0088-module-size-remaining.md)

## Что было

`grammar/tests/lsp_tests.rs` — **1266 строк** (нарушитель). Структура — **четыре**
отдельных `#[cfg(feature = "lsp")] mod`-блока: `lsp_integration` (10–803),
`diagnostic_location_tests`, `formatting_tests`, `lsp_multifile`.

## Что сделано

Три меньших mod-блока (`diagnostic_location_tests`, `formatting_tests`,
`lsp_multifile`, строки 805–1266) вынесены **целиком** в подмодуль
`grammar/tests/lsp_tests/more.rs`:

- Блоки самодостаточны (свои `use` внутри каждого), поэтому `use super::*` не
  нужен — перемещены как есть.
- В `lsp_tests.rs` (crate root) добавлено `#[cfg(feature = "lsp")]
  #[path = "lsp_tests/more.rs"] mod more;`. **`#[path]` относителен `tests/`**
  (объявление на **корне** тест-бинарника, как в 0088-06), поэтому путь —
  `lsp_tests/more.rs`.

⚠️ **Урок:** первая попытка резала по строке **внутри** первого mod — граница
пересекла закрытие `mod lsp_integration` (`unexpected closing delimiter`). Резать
тестовые файлы нужно **по границам mod-блоков**, а не по номеру строки: `mod`
вложен → `#[path]` от каталога-имени-mod; `mod` на корне → `#[path]` от `tests/`.

**Чистое перемещение:** утверждения не менялись. `lsp_tests.rs`: **1266 → 809**;
`more.rs` — 467; оба ≤ 1000 → запись удалена из реестра (**12 → 11**).

Стеки: только `grammar` (тесты). `simulation` — н/п.

## Проверки

- `cargo test --features lsp --test lsp_tests` — **69 passed, 0 failed** (один
  бинарник: `lsp_integration` в корне + `more` с тремя mod).
- `./scripts/precheck.sh` — зелёный (`--all-features`, `check-module-size.sh` −1).
