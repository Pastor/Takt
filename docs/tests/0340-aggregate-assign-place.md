# Тест-план 0340: место записи агрегата

> Фича: [../features/0340-aggregate-assign-place.md](../features/0340-aggregate-assign-place.md) · ADR: [../adr/0340-aggregate-assign-place.md](../adr/0340-aggregate-assign-place.md) · отчёт: [../reports/0340-aggregate-assign-place.md](../reports/0340-aggregate-assign-place.md)

## Условия

| # | Условие | Проверка |
|---|---|---|
| П1 | `c`: поэлементно, `cc -Werror` принимает | `c_aggregate_assignment_compiles` |
| П2 | `st`: имя поля, `iec2c` принимает | `st_aggregate_assignment_is_accepted` |
| П3 | `sv`: имя поля, линт чист | `sv_aggregate_assignment_is_accepted` |
| П4 | Массив по индексу (контроль) | в тех же тестах |
| П5 | Значения совпадают с эталоном | `aggregate_assignment_matches_simulator_and_generated_c` |
| П6 | Предкоммит | `./scripts/precheck.sh` |

## Почему тесты именно такие

Поэлементная запись, перепутавшая поля местами, **компилируется** — агрегат
позиционный, и вердикт даёт только сверка значений.
