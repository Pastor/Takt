# Задача 0278-02: Публичность перестаёт прятать мёртвое

> Фича: [../features/0278-compact-implement-dead-branch.md](../features/0278-compact-implement-dead-branch.md) · ADR: [../adr/0278-compact-implement-dead-branch.md](../adr/0278-compact-implement-dead-branch.md) · анализ: [../analyze/0278-compact-implement-dead-branch.md](../analyze/0278-compact-implement-dead-branch.md)

## Что было

Мёртвая функция дожила четыре с половиной месяца не случайно: `pub fn` в
`pub mod extend` **глушит `dead_code`** — компилятор молчит о неиспользуемом
элементе, потому что тот виден наружу крейта.

Замер 2026-08-19 показал, что запись в модуле не одна:

| Элемент | Кто зовёт вне тестов |
|---|---|
| `pub fn compact_implement` | никто (снято задачей 0278-01) |
| `pub fn unroll_extend_expression` | `tree.rs` — внутри крейта |
| `Extend::is_model` / `is_parentless` / `is_sequence` / `is_parallel` | никто |

Снаружи крейта из `semantic::extend` нужны только **типы** `Extend` и
`ParameterArgument` (`takt-sim/src/unit/builder.rs` и два теста).

## Что сделано

- `unroll_extend_expression` сужена до `pub(crate)`;
- четыре предиката `Extend` удалены — после сужения их не звал никто, кроме
  собственного теста, и это назвал компилятор;
- `ModelNode::new` помечен `#[cfg(test)]`: конструктор завёл снятый проход, и
  единственными его потребителями остались юнит-тесты. Атрибут — сторож:
  появится настоящий вызов, атрибут снимут вместе с ним.

Результат: `dead_code` включён как сторож **по построению** — следующий мёртвый
элемент модуля назовёт компилятор, а не замер через четыре месяца.

## Проверки

```sh
cargo clippy --all-targets --all-features   # без предупреждений
cargo test --all-features                   # 3291 тест, 0 провалов
```

Сужение видимости проверяется сборкой `takt-sim` и интеграционных тестов: они
используют только типы, и обе цели собираются.
