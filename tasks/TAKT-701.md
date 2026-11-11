---
okf_version: 0.2
format: 2
type: Task
key: TAKT-701
list: ТАКТ
title: Сверки времени в реализации состояния есть только у цели `c`
status: todo
priority: low
author: pastor
model: gpt-6
generated:
  by: agent/gpt-6
  for: human:pastor
  at: 2026-11-11T07:41:33Z
published: 2026-11-11T07:41:33Z
created: 2026-11-11T07:41:33Z
updated: 2026-11-11T07:41:33Z
anal:
  project: 22
  id: 1758
  version: 1
checksum: sha256:3a0b8f46bb6d2ce355bf4789ad2a0879c3726b94de8853f419b1cf3012aa2a44
---

Исправление
[выдержка after и every не срабатывают в модели, реализующей состояние](https://github.com/Pastor/Takt/blob/3c154a13d979b18312a1250ebfbe3db2fa5247d3/docs/fixes/0181-02-time-in-state-implementation.md) завело сверку эталона с `c` на
фикстурах `time_in_implementation.takt` и `every_in_implementation.takt`. У
`st`, `rust` и `sv` сверки времени завязаны на корневые фикстуры, и поведение
этих целей на реализации не снято: `probe.sh` подтвердил только валидность
вывода. Tier 3. Обнаружено исправлением.
