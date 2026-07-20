# Тест-план фичи 0073: `Location::filename()` возвращает номер, а не путь

> Фича: [../features/0073-location-filename-path.md](../features/0073-location-filename-path.md) · ADR: [../adr/0073-location-filename-path.md](../adr/0073-location-filename-path.md) · анализ: [../analyze/0073-location-filename-path.md](../analyze/0073-location-filename-path.md)

## Область и цель

Проверить, что ложно названный `Location::filename()` удалён, покрытие
«`Location` несёт верный `file_no`» сохранено через `try_file_no()`, а прочая
диагностика и вывод генераторов не задеты.

## Проверки (условие → ожидаемый результат)

| # | Проверка | Предусловие | Ожидаемый результат | Ссылка на R/A |
|---|---|---|---|---|
| T1 | `filename()` удалён | дифф применён | `grep '\.filename()\|pub fn filename' grammar/ simulation/` — пусто (кроме `import.rs::filename_path`/`filename_found…`, иное имя) | R1 / A1 |
| T2 | `try_file_no` покрывает свойство | тот же дифф | `parser_tests::location_methods` → зелёный (проверяет `try_file_no() == Some("0")`); `ast_tests` с `try_file_no` → зелёный | R2 / A2 |
| T3 | Прочие аксессоры целы | тот же дифф | `start`/`end`/`exclusive_end`/`begin_range` в `location_methods` — зелёные; `not_a_file()` компилируется (востребован ими) | R3 / A3 |
| T4 | Регресс отсутствует | тот же дифф | `cargo test -- --test-threads=1` (+ `--features lsp`) зелёный | R3 / A3 |
| T5 | Кодоген неизменен | весь `examples/` | `git diff examples/generated/` пуст; гейт детерминизма 0048 | R4 / A4 |
| T6 | `precheck.sh` зелёный | все инструменты | `EXIT=0` | R4 / A4 |

## Разбивка проверок по функциональности

<!-- Легенда: ✅ пройдено · ❌ провалено · ⬜ не проверялось · — не применимо -->

| Функциональность | Условие | Статус |
|---|---|---|
| Диагностика (`Location`) | `filename()` удалён, `try_file_no` покрывает | ⬜ |
| Тесты `parser`/`ast` | без регресса, без потери покрытия | ⬜ |
| Кодоген всех целей | вывод байт-в-байт неизменен | — (не задет) |
| Язык / семантика | не задеты | — |

## Тестовые данные и окружение

- **Данные:** `Location::Source(0, 10, 20)` (`parser_tests::location_methods`),
  `Location::Source(7, 0, 0)` (`ast_tests`).
- **Окружение:** `cargo test -- --test-threads=1`; `cargo test --features lsp`;
  полный `./scripts/precheck.sh` (гейт детерминизма 0048).
