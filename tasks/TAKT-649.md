---
okf_version: 0.2
format: 2
type: Task
key: TAKT-649
list: ТАКТ
title: "[0378-01] `ST-023` отвергала тёзку перечисления"
status: done
priority: none
author: pastor
model: gpt-6
links:
  part:
  - TAKT-375
generated:
  by: agent/gpt-6
  for: human:pastor
  at: 2026-11-11T07:09:37Z
published: 2026-11-11T07:09:37Z
created: 2026-11-11T07:09:37Z
updated: 2026-11-11T07:09:37Z
anal:
  project: 22
  id: 1703
  version: 1
checksum: sha256:4daf53f390ebfc80fb583ec7f9c21bf5fada031162f03ba94f4e2ce741366faa
---

Источник: [docs/fixes/0378-01-enum-namesake-false-positive.md](https://github.com/Pastor/Takt/blob/3c154a13d979b18312a1250ebfbe3db2fa5247d3/docs/fixes/0378-01-enum-namesake-false-positive.md)

# Исправление: `ST-023` отвергала тёзку перечисления

> Фича: [../features/0378-st-name-type-clash.md](../features/0378-st-name-type-clash.md) · ADR: [../features/0378-st-name-type-clash.md#архитектура-adr](../features/0378-st-name-type-clash.md#архитектура-adr)

## Что было

Первая редакция проверки собирала имена типов у **трёх** карт модели —
`structs`, `enums` и `types`, — то есть у всего, что объявлено. Замер-22 (проба порта перечислимого типа) показал ложный отказ:

```takt
enum Mode { Idle = 1, Work = 2 }
var mode: Mode := Idle;      // ← ST-023, хотя вывод валиден
```

Прогон с отключённой проверкой: `iec2c` такой файл **принимает**.

## Почему

Цель `st` перечисление в раздел `TYPE` **не печатает**: перечислимый тип с
явными значениями MatIEC не принимает, и действует откат Option C задачи — целый тип плюс `VAR CONSTANT`. Псевдоним раскрывается в базовый тип.
В выводе есть только **структуры**:

```st
TYPE
    Pair :
    STRUCT
        lo : USINT;
        hi : USINT;
    END_STRUCT;
END_TYPE
```

То есть занять пространство имён IEC может лишь имя структуры, а список брался
из объявлений модели.

**Ложный отказ хуже пропуска:** он ломает валидную модель. Пропуск оставил
бы прежнее поведение (отказ придёт от `iec2c`), а ложный отказ отнимает
работающий вход.

## Что сделано

- `declared_type_names` строит список **только из структур** модели и её
  предков — из того, что цель **печатает**, а не из того, что объявлено.
- Добавлены два контрпримера: тёзка перечисления и тёзка псевдонима проходят
  (`test_enum_namesake_is_allowed`, `test_alias_namesake_is_allowed`).

## Проверки

```sh
cargo test -p takt-lang --lib st_reserved     # 6 тестов, 0 провалов
./scripts/probe.sh -n 2 <проба>.takt          # st: OK на тёзке перечисления
./scripts/precheck.sh
```

Класс нашла **следующая проба той же сессии** (порт перечислимого типа), а
не тесты фичи: её контроля проверяли то, что фича делает, и молчали о том, чего
она делать не должна. Контрпример был — но только на имени, ни с чем не
столкнувшемся.
