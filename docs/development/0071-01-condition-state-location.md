# Задача 0071-01: use-site `Location` на `ConditionNode::State` + индекс/goto

> Фича: [../features/0071-lsp-goto-state-name.md](../features/0071-lsp-goto-state-name.md) · ADR: [../adr/0071-lsp-goto-state-name.md](../adr/0071-lsp-goto-state-name.md) · анализ: [../analyze/0071-lsp-goto-state-name.md](../analyze/0071-lsp-goto-state-name.md)

## Что было

`ConditionNode::State(Rc<RefCell<StateNode>>)` (`semantic/mod.rs:1320`) не несёт
use-site позицию. `condition.rs:195` отбрасывает доступный `id.loc`:

```rust
} else if let Some(state) = model.borrow().search_state(&name) {
    return Ok(ConditionNode::State(state.clone()));   // id.loc отброшен
```

Следствие: `collect_condition_entries` (`index.rs:587`) не имеет ветки для
`State` (падает в `_ => {}`, `index.rs:670`), узел в индекс не попадает, goto на
`End` в `S(Ping) = End` возвращает `None`.

## Что сделано (план по образцу 0056-04)

1. **Поле позиции** — `mod.rs:1320`: `State(Rc<RefCell<StateNode>>)` →
   `State(Rc<RefCell<StateNode>>, Location)` (второе поле — use-site, как у
   `Variable`/`Model`).
2. **Ручной `PartialEq`** — `mod.rs:1357`: `(Self::State(a, _), Self::State(b, _))
   => a == b` (позиция игнорируется, как уже сделано у `Variable`/`Model`,
   `mod.rs:1354-1356`). Проверить транзитивный путь `ModelNode::PartialEq`
   (`mod.rs:286`).
3. **Передача `id.loc`** — `condition.rs:195`: `ConditionNode::State(state.clone(),
   id.loc)`; в `rebuild_condition` (`condition.rs:210`) — тем же образом, что уже
   сделано для `Function` (line 219).
4. **Обновить match-места** (компилятор форсирует; добавить `_`/`..`): генераторы
   `sv_expr.rs:453`, `rust_expr.rs:947`/`981`, `st_expr.rs:300`,
   `c_expr/condition.rs:71`/`385`, `rust_needs.rs:374`; `lower_float.rs:523`;
   `validate/common.rs:133`. Все они состояние-в-условии не эмитят как значение —
   новое поле не читают.
5. **Индекс** — `index.rs`: вид `SemanticNodeKind::ReferenceState` (use-site
   состояния, рядом с `ReferenceModel`); ветка в `collect_condition_entries` для
   `ConditionNode::State(target, loc)` (гвард `Location::Source`; имя — из
   `target.borrow().name()`; `model: Some(...)` — контекст поиска).
6. **goto** — `lsp/goto.rs:159`: арм `declaration_location_of` для `ReferenceState`
   → диапазон **декларации** целевого состояния. Предпочтительно — через уже
   разрешённый `Rc` (loc декларации = `state.borrow().loc()`, без повторного поиска
   по имени и его неоднозначности между моделями); запасной вариант — зеркало
   резолвера `Reference` (`goto.rs:167-172`, `search_state`).

⚠️ **Позиция невидима для равенства** (R4): без этого две ссылки на одно
состояние из разных мест текста стали бы разными узлами → тихий регресс кодогена
(урок 0056-04). Сторож — unit-тест равенства + гейт детерминизма 0048.

## Примеры/контрпримеры и тесты

- **Зонд** (правило «сперва зонд»): на модели с `ref Stop: S(Ping) = End;`
  снять **фактический** диапазон, куда резолвится `End` (какая декларация
  состояния), — не угадывать строку.
- **Тест goto** (`lsp_goto_tests.rs` или `lsp_tests.rs`, `#[cfg(feature="lsp")]`):
  курсор на `End` → диапазон декларации состояния (по зонду). Образец —
  `t5_goto_opens_imported_file` (модель) / `goto_declaration_reference_resolves_to_state`.
- **Негативный сторож** (R4): два `ConditionNode::State` одного состояния с разными
  `Location` — равны (unit в `semantic`).

## Проверки

- Сборка `--features lsp`; `cargo test --features lsp -- --test-threads=1`.
- `./scripts/precheck.sh` зелёный; `git diff examples/generated/` пуст (A3).
- Регресс goto: существующие `lsp_tests`/`lsp_goto_tests` зелёные (A2).
