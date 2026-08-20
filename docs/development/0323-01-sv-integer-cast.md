# Разработка 0323-01: приведение `as` в цели `sv`

> Фича: [../features/0323-sv-integer-cast.md](../features/0323-sv-integer-cast.md) · ADR: [../adr/0323-sv-integer-cast.md](../adr/0323-sv-integer-cast.md)

## Состав правки

| Файл | Что сделано |
|---|---|
| `takt-lang/src/generator/sv/sv_cast.rs` | новый модуль: `integer_cast` — `W'(expr)`, знаковая цель `$signed(...)`, нескалярная — отказ с причиной |
| `takt-lang/src/generator/sv/sv_expr.rs` | ветвь `Cast` зовёт его |
| `takt-sim/tests/data/eval/conformance_sv_cast.takt` | фикстура: расширение и сужение с обёрткой |
| `takt-sim/tests/conformance/conformance_sv_cast_tests.rs` | потактовая сверка + тест устройства (`$signed`) |

⚠️ Модуль выделен, потому что `sv_expr.rs` перевалил лимит размера (1044 при
1000). Граница по смыслу: печать выражения отвечает «как выглядит операция»,
приведение — «как выглядит смена типа».

## Проверено

- `cargo test --test conformance conformance_sv_cast` — 2/2.
- `verilator --lint-only -Wall` и `yosys synth` — код 0.
- Проба: `as` между целыми переводят все восемь целей.
