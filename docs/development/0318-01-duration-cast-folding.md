# Разработка 0318-01: свёртка моста длительности

> Фича: [../features/0318-duration-cast-folding.md](../features/0318-duration-cast-folding.md) · ADR: [../adr/0318-duration-cast-folding.md](../adr/0318-duration-cast-folding.md)

## Состав правки

| Файл | Что сделано |
|---|---|
| `takt-lang/src/semantic/const_eval/mod.rs` | две ветви: `ЦЕЛОЕ as duration` (через `duration::from_millis`) и `duration as ЦЕЛОЕ` (через `to_millis` + правило целого 0310) |
| `takt-lang/tests/semantic/duration_cast_tests.rs` | четыре проверки: обе стороны, контроль, граница переполнения |
| `takt-sim/tests/sim/cast_in_initializer_tests.rs` | ожидание T8 обновлено (сверялся текст вывода) |

## Проверено

- `cargo test --test semantic duration_cast` — 4/4.
- Проба: обе формы — все восемь целей и эталон согласны.
- `cargo test --all-features` — провалов нет.
