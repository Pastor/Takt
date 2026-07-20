# Отчёт о тестировании фичи 0081: `lamc compile` печатает предупреждения

> Фича: [../features/0081-lamc-print-warnings.md](../features/0081-lamc-print-warnings.md) · тест-план: [../tests/0081-lamc-print-warnings.md](../tests/0081-lamc-print-warnings.md) · ADR: [../adr/0081-lamc-print-warnings.md](../adr/0081-lamc-print-warnings.md)

- **Дата:** 2026-07-20
- **Окружение:** macOS (darwin 25.5.0), cargo nightly.
- **Вердикт:** **готово.** `cli_warnings_tests` (4) зелёные, `precheck.sh` зелёный,
  код возврата и вывод целей не изменились.

## Сверка с критериями приёмки (ADR 0081)

| Критерий | Проверка | Результат |
|---|---|---|
| **A1** `SE-036` на неиспользуемой переменной | `unused_variable_warning_is_printed` | ✅ stderr содержит `SE-036`, формат «Предупреждение» |
| **A2** `SE-037` на недетерминизме | `nondeterministic_transition_warning_is_printed` | ✅ |
| **A3** `--quiet` глушит | `quiet_suppresses_warnings` | ✅ |
| **A4** чистый файл молчит | `clean_model_has_no_warnings` | ✅ (нет ложных) |
| **A5** rc не изменился | `precheck.sh` + прогон корпуса | ✅ корпус собирается, rc=0 |
| **A6** язык не изменился | — | ✅ версия не поднята |

## Замер корпуса

`lamc compile` по всем `examples/*.lam` даёт **одно** предупреждение —
`elevator.lam`: `SE-036` (переменная `action` не используется). Реальная находка,
не ложное срабатывание; прочие категории (`unreachable`/`constant_condition`/
`ltl`/`stray_semicolon`/`unknown_named_block`) на корпусе молчат. Набор безопасен.

## Наблюдения

- **Механизм печати уже существовал**, но был частичным: только два
  адрес-специфичных предупреждения и только для не-адрес-потребляющих целей.
  0081 обобщил его до единой точки `collect_model_warnings` для всех целей.
- **Формат унифицирован** с ошибкой (`print_compile_error`): позиция + код + текст.

## Кандидаты (вне объёма)

- **A-2:** чистка `elevator.lam` (unused `action`).
- **A-1:** двойное построение модели в CLI — оптимизация при необходимости.
