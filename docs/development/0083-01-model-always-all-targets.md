# Разработка 0083-01: model-level `always` в симуляторе и всех целях

- **Фича:** [0083](../features/0083-model-always-block-c.md)
- **ADR:** [0083](../adr/0083-model-always-block.md) (Option B)
- **Дата:** 2026-07-20

## Что сделано

Model-level `always` (тело `always` вне состояния) теперь исполняется **каждый
такт до диспетчеризации состояния** в симуляторе (эталон) и во всех четырёх целях.

### Симулятор (`simulation/src/unit/builder.rs`)

`Unit::Node.executions` наполняется из `model.named_blocks` (был
`HashMap::new()`). Шаг 2 `execution("always")` (`unit/mod.rs`) их исполняет —
каждый такт, до `tick_node`, безусловно по состоянию.

### Генераторы (эмиссия перед диспетчером состояния)

| Цель | Файл | Точка | Помощник |
|---|---|---|---|
| C | `c_model.rs` | перед `switch (model->state)` | `generate_model_named_blocks` |
| rust | `rust_model.rs` | перед `match self.state` | `emit_model_named_blocks` |
| st | `st_model.rs` | перед `CASE state OF` | `emit_model_block` |
| sv | `sv_fsm.rs` | перед `unique case` (над `_next`) | `emit_model_named_blocks` |

- **rust:** присваивания model-level `always` добавлены в сбор `assigned` —
  иначе `let` затираемой переменной вышел бы немутабельным (`-D warnings`).
- **sv:** эмиссия в `always_comb` после умолчаний `_next`, до `unique case`.

### Вынос по лимиту размера модуля

Добавление превысило лимит у `c_model` (1084→1111), `rust_model` (1349→1377) и
`sv_fsm` (995→1015). Помощники именованных блоков вынесены в новые модули:

- `generator/c/c_blocks.rs` — `generate_named_blocks` + `generate_model_named_blocks`;
- `generator/rust/rust_blocks.rs` — `emit_named_blocks` + `emit_model_named_blocks`;
- `generator/sv/sv_blocks.rs` — `emit_named_blocks` + `emit_model_named_blocks`
  (`Fsm::scope` повышен до `pub(crate)`; `sv_compose` импортирует из `sv_blocks`).

Baseline размеров обновлён вниз: `c_model` 1084→1075, `rust_model` 1349→1345;
`sv_fsm` 985 (< 1000, из реестра не в долге).

## Проверка

- Потактовая сверка C↔симулятор: `n = 1,2,3,4`
  (`conformance_c_tests::model_level_always_matches_generated_c`).
- Структурная эмиссия «до диспетчера» + компиляция вывода `rustc`/`iec2c`/
  `verilator` (`grammar/tests/model_always_tests.rs`).
- Вывод корпуса `examples/generated/` байт-в-байт неизменен; `precheck.sh` EXIT=0.

## Замечания

- `ModelNode.named_blocks` — поле (не метод); `get_named_blocks(name)` фильтрует.
- Объём — `always`; `enter`/`exit` уровня модели вне объёма (механизм принимает
  любой `block_name`).
