# Фича 0081: `lamc` не печатает предупреждения вообще

- **Номер:** 0081
- **Статус:** ГОТОВО (2026-07-20; `precheck.sh` зелёный)
- **Зависит от:** нет (опора — публичный API предупреждений и печать позиций 0053)
- **Приоритет / Tier:** **Tier 2** — диагностика есть, но до пользователя не доходит; корпус собирается (rc=0)
- **Крейт:** `grammar` (`bin/lamc.rs`)
- **Связанные issue (анализ):** новая фича (перевод кандидата из `FEATURES.md`)

## Ссылки на артефакты (жизненный цикл, правило 17)

| Стадия | Артефакт |
|---|---|
| Архитектура (ADR) | [`docs/adr/0081-lamc-print-warnings.md`](../adr/0081-lamc-print-warnings.md) |
| Анализ | [`docs/analyze/0081-lamc-print-warnings.md`](../analyze/0081-lamc-print-warnings.md) |
| Разработка | [`docs/development/0081-01-collect-and-print-warnings.md`](../development/0081-01-collect-and-print-warnings.md) |
| Тест-план | [`docs/tests/0081-lamc-print-warnings.md`](../tests/0081-lamc-print-warnings.md) |
| Отчёт о тестировании | [`docs/reports/0081-lamc-print-warnings.md`](../reports/0081-lamc-print-warnings.md) |
| Исправления | [`docs/fixes/`](../fixes/README.md) (не потребовались) |

## Краткое описание

`unused_variable_warnings` и `nondeterministic_transition_warnings` — часть
**публичного API** (`lib.rs`), но из CLI **не вызываются**: пользователь не видит
ни Ce13, ни Ce14.

Влияет на доставку **любых новых** предупреждений (SE-053/SE-054 из 0035 и 0042):
диагностика, которую никто не печатает, равносильна её отсутствию.

Выявлено при [0035](0035-ltl-in-blocks.md).

## Итог (что сделано) — 2026-07-20

Принята **Option A** (ADR 0081): единая точка сбора
`semantic::warnings::collect_model_warnings(ast, model)` (вынесена в
`grammar/src/semantic/warnings.rs` — `bin/lamc.rs` и `lib.rs` пришпилены к лимиту
размера) над построенной моделью + печать inline форматом, общим с ошибкой
(позиция + код), для **всех** целей, уважая `--quiet`. Набор:
`unused_variable` (`SE-036`), `nondeterministic` (`SE-037/042`),
`unreachable_state`, `constant_condition`, `ltl`, `stray_semicolon`,
`unknown_named_block`; адрес-предупреждения (`address_expr`/`overlay`) —
по-прежнему только для целей, адрес не потребляющих.

**Замер корпуса:** один `SE-036` — `examples/elevator.lam` (unused `action`,
реальная находка; кандидат на чистку вне объёма); прочие категории молчат.

**Проверки** — `grammar/tests/cli_warnings_tests.rs` (прогон бинаря, перехват
stderr): `SE-036`/`SE-037` доезжают, `--quiet` глушит, чистый файл молчит. Код
возврата не изменился (rc=0), `precheck.sh` зелёный. Язык не менялся, версия не
поднята.

## История

> Фича зарегистрирована **2026-07-17** переводом кандидата из `FEATURES.md`
> (решение заказчика: «завести фичи по кандидатам, пока без проработки»).
> Проработана и закрыта 2026-07-20. Текст ниже — исходная находка кандидата.
