# Разработка 0326-01: насыщение сдвига в цели `rust`

> Фича: [../features/0326-rust-shift-width.md](../features/0326-rust-shift-width.md) · ADR: [../adr/0326-rust-shift-width.md](../adr/0326-rust-shift-width.md)

## Состав правки

| Файл | Что сделано |
|---|---|
| `takt-lang/src/generator/rust/rust_shift.rs` | новый модуль: `saturating_right` — ширина типа, знаковость, литеральная величина |
| `takt-lang/src/generator/rust/rust_expr.rs` | ветвь `ShiftRight` спрашивает его перед обычной печатью |
| `takt-lang/tests/targets/rust_shift_width_tests.rs` | три проверки: предмет, контроль, граница |

## Проверено

- `cargo test --test targets rust_shift_width` — 3/3.
- Порождённый Rust собран `rustc` и запущен: `v = -1`, `w = 0` — как у эталона
  (до правки `rustc` отказывался компилировать).
- Проба: все восемь целей и эталон согласны.
