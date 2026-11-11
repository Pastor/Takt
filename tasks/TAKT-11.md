---
okf_version: 0.2
format: 2
type: Task
key: TAKT-11
list: ТАКТ
title: "[0011] LSP-сервер lam-lsp"
status: done
priority: none
author: pastor
model: gpt-6
links:
  depends:
  - TAKT-1
  whole:
  - TAKT-598
generated:
  by: agent/gpt-6
  for: human:pastor
  at: 2026-11-11T07:09:26Z
published: 2026-11-11T07:09:26Z
created: 2026-11-11T07:08:38Z
updated: 2026-11-11T07:09:26Z
anal:
  project: 22
  id: 1065
  version: 2
checksum: sha256:3c8ca1a3d65765ff8682295361961e243f0e9a45b30567be5925b9553d357dd5
---

Источник: [docs/features/0011-lsp-server.md](https://github.com/Pastor/Takt/blob/3c154a13d979b18312a1250ebfbe3db2fa5247d3/docs/features/0011-lsp-server.md)

# Фича: LSP-сервер lam-lsp

- **Номер:** 0011
- **Статус:** ГОТОВО
- **Зависит от:** 0001
- **Крейт:** `grammar` (feature `lsp`)

## Краткое описание

Языковой сервер `lam-lsp`: поиск узла по позиции, hover-подсказки, подсветка
встроенных типов и ключевых слов.

## Итог (что сделано)

- `semantic/index.rs` — `SemanticIndex`, `SemanticNodeRef`, `SemanticNodeKind`.
- `lsp.rs` — `position_to_offset`, `node_at_position`, `hover_info` (2-этапный
  поиск: по позиции + резервный по имени); `Display` для `TypeNode` (без `{:?}`).
- Подсветка встроенных типов, `while`/`match`/`inout`. Бинарник `bin/lam_lsp.rs`.
- Тесты: `grammar/tests/lsp_tests.rs`, фикстуры `grammar/tests/data/lsp/`.

> Ретроспективная карточка. Источники: память проекта (LSP1),
> коммиты `c7c7027`, `908ccc5`, `CHANGES.md`.
