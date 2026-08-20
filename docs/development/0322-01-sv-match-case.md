# Разработка 0322-01: `match` → `case`

> Фича: [../features/0322-sv-match-case.md](../features/0322-sv-match-case.md) · ADR: [../adr/0322-sv-match-case.md](../adr/0322-sv-match-case.md)

## Состав правки

| Файл | Что сделано |
|---|---|
| `takt-lang/src/generator/sv/sv_stmt.rs` | печать `case`/`endcase`, образцы через запятую, обязательный `default`; `hoist_locals` спускается в ветки |
| `takt-sim/tests/data/eval/conformance_sv_match.takt` | фикстура: три ветки, наблюдаемая меняется по тактам |
| `takt-sim/tests/conformance/conformance_sv_match_tests.rs` | потактовая сверка + тест устройства (`default` есть всегда) |

## Проверено

- `cargo test --test conformance conformance_sv_match` — 2/2.
- `verilator --lint-only -Wall` и `yosys synth` — код 0, защёлок нет.
- Проба: `match` переводят все восемь целей.
