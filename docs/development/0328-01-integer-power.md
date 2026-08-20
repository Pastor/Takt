# Разработка 0328-01: целая степень у `c` и `st`

> Фича: [../features/0328-integer-power.md](../features/0328-integer-power.md) · ADR: [../adr/0328-integer-power.md](../adr/0328-integer-power.md)

## Состав правки

| Файл | Что сделано |
|---|---|
| `takt-lang/src/generator/c/c_expr/expr.rs` | `pow((double)…)` → `takt_ipow((int64_t)…)` |
| `takt-lang/src/generator/c/c_expr/fixed.rs` | определение `takt_ipow`, эмиссия по факту вызова |
| `takt-lang/src/generator/st/st_arith.rs` | новый модуль: `power` (разворот в умножения) и `arithmetic_shift_right` (0324) — то, что в IEC **есть, но означает другое** |
| `takt-lang/src/generator/st/st_expr.rs` | ветви `Power` и `SHR` зовут его |
| `takt-sim/tests/data/eval/conformance_power.takt` | фикстура: широкий и узкий типы |
| `takt-sim/tests/conformance/conformance_power_tests.rs` | сверка значений с целью `c` + прогон `iec2c` на выводе `st` |

## Проверено

- `cargo test --test conformance conformance_power` — 2/2.
- Прогон харнесса цели `c`: `12157665459056928801` — как у эталона (до правки
  `12157665459056928768`).
- `iec2c` принимает порождённый ST (до правки отвергал).
- `cargo test --all-features` — провалов нет.
