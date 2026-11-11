---
okf_version: 0.2
format: 2
type: Task
key: TAKT-4
list: ТАКТ
title: "[0004] Управляющие конструкции и match/switch"
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
  id: 1058
  version: 1
checksum: sha256:058af90bee48fe6704b371cda53f5837ce893da1ecc10eaf10291b0b44511345
---

Источник: [docs/features/0004-control-flow.md](https://github.com/Pastor/Takt/blob/3c154a13d979b18312a1250ebfbe3db2fa5247d3/docs/features/0004-control-flow.md)

# Фича: Управляющие конструкции и match/switch

- **Номер:** 0004
- **Статус:** ГОТОВО
- **Зависит от:** 0001
- **Крейт:** `grammar`

## Краткое описание

Конструкции `if`/`while`/`for`/`loop` в блоках кода и полноценный оператор
`match`/`switch` (лексер → грамматика → AST → семантика → C-генератор).

## Итог (что сделано)

- `while` как синоним `loop` (`lexer.rs`, `grammar.lalrpop`).
- `match`/`switch`: `lexer.rs`, `grammar.lalrpop`, `parser/ast.rs`
  (`Statement`, `MatchPattern`), `semantic/statement.rs`, `generator/c/c_expr.rs`.
- Фикстура `if_while_for.lam`.

> Ретроспективная карточка. Источники: `STATUS.md` (задачи 8, 10),
> коммиты `a49fcfd`, `b703e74`, `CHANGES.md`.
