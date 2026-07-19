# Задача 0097-01: Пример ПИД-регулятора на языке Lam (fixed-point)

> Фича: [../features/0097-pid-regulator-example.md](../features/0097-pid-regulator-example.md) · ADR: [../adr/0097-pid-regulator-example.md](../adr/0097-pid-regulator-example.md) · анализ: [../analyze/0097-pid-regulator-example.md](../analyze/0097-pid-regulator-example.md)

## Что было

Корпус содержит **пропорциональный** регулятор (`examples/regulator.lam`, задача
0061-05). Полного ПИД (с I и D) нет.

## Что сделано

`examples/pid_regulator.lam` — позиционный дискретный ПИД на `q(m, n)` с
anti-windup и объектом первого порядка (Option A [ADR 0097](../adr/0097-pid-regulator-example.md)).
Реализация — единственная задача фичи (пример, компилятор не меняется).

- **Модель:** состояние `Control` (`e = SP−PV; I = clamp(I+e); D = e−e_prev;
  u = Kp·e + Ki·I + Kd·D; PV += Kplant·u; e_prev = e`), переход по `|e| < eps` →
  `Settled → Done`; обёртка под-моделью (цель `c`), порт `ready` (цель `rust`).
- **Anti-windup:** `I` ограничен `[−Imax, Imax]` `q`-сравнениями (насыщения в
  языке нет — кандидат A-5 ADR 0061).
- **Коэффициенты** подобраны замером сходимости в симуляторе.
- **Контракт** в `simulation/tests/examples_scenario_tests.rs`.
- **Документация:** шапка примера (формулы) + README.

Функциональности (правило 11): язык/генераторы — **н/п** (пример их не меняет,
R6); симулятор — контракт; цели `c`/`st`/`rust`/`sv` — гейты корпуса.

## Проверки

- `cargo test --test examples_scenario_tests -- --test-threads=1` (T1: цепочка +
  завершение; зонд T2: интеграл не переполняется).
- `./scripts/precheck.sh` — все гейты корпуса (T4–T8) + `git diff grammar/` пуст (T9).
