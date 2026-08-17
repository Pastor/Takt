# Задача 0200-02: сторожа и документирование

> Фича: [../features/0200-non-ascii-identifier-targets.md](../features/0200-non-ascii-identifier-targets.md) · ADR: [../adr/0200-non-ascii-identifier-targets.md](../adr/0200-non-ascii-identifier-targets.md) · анализ: [../analyze/0200-non-ascii-identifier-targets.md](../analyze/0200-non-ascii-identifier-targets.md)

## Что сделано

`takt-lang/tests/semantic/non_ascii_identifier_tests.rs` — 6 тестов:

| Тест | Что доказывает |
|---|---|
| `sv_rejects_non_ascii_identifier` | `SV-018` вместо отказа `verilator` |
| `st_rejects_non_ascii_identifier` | `ST-020` вместо отказа `iec2c` |
| `c_and_rust_still_accept_non_ascii_identifier` | язык **не сужен** |
| `generated_c_with_non_ascii_names_compiles` | вывод `c` собирает настоящий `cc -Wall -Werror` |
| `ascii_identifiers_are_unaffected` | обычные имена не задеты ни одной целью |
| `every_declaration_kind_is_checked` | отказ в **каждой** позиции: переменная, порт, состояние, константа |

⚠️ **Последний тест окупился сразу:** он нашёл две дыры — имена состояний не
проверялись **ни целью `sv`, ни целью `st`**. Правило существовало, но не во
всех позициях; чтение кода этого не показало, потому что вызовов проверки было
девять и десять соответственно.

⚠️ Проверка вывода цели `c` идёт **сборкой**, а не строкой: строковая проверка
закрепила бы наше представление о правильном, а `cc` проверяет то, что считает
правильным компилятор C.

## Документирование (правило 24)

Появляется ограничение **целей**, а не языка, — сказано в разделе о целях
генерации: `sv` и `st` требуют латиницы в именах, `c` и `rust` — нет.

## Проверки

```sh
cargo test -p takt-lang --test non_ascii_identifier_tests
```
