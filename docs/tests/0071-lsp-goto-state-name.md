# Тест-план фичи 0071: Переход на имя состояния в `S(Ping) = End`

> Фича: [../features/0071-lsp-goto-state-name.md](../features/0071-lsp-goto-state-name.md) · ADR: [../adr/0071-lsp-goto-state-name.md](../adr/0071-lsp-goto-state-name.md) · анализ: [../analyze/0071-lsp-goto-state-name.md](../analyze/0071-lsp-goto-state-name.md)

## Область и цель

Проверить, что goto на имени состояния в условии открывает декларацию состояния,
без регресса прочих goto и без изменения вывода генераторов. Фича — LSP/индекс,
язык не меняет (правило 16 о примерах языка неприменимо в части синтаксиса;
«пример» здесь — сценарий навигации).

## Проверки (условие → ожидаемый результат)

| # | Проверка | Предусловие | Ожидаемый результат | Ссылка на R/A |
|---|---|---|---|---|
| T1 | **Зонд:** куда резолвится `End` | модель с `ref Stop: S(Ping) = End;` | захвачен фактический диапазон декларации состояния (для T2) | R3 / A1 |
| T2 | goto на `End` в `S(Ping) = End` | курсор на `End` | диапазон декларации состояния (по T1) | R1,R2,R3 / A1 |
| T3 | запись индекса создана | тот же исходник | `ReferenceState` под смещением `End` (зонд-guard) | R2 / A1 |
| T4 | goto на модели `S(Ping)` (курсор на `Ping`) | 0056 | `ReferenceModel`, декларация модели — без регресса | R6 / A2 |
| T5 | goto на `ref`-ребре (`ref Moving`) | существующий | декларация состояния — без регресса | R6 / A2 |
| T6 | goto на переменной условия | существующий | декларация переменной — без регресса | R6 / A2 |
| T7 | равенство игнорирует позицию | два `State(rc, loc1)`/`State(rc, loc2)` | равны | R4 / A4 |
| T8 | вывод генераторов байт-в-байт | весь `examples/` | `git diff examples/generated/` пуст; гейт 0048 | R5 / A3 |
| T9 | `precheck.sh` зелёный (в т.ч. `--features lsp`) | все инструменты | `EXIT=0` | R5,R6 / A5 |

## Разбивка проверок по функциональности

<!-- Легенда: ✅ пройдено · ❌ провалено · ⬜ не проверялось · — не применимо -->

| Функциональность | Условие | Статус |
|---|---|---|
| Индекс (`ReferenceState`) | запись под use-site `End` | ✅ (зонд подтвердил узел `ReferenceState`) |
| LSP goto (кросс-модельный `S(Ping)=Done`) | `Done` → декларация состояния | ✅ `goto_state_name_in_condition_resolves_to_declaration` |
| LSP goto (внутримодельный `x=Done`) | `Done` → декларация состояния | ✅ `goto_same_model_state_node_resolves_to_declaration` |
| goto модели `S(Ping)` / ребра / переменной | без регресса | ✅ `goto_model_name_in_state_of_still_resolves_to_model` + `goto_exact_file::*` |
| Кодоген всех целей | вывод байт-в-байт неизменен | ✅ `git diff examples/generated/` пуст |
| Равенство `ConditionNode` | позиция игнорируется | ✅ `condition_state_equality_ignores_use_site` |

## Уточнение (выяснено при разработке)

Предпосылка ADR («`End` → `ConditionNode::State`, ветка теряет `id.loc`») —
**неполна**. Зонд headline-случая: `S(Ping) = End` резолвится в
`Equal(Function(Builtin S, [Model(Ping)]), Unresolved(Variable("End")))` — `End`
остаётся `Unresolved` (сестра `Ping` невидима резолверу текущей модели), а **не**
становится `State`. Поэтому:

- **T2 переформулирован**: разбор `S(Модель) = Состояние` живёт на уровне
  `ConditionNode::Equal` в индексе (`try_collect_state_of_model`), а не через
  поле `ConditionNode::State`.
- **T2b добавлен** (внутримодельный `x = Done`): именно там рождается
  `ConditionNode::State` — это и есть покрытие Option A ADR.
- Подробности — `docs/development/0071-01-condition-state-location.md`,
  раздел «⚠️ Уточнение при разработке».

## Тестовые данные и окружение

- **Фикстура/исходник:** модель с `ref Stop: S(Ping) = End;` (образец — эталонная
  модель `lib.rs` / `c_state_ref_tests.rs`, где `S(Ping) = ...` уже используется).
- **Зонд** (правило проекта): захватить фактический диапазон декларации `End`
  перед ассертами — не угадывать строку.
- **Окружение:** `cargo test --features lsp -- --test-threads=1`; полный
  `precheck.sh` (тесты LSP под `--all-features`), гейт детерминизма 0048.
