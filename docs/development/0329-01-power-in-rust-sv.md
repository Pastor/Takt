# Разработка 0329-01: степень у `rust` и `sv`

> Фича: [../features/0329-power-in-rust-sv.md](../features/0329-power-in-rust-sv.md) · ADR: [../adr/0329-power-in-rust-sv.md](../adr/0329-power-in-rust-sv.md)

## Состав правки

| Файл | Что сделано |
|---|---|
| `takt-lang/src/generator/rust/rust_shift.rs` | `power`: `wrapping_pow`, отказ на отрицательном литеральном показателе |
| `takt-lang/src/generator/rust/rust_expr.rs` | ветвь `Power` зовёт его |
| `takt-lang/src/generator/sv/sv_cast.rs` | `power`: разворот в умножения при литеральном показателе |
| `takt-lang/src/generator/sv/sv_expr.rs` | ветвь `Power` зовёт его |
| `takt-sim/tests/conformance/conformance_power_tests.rs` | сторож перевода у обеих целей |
| `takt-lang/tests/targets/statement_site_tests.rs`, `tests/data/site0308/` | сторож 0308 перестроен на срез |

## Проверено

- `cargo test --test conformance conformance_power` — 3/3.
- `verilator --lint-only -Wall` и `yosys synth` на выводе `sv` — код 0.
- Проба: степень переводят все восемь целей.
- `cargo test --all-features` — провалов нет.
