# Фича 0071: Переход на имя состояния в `S(Ping) = End` не работает

- **Номер:** 0071
- **Статус:** ГОТОВО (реализовано и проверено 2026-07-20; `precheck.sh` зелёный)
- **Зависит от:** **нет** (строится на закрытой 0056 — не блокирует)
- **Приоритет / Tier:** **Tier 3** — эргономика LSP (навигация; не дефект компилируемости)
- **Крейт:** `grammar` (`semantic/mod.rs`, `semantic/condition.rs`, `semantic/index/`, `lsp/goto.rs`)
- **Связанные issue (анализ):** новая фича (перевод кандидата из `FEATURES.md`); прямое продолжение [0056](0056-lsp-goto-exact-file.md) (граница 0056-04)

## Ссылки на артефакты (жизненный цикл, правило 17)

| Стадия | Артефакт |
|---|---|
| Архитектура (ADR) | [`0071-lsp-goto-state-name.md`](../adr/0071-lsp-goto-state-name.md) |
| Анализ | [`0071-lsp-goto-state-name.md`](../analyze/0071-lsp-goto-state-name.md) |
| Разработка | [`0071-01`](../development/0071-01-condition-state-location.md) (use-site Location + индекс/goto) |
| Тест-план | [`0071-lsp-goto-state-name.md`](../tests/0071-lsp-goto-state-name.md) |
| Отчёт о тестировании | [`docs/reports/`](../reports/README.md) (`0071-*` — при тестировании) |
| Исправления | [`docs/fixes/`](../fixes/README.md) (при необходимости `0071-YY-*`) |

## Краткое описание

`ConditionNode::State(Rc<RefCell<StateNode>>)` позиции использования **не несёт** —
ровно тот класс, что [0056](0056-lsp-goto-exact-file.md) починила у ссылок на
модель (`Extend::Model`, `ConditionNode::Model` получили `Location`), но другой
сценарий.

Лечение то же и по тому же образцу: второе поле — позиция use-site, ветка в
`collect_condition_entries`, вид узла (`ReferenceState`), ветка в
`declaration_location_of`.

⚠️ **Равенство обязано позицию игнорировать**: `ConditionNode` сравнивается
транзитивно через `ModelNode::PartialEq`, и автовыведённое равенство сделало бы две
ссылки на одно состояние **разными узлами** (→ поехал бы кодоген). Разбор приёма —
задача [0056-04](../development/0056-04-model-reference-location.md).

Выявлено при закрытой [0056](0056-lsp-goto-exact-file.md).

> Фича зарегистрирована **2026-07-17** переводом кандидата из `FEATURES.md`
> (решение заказчика: «завести фичи по кандидатам, пока без проработки»).
> **Проработка не проводилась:** ADR, анализ, зависимости, Tier и объём — за
> стадиями 2–3 (правило 17). Текст ниже — **перенос находки кандидата** вместе с
> подтверждающими её пробами; это описание проблемы, а **не** принятое решение.

## Итог (что сделано)

`goto declaration` на имени состояния в условии открывает декларацию состояния.
Реализовано **два** механизма — при разработке зонд выяснил, что предпосылка ADR
(«`End` → `ConditionNode::State`») покрывала лишь один из них:

1. **Кросс-модельный `S(Ping) = End`** (headline). `End` — состояние
   модели-аргумента, текущая модель его не видит → резолвер оставляет
   `ConditionNode::Unresolved(Variable)` (инвариант «`ref` не разрешается»). Разбор
   `S(Модель) = Состояние` — на уровне `ConditionNode::Equal` в
   `semantic/index/collect.rs` (`try_collect_state_of_model` + `state_of_model_cond`,
   зеркало `c_expr::condition::state_of_model`): имя резолвится в области
   модели-аргумента и кладётся `SemanticNodeKind::ReferenceState` с ней в контексте.
2. **Внутримодельный `x = Done`** (Option A ADR). Имя состояния **той же** модели
   резолвится в `ConditionNode::State(Rc, use-site)`; второе поле — позиция,
   как у `ConditionNode::Model` (0056). Ветка `ConditionNode::State` в индексе
   создаёт `ReferenceState`.

`declaration_location_of` получил арм `Reference | ReferenceState` (поиск состояния
в модели-контексте узла). Равенство `ConditionNode::State` позицию **игнорирует**
(ручной `PartialEq`; сторож `condition_state_equality_ignores_use_site`). Вывод
генераторов на корпусе **байт-в-байт неизменен** (гейт 0048). `index.rs` разделён на
`index/{mod,collect}.rs` (лимит размера). Версия языка не поднята.

**Тесты** (`grammar/tests/lsp_goto_tests.rs`): `goto_state_name_in_condition_
resolves_to_declaration` (T2), `goto_same_model_state_node_resolves_to_declaration`
(T2b), `goto_model_name_in_state_of_still_resolves_to_model` (T4, без регресса
0056), `condition_state_equality_ignores_use_site` (T7). `precheck.sh` зелёный.

**Расхождение с ADR** зафиксировано в `docs/development/0071-01` (раздел «⚠️
Уточнение при разработке») и тест-плане — ADR остаётся `Accepted`, но его механизм
дополнен разбором на уровне `Equal`, без которого headline-случай не закрывался.
