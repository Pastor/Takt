# Тест-план 0336: приведение в аргументе и возврате

> Фича: [../features/0336-call-return-coercion.md](../features/0336-call-return-coercion.md) · ADR: [../adr/0336-call-return-coercion.md](../adr/0336-call-return-coercion.md) · отчёт: [../reports/0336-call-return-coercion.md](../reports/0336-call-return-coercion.md)

## Условия

| # | Условие | Проверка |
|---|---|---|
| П1 | `rust`: аргумент, литерал `bit`, вариант перечисления, возврат | `rust_call_and_return_compile` (+ настоящий `rustc`) |
| П2 | `st`: `BOOL_TO_<тип>` в обеих позициях, `iec2c` принимает | `st_call_and_return_accepted_by_iec2c` |
| П3 | `sv`: размерная форма в обеих позициях, линт чист | `sv_call_and_return_have_no_width_warning` |
| П4 | Значения совпадают с эталоном | `call_coercion_matches_simulator_and_generated_rust` |
| П5 | Контрольный вход не изменился | `plain = 42` в том же ожидании |
| П6 | Мутации ловятся | две мутации валят разные тесты |
| П7 | Предкоммит | `./scripts/precheck.sh` |

## Почему тесты именно такие

Сборка доказывает валидность, числа — верность: обёртка, берущая не тот разряд,
собирается прекрасно. Поэтому есть и то, и другое.
