# Разработка 0324-01: арифметический сдвиг у `sv` и `st`

> Фича: [../features/0324-arithmetic-shift-right.md](../features/0324-arithmetic-shift-right.md) · ADR: [../adr/0324-arithmetic-shift-right.md](../adr/0324-arithmetic-shift-right.md)

## Состав правки

| Файл | Что сделано |
|---|---|
| `takt-lang/src/generator/sv/sv_expr.rs` | `>>` → `>>>` при знаковом операнде |
| `takt-lang/src/generator/sv/sv_cast.rs` | предикат `is_signed_expression` (осторожный: не узнав знака, отвечает `false`) |
| `takt-lang/src/generator/st/st_expr.rs` | `arithmetic_shift_right`: floor-деление через `SEL`; переменная величина — `ST-011` |
| `takt-sim/tests/data/eval/conformance_shift.takt` | фикстура: `-7 >> 1` и `3 << 2` |
| `takt-sim/tests/conformance/conformance_shift_tests.rs` | потактовая сверка + тест устройства (`>>>`) |
| `takt-lang/src/generator/sv/sv_call.rs` | печать вызова вынесена из `sv_expr` — тот перевалил лимит размера |
| `CLAUDE.md` | исправлено утверждение «операторов сдвига в языке нет» |

## Проверено

- `cargo test --test conformance conformance_shift` — 2/2.
- `iec2c` принимает порождённый ST со `SEL`-формой.
- `verilator` и `yosys` принимают модуль с `>>>`.
- Формула ST проверена на четырёх значениях (−7, −8, 7, 8) — совпадает с floor.
