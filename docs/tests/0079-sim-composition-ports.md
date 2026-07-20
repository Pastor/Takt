# Тест-план 0079: Порты под-модели композиции

- **Фича:** [0079](../features/0079-sim-composition-ports.md)
- **ADR:** [0079](../adr/0079-sim-composition-ports.md)
- **Дата:** 2026-07-20
- **Роль:** Тестировщик

## Стратегия

Дефект симулятора/инструмента (не языка). Регрессия гейтится `cargo test`
(`composition_ports_tests.rs`); сквозная реакция — driven-сценарием через
`run_simulations.sh`.

## Условия проверок и ожидаемые результаты

| № | Проверка | Ожидание | Где |
|---|---|---|---|
| T1 | `PortNames::from_model` перечисляет in/out-порты под-моделей `Cabin`/`Motor` | `Sensor`,`Limit`,`Alarm` в списках | `composition_ports_tests::composition_submodel_ports_are_enumerated` |
| T2 | Порт под-модели читается (0 по умолчанию, не `SIM-009`) | `Some(Number(0))` | `composition_ports_tests::submodel_port_is_readable_in_composition` |
| T3 | Driven-вход доходит: `FloorSensor_F2_Bottom` → `current_floor=2` | guard OK | `elevator_mini_floor2.json` + `run_simulations.sh` |
| T4 | `stacker_*.json` не сломаны (порядок портов цел) | все OK | `run_simulations.sh` (6/6) |
| T5 | Матчинг модели по длинному префиксу | `elevator_mini_floor2`→`elevator_mini` | `run_simulations.sh` |
| T6 | Вывод корпуса генераторов неизменен | нет диффа | гейт детерминизма |
| T7 | Весь `precheck.sh` | зелёный | `./scripts/precheck.sh` |

## Примеры (правило 16)

Driven-сценарий `elevator_mini_floor2.json` (реакция на порт под-модели):

```json
[ { "in_ports": [ …, FloorSensor_F2_Bottom=1, … ], "guard": {"vars": {"current_floor": 2}} } ]
```

## Границы

- `elevator_mini` **реактивен** — самозавершающегося контракта не имеет; остаётся
  исключением в `examples_scenario_tests` (сценарии извне, как `stacker`).
- `run_simulations.sh` **не** в `precheck.sh` (тот гоняет C-`stacker`), поэтому
  сквозной драйв гейтится вручную/CI, а регрессия перечисления — `cargo test`.
- Плоское пространство имён портов — вне объёма (0084).
