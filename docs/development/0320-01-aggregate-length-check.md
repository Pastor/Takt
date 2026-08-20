# Разработка 0320-01: проверка длины агрегата

> Фича: [../features/0320-aggregate-length-check.md](../features/0320-aggregate-length-check.md) · ADR: [../adr/0320-aggregate-length-check.md](../adr/0320-aggregate-length-check.md)

## Состав правки

| Файл | Что сделано |
|---|---|
| `takt-lang/src/semantic/validate/aggregate_length.rs` | новая проверка: массив по размеру, структура по числу полей; `SE-123` |
| `takt-lang/src/semantic/validate/mod.rs` | вызов рядом с `check_literal_ranges` |
| `takt-lang/tests/semantic/aggregate_length_tests.rs` | пять проверок: лишний, недостача, структура, контроль, граница |
| `takt-sim/tests/conformance/conformance_sv_array_tests.rs` | ожидание сторожа 0309: теперь `SE-123` |
| `docs/diagnostics/README.md`, `book/src/appendix-errors/index.typ` | `SE-123` зарегистрирован |

## Проверено

- `cargo test --test semantic aggregate_length` — 5/5.
- Пробы: три входа (лишний, недостача, структура) — `SE-123` у всех девяти
  потребителей; контроль верной длины работает.
- `cargo test --all-features` — провалов нет.
