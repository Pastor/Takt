# Тест-план 0350: порт составного типа

> Фича: [../features/0350-port-composite-type.md](../features/0350-port-composite-type.md) · ADR: [../adr/0350-port-composite-type.md](../adr/0350-port-composite-type.md) · отчёт: [../reports/0350-port-composite-type.md](../reports/0350-port-composite-type.md)

## Условия

| # | Условие | Проверка |
|---|---|---|
| П1 | `c`: отказ `CC-015` | `c_refuses_composite_port` |
| П2 | `sv`: тип до шапки, линт чист | `sv_struct_port_declares_type_before_module` |
| П3 | `sv`: порт-массив отвергается с причиной | `sv_refuses_array_port` |
| П4 | Снимки `book/` совпадают | гейт 0274 |
| П5 | Предкоммит | `./scripts/precheck.sh` |

## Почему тесты именно такие

Позиция `typedef` проверяется текстом, а выразимость — прогоном **обоих**
инструментов SV: verilator принимает то, что yosys отвергает, и наоборот
(урок 0045).
