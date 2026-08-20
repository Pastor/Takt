# Разработка 0340-01: место записи агрегата

> Фича: [../features/0340-aggregate-assign-place.md](../features/0340-aggregate-assign-place.md) · ADR: [../adr/0340-aggregate-assign-place.md](../adr/0340-aggregate-assign-place.md)

## Состав правки

| Файл | Что сделано |
|---|---|
| `takt-lang/src/generator/aggregate.rs` | **новый**: общий носитель правила «куда писать элемент» |
| `takt-lang/src/generator/c/c_expr/aggregate.rs` | **новый**: разворот присваивания агрегата в операторы |
| `takt-lang/src/generator/c/c_expr/stmt.rs` | вызов разворота перед общей печатью выражения |
| `takt-lang/src/generator/st/st_stmt.rs` | место записи берётся у носителя |
| `takt-lang/src/generator/sv/sv_stmt.rs` | то же |
| `takt-lang/src/generator/sv/sv_fsm.rs`, `sv_expr.rs` | карта полей структур в `Fsm` и `Scope` |
| `takt-lang/src/generator/sv/sv_scope.rs` | **новый**: контекст печати выделен из `sv_expr` по границе ответственности (печать выражений против вопросов о среде печати) |
| `takt-sim/tests/data/eval/conformance_struct_assign.takt` | фикстура: структура плюс контрольный массив |
| `takt-lang/tests/targets/aggregate_assign_targets_tests.rs` | три цели: текст **и** прогон `cc`, `iec2c`, `verilator` |
| `takt-sim/tests/conformance/conformance_struct_assign_tests.rs` | сверка **значений** с порождённым C |

## Проверено

- Вывод корпуса не изменился; `cargo test` зелёный.
- Контрольный вход (массив) сохранил индексную форму у всех трёх целей.
