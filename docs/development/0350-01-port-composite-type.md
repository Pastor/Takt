# Разработка 0350-01: порт составного типа

> Фича: [../features/0350-port-composite-type.md](../features/0350-port-composite-type.md) · ADR: [../adr/0350-port-composite-type.md](../adr/0350-port-composite-type.md)

## Состав правки

| Файл | Что сделано |
|---|---|
| `takt-lang/src/generator/c/mod.rs` | предикат `PortClass::fits_hal` |
| `takt-lang/src/generator/c/c_ports.rs` | **новый**: проверка типов портов (граница ответственности — `c_header` печатает, здесь отвечают «ложится ли порт на HAL») |
| `takt-lang/src/generator/c/c_header.rs` | зовёт проверку перед сбором портов |
| `takt-lang/src/generator/sv/mod.rs` | пользовательские типы печатаются **вне** модуля |
| `takt-lang/src/generator/sv/sv_module.rs` | порт-массив отвергается `SV-002` с причиной |
| `takt-lang/tests/targets/port_composite_tests.rs` | три случая: отказ `c`, порядок и линт `sv`, отказ на массиве |

## Проверено

- `verilator` и `yosys` принимают модуль с портом-структурой.
- Снимки `book/` совпадают; вывод корпуса не изменился.
