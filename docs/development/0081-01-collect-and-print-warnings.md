# Задача 0081-01: единая точка сбора и печати предупреждений в `lamc`

> Фича: [../features/0081-lamc-print-warnings.md](../features/0081-lamc-print-warnings.md) · ADR: [../adr/0081-lamc-print-warnings.md](../adr/0081-lamc-print-warnings.md)

## Что было

`lamc compile` печатал только `address_expr`/`address_map_overlay` и только для
не-адрес-потребляющих целей. `unused_variable_warnings` (`SE-036`),
`nondeterministic_transition_warnings` (`SE-037`) и прочие предупреждения
публичного API из CLI **не вызывались вовсе**.

## Что сделано

1. **`print_compile_warning`** (`bin/lamc.rs`) — формат общий с
   `print_compile_error` (позиция + код + текст + примечания).
2. **`semantic::warnings::collect_model_warnings(ast, model)`** — единая точка
   сбора всех предупреждений публичного API над построенной моделью (`SE-036`,
   `SE-037/042`, `unreachable`, `constant_condition`, `ltl`, `stray_semicolon`,
   `unknown_named_block`). Вынесена из `bin/lamc.rs` в новый модуль
   `grammar/src/semantic/warnings.rs`: `bin/lamc.rs` и `lib.rs` пришпилены к лимиту
   размера, добавление в них отвергается гейтом.
3. **Печать для всех целей** (было — не-адресные): модель строится один раз,
   предупреждения печатаются перед генерацией, `--quiet` уважается.
4. **Адрес-предупреждения оставлены отдельно** — только для целей, адрес не
   потребляющих (у `c-hal`/`st-at` те же ситуации — ошибки).

## Проверки

- **T1/A1:** `SE-036` на `unused_variable.lam` — `cli_warnings_tests`.
- **T2/A2:** `SE-037` на `nondeterministic_warn.lam`.
- **T3/A3:** `--quiet` глушит.
- **T4/A4:** `all_vars_used.lam` — без предупреждений.
- **T5/A5:** `precheck.sh` зелёный; rc и вывод целей не изменились.
