---
okf_version: 0.2
format: 2
type: Task
key: TAKT-14
list: ТАКТ
title: "[0014] Сохранение и загрузка состояния модели"
status: done
priority: none
author: pastor
model: gpt-6
links:
  depends:
  - TAKT-12
generated:
  by: agent/gpt-6
  for: human:pastor
  at: 2026-11-11T07:08:38Z
published: 2026-11-11T07:08:38Z
created: 2026-11-11T07:08:38Z
updated: 2026-11-11T07:08:38Z
anal:
  project: 22
  id: 1068
  version: 1
checksum: sha256:649dbb07b741abaad5f4af2bc0b765dabb1c510002e174c5f20c6f342269bc64
---

Источник: [docs/features/0014-state-io.md](https://github.com/Pastor/Takt/blob/3c154a13d979b18312a1250ebfbe3db2fa5247d3/docs/features/0014-state-io.md)

# Фича: Сохранение и загрузка состояния модели

- **Номер:** 0014
- **Статус:** ГОТОВО
- **Зависит от:** 0012
- **Крейт:** `simulation`

## Краткое описание

Сохранение текущего состояния симулируемой модели в файл и загрузка из файла —
для продолжения прогона с сохранённого места.

## Итог (что сделано)

- `simulation/src/state_io.rs`; CLI-флаги `--save-state`/`--load-state`
  (`bin/simulation.rs`, `lib.rs`).

> Ретроспективная карточка. Источники: `STATUS.md` (TASKS §2),
> `CHANGES.md`.
