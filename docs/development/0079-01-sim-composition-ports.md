# Разработка 0079-01: рекурсивное перечисление портов композиции

- **Фича:** [0079](../features/0079-sim-composition-ports.md)
- **ADR:** [0079](../adr/0079-sim-composition-ports.md)
- **Дата:** 2026-07-20

## Что сделано

### Рекурсивное перечисление (`simulation/src/runner.rs`, `bin/simulation.rs`)

`PortNames::from_model(model)` (новый метод в библиотеке) собирает порты и
переменные модели **и всех её под-моделей** (`model.models`) рекурсивно, с
сортировкой и дедупом имён. `extract_port_names` в `bin/simulation.rs` сведён к
вызову `PortNames::from_model` (было — сбор только с корня, inline). Вынос в
библиотеку — ради тестируемости.

Драйв и чтение (`Unit::set_value`/`get_value`) **не трогались**: на `Parallel`
они уже адресуют все ветви композиции. Достаточно перечислить порт — и он
драйвится.

### Инструмент (`scripts/run_simulations.sh`)

Сопоставление sim-файла с моделью: было `model="${base%%_*}"` (префикс до
первого `_`), ломалось на именах с подчёркиванием. Теперь — цикл, отсекающий
суффикс справа до первого существующего `${candidate}.lam`, то есть **самый
длинный** префикс-модель. `elevator_mini_floor2` → `elevator_mini` (а не
`elevator`).

### Сценарий и исключение

- `examples/simulations/elevator_mini_floor2.json` — driven-сценарий:
  `FloorSensor_F2_Bottom=1` → guard `current_floor = 2`.
- `examples_scenario_tests`: причина исключения `elevator_mini` переписана с
  «ВРЕМЕННО: SIM-009» на «реактивный, сценарии извне; дефект 0079 исправлен».

## Проверка

- `composition_ports_tests.rs`: порты под-моделей перечисляются; порт под-модели
  читается (0 по умолчанию, не `SIM-009`).
- `run_simulations.sh`: `elevator_mini_floor2` проходит (guard `current_floor=2`);
  `stacker_*` без изменений (6/6).
- `precheck.sh` EXIT=0; вывод корпуса генераторов неизменен.

## Замечания

- Одноимённые порты под-моделей делят значение (плоское пространство имён) —
  дедуп; строгая квалификация — 0084.
- `run_simulations.sh` не входит в `precheck.sh` (тот гоняет C-`stacker`), поэтому
  гейтом регрессии служит `composition_ports_tests.rs`.
