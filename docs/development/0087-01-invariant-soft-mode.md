# Задача 0087-01: Мягкий режим инвариантов симулятора (записать и продолжить)

> Фича: [../features/0087-invariant-soft-mode.md](../features/0087-invariant-soft-mode.md) · ADR: [../adr/0087-invariant-soft-mode.md](../adr/0087-invariant-soft-mode.md) · анализ: [../analyze/0087-invariant-soft-mode.md](../analyze/0087-invariant-soft-mode.md)

## Что было

Ядро [0044](../features/0044-sim-assert-invariant.md): нарушение инварианта
(`SIM-025`) **останавливает** прогон (`TickResult::Failed` → `RunResult::EvalFailed`)
— совпадает с `assert()` → `abort()` в C, нужно для потактовой сверки. Для отладки
(«когда и сколько раз нарушается») останов на первом же нарушении неудобен.

## Что сделано

Реализована **Option A** [ADR 0087](../adr/0087-invariant-soft-mode.md).

- **Слой `Unit` (`unit/mod.rs`):**
  - `Unit::tick` (жёсткий, публичный, C-конформный) — логика вынесена в
    `tick_mode(&mut self, soft: bool)`; `tick = tick_mode(false)`, новый
    `tick_soft = tick_mode(true)`.
  - `soft` протаскивается через `tick_node` (без изменений) / `tick_parallel` /
    `tick_sequential` в `check_guards(soft)`.
  - `check_guards(soft)`: нарушение (SIM-025) → мягкий режим пишет строку в новое
    поле `UnitKind::Node::invariant_violations` и возвращает `None` (такт
    продолжается); жёсткий — `Some(Failed)` (как 0044). **Ошибка вычисления**
    условия — `Some(Failed)` в **обоих** режимах (R4).
  - `take_invariant_violations(&mut self)` — рекурсивный слив нарушений из дерева
    `Unit` (зеркало `take_last_transitions`).
- **Бегун (`runner.rs`):** поле `soft_invariants` + сеттер `set_invariant_soft`;
  цикл в мягком режиме зовёт `tick_soft`, после такта сливает нарушения и тегирует
  номером шага; новый `RunResult::CompletedWithInvariantViolations { steps,
  terminated, violations }`.
- **CLI (`bin/simulation.rs`):** флаг `--invariant-soft`; печать нарушений с
  шагами; **ненулевой** код возврата при их наличии (не молчим).
- **Места построения `Node`:** builder (реальный путь) + state_io/viewport/tests
  получили `invariant_violations: Vec::new()` (компилятор указал все).

| Стек | Статус |
|---|---|
| `simulation` слой Unit | ✅ tick_soft + запись + рекурсивный слив |
| `simulation` runner/CLI | ✅ флаг, RunResult, код возврата |
| `grammar` / язык | н/п — не трогаются |
| жёсткий режим (0044) / C-конформность | н/п — байт-в-байт неизменны |

## Проверки

- 6 тестов инвариантов зелёные: 3 жёстких (0044, без правок) + 3 новых мягких:
  `invariant_soft_records_and_continues` (нарушения на шагах 2–5, прогон идёт,
  `c` растёт), `invariant_soft_does_not_swallow_eval_error` (SIM-010 → `Failed` в
  мягком режиме, нарушений 0), `invariant_soft_collects_from_composition`
  (нарушение `PA` под-модели всплыло).
- CLI-проба: жёсткий — стоп на шаге 2 (`SIM-025`, exit 1); мягкий — 5 шагов, 4
  нарушения, exit 1.
- `cargo clippy -p simulation --all-targets -D warnings` → чисто.
- Фикстуры: `invariant_eval_error.lam`, `invariant_composite.lam`.
- Полный `./scripts/precheck.sh` → зелёный (см.
  [отчёт](../reports/0087-invariant-soft-mode.md)).
