# Тест-план 0347: константа-агрегат

> Фича: [../features/0347-const-aggregate.md](../features/0347-const-aggregate.md) · ADR: [../adr/0347-const-aggregate.md](../adr/0347-const-aggregate.md) · отчёт: [../reports/0347-const-aggregate.md](../reports/0347-const-aggregate.md)

## Условия

| # | Условие | Проверка |
|---|---|---|
| П1 | `rust`: литерал структуры, сборка чиста | `rust_const_struct_compiles` |
| П2 | `sv`: агрегат в `localparam`, линт чист | `sv_const_aggregate_is_accepted` |
| П3 | Типы объявлены раньше констант | там же (позиции в тексте) |
| П4 | Константа-скаляр не изменилась | контроль в тесте `rust` |
| П5 | Предкоммит | `./scripts/precheck.sh` |

## Почему тесты именно такие

Порядок разделов проверяется **позициями в тексте**, а валидность — прогоном
verilator: линт видит нарушение, а проверка позиций объясняет, **что** именно
нарушено.
