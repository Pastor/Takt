# Тест-план 0345: агрегат в локальном объявлении

> Фича: [../features/0345-local-aggregate.md](../features/0345-local-aggregate.md) · ADR: [../adr/0345-local-aggregate.md](../adr/0345-local-aggregate.md) · отчёт: [../reports/0345-local-aggregate.md](../reports/0345-local-aggregate.md)

## Условия

| # | Условие | Проверка |
|---|---|---|
| П1 | `st`: поэлементно, `iec2c` принимает | `st_local_aggregate_is_accepted` |
| П2 | `sv`: поэлементно, линт чист | `sv_local_aggregate_is_accepted` |
| П3 | Значения совпадают с эталоном | `local_aggregate_matches_generated_st` |
| П4 | Обычный инициализатор не изменился | контроль в тех же тестах |
| П5 | Предкоммит | `./scripts/precheck.sh` |

## Почему тесты именно такие

Поэлементная запись, перепутавшая поля, компилируется — вердикт даёт сверка
значений.
