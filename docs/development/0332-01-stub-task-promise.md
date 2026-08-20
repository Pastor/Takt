# Разработка 0332-01: второй признак заглушки

> Фича: [../features/0332-stub-task-promise.md](../features/0332-stub-task-promise.md) · ADR: [../adr/0332-stub-task-promise.md](../adr/0332-stub-task-promise.md)

## Состав правки

| Файл | Что сделано |
|---|---|
| `scripts/check-stub-branches.py` | второй признак (`TASK_REFERENCE`), сужение до строк с `with_code`; самопроверка пополнена тремя случаями |
| `takt-lang/src/generator/st/st_expr.rs` | четыре текста переписаны на причину; две ветви названы недостижимыми |
| `takt-lang/src/semantic/ltl_check.rs` | из текста `SE-055` убраны номера фич |

## Проверено

- `python3 scripts/check-stub-branches.py --self-test` — все условия.
- `python3 scripts/check-stub-branches.py` — 2 объявлено, расхождений нет.
- Прогон: новый текст отказа `st` на `return {1, 2};` называет причину.
- `cargo test --all-features` — провалов нет.
