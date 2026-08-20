# Разработка 0321-01: разворот статического `for`

> Фича: [../features/0321-sv-for-unroll.md](../features/0321-sv-for-unroll.md) · ADR: [../adr/0321-sv-for-unroll.md](../adr/0321-sv-for-unroll.md)

## Состав правки

| Файл | Что сделано |
|---|---|
| `takt-lang/src/generator/sv/sv_unroll.rs` | разбор статических границ: начало, сравнение, шаг; предел `MAX_ITERATIONS = 64` |
| `takt-lang/src/generator/sv/sv_stmt.rs` | ветвь `For`: разворот либо отказ с названной формой; `hoist_locals` спускается в цикл |
| `takt-lang/src/generator/sv/mod.rs` | подключение модуля |
| `takt-sim/tests/data/eval/conformance_sv_for.takt` | фикстура: накапливающее тело, зависящее от переменной цикла |
| `takt-sim/tests/conformance/conformance_sv_for_tests.rs` | потактовая сверка + граница (динамические границы) |

## Проверено

- `cargo test --test conformance conformance_sv_for` — 2/2.
- `verilator --lint-only -Wall` и `yosys synth` на порождённом модуле — код 0.
- Проба: статический `for` переводят все восемь целей.
- `cargo test --all-features` — провалов нет.
