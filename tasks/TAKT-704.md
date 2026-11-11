---
okf_version: 0.2
format: 2
type: Task
key: TAKT-704
list: ТАКТ
title: Тип `unit` в документе не объяснён
status: todo
priority: low
author: pastor
model: gpt-6
generated:
  by: agent/gpt-6
  for: human:pastor
  at: 2026-11-11T07:41:41Z
published: 2026-11-11T07:41:41Z
created: 2026-11-11T07:41:41Z
updated: 2026-11-11T07:41:41Z
anal:
  project: 22
  id: 1761
  version: 1
checksum: sha256:afa14da6d79c10262a1a44fb332620a75f36ad65a0e0a3670b0e7fdfcfd583f7
---

`builtin_type_by_name` знает `unit`
наравне с `u8`…`float`, цели печатают его (`()` у `rust`, `void` у `sv`), а
документ упоминает имя один раз — в перечне типов, которые нельзя объявить
заново. Что это за тип, когда его пишут явно (`fn f() -> unit`) и чем он
отличается от опущенной стрелки, не сказано нигде. Tier 4. Обнаружено
анализом покрытия book/ 2026-09-03.
