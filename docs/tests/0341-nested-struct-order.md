# Тест-план 0341: вложенная структура

> Фича: [../features/0341-nested-struct-order.md](../features/0341-nested-struct-order.md) · ADR: [../adr/0341-nested-struct-order.md](../adr/0341-nested-struct-order.md) · отчёт: [../reports/0341-nested-struct-order.md](../reports/0341-nested-struct-order.md)

## Условия

| # | Условие | Проверка |
|---|---|---|
| П1 | `c`: порядок и сборка | `c_nested_struct_compiles` |
| П2 | `st`: порядок и `iec2c` | `st_nested_struct_is_accepted` |
| П3 | `sv`: порядок и линт | `sv_nested_struct_is_accepted` |
| П4 | `rust`: поле переводится, сборка чиста | `rust_nested_struct_compiles_and_bit_stays_boolean` |
| П5 | Разряд остался логическим (контроль) | там же |
| П6 | Носитель порядка: зависимость, массив структур, цикл | юнит-тесты `struct_order` |
| П7 | Предкоммит | `./scripts/precheck.sh` |

## Почему тесты именно такие

Порядок проверяется **позицией в тексте** (что раньше — `Point` или `Line`), а
валидность — прогоном настоящих инструментов: линт видит порядок, а тест
позиций объясняет, **почему** он такой.
