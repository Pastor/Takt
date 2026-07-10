# Фича 0018: Приведение кода к требованиям docs/CODE.md

- **Номер:** 0018
- **Статус:** РАЗРАБОТКА
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
- **Осталось:** P08 (аудит `.clone()`), P10 (`mem::take`).

> Детальный план — в аналитическом документе (перенесён из бывшего корневого `PLAN.md`).
