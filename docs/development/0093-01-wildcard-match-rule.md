# Задача 0093-01: Запрет _ => в семантических узлах и разрешение конфликта с #[non_exhaustive]

> Фича: [../features/0093-wildcard-match-rule.md](../features/0093-wildcard-match-rule.md) · ADR: [../adr/0093-wildcard-match-rule.md](../adr/0093-wildcard-match-rule.md) · анализ: [../analyze/0093-wildcard-match-rule.md](../analyze/0093-wildcard-match-rule.md)

## Что было

Инвариант «семантические узлы разбираются исчерпывающе» (ADR 0025) жил только в
коде `eval/` (`#![deny(clippy::wildcard_enum_match_arm)]`) и в `CLAUDE.md`, но
**не** в своде `docs/CODE.md`. Свод при этом **предписывал** `#[non_exhaustive]` без
исключения — атрибут, который тихо **отключил** бы этот инвариант, если применить
его к `ExpressionNode`.

## Что сделано

Реализована **Option A** [ADR 0093](../adr/0093-wildcard-match-rule.md).

- **`docs/CODE.md` («Расширяемость и API»):**
  - к рекомендации `#[non_exhaustive]` добавлено **явное исключение**:
    `ExpressionNode`/`ConditionNode`/`StatementNode` пометке **не** подлежат (иначе
    инвариант ADR 0025 тихо умрёт);
  - новое правило: разбирать эти узлы **исчерпывающе** (без `_ =>`), закрепляя
    `#![deny(clippy::wildcard_enum_match_arm)]` в модуле семантики вычислений
    (`eval/`); точечные `#[allow]` — только где `_` возвращает **ошибку**, не тихий
    `None`.
- **Гейт** `scripts/check-exhaustive-nodes.sh` в `precheck.sh`: падает, если
  (1) любой из трёх узлов помечен `#[non_exhaustive]` (awk привязывает атрибут к
  узлу через блок атрибутов), либо (2) `eval/mod.rs` потерял `deny`. Ловит оба
  пути «тихой смерти» инварианта.
- **Указатель в коде:** `eval/mod.rs` — заметка, что снятие `deny`/пометка узла
  ловится гейтом (не снимать).

| Функциональность | Статус |
|---|---|
| правило + исключение в `docs/CODE.md` | ✅ |
| гейт `check-exhaustive-nodes.sh` в precheck | ✅ |
| код крейтов / вывод / поведение | н/п — не тронуты (аддитивно) |
| рецидив `builder.rs::eval_expr` | вне объёма → кандидат (владелец 0034) |

## Проверки

- Гейт зелёный на текущем дереве (`exit 0`).
- Пробы (на временных копиях): пометка `ExpressionNode` `#[non_exhaustive]` → awk
  возвращает `ExpressionNode` (гейт красный); снятие `deny` в `eval/mod.rs` → grep
  не находит (гейт красный); чистый файл → awk пусто (нет ложных срабатываний).
- Код не тронут → вывод целей и поведение симулятора неизменны.
- Полный `./scripts/precheck.sh` → зелёный (см.
  [отчёт](../reports/0093-wildcard-match-rule.md)).
