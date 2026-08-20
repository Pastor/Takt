# Разработка 0344-01: порядок функций у цели `st`

> Фича: [../features/0344-st-call-order.md](../features/0344-st-call-order.md) · ADR: [../adr/0344-st-call-order.md](../adr/0344-st-call-order.md)

## Состав правки

| Файл | Что сделано |
|---|---|
| `takt-lang/src/generator/call_order.rs` | **новый**: порядок по графу вызовов |
| `takt-lang/src/generator/st/st_func.rs` | `emit_functions` печатает в этом порядке |
| `takt-sim/tests/data/eval/conformance_call_order.takt` | фикстура: имена подобраны так, что алфавит противоречит вызову |
| `takt-lang/tests/targets/st_call_order_tests.rs` | порядок в тексте **и** прогон `iec2c` |

## Проверено

- Мутация «вернуть алфавитный порядок» валит сторож.
- Вывод корпуса не изменился; `cargo test` зелёный.
