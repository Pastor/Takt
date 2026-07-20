# Задача 0073-01: удалить `Location::filename()`, тесты — на `try_file_no`

> Фича: [../features/0073-location-filename-path.md](../features/0073-location-filename-path.md) · ADR: [../adr/0073-location-filename-path.md](../adr/0073-location-filename-path.md) · анализ: [../analyze/0073-location-filename-path.md](../analyze/0073-location-filename-path.md)

## Что было

`Location::filename()` (`grammar/src/diagnostics.rs:645`) возвращает
`format!("{}", file_no)` — **номер файла строкой** при имени, обещающем путь.
Рядом (`:655`) — честный `try_file_no() -> Option<String>` с тем же значением.
Метод зовут **только** два теста, дублирующих проверку `try_file_no`:

- `grammar/tests/parser_tests.rs:944` — `assert_eq!(loc.filename(), "0");`
  (строка `:945` уже проверяет `try_file_no() == Some("0")`);
- `grammar/tests/ast_tests.rs:76` — тест `location_filename`, стоящий вплотную к
  тесту `try_file_no()`.

## Что сделано

Реализовано по **Option A** ADR 0073:

1. **Удалён** `pub fn filename(&self) -> String` (`diagnostics.rs:643–650`) вместе
   с doc-комментарием.
2. **`parser_tests.rs`** — удалена строка `assert_eq!(loc.filename(), "0");`
   (свойство покрыто соседним `assert_eq!(loc.try_file_no(), Some("0"…))`).
3. **`ast_tests.rs`** — удалён тест `location_filename` (дубль теста `try_file_no`
   ниже).

`not_a_file()` **оставлен** — востребован `start`/`end`/`exclusive_end`.

**Статус по функциональности (правило 11):**

| Функциональность | Работа | Обоснование |
|---|---|---|
| Диагностика (`Location`) | **да** | Удалён один `pub`-метод; API сужен (потребителей вне тестов нет) |
| Тесты `parser`/`ast` | **да** | Две дублирующие проверки удалены; покрытие держит `try_file_no` |
| Язык / семантика / кодоген / симуляция | **н/п** | Не затрагиваются; версия языка **0.2.0** без изменений |

## Проверки

- `grep -rn '\.filename()\|pub fn filename' grammar/ simulation/` → пусто (кроме
  `import.rs::filename_path`/`filename_found_returns_content_and_path` — иное имя).
- `cargo test -- --test-threads=1` и `cargo test --features lsp -- --test-threads=1`
  → зелёные; `location_methods`, `ast_tests::*try_file_no*` — проходят.
- `./scripts/precheck.sh` зелёный; `git diff examples/generated/` пуст.
