# Задача 0090-02: `ci.yml` вызывает `precheck.sh` под строгим режимом

> Фича: [../features/0090-ci-precheck.md](../features/0090-ci-precheck.md) · ADR: [../adr/0090-ci-precheck.md](../adr/0090-ci-precheck.md) · анализ: [../analyze/0090-ci-precheck.md](../analyze/0090-ci-precheck.md)

## Что было

`.github/workflows/ci.yml` — единственный ubuntu-джоб на архивных `actions-rs/*`
(`toolchain@v1`, `cargo@v1`) и `actions/checkout@v2`. Прогоняет `build`/`check`/
`clippy -D warnings`/`check-module-size`/`test` (без `--test-threads=1`) и **один**
живой гейт — SV, **переписанный руками** в шаге workflow (свой цикл по
`examples/*.lam`, свой список обязательных `stacker`/`elevator_mini`, свои вызовы
`verilator`/`yosys`). То есть CI несёт **вторую реализацию** SV-гейта рядом с
`precheck.sh` — источник дрейфа (причина фичи).

## Что сделано

`ci.yml` переписан на **единый источник истины** (Option A ADR 0090): установка
инструментов + один вызов `./scripts/precheck.sh` под `PRECHECK_STRICT=1`. Вся
логика гейтов уходит из workflow.

Каркас джоба (`ubuntu-latest`):

1. `actions/checkout@v4`.
2. Toolchain: `dtolnay/rust-toolchain@nightly` с `components: rustfmt, clippy`
   (даёт `rustc`/`clippy-driver` на PATH — их требует rust-гейт `precheck.sh`).
3. Системные инструменты одним `apt-get install -y`: `cmake ninja-build verilator
   yosys bison flex build-essential` (`python3`, `cc`/gcc — уже на образе).
4. Сборка `iec2c`: `./scripts/ensure-iec2c.sh` (bison 3.x на ubuntu; кладёт в
   `~/.local/bin/iec2c` + библиотеку). Отдельным шагом — чтобы отказ сборки был
   виден именно как сборка арбитра, а не внутри прогона.
5. Прогон: `env: { PRECHECK_STRICT: 1 }` → `run: ./scripts/precheck.sh`.

`SV_GATE_REQUIRED` явно **не** задаётся — он наследует `PRECHECK_STRICT` (0090-01).
Логики гейтов (циклы, списки, вызовы инструментов) в `ci.yml` **не остаётся**
(A1): workflow только ставит инструменты и зовёт скрипт.

Замена `actions-rs/*` (архивированы) на `dtolnay/rust-toolchain` — неизбежна: без
toolchain-шага `run:`-скрипт не выполнить. Это закрывает CI-модернизацию,
номинально числившуюся за 0037; за 0037 остаётся только матрица Windows
(зафиксировано в анализе 0090, пересмотр по правилу 19).

## Проверки

- **A1:** grep по `ci.yml` — нет `verilator`/`yosys`/`cmake`/`iec2c -T`/`rustc`
  как команд гейтов; есть ровно один `./scripts/precheck.sh`.
- **A2 + A5:** зелёный прогон workflow на push; в логе — строка успеха каждого
  живого гейта (C, c-hal, ST 8/8, rust+исполнение, sv, sv-mmio, тестбенчи,
  воспроизводимость, float→q, fmt-check примеров, check-links).
- **A3 (в CI):** временная проба на служебной ветке — убрать `verilator` из шага
  установки → workflow **красный** (доказывает, что гейт обязателен, а не мягко
  пропущен). После доказательства проба откатывается.
- **Локальный регресс:** `./scripts/precheck.sh` без флага по-прежнему зелёный и
  идемпотентный (правило 5).
