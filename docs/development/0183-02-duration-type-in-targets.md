# Задача 0183-02: цель `rust`

> Фича: [../features/0183-duration-type-in-targets.md](../features/0183-duration-type-in-targets.md) · ADR: [../adr/0183-duration-type-in-targets.md](../adr/0183-duration-type-in-targets.md) · анализ: [../analyze/0183-duration-type-in-targets.md](../analyze/0183-duration-type-in-targets.md)

## Что было

Тип `duration` цель `rust` отвергала тремя точками (`RS-023`): отображение типа,
литерал в выражении, литерал в условии.

## Что сделано

- `rust_type(Duration)` → `u32` (миллисекунды, ширина `duration::VALUE_BITS`);
- литерал длительности в выражении и в условии печатается миллисекундами через
  общий слой `value_millis` — своей арифметики времени печатник не заводит;
- **потактовая сверка значений** с эталоном — новый файл
  `takt-sim/tests/conformance_rust_duration_tests.rs`: драйвер реализует `Hal`,
  запоминает записанное в порты и печатает; сверяются числа (`ms=1750`,
  `late=1`), а не факт сборки — гейт `rustc`/`clippy` доказывает лишь
  компилируемость (урок 0050);
- тест по **тексту**: `elapsed as u32` — простое `as`, ни делений, ни умножений.

## Найден чужой дефект (не исправлен)

Первая редакция фикстуры писала `late := (elapsed > 500ms) ? 1 : 0;` — и
порождённый Rust **не скомпилировался**: ветви тернарника печатаются числами при
`write_bit(…, bool)` (`E0308` ×2) плюс лишние скобки вокруг условия `if`
(`unused_parens` под `-D warnings`). Воспроизводится **без** `duration`
(`late := (n > 5) ? 1 : 0;`), то есть дефект цели `rust`, а не этой фичи —
заведён фиксом [0148-03](../fixes/0148-03-rust-ternary-to-bit-port.md). Фикстура
обходит его формой `if … { … } else { … }` с явным комментарием.

## Проверки

```sh
cargo test -p takt-sim --test conformance_rust_duration_tests -- --test-threads=1
```
