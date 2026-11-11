---
okf_version: 0.2
format: 2
type: Task
key: TAKT-643
list: ТАКТ
title: "[0150-02] проверка сценариев видела чужие рабочие копии"
status: done
priority: none
author: pastor
model: gpt-6
links:
  part:
  - TAKT-149
generated:
  by: agent/gpt-6
  for: human:pastor
  at: 2026-11-11T07:09:37Z
published: 2026-11-11T07:09:37Z
created: 2026-11-11T07:09:37Z
updated: 2026-11-11T07:09:37Z
anal:
  project: 22
  id: 1697
  version: 1
checksum: sha256:82fefab05dad9e3725255582717eda3dd216d8978c7c821fe523af5605328867
---

Источник: [docs/fixes/0150-02-gate-sees-worktrees.md](https://github.com/Pastor/Takt/blob/3c154a13d979b18312a1250ebfbe3db2fa5247d3/docs/fixes/0150-02-gate-sees-worktrees.md)

# Исправление: проверка сценариев видела чужие рабочие копии

> Фича: [../features/0150-sim-positional-scenario-deprecation.md](../features/0150-sim-positional-scenario-deprecation.md)

## Проблема

Предкоммит отказал на файлах, которых в проекте нет: проверка позиционных
сценариев нашла их в `.claude/worktrees/...` — рабочей копии дерева, заведённой
параллельной сессией инструмента. Своё дерево при этом было чистым, и причина
отказа читалась как дефект проекта.

## Причина

Проверка обходит дерево от корня и пропускает известные служебные каталоги
(`.git`, `target`, `node_modules`, `.idea`, `build`). Каталога инструмента в
списке не было, а соседняя проверка ссылок его уже пропускала: правило
существовало, но жило в одном месте из двух.

## Решение

`.claude` добавлен в список пропускаемых каталогов проверки сценариев. Чужая
рабочая копия судится в своём прогоне, а не в этом.

## Проверки

`./scripts/precheck.sh` при заведённой рабочей копии внутри дерева проходит;
находки проверки остаются на файлах самого проекта.
