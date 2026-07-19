# Задача 0046-01: Ужатие `Location::Source` до `u32×3` (`Diagnostic` < 128 байт)

> Фича: [../features/0046-build-warnings-cleanup.md](../features/0046-build-warnings-cleanup.md) · ADR: [../adr/0046-build-warnings-cleanup.md](../adr/0046-build-warnings-cleanup.md) · анализ: [../analyze/0046-build-warnings-cleanup.md](../analyze/0046-build-warnings-cleanup.md) · тест-план: [../tests/0046-build-warnings-cleanup.md](../tests/0046-build-warnings-cleanup.md)

## Что было

`Location::Source(u64, usize, usize)` = 32 байта → `Diagnostic` = **136** байт >
порога 128 линта `clippy::result_large_err` (включён по умолчанию в clippy 0.1.99).
**414** предупреждений на 203+ `Result<_, Diagnostic>`.

## Что сделано

1. **`diagnostics.rs`:** `Source(u32, u32, u32)` (вариант 16 байт → `Diagnostic`
   120 байт). Аксессоры (`start`/`end`/`range`/`try_start`/`try_end`/`try_range`)
   кастуют `u32 → usize` внутри — публичный API методов неизменен. Новый
   хелпер-конструктор `Location::source(file: u64, start: usize, end: usize)`
   кастует «широкие» типы вызывающего в `u32`.
2. **Конструирование (160 в `grammar.lalrpop` + 9 в лексере)** — sed
   `Location::Source(` → `Location::source(` (всё в action-коде — конструирование,
   не паттерны). `grammar.rs` генерируется в `OUT_DIR` из `.lalrpop`.
3. **Разбор** (`index.rs`, `docs`, `comments`, `lib.rs`, `address_map`, `lsp/*`,
   тесты) — `as usize` у места использования смещений, `as u64` при сравнении
   `file_no` с `ROOT_FILE_NO`/передаче в `path`.

## Проверки

- `cargo build --all-targets --all-features` — 0 ошибок; `result_large_err` = 0.
- `cargo test -p grammar --all-features` — позиции/LSP/диагностики зелёные
  (усечение `usize → u32` безопасно: смещения `.lam` малы).
- `git diff examples/generated` — пусто (вывод не зависит от типа `Location`).
