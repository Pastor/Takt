# Задача 0194-02: сторожа на значениях и мутационная проверка

> Фича: [../features/0194-sim-composition-model-always.md](../features/0194-sim-composition-model-always.md) · ADR: [../adr/0194-sim-composition-model-always.md](../adr/0194-sim-composition-model-always.md) · анализ: [../analyze/0194-sim-composition-model-always.md](../analyze/0194-sim-composition-model-always.md)

## Что было

Сторожей у класса не было: корпус его не покрывает — `always` уровня модели есть
только у моделей со **своими** состояниями. Хуже, что покрытие такого рода легко
сделать бесполезным: ⚠️ на идемпотентном теле (`n := выражение`), которым полон
корпус, **ни пропуск, ни двойное исполнение неразличимы**. Именно поэтому фикс
0181-01 (тело исполнялось по разу на ветвь) дожил незамеченным при зелёных
тестах.

## Что сделано

**Значенческие тесты** — `takt-sim/tests/sim/composition_model_always_tests.rs`
(4 теста, тело **накапливающее**: `n := n + 1`):

| Тест | Что доказывает |
|---|---|
| `anonymous_root_parallel_runs_body_once_per_tick` | анонимный корень `\|` — `1, 2, 3, 4` |
| `named_composition_model_runs_body_once_per_tick` | именованная модель — вторая делегирующая ветвь `build_impl` |
| `sequential_composition_runs_body_once_per_tick` | композиция `+` — ветвь `tick_sequential` |
| `model_with_own_states_is_unchanged` | контрпример: направление правки (чинили ветвь композиции, а не `always` вообще) |

⚠️ Тест на `+` заведён **отдельно**, а не «то же самое»: у `|` и `+` разные
ветви такта, а вызов `execution("always")` общий — совпадение надо доказать.

**Потактовая сверка с целью `c`** —
`takt-sim/tests/conformance_c_tests/composition_always.rs` на фикстуре
`tests/data/eval/conformance_composition_always.takt`. Вынесена подмодулем
(`#[path]`, приём 0088/0127): `conformance_c_tests.rs` упирается в лимит размера,
а правило требует делить по логике, а не расширять реестр долга.

**Мутационная проверка (критерий A4) — выполнена.** Снятие наполнения
`executions` роняет **три** теста композиции и сверку с `c`, и трасса показывает
ровно дефект:

```
эталон=[0, 0, 0, 0]   ожидалось [1, 2, 3, 4]
C-сверка: [[0],[0],[0],[0],[0],[0]] против [[1],[2],[3],[4],[5],[6]]
```

Контрпример при этом остаётся **зелёным** — как и задумано: он сторожит
направление, а не сам факт исполнения.

## Проверки

```sh
cargo test -p takt-sim --test composition_model_always_tests
cargo test -p takt-sim --test conformance_c_tests composition
./scripts/precheck.sh
```

- новые сторожа: 4 + 1 зелёных;
- мутация роняет 4 из 5 (пятый — контрпример, по замыслу);
- после снятия мутации — снова зелёные.
