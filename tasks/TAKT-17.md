---
okf_version: 0.2
format: 2
type: Task
key: TAKT-17
list: ТАКТ
title: "[0017] Внедрение процесса разработки (жизненный цикл фич)"
status: done
priority: none
author: pastor
model: gpt-6
links:
  depends:
  - TAKT-15
generated:
  by: agent/gpt-6
  for: human:pastor
  at: 2026-11-11T07:08:38Z
published: 2026-11-11T07:08:38Z
created: 2026-11-11T07:08:38Z
updated: 2026-11-11T07:08:38Z
anal:
  project: 22
  id: 1071
  version: 1
checksum: sha256:208a420aa9342c2dc784cd5e954defc5211047f73c5cb67f076ad77c9a41d9b6
---

Источник: [docs/features/0017-lifecycle-process.md](https://github.com/Pastor/Takt/blob/3c154a13d979b18312a1250ebfbe3db2fa5247d3/docs/features/0017-lifecycle-process.md)

# Фича: Внедрение процесса разработки (жизненный цикл фич)

- **Номер:** 0017
- **Статус:** ГОТОВО
- **Зависит от:** 0015
- **Стек:** документация/процесс

## Краткое описание

Перевод проекта на единый процесс управления работой через жизненный цикл фич:
`RULE.md` (только правила) + `FEATURES.md` (витрина незакрытых фич) + реестры
стадий; `CLAUDE.md` — только контекст. Упразднение плоских списков задач.

## Итог (что сделано)

- `docs/RULE.md` — очищен от контекста, объявлен эталоном правил; процесс = цикл
  фич, `FEATURES.md` — основной.
- `CLAUDE.md` — переписан как контекст-only, со ссылкой на `RULE.md`/`CODE.md`;
  `AGENTS.md` — симлинк на `CLAUDE.md`.
- `FEATURES.md` (3 блока) и реестры
  `docs/{features,adr,analyze,development,tests,reports,fixes}/README.md`.
- Все скрипты перенесены в `scripts/` (`precheck.sh`, `run_simulations.sh`,
  `new-feature.sh`) с корректировкой путей.
- История проекта реконструирована в карточки фич `0001`–`0016`; задачи из
  `TODO.md`/`STATUS.md` оформлены как открытые фичи `0018`–`0020`; `TASKS.md`,
  `STATUS.md`, `TODO.md` удалены.

> Источники: `docs/RULE.md`, `docs/CODE.md`, история git, `CHANGES.md`.
