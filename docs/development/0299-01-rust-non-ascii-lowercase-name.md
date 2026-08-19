# Задача 0299-01: Регистр имени приводится по Unicode

> Фича: [../features/0299-rust-non-ascii-lowercase-name.md](../features/0299-rust-non-ascii-lowercase-name.md) · ADR: [../adr/0299-rust-non-ascii-lowercase-name.md](../adr/0299-rust-non-ascii-lowercase-name.md) · анализ: [../analyze/0299-rust-non-ascii-lowercase-name.md](../analyze/0299-rust-non-ascii-lowercase-name.md)

## Что было

`semantic::naming::normalize_camelcase_name` поднимала первую букву слова через
`char::to_ascii_uppercase` — не-ASCII буква оставалась строчной. Цель `rust`
печатала `кнопка,` вариантом перечисления, и `clippy -D warnings` отвечал
ошибкой «variant should have an upper camel case name» при нулевом коде
возврата `taktc`.

## Что сделано

Одна строка: `result.push(ch.to_ascii_uppercase())` →
`result.extend(ch.to_uppercase())` (Unicode-приведение возвращает итератор:
у некоторых букв верхний регистр — несколько символов).

Правка стоит в **общем слое**, а не в цели: правило регистра зовут семь
потребителей (цели `rust`, `st`, `c`, карта, документация, дерево, minimap), и
собственная копия в одной цели разошлась бы с остальными — класс 0084/0193/0195.

⚠️ **Нового класса коллизий не заведено:** слипание `кнопка`/`Кнопка` ловит
существующая `RS-005` — тем же механизмом, что и `button`/`Button` (проверено
прогоном, тест T3).

## Проверки

```sh
cargo test --lib -p takt-lang normalize_model_name   # юнит-тесты, включая не-ASCII
cargo test --test targets rust_non_ascii            # 4 теста + настоящий clippy-driver
cargo test --all-features                           # 3256 тестов
./scripts/precheck.sh                               # полный гейт
```

Снимки `examples/generated/` не изменились: не-ASCII имён в корпусе нет, а для
ASCII результат тождествен прежнему.
