# Разработка 0345-01: агрегат в локальном объявлении

> Фича: [../features/0345-local-aggregate.md](../features/0345-local-aggregate.md) · ADR: [../adr/0345-local-aggregate.md](../adr/0345-local-aggregate.md)

## Состав правки

| Файл | Что сделано |
|---|---|
| `takt-lang/src/generator/st/st_stmt.rs` | ветвь `Variable`: агрегат печатается поэлементно |
| `takt-lang/src/generator/sv/sv_stmt.rs` | то же |
| `takt-sim/tests/data/eval/conformance_local_aggregate.takt` | фикстура: функция со структурой плюс контрольная без агрегата |
| `takt-sim/tests/conformance/conformance_st_tests/local_aggregate.rs` | сверка **значений** с порождённым ST |
| `takt-lang/tests/targets/local_aggregate_tests.rs` | `st` и `sv`: текст **и** прогон инструментов |

## Проверено

- Исходная проба («функция возвращает структуру») теперь переводится всеми
  восемью целями; все четыре инструмента чисты.
- Вывод корпуса не изменился; `cargo test` зелёный.
