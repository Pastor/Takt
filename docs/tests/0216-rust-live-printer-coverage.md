# Тест-план фичи 0216: Печатник живости цели `rust`

> Фича: [../features/0216-rust-live-printer-coverage.md](../features/0216-rust-live-printer-coverage.md) · анализ: [../analyze/0216-rust-live-printer-coverage.md](../analyze/0216-rust-live-printer-coverage.md) · отчёт: [../reports/0216-rust-live-printer-coverage.md](../reports/0216-rust-live-printer-coverage.md)

## Предмет проверки

Формы, в которые печатается локальное объявление, и приёмка вывода линтом.
Проверяется **поведение**, а не процент покрытия (ADR 0138: порога нет
намеренно, и предупреждение карточки — «покрыли процент, а не поведение»).

## Условия проверок

| # | Условие | Как проверяется | Ожидаемый результат |
|---|---|---|---|
| П1 | Обе ветки пишут → свёртка в значение | `both_branches_fold_into_value` | `let ds: u8 = if a > b {`, без `mut` |
| П2 | Безусловная запись → свёртка | `unconditional_assign_folds_into_value` | `let t: u8 = a;` |
| П3 | Второе присваивание → `mut` | `second_assignment_requires_mut` | `let mut t: u8 = a;` |
| П4 | `match` с `_` → цепочка | `exhaustive_match_folds_into_chain` | `let t: u8 = if a == 1 { 5 } else { 7 };` |
| П5 | `for` с `init` → нет мёртвого значения | `for_init_kills_initializer` | `let mut i: u8;` |
| П6 | Две переменные на одном операторе | `two_variables_share_one_statement` | `let x: u8;` и `let y: u8;` |
| П7 | **Контрпример:** `if` без `else` | `if_without_else_keeps_initializer` | `let mut t: u8 = 0;` |
| П8 | **Контрпример:** цикл | `loop_keeps_initializer` | значение живо |
| П9 | **Контрпример:** чтение до записи | `read_before_assign_keeps_initializer` | значение живо |
| П10 | **Контрпример:** `match` без `_` | `open_match_keeps_initializer` | значение живо |
| П11 | Вывод принимается линтом | `generated_module_passes_clippy_gate` | `clippy -D warnings` — код 0 |
| П12 | Регрессия | `cargo test --all-features` | все наборы зелёные |
| П13 | Корпус и гейт цели | `./scripts/precheck.sh` | код 0 |

## Примеры и контрпримеры (правило 16)

**Пример** (дефект, исправленный фичей):

```takt
fn matched(a: u8) -> u8 {
    var t: u8 := 0;
    match a { 1 => { t := 5; } _ => { t := 7; } }
    return t;
}
```

Прежде: `let mut t: u8 = 0;` → `error: value assigned to t is never read`.
Теперь: `let t: u8 = if a == 1 { 5 } else { 7 };`.

**Контрпример** (перезапись не доказана — значение обязано остаться):

```takt
fn matched_open(a: u8) -> u8 {
    var t: u8 := 0;
    match a { 1 => { t := 5; } 2 => { t := 6; } }
    return t;
}
```

⚠️ Без контрпримера правка читалась бы как «любой `match` затирает»: путь мимо
всех образцов оставил бы переменную неинициализированной.

## Мутационные проверки

- **М1.** Убрать требование ветви `_` в разборе `match` → П10 обязана
  провалиться.
- **М2.** Вернуть `Verdict::Unknown` для `for` → П5 и П11 обязаны провалиться.
- **М3.** Печатать свёртку `match` отложенной формой (`let t: u8;`) → П11
  обязана провалиться (`needless_late_init`).
