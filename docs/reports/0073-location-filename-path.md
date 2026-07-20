# Отчёт о тестировании фичи 0073: `Location::filename()` возвращает номер, а не путь

> Фича: [../features/0073-location-filename-path.md](../features/0073-location-filename-path.md) · тест-план: [../tests/0073-location-filename-path.md](../tests/0073-location-filename-path.md) · анализ: [../analyze/0073-location-filename-path.md](../analyze/0073-location-filename-path.md)

## Резюме

Реализована подзадача 0073-01 (Option A ADR): ложно названный
`Location::filename()` удалён, две дублирующие тестовые проверки убраны.
Покрытие «`Location` несёт верный `file_no`» держит `try_file_no()`. **Все
проверки тест-плана пройдены; блокеров нет.**

**Окружение:** macOS (darwin 25.5), rustc/clippy nightly, `cargo test`
однопоточно, полный `./scripts/precheck.sh`.

## Фактические результаты по проверкам

| # | Проверка | Результат | Комментарий |
|---|---|---|---|
| T1 | `filename()` удалён | ✅ | `grep '\.filename()\|pub fn filename' grammar/ simulation/` — пусто (совпадения `import.rs::filename_path`/`filename_found…` — иное имя, не `Location`) |
| T2 | `try_file_no` покрывает свойство | ✅ | `parser_tests::location_methods` (`try_file_no() == Some("0")`) и `ast_tests::location_try_file_no_source` (`Some("42")`) — зелёные |
| T3 | Прочие аксессоры целы | ✅ | `start`/`end`/`exclusive_end`/`begin_range` — зелёные; `not_a_file()` компилируется (востребован ими) |
| T4 | Регресс отсутствует | ✅ | `cargo test --features lsp -- --test-threads=1` — зелёный |
| T5 | Кодоген неизменен | ✅ | `git diff examples/generated/` пуст; гейт детерминизма 0048 зелёный |
| T6 | `precheck.sh` зелёный | ✅ | `EXIT=0` (после обновления baseline размера `parser_tests.rs` 1893→1892 — долг фиксируется по факту) |

## Замечания

- `parser_tests.rs` ужат на 1 строку → запись в `scripts/module-size-baseline.txt`
  уменьшена (1893 → 1892), как требует храповик размера («долг фиксируется по
  факту, допустима только правка-уменьшение»).
- `ast_tests.rs` (737) и `diagnostics.rs` (979) под лимитом 1000 — в реестре не
  числятся, их сжатие проверок не требует.
- API `grammar` формально сужен (удалён `pub fn filename`) — потребителей вне
  тестов нет (правило 11, санкционировано ADR).
