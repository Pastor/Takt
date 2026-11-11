---
okf_version: 0.2
format: 2
type: Task
key: TAKT-7
list: ТАКТ
title: "[0007] Порты и индексный доступ"
status: done
priority: none
author: pastor
model: gpt-6
links:
  depends:
  - TAKT-1
  - TAKT-2
generated:
  by: agent/gpt-6
  for: human:pastor
  at: 2026-11-11T07:08:38Z
published: 2026-11-11T07:08:38Z
created: 2026-11-11T07:08:38Z
updated: 2026-11-11T07:08:38Z
anal:
  project: 22
  id: 1061
  version: 1
checksum: sha256:120c7afdde4cd3463e339464f0aa51799d6b519fada7fd19dbac009c872bd8fd
---

Источник: [docs/features/0007-ports.md](https://github.com/Pastor/Takt/blob/3c154a13d979b18312a1250ebfbe3db2fa5247d3/docs/features/0007-ports.md)

# Фича: Порты и индексный доступ

- **Номер:** 0007
- **Статус:** ГОТОВО
- **Зависит от:** 0001, 0002
- **Крейт:** `grammar`

## Краткое описание

Порты (`bit`/`rational`/`numeric`) с направлениями `in`/`out`/`inout` и доступ по
индексу-выражению (`arr[i]`, `port[i]`) в условиях и выражениях.

## Итог (что сделано)

- Направление `inout` (двунаправленный порт): `parser/ast.rs` (`PortDirection`),
  `lexer.rs`, `grammar.lalrpop`, `generator/c/{mod,c_header}.rs`.
- Индекс массива/порта — произвольное выражение: `ast.rs`, `grammar.lalrpop`,
  `semantic/{mod,condition,expression,validate}.rs`, `generator/c/c_expr.rs`.
- Интерфейс портов в C: `read_bit`/`write_bit`/`read_float`/`write_float`/
  `read_numeric`/`write_numeric`.
- Фикстуры: `array_access.lam`; контрпримеры `array_out_of_bounds.lam`,
  `non_array_subscript.lam`.

> Ретроспективная карточка. Источники: `STATUS.md` (задачи 3, 12),
> `CHANGES.md`.
