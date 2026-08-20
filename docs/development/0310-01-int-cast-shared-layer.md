# Разработка 0310-01: перенос правила приведения в общий слой

> Фича: [../features/0310-int-cast-shared-layer.md](../features/0310-int-cast-shared-layer.md) · ADR: [../adr/0310-int-cast-shared-layer.md](../adr/0310-int-cast-shared-layer.md)

## Состав правки

| Файл | Что сделано |
|---|---|
| `takt-lang/src/semantic/const_eval/int_cast.rs` | новый носитель: `integer(value, bits, signed)`, тип `SignedOverflow`, юнит-тесты правила |
| `takt-lang/src/semantic/const_eval/mod.rs` | целочисленная цель приведения считается носителем; `SE-121` на знаковом переполнении; предикат `is_not_constant` |
| `takt-lang/src/semantic/declaration/mod.rs` | диагностика не первого рода терминальна в свёртке |
| `takt-sim/src/eval/mod.rs` | `coerce_integer` зовёт носитель, переводя ошибку в `SIM-003` |
| `takt-lang/tests/semantic/int_cast_tests.rs` | пять проверок: обёртка, отказ, контроль, устройство, граница |
| `takt-sim/tests/conformance/conformance_sv_tests/const_cast_init.rs` | граница 0286 переписана: значение в выводе + отказ на знаковом |
| `takt-sim/tests/sim/cast_in_initializer_tests.rs` | ожидание T2 обновлено: цель печатает готовое `44` |
| `docs/diagnostics/README.md`, `book/src/appendix-errors/index.typ` | `SE-121` зарегистрирован |

## Проверено

- `cargo test --test semantic int_cast` — 5/5, **мутация** (снять маску обёртки)
  ловится.
- `cargo test --all-features` — провалов нет.
- Проба: `300 as u8` — все восемь целей и эталон дают `44`; `300 as i8` — все
  отвечают `SE-121`; `Tuner(limit := 300 as u8)` — `44`.
