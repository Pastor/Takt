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

## Прогресс

- **0018-01 (ВЫПОЛНЕНО):** P01–P04, P06 — срезовые типы в сигнатурах,
  `#[non_exhaustive]` на `Language`/`ErrorType`, удаление мёртвого алиаса `Source`.
- **0018-02 (ВЫПОЛНЕНО):** P05 — `GenerateOptions` вместо `guard_enable: bool`;
  P07 — Builder `GraphicsConfig` признан ненужным (YAGNI: конфиг только из
  serde/`Default`).
- **Осталось:** P08–P13, P04b (см. план и подзадачи).

> Детальный план — в аналитическом документе (перенесён из бывшего корневого `PLAN.md`).
