# Отчёт о тестировании фичи 0066: литералы по целевому типу в телах цели `st`

> Фича: [../features/0066-st-bool-literals.md](../features/0066-st-bool-literals.md) · тест-план: [../tests/0066-st-bool-literals.md](../tests/0066-st-bool-literals.md)

- **Дата:** 2026-07-18
- **Окружение:** macOS (darwin 25.5.0), MatIEC `iec2c` (`~/.local/bin`), cargo nightly 1.99.
- **Вердикт:** **готово.** 1988 тестов зелёные; `iec2c` принимает весь корпус;
  дифф `st` — только литералы; прочие цели побайтово прежние.

## Сверка с критериями приёмки

| # | Критерий | Результат | Способ |
|---|---|---|---|
| A1 | `:= 0`/`:= 1` (`BOOL`) → `FALSE`/`TRUE` | ✅ | тест `bool_literal_coerced_to_false_true`; дифф `stacker.st` (`cmd_fork := 0` → `FALSE`) |
| A2 | `command := 2` → `command := Command_Stop` | ✅ | тест `enum_value_coerced_to_constant_name`; дифф `elevator_mini.st` |
| A3 | Все три места (присваивание, сброс, `enter`) покрыты | ✅ | `Assign` в `Expression` (тела + `enter`/`exit`/`always` через `print_statement`) + инициализатор объявления |
| A4 | Значение без варианта → число | ✅ | тест `enum_value_without_variant_stays_number` (T13): `7` → `"7"`, `2` → `Command_Stop` |
| A5 | Гейт `iec2c` по корпусу — зелёный | ✅ | `iec2c -I … -T` по 4 изменённым `.st` → rc=0 |
| A6 | `c`/`c-hal`/`rust`/`sv`/`plantuml` побайтово прежние | ✅ | регенерация всех целей → `git diff` вне `st/` пуст |
| A7 | Поведение прежнее (потактово) | ✅ | `conformance_st_tests` — 1 passed, вердикт не изменился |
| A8 | Язык/версия не изменены | ✅ | правка только в печати цели `st` |

## Дифф `st` — построчно только литералы (A7/T11)

- `stacker.st`: `cmd_fork := 0` → `cmd_fork := FALSE` (и `:= 1` → `TRUE`).
- `elevator_mini.st`: `command := 0/1/2` → `Command_Up/Command_Down/Command_Stop`.
- ⚠️ `state := 2` **не тронут** — синтетический регистр состояния имеет тип
  `USINT`, а не Lam-перечисление; `coerce_to` его не касается.

## Примеры и контрпримеры (правило 16)

- **Пример:** `command := Stop;` → `command := Command_Stop;` (константа
  объявлена рядом: `Command_Stop : USINT := 2;`).
- **Контрпример (не догадываться):** значение без соответствующего варианта
  печатается **числом** — подмена «похожим» вариантом была бы тихой ложью.

## Найденные дефекты

Нет. Побочная находка (цель `c` не эмитит констант перечисления вовсе) — вне
объёма 0066, живёт кандидатом в `FEATURES.md`.
