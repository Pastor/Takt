---
okf_version: 0.2
format: 2
type: Task
key: TAKT-1
list: ТАКТ
title: "[0001] Ядро языка Lam и компилятор в C"
status: done
priority: none
author: pastor
model: gpt-6
links:
  blocks:
  - TAKT-10
  - TAKT-11
  - TAKT-12
  - TAKT-16
  - TAKT-2
  - TAKT-3
  - TAKT-4
  - TAKT-5
  - TAKT-6
  - TAKT-7
  - TAKT-8
  - TAKT-9
generated:
  by: agent/gpt-6
  for: human:pastor
  at: 2026-11-11T07:08:38Z
published: 2026-11-11T07:08:38Z
created: 2026-11-11T07:08:38Z
updated: 2026-11-11T07:08:38Z
anal:
  project: 22
  id: 1055
  version: 1
checksum: sha256:feb4c1e4d3df0f93e5e0b94a85358d09e1dda34b337d5183812989e72124d3c1
---

Источник: [docs/features/0001-core-language-c.md](https://github.com/Pastor/Takt/blob/3c154a13d979b18312a1250ebfbe3db2fa5247d3/docs/features/0001-core-language-c.md)

# Фича: Ядро языка Lam и компилятор в C

- **Номер:** 0001
- **Статус:** ГОТОВО
- **Зависит от:** нет
- **Крейт:** `grammar`

## Краткое описание

Базовый язык Lam для описания FSM и его компиляция в C: лексер, LALRPOP-парсер
LR(1), AST, семантическое дерево (7 проходов) и генератор C-кода. CLI `lamc`.

## Итог (что сделано)

- Лексер (`parser/lexer.rs`), грамматика (`grammar.lalrpop`), AST (`parser/ast.rs`).
- Семантика: `semantic/{mod,tree,type_node,...}.rs`, 7 проходов построения модели.
- Генератор C: `generator/c/{mod,c_header,c_source,c_model,c_decl,c_expr,c_map}.rs`.
- Публичный API `compile_to_c()`, CLI `lamc compile --target c … -o …`
  (`bin/lamc.rs`). Анонимная корневая модель получает имя `Root`.
- Инвариант: условия рёбер `ref` не разрешаются (тест `syntax_simple`) — см. `CLAUDE.md`.

> Ретроспективная карточка (фича реализована до внедрения процесса).
> Детали — в истории git (ранние коммиты `G1`–`G12`, «Обновление спецификации
> языка Lam») и `CHANGES.md`.
