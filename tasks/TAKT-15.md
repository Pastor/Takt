---
okf_version: 0.2
format: 2
type: Task
key: TAKT-15
list: ТАКТ
title: "[0015] Шаблоны стадий жизненного цикла и генератор"
status: done
priority: none
author: pastor
model: gpt-6
links:
  blocks:
  - TAKT-17
generated:
  by: agent/gpt-6
  for: human:pastor
  at: 2026-11-11T07:08:38Z
published: 2026-11-11T07:08:38Z
created: 2026-11-11T07:08:38Z
updated: 2026-11-11T07:08:38Z
anal:
  project: 22
  id: 1069
  version: 1
checksum: sha256:135b0185fa3389d5064907aa15cfc9de02c403142ee84895521539dd1cbecda1
---

Источник: [docs/features/0015-stage-templates.md](https://github.com/Pastor/Takt/blob/3c154a13d979b18312a1250ebfbe3db2fa5247d3/docs/features/0015-stage-templates.md)

# Фича: Шаблоны стадий жизненного цикла и генератор

- **Номер:** 0015
- **Статус:** ГОТОВО
- **Зависит от:** нет
- **Стек:** документация/инфраструктура

## Краткое описание

Скелеты артефактов всех стадий жизненного цикла фичи и генератор
заготовок из шаблонов.

## Итог (что сделано)

- Шаблоны `docs/templates/{feature,adr,analyze,development,tests,reports,fixes}.md`
  + `docs/templates/README.md`.
- Генератор `scripts/new-feature.sh [--with-dev] [--register] XXXX slug "Заголовок"`
  (POSIX sh): подстановка плейсхолдеров и (`--register`) дозапись строк в реестры.

> Ретроспективная карточка. До внедрения нумерации фича значилась
> под номером `0031`; перенумерована в `0015` при реконструкции истории.
