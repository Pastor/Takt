# Тест-план 0337: неиспользуемый параметр функции

> Фича: [../features/0337-unused-function-parameter.md](../features/0337-unused-function-parameter.md) · ADR: [../adr/0337-unused-function-parameter.md](../adr/0337-unused-function-parameter.md) · отчёт: [../reports/0337-unused-function-parameter.md](../reports/0337-unused-function-parameter.md)

## Условия

| # | Условие | Проверка |
|---|---|---|
| П1 | `c`: заглушка, `cc -Werror` молчит | `c_unused_parameter_is_guarded` |
| П2 | `rust`: заглушка, сборка под `-D warnings` | `rust_unused_parameter_is_guarded` |
| П3 | `sv`: поглощение, `verilator -Wall` молчит | `sv_unused_parameter_is_absorbed` |
| П4 | Используемый параметр заглушки не получает | контрольная функция `echo` в тех же тестах |
| П5 | Имя параметра в сигнатуре прежнее | проверка текста у цели `rust` |
| П6 | `lint_off` не появился | проверка текста у цели `sv` |
| П7 | Предкоммит | `./scripts/precheck.sh` |

## Почему тесты именно такие

Предмет — принимает ли вывод **целевой инструмент** под политикой проекта,
поэтому все три теста запускают настоящие `cc`, `rustc`, `verilator`.
Контрольная функция обязательна: без неё правило читалось бы как «заглушка
всегда», и лишняя строка ушла бы в вывод незамеченной.
