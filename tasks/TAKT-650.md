---
okf_version: 0.2
format: 2
type: Task
key: TAKT-650
list: ТАКТ
title: "[0429-01] сборка verilator падала при параллельном прогоне"
status: done
priority: none
author: pastor
model: gpt-6
links:
  part:
  - TAKT-426
generated:
  by: agent/gpt-6
  for: human:pastor
  at: 2026-11-11T07:09:38Z
published: 2026-11-11T07:09:38Z
created: 2026-11-11T07:09:38Z
updated: 2026-11-11T07:09:38Z
anal:
  project: 22
  id: 1704
  version: 1
checksum: sha256:2ce1e3ef97aa81039fe4fc397dcb7237062ad1ca968c623a624f3d8a57b7cea3
---

Источник: [docs/fixes/0429-01-sv-conformance-flake.md](https://github.com/Pastor/Takt/blob/3c154a13d979b18312a1250ebfbe3db2fa5247d3/docs/fixes/0429-01-sv-conformance-flake.md)

# Исправление: сборка verilator падала при параллельном прогоне

> Фича: [../features/0429-sv-conformance-flake.md](../features/0429-sv-conformance-flake.md) · ADR: [../features/0429-sv-conformance-flake.md#архитектура-adr](../features/0429-sv-conformance-flake.md#архитектура-adr)

## Что было

Полный прогон `cargo test -p takt-sim --test conformance` изредка падал:

```text
verilator не собрал тестбенч длительностей:
cat: Vtb__ALL.a.verilator_deplist.tmp: No such file or directory
ranlib: warning: file not found 'Vtb__ALL.a'
Undefined symbols for architecture arm64: "_main"
%Error: 'make -C obj_dir -f Vtb.mk -j 12' exited with 2
```

Тот же тест по отдельности проходил всегда. Каталог сборки уникален по потоку и
процессу, то есть дело было **не** в общем каталоге.

## Почему

Тестов, гоняющих `verilator --binary`, — **31**, и `cargo` запускает их
параллельно. Каждый передавал `-j 0` — «занять все ядра», как это делает проверка
предкоммита. На 12 ядрах выходило 12 одновременных сборок по 12
джобов каждая: перегрузка, на которой `make` внутри `obj_dir` изредка спотыкался
о собственные временные файлы.

Ключ `-j 0` в проверке **верен**: там verilator работает в одиночку, и замер дал 110.6 с против 316 с. Ошибка была в переносе этого ключа в тесты, где
параллелизм уже обеспечен `cargo`.

## Что сделано

Во всех 27 наборах, гоняющих `verilator --binary`, ключ заменён на `-j 4` —
потолок, снимающий кратную перегрузку. Замер полного прогона:

| Ключ | Время |
|---|---|
| `-j 0` (было) | 49.5 с |
| `-j 1` | 65.6 с |
| **`-j 4`** | **49.6 с** |

То есть потолок снимает перегрузку **даром**: последовательная сборка стоила бы
16 с, а ограниченная — ничего.

## Урок

Ключ, верный для проверки, не обязан быть верным для теста: проверка — один процесс,
тестов — три десятка, и «занять все ядра» в каждом означает переподписку в
разы. Настройка параллелизма принадлежит **месту запуска**, а не команде.
