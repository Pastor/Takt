# Разработка 0319-01: свёртка приведения агрегата

> Фича: [../features/0319-array-cast-folding.md](../features/0319-array-cast-folding.md) · ADR: [../adr/0319-array-cast-folding.md](../adr/0319-array-cast-folding.md)

## Состав правки

| Файл | Что сделано |
|---|---|
| `takt-lang/src/semantic/const_eval/mod.rs` | ветвь приведения агрегата + `array_cast` (длина, элементы правилом целого) |
| `takt-lang/tests/semantic/array_cast_tests.rs` | четыре проверки: предмет, правило элементов, контроль, граница |

## Проверено

- `cargo test --test semantic array_cast` — 4/4.
- Проба: `{1, 2} as [u8; 2]` — все восемь целей и эталон согласны;
  `{300, 2} as [u8; 2]` — `[44, 2]` у эталона, цель `c` собирается.
- `cargo test --all-features` — провалов нет.
