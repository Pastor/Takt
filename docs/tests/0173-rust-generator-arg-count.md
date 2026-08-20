# Тест-план фичи 0173: Заглушка `too_many_arguments` в цели `rust`

> Фича: [../features/0173-rust-generator-arg-count.md](../features/0173-rust-generator-arg-count.md) · анализ: [../analyze/0173-rust-generator-arg-count.md](../analyze/0173-rust-generator-arg-count.md) · отчёт: [../reports/0173-rust-generator-arg-count.md](../reports/0173-rust-generator-arg-count.md)

## Предмет проверки

Рефакторинг: сигнатуры изменились, **вывод — нет**. Главная проверка — не
«тесты зелёные», а побайтовое совпадение порождённого корпуса.

## Условия проверок

| # | Условие | Как проверяется | Ожидаемый результат |
|---|---|---|---|
| П1 | Заглушек нет | `grep 'too_many_arguments' takt-lang/src/generator/rust/` | только тест о порождаемом коде |
| П2 | Вывод корпуса не изменился | компиляция всех `examples/*.takt` целью `rust` + `diff -r` | различий нет |
| П3 | Линт чист | `cargo clippy --all-targets --all-features` | без замечаний |
| П4 | Регрессия | `cargo test --all-features` | провалов нет |
| П5 | Гейт цели `rust` на корпусе | `./scripts/precheck.sh` | код 0 |
| П6 | Сверки поведения целы | `conformance_rust_*` в предкоммите | зелёные |

## Границы

Заглушки в других целях (`sv`, `st`, `c`), в верификации и симуляторе — 20 мест
— не входят в объём: карточка говорит о генераторе `rust`, и смешение сделало
бы дифф нечитаемым.
