# Анализ фичи 0081: `lamc compile` печатает предупреждения

> Фича: [../features/0081-lamc-print-warnings.md](../features/0081-lamc-print-warnings.md) · ADR: [../adr/0081-lamc-print-warnings.md](../adr/0081-lamc-print-warnings.md)

## Зависимости

- **Нет.** Опора — публичный API предупреждений (`lib.rs`, фичи 0004/0005/0035/0042)
  и печать позиций (0053). Разблокирует доставку **всех** будущих предупреждений.
- **Приоритет / Tier:** **Tier 2** — не «ничего не сломано» (диагностика есть, но
  не доходит до пользователя), однако и не Tier 1 (корпус собирается, rc=0).

## Точки интеграции (замер по коду)

| Что | Где | Как |
|---|---|---|
| Печать ошибки | `bin/lamc.rs::print_compile_error` | образец формата (позиция+код) |
| Частичная печать предупреждений | `bin/lamc.rs` (было ~991–1022) | только `address_expr`/`overlay`, только не-адресные цели |
| API предупреждений | `lib.rs` | `unused_variable_warnings` (`SE-036`), `nondeterministic_transition_warnings` (`SE-037/042`), `unreachable_state`, `constant_condition`, `ltl`, `stray_semicolon`, `unknown_named_block` |

## Ключевые решения анализа

1. **Вход функций смешанный:** большинство берут `Rc<RefCell<ModelNode>>`,
   `stray_semicolon`/`unknown_named_block` — `&ast::Model`. `collect_model_warnings`
   принимает **оба** (`parse` даёт ast, `construct_model` — модель).
2. **Адрес-предупреждения оставлены отдельно:** они зависят от цели (у `c-hal`/
   `st-at` — ошибки), нельзя влить в общий сбор.
3. **Замер корпуса:** один `SE-036` на `elevator.lam` (реальный unused `action`);
   прочие категории молчат → набор безопасен, шума нет.

## Риски

- **Двойное построение модели** (A-1 ADR) — приемлемо для CLI.
- **Шум на корпусе** — снят замером (0 ложных; 1 реальная находка).
