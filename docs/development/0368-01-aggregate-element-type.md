# Задача 0368-01: Элемент агрегата печатается по типу элемента

> Фича: [../features/0368-aggregate-element-type.md](../features/0368-aggregate-element-type.md) · ADR: [../adr/0368-aggregate-element-type.md](../adr/0368-aggregate-element-type.md) · анализ: [../analyze/0368-aggregate-element-type.md](../analyze/0368-aggregate-element-type.md)

## Что было

Понижение q-литерала смотрело на скалярный тип, печать массива у цели `rust`
знала только элемент-структуру, а тип приёмника у `sv` при записи в элемент
массива был `None`.

## Что сделано

**`takt-lang/src/semantic/declaration/mod.rs`** — `lower_folded_fixed`
понижает агрегат поэлементно по типу элемента (рекурсивно).

**`takt-lang/src/generator/rust/rust_coerce.rs`** — ветвь массива обобщена:
элементы печатаются `coerce_to` по типу элемента для **любого** типа;
бит-вектор `[bit;N≤64]` исключён (правило 0078).

**`takt-lang/src/generator/sv/sv_stmt.rs`** — `target_type` для индексации
отдаёт тип элемента (тип базы даёт `sv_array::array_type_expr`, фича 0365).

**Сверка**: `aggregate_element_types_match_generated_c` — значения `whole`,
`code` и **представление** `gains[0] = 384`.

⚠️ **Первая правка была не в том месте.** Понижение добавлялось в
`lower_fixed_var` (вывод типов) и **работало**, но результат перезаписывался
свёрткой инициализаторов: она берёт сырой АСД и подменяет значение (защита
`is_literal` агрегат не покрывает). Показал это отладочный прогон, а не
чтение.

## Проверки

```sh
cargo test --test conformance aggregate_element_types
cargo test --test conformance   # 166
cargo test --test targets       # 391
cargo test -p takt-lang --lib   # 1106
scripts/probe.sh -n 2 qarr.takt; scripts/probe.sh -n 2 enumarr.takt
./scripts/precheck.sh
```

Мутации: «не понижать агрегат» — сверка красная («порождённый C не
компилируется»); «`sv` не берёт тип элемента» — проба даёт `verilator ОТВЕРГ`.
