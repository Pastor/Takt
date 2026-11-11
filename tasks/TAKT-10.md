---
okf_version: 0.2
format: 2
type: Task
key: TAKT-10
list: ТАКТ
title: "[0010] Верификация свойств (LTL и автоматы Бюхи)"
status: done
priority: none
author: pastor
model: gpt-6
links:
  depends:
  - TAKT-1
  blocks:
  - TAKT-35
  - TAKT-49
  whole:
  - TAKT-630
generated:
  by: agent/gpt-6
  for: human:pastor
  at: 2026-11-11T07:09:37Z
published: 2026-11-11T07:09:37Z
created: 2026-11-11T07:08:38Z
updated: 2026-11-11T07:09:37Z
anal:
  project: 22
  id: 1064
  version: 4
checksum: sha256:92647c0fa4d2df026975cc6c2f0d618795edee71000cd6042ca12e3f231ce5cf
---

Источник: [docs/features/0010-verification-ltl.md](https://github.com/Pastor/Takt/blob/3c154a13d979b18312a1250ebfbe3db2fa5247d3/docs/features/0010-verification-ltl.md)

# Фича: Верификация свойств (LTL и автоматы Бюхи)

- **Номер:** 0010
- **Статус:** ГОТОВО
- **Зависит от:** 0001
- **Крейт:** `grammar`

## Краткое описание

Проверка формальных свойств моделей: разбор LTL-формул и построение автоматов
Бюхи (алгоритм GPVW).

## Итог (что сделано)

- `verification/ltl.rs` (`Ltl`), `verification/buchi.rs` (`BuchiAutomaton`, `NodeMap`),
  `verification/mod.rs`.
- Встроенные формулы в AST: `InlineFormulaDefine`, `LtlExpr`, `FormulaStatement`,
  `FormulaExpression` (`parser/ast.rs`); `semantic/formula.rs` (`Formula`).

## Исправления

| Исправление | Заголовок |
|---|---|
| [вырожденное условие принятия автомата Бюхи (GPVW)](../fixes/0010-01-buchi-acceptance.md) | Вырожденное условие принятия автомата Бюхи (GPVW): `build_buchi` принимал любой бесконечный прогон. Обнаружено зондом при разработке [фичи](0049-model-checking-ltl.md), когда у затравки впервые появился потребитель. |

> Ретроспективная карточка. Детали — в истории git и `CHANGES.md`.
