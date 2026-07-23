# Задача 0088-02: Генератор Rust — вынос печатника условий (`rust_expr.rs`)

> Фича: [../features/0088-module-size-remaining.md](../features/0088-module-size-remaining.md) · ADR: [../adr/0088-module-size-remaining.md](../adr/0088-module-size-remaining.md) · анализ: [../analyze/0088-module-size-remaining.md](../analyze/0088-module-size-remaining.md)

## Что было

`grammar/src/generator/rust/rust_expr.rs` — **1112 строк** (нарушитель лимита).
Содержал и печатник **выражений** (`print_expression`), и печатник **условий**
(`print_condition` и спутники) — две разные ветви (ADR 0019: у `=` разная
семантика).

## Что сделано

Печатник **условий** вынесен в новый модуль
`grammar/src/generator/rust/rust_cond.rs` (281 строка):

- Функции: `print_condition`, `state_comparison`, `model_of`, `cond_binary`,
  `cond_bool_binary`, `condition_type`, `condition_as_bool`.
- Наружу нужен только `condition_as_bool` (потребитель — `rust_model.rs`);
  путь сохранён **реэкспортом** `pub(crate) use …rust_cond::condition_as_bool;`
  в `rust_expr.rs` — импорт в `rust_model.rs` **не менялся** (правило 11).
- Приватные функции `rust_expr`, понадобившиеся вынесенному блоку, повышены до
  `pub(crate)`: `variable`, `rational`, `bit_mask`, `member_index`,
  `call_arguments`, метод `Scope::hal_receiver`. Осевшие импорты (`ConditionNode`,
  `function_return`) удалены.
- `generator/rust/mod.rs` — добавлено `mod rust_cond;`.

**Чистое перемещение:** тело функций скопировано дословно, вывод Rust неизменен.
`rust_expr.rs`: **1112 → 847** (уложился) — запись **удалена** из реестра
(**17 → 16**). `rust_cond.rs` (281) — не нарушитель.

Стеки: только `grammar`. `simulation` — н/п.

## Проверки

- `cargo build --bin lamc` — без предупреждений.
- `./scripts/precheck.sh` — зелёный (детерминизм-гейт + `conformance_rust_tests`
  подтверждают вывод Rust байт-в-байт неизменным; `check-module-size.sh` учитывает
  −1 запись; все тесты, сборка примеров).
