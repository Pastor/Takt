# Задача 0215-02: Сверка значений `duration` цели `sv`

> Фича: [../features/0215-duration-per-tick-conformance-st-sv.md](../features/0215-duration-per-tick-conformance-st-sv.md) · ADR: [../adr/0215-duration-per-tick-conformance-st-sv.md](../adr/0215-duration-per-tick-conformance-st-sv.md) · анализ: [../analyze/0215-duration-per-tick-conformance-st-sv.md](../analyze/0215-duration-per-tick-conformance-st-sv.md)

## Что сделано

**Фикстура** `takt-sim/tests/data/eval/conformance_duration_value_sv.takt` — та
же арифметика длительностей, но **без приведения** `as`: цель `sv` его не
транслирует (`SV-002`), и число миллисекунд через порт не выдать. Значение
наблюдается косвенно и потому тройной проверкой:

| Порт | Условие | Смысл |
|---|---|---|
| `late` | `elapsed > 500ms` | нижняя граница |
| `exact` | `elapsed = 1750ms` | точное значение |
| `over` | `elapsed > 1800ms` | верхняя граница |

Плюс сам сигнал `elapsed` читается тестбенчем иерархически
(`dut.svdurvalue_timers_elapsed`) — у цели это `logic [31:0]` в миллисекундах.

**Сверка** `takt-sim/tests/conformance/conformance_sv_duration_tests.rs`:
порождение → тестбенч `verilator --binary -j 0 --timing` → печать четвёрки
значений на каждом такте → сверка с эталоном. Ожидание эталона записано
числами `(1750, 1, 1, 0)`.

⚠️ Равенство длительностей (`exact`) — форма, которой **нет** в фикстуре
`c`/`rust`: там сравнение одно. То есть сверка `sv` заодно покрывает то, что не
покрыто у соседей.

## Проверка

```sh
cargo test --test conformance conformance_sv_duration_tests::
```
