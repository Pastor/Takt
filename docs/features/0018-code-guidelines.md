# Фича 0018: Приведение кода к требованиям docs/CODE.md

- **Номер:** 0018
- **Статус:** ГОТОВО
- **Зависит от:** нет
- **Крейты:** `grammar`, `simulation`

## Краткое описание

Рефакторинг кодовой базы под чек-лист `docs/CODE.md` (Rust Design Patterns):
срезовые типы в сигнатурах, `#[non_exhaustive]`, Newtype/Builder, аудит клонов,
`with_capacity` в горячих путях и т.д.

## Мотивация / контекст

Проведён аудит проекта против `docs/CODE.md`; выявлено 13 групп улучшений (P01–P13)
с конкретными точками (file:line), приоритетами и планом по фазам.

## Ссылки на артефакты

| Стадия | Артефакт |
|---|---|
| Анализ/план | [`docs/analyze/0018-code-guidelines.md`](../analyze/0018-code-guidelines.md) |
| Разработка 0018-01 | [`docs/development/0018-01-slice-types-and-api.md`](../development/0018-01-slice-types-and-api.md) |
| Разработка 0018-02 | [`docs/development/0018-02-generate-options.md`](../development/0018-02-generate-options.md) |
| Разработка 0018-03 | [`docs/development/0018-03-with-capacity-nonexhaustive-ast.md`](../development/0018-03-with-capacity-nonexhaustive-ast.md) |
| Разработка 0018-04 | [`docs/development/0018-04-doctests-and-ownership.md`](../development/0018-04-doctests-and-ownership.md) |
| Разработка 0018-05 | [`docs/development/0018-05-clone-audit.md`](../development/0018-05-clone-audit.md) |
| Отчёт о тестировании | [`docs/reports/0018-code-guidelines.md`](../reports/0018-code-guidelines.md) |

## Прогресс

- **0018-01 (ВЫПОЛНЕНО):** P01–P04, P06 — срезовые типы в сигнатурах,
  `#[non_exhaustive]` на `Language`/`ErrorType`, удаление мёртвого алиаса `Source`.
- **0018-02 (ВЫПОЛНЕНО):** P05 — `GenerateOptions` вместо `guard_enable: bool`;
  P07 — Builder `GraphicsConfig` признан ненужным (YAGNI: конфиг только из
  serde/`Default`).
- **0018-03 (ВЫПОЛНЕНО):** P11 — `" ".repeat` в `Printer::calculate_padding`;
  P04b — `#[non_exhaustive]` на `Type`/`ModelElement`/`StateElement`/`Expression`/
  `Statement`/`TypeNode`; P13 — аудит `new()`/`Default` (изменений не требуется).
- **0018-04 (ВЫПОЛНЕНО):** P12 — 2 примера `own_doc`/`element_doc` переведены в
  компилируемые doctests; P09 — возврат владения при `Err` (изменений не требуется).
- **0018-05 (ВЫПОЛНЕНО):** P08 — первый проход аудита клонов (клоны Rc-хэндлов
  → `Rc::clone`, 2 клона устранены); P10 — `mem::take` (покрытие адекватно).

## Итог (что сделано)

Кодовая база приведена к чек-листу `docs/CODE.md` (Rust Design Patterns) по всем
13 задачам плана (P01–P13 + P04b), декомпозированным на 5 подзадач 0018-01…05:

- **Аргументы/типы (P01–P03):** срезовые типы в сигнатурах и возвратах
  (`&str`/`&[T]` вместо `&String`/`&Vec`).
- **API/расширяемость (P04/P04b/P05/P06):** `#[non_exhaustive]` на публичных
  `Language`, `ErrorType` и узлах AST/`TypeNode`; «bool trap» `guard_enable`
  заменён на `GenerateOptions` (Default + non_exhaustive + `new()`); удалён
  мёртвый алиас `Source`.
- **Строки/владение (P11/P08/P10):** `" ".repeat` для отступов; клоны Rc-хэндлов
  сделаны явными/устранены; `mem::take` — по месту.
- **Документация (P12):** ключевые примеры переведены в компилируемые doctests.
- **Осознанные «не требуется» (P07/P09/P13):** Builder `GraphicsConfig`, возврат
  владения при `Err`, доп. конструкторы — отклонены по YAGNI с обоснованием.

**Ключевые решения:** осознанный выбор вместо слепого применения паттернов
(non_exhaustive только на растущих enum, не на стабильном `Level`; удаление
мёртвого кода вместо обёрток). Поведение генерации/диагностик **не изменилось**.
Публичный Rust-API изменён (ломающе) — `grammar` `0.0.4 → 0.0.5`; язык не менялся.

**Проверка:** 1438 тестов + 36 doc-тестов зелёные (отчёт:
[docs/reports/0018-code-guidelines.md](../reports/0018-code-guidelines.md)).
Дальнейший полный проход по клонам — итеративно, вне рамок фичи (см. 0018-05).

> Детальный план — в аналитическом документе (перенесён из бывшего корневого `PLAN.md`).
