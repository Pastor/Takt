# Фича 0082: `semantic/unused.rs` не обходит `formulas`

- **Номер:** 0082
- **Статус:** ГОТОВО (2026-07-20; `precheck.sh` зелёный)
- **Зависит от:** нет (опора — `semantic/unused.rs`, типы `Formula`/`Ltl`); обострено [0081](0081-lamc-print-warnings.md)
- **Приоритет / Tier:** **Tier 2** — ложное предупреждение (вводит в заблуждение, кода не портит)
- **Крейт:** `grammar` (`semantic/unused.rs`)
- **Связанные issue (анализ):** новая фича (перевод кандидата из `FEATURES.md`)

## Ссылки на артефакты (жизненный цикл, правило 17)

| Стадия | Артефакт |
|---|---|
| Архитектура (ADR) | [`docs/adr/0082-unused-formulas.md`](../adr/0082-unused-formulas.md) |
| Анализ | [`docs/analyze/0082-unused-formulas.md`](../analyze/0082-unused-formulas.md) |
| Разработка | [`docs/development/0082-01-traverse-formulas.md`](../development/0082-01-traverse-formulas.md) |
| Тест-план | [`docs/tests/0082-unused-formulas.md`](../tests/0082-unused-formulas.md) |
| Отчёт о тестировании | [`docs/reports/0082-unused-formulas.md`](../reports/0082-unused-formulas.md) |
| Исправления | [`docs/fixes/`](../fixes/README.md) (не потребовались) |

## Краткое описание

Переменная, используемая **только в формуле** (LTL/Guard), считается
неиспользуемой — то есть Ce13 даёт **ложное** предупреждение.

Выявлено при [0035](0035-ltl-in-blocks.md).

## Итог (что сделано) — 2026-07-20

Принята **Option A** (ADR 0082): `unused.rs` обходит формулы наравне с прочими
узлами. Добавлены `collect_from_formula` (Guard → `collect_from_condition`, LTL →
`collect_from_ltl`, `Formulas` → рекурсия) и `collect_from_ltl` (обход атомов);
вызов — для `ModelNode::formulas` (в `collect_from_model_tree`) и
`StateNode::formulas` (в `collect_from_state`, добавлено в деструктуризацию обоих
вариантов). Переменная только в `: [LTL] G flag;` или `invariant Inv = g` больше
**не** даёт ложный `SE-036`; реально мёртвая — по-прежнему даёт (негативный
сторож). Дефект обострён 0081 (Ce13 теперь печатается). Язык не менялся, версия
не поднята.

## История

> Фича зарегистрирована **2026-07-17** переводом кандидата из `FEATURES.md`.
> Проработана и закрыта 2026-07-20. Текст ниже — исходная находка кандидата.
