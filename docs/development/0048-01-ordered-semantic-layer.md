# Задача 0048-01: Упорядоченный общий слой (`ModelNode`, `minimap::Map`, `Ord` для `Name`)

> Фича: [../features/0048-deterministic-codegen.md](../features/0048-deterministic-codegen.md) · ADR: [../adr/0048-deterministic-codegen.md](../adr/0048-deterministic-codegen.md) · анализ: [../analyze/0048-deterministic-codegen.md](../analyze/0048-deterministic-codegen.md)

## Что было

Словари общего слоя — `HashMap`, порядок обхода которого рандомизирован
(`RandomState`). Генераторы печатают то, что им выдал обход, поэтому все пять
примеров корпуса дают 5 разных `.h` из 5 прогонов одного бинарника, а значение
`enum` порта `CABIN_BUTTON_DC` плавает между `= 0` и `= 2` (проба, 15 прогонов).

Затронутые места (аудит 2026-07-16):

- `semantic/mod.rs:73-115` — 11 полей `ModelNode`. Вывода достигают шесть:
  `models` (→ `minimap.rs:461` `visit_extend` → `elements` → `used_models()` →
  все три бэкенда), `states` (→ `minimap.rs:195` и `:470` → `Element::Model.states`
  → печать), `variables` (→ `c_header.rs:339`, `c_model.rs:104` — **без**
  сортировки), `enums`, `structs`, `functions` (в C/ST обезврежены локальными
  сортировками).
- `semantic/minimap.rs:170` — `elements: HashMap<Name, Element>`; единственный
  обход `.values()` в `used_models()` (`:247-253`) идёт прямо во все бэкенды.
- `semantic/minimap.rs:195` и **`:470-472`** — **два независимых** пути
  построения `Element::Model.states`: первый для корневой модели, второй
  (`m_rc.borrow().states.keys().cloned()`) для вложенных.
- `semantic/tree.rs:795,873` — `HashMap::with_capacity(...)`.

## Что сделано

Порядок стал свойством типа контейнера (ADR 0048, Option A):

1. **`ModelNode` → `BTreeMap<String, _>`** для всех 11 полей: `models`,
   `functions`, `variables`, `types`, `type_locs`, `raw_type_defs`, `conditions`,
   `enums`, `structs`, `states`, `docs`. Шесть обязательны (достигают вывода),
   пять взяты по «гигиене» (анализ, раздел «Объём»): цена нулевая, а
   `types`/`conditions` достигают автодополнения LSP (`lsp.rs:758`, `:768`).
2. **`minimap::Map.elements` → `BTreeMap<Name, Element>`**, вместе с сигнатурами
   `visit_state` (`:361`) и `visit_extend` (`:461`).
3. **`impl Ord`/`PartialOrd` для `Name`** — **ручной**, по паре `(unique, local)`.
   Не `derive`: тот дал бы лексикографику по `local` первым, разойдясь с
   конвенцией остального кода (`st/mod.rs:153`, `st_model.rs:93` сортируют по
   `unique()`). Сравнение обоих полей сохраняет согласованность с `Eq`
   (требование `BTreeMap`). `Hash` оставлен — `Name` остаётся ключом
   промежуточных `HashMap`.
4. **`with_capacity` снят** (`tree.rs:795,873`) — у `BTreeMap` его нет; это
   единственные два места, где механическая замена не проходит компиляцию.

**Не тронуто** (обосновано анализом): `PortMap` (`c_header.rs:16,234`; обход
фиксированными циклами, ключ — не источник порядка), `UsageSet` (`unused.rs`;
только `contains`), `address_map.rs` (только `get`), `verification/buchi.rs`
(вне пути кодогенерации), 13 сортировок цели `st` (решение заказчика — остаются
контрольным тестом).

## Проверки

- `cargo build --all-features --all-targets` — сборка (правило 5).
- `cargo test -- --test-threads=1` — ~1400 тестов зелены (A8).
- **Контрольный тест R3/A3:** вывод цели `st` **не изменился ни в одном байте** —
  `st` детерминирована и до правки, значит правка общего слоя дала тот же
  порядок, что и её 13 локальных сортировок. Изменился `.st` → правка неверна.
- Проба A4: 15 прогонов `elevator_mini -t c`, `grep CABIN_BUTTON_DC` → одно
  значение.
