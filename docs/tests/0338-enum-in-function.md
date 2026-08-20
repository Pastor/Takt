# Тест-план 0338: перечисление внутри функции

> Фича: [../features/0338-enum-in-function.md](../features/0338-enum-in-function.md) · ADR: [../adr/0338-enum-in-function.md](../adr/0338-enum-in-function.md) · отчёт: [../reports/0338-enum-in-function.md](../reports/0338-enum-in-function.md)

## Условия

| # | Условие | Проверка |
|---|---|---|
| П1 | `sv`: вариант вместо числа, verilator принимает | `sv_enum_local_is_variant_not_number` |
| П2 | `st`: константы объявлены, `iec2c` принимает | `st_enum_constants_are_declared_inside_function` |
| П3 | Значения совпадают с эталоном | `enum_in_function_matches_generated_st` |
| П4 | Функция без перечисления лишнего не получает | контрольная `plain` в тех же тестах |
| П5 | Предкоммит | `./scripts/precheck.sh` |

## Почему тесты именно такие

Прогон целевых инструментов доказывает **валидность**, сверка значений —
**верность**: дублированная константа с неверным значением компилируется
прекрасно.
