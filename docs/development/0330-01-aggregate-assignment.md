# Разработка 0330-01: присваивание агрегата у `st` и `sv`

> Фича: [../features/0330-aggregate-assignment.md](../features/0330-aggregate-assignment.md) · ADR: [../adr/0330-aggregate-assignment.md](../adr/0330-aggregate-assignment.md)

## Состав правки

| Файл | Что сделано |
|---|---|
| `takt-lang/src/generator/st/st_stmt.rs` | ветвь присваивания агрегата: поэлементно, значение по типу элемента |
| `takt-lang/src/generator/sv/sv_stmt.rs` | то же для `always_comb` (`a_next[i] = …`) |
| `takt-sim/tests/data/eval/conformance_array_assign.takt` | фикстура: разные значения, меняются по тактам |
| `takt-sim/tests/conformance/conformance_array_assign_tests.rs` | сверка с RTL + прогон `iec2c` на выводе `st` |

## Проверено

- `cargo test --test conformance conformance_array_assign` — 2/2.
- `iec2c` принимает порождённый ST.
- Проба: присваивание агрегата переводят все восемь целей.
