---
okf_version: 0.2
format: 2
type: Task
key: TAKT-2
list: ТАКТ
title: "[0002] Система типов и вывод типов"
status: done
priority: none
author: pastor
model: gpt-6
links:
  depends:
  - TAKT-1
  blocks:
  - TAKT-5
  - TAKT-6
  - TAKT-7
generated:
  by: agent/gpt-6
  for: human:pastor
  at: 2026-11-11T07:08:38Z
published: 2026-11-11T07:08:38Z
created: 2026-11-11T07:08:38Z
updated: 2026-11-11T07:08:38Z
anal:
  project: 22
  id: 1056
  version: 1
checksum: sha256:f10855b6d8d966585ea901a0cbb152125adbe57a457c3e7540a48646501a7caf
---

Источник: [docs/features/0002-type-system.md](https://github.com/Pastor/Takt/blob/3c154a13d979b18312a1250ebfbe3db2fa5247d3/docs/features/0002-type-system.md)

# Фича: Система типов и вывод типов

- **Номер:** 0002
- **Статус:** ГОТОВО
- **Зависит от:** 0001
- **Крейт:** `grammar`

## Краткое описание

Псевдонимы типов, вывод типов из пользовательских псевдонимов, встроенные
целочисленные типы `u8..u64`/`i8..i64` и составные типы `[T;N]` в параметрах.

## Итог (что сделано)

- Вывод типов и разрешение псевдонимов: `semantic/type_inference.rs`
  (`ast_type_to_node_ctx`), `semantic/type_node.rs`.
- Целочисленные типы `u8/i8 … u64/i64` → `TypeNode::Integer` (`type_node.rs`,
  `function.rs`, `generator/c/mod.rs`).
- Составные типы `[T;N]` в параметрах функций (`grammar.lalrpop`,
  `ParameterTypeExpr`).
- Фикстуры: `type_alias_inference.lam`, `fn_array_param.lam`.

> Ретроспективная карточка. Источники: `STATUS.md` (задачи 4+5),
> память проекта (FE2, FE6), `CHANGES.md`.
