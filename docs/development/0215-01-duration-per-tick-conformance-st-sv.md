# Задача 0215-01: Сверка значений `duration` цели `st`

> Фича: [../features/0215-duration-per-tick-conformance-st-sv.md](../features/0215-duration-per-tick-conformance-st-sv.md) · ADR: [../adr/0215-duration-per-tick-conformance-st-sv.md](../adr/0215-duration-per-tick-conformance-st-sv.md) · анализ: [../analyze/0215-duration-per-tick-conformance-st-sv.md](../analyze/0215-duration-per-tick-conformance-st-sv.md)

## Что сделано

`takt-sim/tests/conformance/conformance_st_duration_tests.rs` — сверка значений
на фикстуре `conformance_duration_value.takt` (**той же**, что у целей `c` и
`rust`):

1. порождение ST под именем `stdurvalue.takt` — корень ST берёт имя из имени
   файла, и от него же зависят C-символы `iec2c` (`STDURVALUE_data__`);
2. трансляция `iec2c -I <lib>` → `POUS.c`;
3. драйвер на C вызывает `STDURVALUE_body__` три скана и печатает
   `TIMERS0.MS`, `TIMERS0.LATE` и приватное `TIMERS0.ELAPSED`;
4. сверка с эталоном, причём ожидание эталона записано **числами**
   (`(1750, 1, 1750)`) — иначе сверка проверяла бы цель против себя.

⚠️ Наносекунды эталона переводятся в миллисекунды **в тесте**, а не в цели:
этот перевод и есть предмет сверки (ADR 0183).

⚠️ Каталог сборки уникален по тесту, двоеточие из имени потока вычищается
(инварианты 0190 и 0244).

Набор объявлен модулем темы `conformance` — новой тестовой цели не появилось.

## Проверка

```sh
cargo test --test conformance conformance_st_duration_tests::
```
