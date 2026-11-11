---
okf_version: 0.2
format: 2
type: Task
key: TAKT-12
list: ТАКТ
title: "[0012] Крейт simulation — симуляция моделей"
status: done
priority: none
author: pastor
model: gpt-6
links:
  depends:
  - TAKT-1
  blocks:
  - TAKT-13
  - TAKT-14
generated:
  by: agent/gpt-6
  for: human:pastor
  at: 2026-11-11T07:08:38Z
published: 2026-11-11T07:08:38Z
created: 2026-11-11T07:08:38Z
updated: 2026-11-11T07:08:38Z
anal:
  project: 22
  id: 1066
  version: 1
checksum: sha256:c869b6f47c4d2bf303ca5775d9dcad55dc98cc82bcfe04d3ae246a5609c9d0fe
---

Источник: [docs/features/0012-simulation-core.md](https://github.com/Pastor/Takt/blob/3c154a13d979b18312a1250ebfbe3db2fa5247d3/docs/features/0012-simulation-core.md)

# Фича: Крейт simulation — симуляция моделей

- **Номер:** 0012
- **Статус:** ГОТОВО
- **Зависит от:** 0001
- **Крейт:** `simulation`

## Краткое описание

Крейт `simulation`: пошаговая симуляция семантических моделей с иерархическим
контекстом (родительские цепочки, ленивое копирование), CLI-бинарник.

## Итог (что сделано)

- Построение дерева `Unit` из `ModelNode`: `simulation/src/unit/builder.rs`,
  `unit/mod.rs`, `unit/viewport.rs`; `ModelNodeContext` (`context.rs`) —
  `Rc<RefCell<ModelNode>>`, ленивая копия, цепочка родителей.
- Предикаты переходов `Predicate` (`predicate.rs`) — именованный struct,
  `Rc<dyn Fn>`, читаемые метки (`condition_label`).
- Раннер (`runner.rs`), CLI (`bin/simulation.rs`), лимит шагов из сценария.

> Ретроспективная карточка. Источники: `STATUS.md` (TASKS §1),
> коммиты `2c4527e` (П1–П8), `483aaf0`, `CHANGES.md`.
