# Разработка 0317-01: перенос представления `q` в общий слой

> Фича: [../features/0317-fixed-cast-shared-layer.md](../features/0317-fixed-cast-shared-layer.md) · ADR: [../adr/0317-fixed-cast-shared-layer.md](../adr/0317-fixed-cast-shared-layer.md)

## Состав правки

| Файл | Что сделано |
|---|---|
| `takt-lang/src/semantic/const_eval/fixed_repr.rs` | носитель: `wrap`, `saturate`, `normalize`, `from_int`, `from_decimal`, `to_decimal_text` + юнит-тесты правил |
| `takt-lang/src/semantic/const_eval/decimal.rs` | `parts()` — мантисса и масштаб для точного счёта |
| `takt-lang/src/semantic/const_eval/mod.rs` | ветвь `TypeNode::Fixed` в вычислении приведения; разрешение типа вынесено в `target_of` |
| `takt-sim/src/eval/fixed.rs` | `wrap`, `saturate`, `make` — тонкие обёртки над носителем |
| `takt-lang/tests/semantic/fixed_cast_tests.rs` | семь проверок, включая мутационно проверенный floor |
| `takt-sim/tests/sim/cast_in_initializer_tests.rs` | ожидания T6/T7 обновлены (сверялся текст вывода) |

## Проверено

- `cargo test --test semantic fixed_cast` — 7/7; **мутация** (усечение вместо
  floor) ловится.
- Края на пробе: `1.1` → 17, `−1.1` → −18, `3` → 48, `20.0` → 64 (перенос),
  `20.0 … sat` → 127 — всё совпадает с эталоном.
- `cargo test --all-features` — провалов нет.
