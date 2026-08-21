# Задача 0367-01: Массив структур у цели sv синтезируется

> Фича: [../features/0367-sv-struct-array.md](../features/0367-sv-struct-array.md) · ADR: [../adr/0367-sv-struct-array.md](../adr/0367-sv-struct-array.md) · анализ: [../analyze/0367-sv-struct-array.md](../analyze/0367-sv-struct-array.md)

## Что было

Сброс массива структур печатался шаблоном присваивания (`pts <= '{{…}, {…}};`),
умолчание — целиком (`pts_next = pts;`). Verilator обе формы принимает, yosys
— ни одной: первая даёт «Assignment pattern is only supported for whole
unpacked array assignments», вторая (после починки первой) — «Latch inferred».

## Что сделано

**`takt-lang/src/generator/sv/sv_array.rs`** — `type_leaves(ty, fields_of)`:
суффиксы и типы скалярных мест, когда внутри распакованного массива лежит
структура; `needs_leafwise` — сам признак. Массив скаляров даёт `None` и
печатается целиком, как прежде.

**`takt-lang/src/generator/sv/sv_fsm.rs`** — у `Reg` появилось поле `leaves`;
`leafwise_reset` собирает значения (у инициализатора — через
`aggregate::leaves` фичи 0366, при его отсутствии — умолчания типов). Ветвь
сброса и блок умолчаний печатают по листьям, если они есть.

**`takt-lang/src/generator/sv/sv_stmt.rs`** — снята временная граница 0366
(`refuse_struct_in_array`): вход переводится.

**Сверка**: `takt-sim/tests/data/eval/conformance_sv_struct_array.takt` и два
теста — потактовая сверка значений и **прогон синтеза** `yosys`.

**`takt-lang/src/generator/sv/sv_enums.rs`** (новый модуль) — печать
перечислений состояний и шагов вынесена из `sv_fsm.rs`: тот вышел за предел
размера модуля (1008 строк при лимите 1000). Печать перечислений от сборки
автомата не зависит — границы модуля совпали с границей ответственности.

⚠️ **Класс невидим двум сторожам из трёх.** Сверка значений собирает тестбенч
verilator, линт цели — тоже verilator; yosys видел его один. Отсюда отдельный
тест синтеза.

## Проверки

```sh
cargo test --test conformance struct_array   # сверка значений + синтез
cargo test --test conformance                # 165
cargo test --test targets                    # 391
for f in examples/*.takt; do scripts/probe.sh -n 1 "$f"; done   # корпус чист
./scripts/precheck.sh
```

Мутации (обе пойманы тестом синтеза): печатать умолчание целиком; печатать
сброс целиком.
