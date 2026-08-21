# Разработка 0349-01: длительность в поле структуры

> Фича: [../features/0349-duration-field.md](../features/0349-duration-field.md) · ADR: [../adr/0349-duration-field.md](../adr/0349-duration-field.md)

## Состав правки

| Файл | Что сделано |
|---|---|
| `takt-lang/src/generator/st/st_expr.rs` | `inner_expr_type_in` — тип с доступом к объявлениям модели; приведение спрашивает его |
| `takt-lang/src/generator/sv/sv_const.rs` | размерная форма применяется по **напечатанному** тексту, а не по виду узла |
| `takt-lang/src/generator/st/st_operand_type.rs` | **новый**: тип операнда выделен из `st_expr` по границе ответственности (печать выражения против вопроса «какого типа операнд») |
| `takt-sim/tests/data/eval/conformance_duration_field.takt` | фикстура: поле-длительность плюс контрольный массив |
| `takt-lang/tests/targets/duration_field_tests.rs` | `st` и `sv`: текст **и** прогон инструментов |

## Проверено

- Полный цикл ST (`iec2c` → `cc` → прогон): `sum = 7`, как у эталона.
- `verilator -Wall` чист; вывод корпуса не изменился.
