# Задача 0151-01: Накопление диагностик внутри отдельной проверки `validate`

> Фича: [../features/0151-diagnostics-batch-within-check.md](../features/0151-diagnostics-batch-within-check.md) · ADR: [../adr/0151-diagnostics-batch-within-check.md](../adr/0151-diagnostics-batch-within-check.md) · анализ: [../analyze/0151-diagnostics-batch-within-check.md](../analyze/0151-diagnostics-batch-within-check.md)

## Что было

```rust
let single: [Result<(), Diagnostic>; 11] = [ … ];
found.extend(single.into_iter().filter_map(Result::err));
```

Одиннадцать проверок, каждая — цикл с `?`: **не более одной** ошибки на модель.
Две неверные переменные давали одно сообщение.

## Что сделано

**Правило названо:** одна диагностика на **элемент**; высказываются все
элементы. Элемент — объявление, ребро перехода, именованное условие, оператор
`address`. Внутри одного выражения ранний выход **сохранён**: вторая ошибка там
почти всегда следствие первой.

Все одиннадцать переведены на `Vec<Diagnostic>`; внешние циклы накапливают:

| Файл | Проверки |
|---|---|
| `states.rs` | `model_only_one_start_state`, `validate_state_references` |
| `ports.rs` | `validate_variables`, `check_port_addresses` |
| `enums.rs` | `validate_bit_values`, `validate_enum_values`, `validate_enum_type_declarations` |
| `types.rs` | `check_array_sizes` |
| `common.rs` | `validate_conditions` |
| `fixed.rs` | `check_fixed_mixing` |
| `anon_init.rs` | `validate_anon_in_initializers` |

⚠️ `check_port_addresses` внутри цикла имела **три** `return Err` — они
заменены на `push` + `continue`: привязка адреса, у которой уже нашлась ошибка,
дальше не проверяется, но следующая проверяется обязательно.

## Проверки

`takt-lang/tests/validate_batch_tests.rs` — 10 тестов: по одному на каждый из
шести измеренных классов, плюс сохранение свойства 0130, граница стадий
построения, «одна диагностика на элемент» и порядок по позиции.

- `cargo test --all-features` — весь набор зелен.
- `./scripts/precheck.sh` — код возврата 0.

**Пригодность проверок подтверждена мутацией:**

| Мутация | Ожидание | Факт |
|---|---|---|
| вернуть ранний выход в `validate_bit_values` | падают тест класса и тест порядка | подтверждено |
| вернуть ранний выход в `validate_conditions` | падает тест именованных условий | подтверждено **со второй попытки** |

⚠️ Вторая мутация сначала **не поймалась**: первый тест на `SE-025` подавал
условия на **рёбрах** (`validate_state_references`), а тот же код рождается ещё
и в объявлении `cond` (`validate_conditions`). Один код — два места; тест,
покрывающий одно, молча оставляет второе. Добавлен
`two_unresolved_named_conditions_give_two_diagnostics`.

## Что осталось за задачей

Две независимые ошибки **внутри одного выражения** по-прежнему дают одну — это
граница принятого Option A, а не долг: чтобы отличать причину от следствия
внутри выражения, нужен анализ другого порядка сложности.
