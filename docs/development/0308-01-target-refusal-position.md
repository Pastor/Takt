# Разработка 0308-01: общий носитель позиции оператора

> Фича: [../features/0308-target-refusal-position.md](../features/0308-target-refusal-position.md) · ADR: [../adr/0308-target-refusal-position.md](../adr/0308-target-refusal-position.md)

## Состав правки

| Файл | Что сделано |
|---|---|
| `takt-lang/src/generator/site.rs` | структура `StatementSite` заменена потоковым носителем: `enter`, `current`, `at`, `reset` |
| `takt-lang/src/generator/mod.rs` | `site::reset()` на входе в `generate` |
| `takt-lang/src/generator/c/c_map.rs`, `c/c_expr/*` | поле карты снято, вызовы переведены на общий носитель |
| `takt-lang/src/generator/rust/rust_stmt.rs`, `rust_expr.rs` | `enter` в печатнике операторов, `at` в `unsupported` |
| `takt-lang/src/generator/st/st_stmt.rs`, `st_expr.rs`, `st_func.rs`, `st_fixed.rs`, `st_compose.rs` | три копии `unsupported` сведены в одну (в `st_expr`), `enter`/`at` подключены |
| `takt-lang/src/generator/sv/sv_stmt.rs`, `sv_expr.rs`, `sv_fsm.rs`, `sv_type.rs`, `sv_mmio.rs` | `enter` в печатнике операторов, `at` в отказах |
| `takt-lang/tests/data/site0308/*.takt` | три фикстуры: срез в теле, `**` в теле, **контроль** — срез в инициализаторе |
| `takt-lang/tests/targets/statement_site_tests.rs` | три теста, прогон бинарника |

## Проверено

- `cargo test --test targets statement_site` — 3/3.
- `cargo test --all-features` — провалов нет.
- Проба до/после (таблица — в ADR и отчёте).
