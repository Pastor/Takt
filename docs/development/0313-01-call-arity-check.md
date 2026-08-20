# Разработка 0313-01: проверка арности вызова

> Фича: [../features/0313-call-arity-check.md](../features/0313-call-arity-check.md) · ADR: [../adr/0313-call-arity-check.md](../adr/0313-call-arity-check.md)

## Состав правки

| Файл | Что сделано |
|---|---|
| `takt-lang/src/semantic/validate/arity.rs` | новая проверка `check_call`: разбор всех видов `FunctionDefinitionNode`, диагностика `SE-122` |
| `takt-lang/src/semantic/validate/common.rs` | ветвь `Function` судьи зовёт проверку до обхода аргументов |
| `takt-sim/src/unit/statement.rs` | текст `SIM-020` обобщён — он больше не называет `S(Модель)` причиной |
| `takt-lang/tests/semantic/call_arity_tests.rs` | пять проверок: локальная, встроенная, лишний аргумент, контроль, граница |
| `docs/diagnostics/README.md`, `book/src/appendix-errors/index.typ` | `SE-122` зарегистрирован |

## Проверено

- `cargo test --test semantic call_arity` — 5/5.
- `cargo test --all-features` — провалов нет.
- Проба: `two(1)` и `min(1)` — `SE-122` у всех девяти потребителей.
