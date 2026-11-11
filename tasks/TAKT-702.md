---
okf_version: 0.2
format: 2
type: Task
key: TAKT-702
list: ТАКТ
title: Восстановление из снимка не спускается в реализацию состояния
status: todo
priority: low
author: pastor
model: gpt-6
generated:
  by: agent/gpt-6
  for: human:pastor
  at: 2026-11-11T07:41:35Z
published: 2026-11-11T07:41:35Z
created: 2026-11-11T07:41:35Z
updated: 2026-11-11T07:41:35Z
anal:
  project: 22
  id: 1759
  version: 1
checksum: sha256:c38f75ca29b6039ef1140e280667d3ea83afecb8a2fb6f642fc49c58bbfa16ee
---

`state_io.rs` (`publish_tree`) обходит узлы, но не `state_impls`: по чтению
кода состояние внутри реализации после `--load-state` в реестр не
публикуется — тот же класс, что у часов реализации. Замера нет. Tier 3.
Обнаружено исправлением [выдержка after и every не срабатывают в модели, реализующей состояние](https://github.com/Pastor/Takt/blob/3c154a13d979b18312a1250ebfbe3db2fa5247d3/docs/fixes/0181-02-time-in-state-implementation.md).
