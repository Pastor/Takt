---
okf_version: 0.2
format: 2
type: Task
key: TAKT-703
list: ТАКТ
title: '`SE-131` не ловит повтор образца внутри одной ветви'
status: todo
priority: low
author: pastor
model: gpt-6
generated:
  by: agent/gpt-6
  for: human:pastor
  at: 2026-11-11T07:41:38Z
published: 2026-11-11T07:41:38Z
created: 2026-11-11T07:41:38Z
updated: 2026-11-11T07:41:38Z
anal:
  project: 22
  id: 1760
  version: 1
checksum: sha256:64e11af16c34dbaa30088cf430ec924410d66f0ac1da39016e35871fa2e878ed
---

Запись
`1, 1 => { … }` принимается молча, тогда как повтор **между** ветвями даёт
предупреждение (замер-03, [многообразцовая ветвь `match` в документе](https://github.com/Pastor/Takt/blob/3c154a13d979b18312a1250ebfbe3db2fa5247d3/docs/features/0528-book-multi-pattern-match.md)). Вреда от дубля нет —
ветвь та же, — но он почти наверняка опечатка: автор имел в виду другое
значение, и молчание это скрывает. Проверка ложится в тот же обход, что и
нынешняя: разница лишь в том, что множество образцов набирается по ветви, а не
по всем предыдущим. Tier 4. Обнаружено фичей.
