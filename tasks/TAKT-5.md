---
okf_version: 0.2
format: 2
type: Task
key: TAKT-5
list: ТАКТ
title: "[0005] Перечисления (enum)"
status: done
priority: none
author: pastor
model: gpt-6
links:
  depends:
  - TAKT-1
  - TAKT-2
  whole:
  - TAKT-597
generated:
  by: agent/gpt-6
  for: human:pastor
  at: 2026-11-11T07:09:26Z
published: 2026-11-11T07:09:26Z
created: 2026-11-11T07:08:38Z
updated: 2026-11-11T07:09:26Z
anal:
  project: 22
  id: 1059
  version: 2
checksum: sha256:99c6da18505c025dc33b006709a9ddabc025a197f83c6c71bdba73ed0c93fd52
---

Источник: [docs/features/0005-enums.md](https://github.com/Pastor/Takt/blob/3c154a13d979b18312a1250ebfbe3db2fa5247d3/docs/features/0005-enums.md)

# Фича: Перечисления (enum)

- **Номер:** 0005
- **Статус:** ГОТОВО
- **Зависит от:** 0001, 0002
- **Крейт:** `grammar`

## Краткое описание

Синтаксис и семантика перечислений `enum` с вариантами и значениями, включая
диагностику дубликатов вариантов.

## Итог (что сделано)

- AST: `EnumDefine`, `EnumVariant` (`parser/ast.rs`); `Token::Enum` (`lexer.rs`);
  правила `EnumDefine`/`EnumVariant` (`grammar.lalrpop`).
- Семантика: `EnumNode` (`semantic/enum_node.rs`, `semantic/tree.rs`).
- Фикстуры: `enum_basic.lam`, `enum_with_values.lam`; контрпример
  `enum_duplicate_variant.lam`.

> Ретроспективная карточка. Источники: память проекта (FE1),
> `CHANGES.md`.
