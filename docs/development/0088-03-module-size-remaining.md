# Задача 0088-03: Генератор Rust — вынос такта и переходов (`rust_model.rs`)

> Фича: [../features/0088-module-size-remaining.md](../features/0088-module-size-remaining.md) · ADR: [../adr/0088-module-size-remaining.md](../adr/0088-module-size-remaining.md) · анализ: [../analyze/0088-module-size-remaining.md](../analyze/0088-module-size-remaining.md)

## Что было

`grammar/src/generator/rust/rust_model.rs` — **1345 строк** (нарушитель). Содержал
и эмиссию структуры/enum/`new`/`init`, и **такт** (`emit_tick`) с guard-формулами и
всеми видами **переходов** (простые, `= Модель`/`|`/`+`).

## Что сделано

Эмиссия **такта и переходов** вынесена в новый модуль
`grammar/src/generator/rust/rust_tick.rs` (548 строк):

- Функции: `emit_tick`, `emit_guard`, `emit_transitions`, `emit_enter_of`,
  `emit_extend`, `emit_extend_transition`, `call_args`.
- Наружу нужен только `emit_tick` (`pub(crate)`, вызывается `emit_model` в
  остатке `rust_model`) — импортируется `use …rust_tick::emit_tick;`.
- Приватные помощники остатка, понадобившиеся группе, повышены до `pub(crate)`:
  `seq_enum_name`, `seq_field_name`, `needs_hal`, `submodel_name` и **поле**
  `StateTable::emit_end`. Осевшие импорты (`emit_named_blocks`/
  `emit_model_named_blocks`, `condition_as_bool`, `unwrap_outer`, `StmtOutput`,
  `TypeNode`, `Formula`, `StateNode`) удалены из `rust_model`.
- `generator/rust/mod.rs` — добавлено `mod rust_tick;`.

**Чистое перемещение:** тело функций скопировано дословно, вывод Rust неизменен.
`rust_model.rs`: **1345 → 819** (уложился) — запись удалена из реестра
(**16 → 15**). `rust_tick.rs` (548) — не нарушитель.

Стеки: только `grammar`. `simulation` — н/п.

## Проверки

- `cargo build --bin lamc` — без предупреждений.
- `./scripts/precheck.sh` — зелёный (детерминизм-гейт + `conformance_rust_tests`
  подтверждают вывод Rust байт-в-байт неизменным; `check-module-size.sh` учитывает
  −1 запись; все тесты, сборка примеров).
