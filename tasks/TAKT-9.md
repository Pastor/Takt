---
okf_version: 0.2
format: 2
type: Task
key: TAKT-9
list: ТАКТ
title: "[0009] Генератор диаграмм PlantUML"
status: done
priority: none
author: pastor
model: gpt-6
links:
  depends:
  - TAKT-1
generated:
  by: agent/gpt-6
  for: human:pastor
  at: 2026-11-11T07:08:38Z
published: 2026-11-11T07:08:38Z
created: 2026-11-11T07:08:38Z
updated: 2026-11-11T07:08:38Z
anal:
  project: 22
  id: 1063
  version: 1
checksum: sha256:dbbb084c0fcf3270061d7ac499269c7b7b0acce6b220081f222dfd2c0a08f75e
---

Источник: [docs/features/0009-plantuml-generator.md](https://github.com/Pastor/Takt/blob/3c154a13d979b18312a1250ebfbe3db2fa5247d3/docs/features/0009-plantuml-generator.md)

# Фича: Генератор диаграмм PlantUML

- **Номер:** 0009
- **Статус:** ГОТОВО
- **Зависит от:** 0001
- **Крейт:** `grammar`

## Краткое описание

Второй целевой генератор — диаграммы состояний PlantUML (`--target plantuml`).

## Итог (что сделано)

- `generator/plantuml/puml_map.rs` — снимок семантической карты модели.
- `generator/plantuml/mod.rs` — генерация `.puml` из `PumlMap`.
- `Language::PlantUML` (`generator/mod.rs`), API `compile_to_plantuml()` (`lib.rs`),
  поддержка `--target plantuml` в CLI (`bin/lamc.rs`).

> Ретроспективная карточка. Источники: `CHANGES.md`,
> коммит `bdcbb1a`.
