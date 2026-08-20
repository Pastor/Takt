# Разработка 0307-01: ширина в отказе доступа к разряду

> Фича: [../features/0307-sim-bit-range-text.md](../features/0307-sim-bit-range-text.md) · ADR: [../adr/0307-sim-bit-range-text.md](../adr/0307-sim-bit-range-text.md)

## Состав правки

| Файл | Что сделано |
|---|---|
| `takt-sim/src/eval/error.rs` | `BitIndexOutOfRange { bit, width }`; текст печатает включительную границу, у логического — отдельная фраза |
| `takt-sim/src/eval/access.rs` | чтение: носитель `64`, вектор — `слов × 64` |
| `takt-sim/src/eval/place.rs` | запись: то же, плюс `width: 1` у логического |
| `takt-sim/tests/data/bitrange/*.takt` | три фикстуры: вектор за границей, **контроль** внутри, скаляр |
| `takt-sim/tests/sim/bit_range_text_tests.rs` | три сквозных теста (гоняют бинарник) |
| `docs/diagnostics/README.md`, `book/src/appendix-errors/index.typ` | разбор кода приведён к новому тексту |

## Тексты

```
номер бита 200 вне разрядов значения: доступны 0..127
номер бита 70 вне разрядов значения: доступны 0..63
номер бита 3 недопустим: у логического значения один разряд — 0
```

## Проверено

- `cargo test --test sim bit_range_text` — 3/3.
- `cargo test --all-features` — провалов нет.
- `python3 scripts/check-book-diagnostics.py` — 264 кода сверены.
