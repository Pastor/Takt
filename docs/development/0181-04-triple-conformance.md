# Задача 0181-04: Тройная сверка sim-C-SV и значенческие тесты композиции

> Фича: [../features/0181-sim-state-implementation-tick.md](../features/0181-sim-state-implementation-tick.md) · ADR: [../adr/0181-sim-state-implementation-tick.md](../adr/0181-sim-state-implementation-tick.md) · анализ: [../analyze/0181-sim-state-implementation-tick.md](../analyze/0181-sim-state-implementation-tick.md)

## Что было

Сверка `+` велась **только** между целью `c` и целью `sv`: симулятор из неё был
исключён с объяснением «он не исполняет композицию с `next`». Потактового
покрытия у самого исполнения `+` в симуляторе не было **нигде** — классическая
дыра, из-за которой дефект 0057-01 и дожил.

## Что сделано

### Тройная сверка (`takt-sim/tests/conformance_sv_tests.rs`)

Комментарий-объяснение «сверка идёт с C, потому что симулятор сломан» заменён
описанием закрытого дефекта. Оба теста расширены до **тройных**:

- `sequential_composition_matches_generated_c` — `A + B`;
- `parallel_step_inside_concatenation_matches_generated_c` — `A + (B | C)`.

Симулятор проверяется **первым**, до мягких пропусков `cc`/`verilator`:
инструментов он не требует, и на машине без них сверка сужается, но не исчезает.
Эталон C остаётся пришпиленным литералом, поэтому одинаковая поломка двух сторон
падает на нём.

### Дефект, вскрытый сверкой

Первый прогон показал: значения совпали, но трасса симулятора **короче** на
такт — состояние не уходило по `next`. Причина: `next` живёт **отдельным полем**
`StateNode::Implement::next`, а `build_transitions` собирал только
`state.references()`. То есть `next` не был переходом вовсе, и `start P = A + B
{ next Done; }` застревал в `P` навсегда.

Исправлено в `takt-sim/src/unit/builder.rs`: `build_transitions` добавляет `next`
**последним** переходом (он безусловен и впереди `ref`-рёбер затенил бы их все).
Проверяется он лишь после завершения реализации — эталон
`generate_extend_transition`, эмитящий переход в ветви `is_done`.

### Значенческие тесты (`takt-sim/tests/sequential_composition_tests.rs`, новый)

Шесть тестов **на значения**, а не на факт перехода (урок 0025):

| Тест | Что доказывает |
|---|---|
| `concatenation_with_next_runs_every_step` | каждый шаг исполняется; трасса равна эталону C |
| `nested_concatenation_with_next_runs_every_step` | глубина роли не играет (форма из пробы 0057-01) |
| `concatenation_without_next_keeps_shared_value` | значение не проваливается в 0 и наблюдается после завершения цепочки |
| `re_entering_implemented_state_does_not_restart_chain` | повторный вход **не** перезапускает композицию — контракт цели `c` |
| `unfinished_implementation_never_fires_next` | **контрпример**: незавершённая реализация перехода не даёт |
| `failure_inside_implementation_propagates` | **контрпример**: ошибка внутри реализации поднимается как `Failed` (R5 ADR 0057) |

Контрпримеры существенны: без них «починка», берущая `next` всегда, тоже была бы
зелёной.

Обратная функциональность (правило 11): затронуты только тесты крейта
`takt-sim`. Продукт не менялся — **н/п**.

## Проверки

| Что | Результат |
|---|---|
| `cargo test --all-features --test conformance_sv_tests` (обе сверки `+`) | зелёный; sim ≡ C ≡ SV |
| `cargo test --all-features --test sequential_composition_tests` | 6 из 6 |
| `cargo test --all-features -- --test-threads=1` | зелёный, провалов нет |
| `cargo clippy --all-targets --all-features -- -D warnings` | чисто |
| `git diff examples/generated` | пусто — порождённый код не изменился (A9) |
| `./scripts/precheck.sh` | зелёный |

Побочная находка, **в объём не вошедшая**: `always` под-модели композиции
исполняется дважды за такт (симулятор `n = 2`, цель `c` `n = 1`). Заведена
фиксом [0181-01](../fixes/0181-01-sim-parallel-always-twice.md).
