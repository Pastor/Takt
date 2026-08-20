# Разработка 0312-01: релизная сборка в гейтах цели `c`

> Фича: [../features/0312-c-gate-release-mode.md](../features/0312-c-gate-release-mode.md) · ADR: [../adr/0312-c-gate-release-mode.md](../adr/0312-c-gate-release-mode.md)

## Состав правки

| Файл | Что сделано |
|---|---|
| `examples/generated/c/batch_cycle_main.c` | `assert` заменён на `check(...)`, работающую в любом режиме; отказ даёт код возврата 1 |
| `scripts/precheck.sh` | шаг «сборка корпуса в РЕЛИЗЕ» + запуск харнессов; `c-hal` компилируется дважды |
| `examples/generated/c/.gitignore` | добавлен `cmake-*-release` |

## Проверено

- Релизная сборка корпуса: 24/24 цели, `batch_cycle` и `stacker` запущены и
  прошли проверки.
- `c-hal` под `-DNDEBUG`: 0 отказов на всех примерах корпуса.
- `./scripts/precheck.sh` — код 0.

## Найденный дефект

Первый прогон релизного шага дал отказ компиляции харнесса
(`-Wunused-but-set-variable` на счётчике `phases`). Причина — проверки на
`assert`, исчезающие под `-DNDEBUG`: харнесс, собравшийся в релизе, не
проверял бы ничего.
