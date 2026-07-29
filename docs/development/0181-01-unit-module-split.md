# Задача 0181-01: Деление takt-sim/src/unit/mod.rs — вынос такта в unit/tick.rs

> Фича: [../features/0181-sim-state-implementation-tick.md](../features/0181-sim-state-implementation-tick.md) · ADR: [../adr/0181-sim-state-implementation-tick.md](../adr/0181-sim-state-implementation-tick.md) · анализ: [../analyze/0181-sim-state-implementation-tick.md](../analyze/0181-sim-state-implementation-tick.md)

## Что было

`takt-sim/src/unit/mod.rs` — **988 строк при лимите 1000**
(`scripts/check-module-size.sh`, правило `docs/CODE.md`). Задачи 0181-02 и
0181-03 правят именно такт и добавляют строки, то есть упёрлись бы в гейт
размера прежде, чем что-либо заработало. Подготовка обязана идти первой.

В реестр долга `scripts/module-size-baseline.txt` файл **не вносился и не
вносится**: реестр замораживает нарушителей, а `mod.rs` в лимит укладывался —
запись там провернула бы храповик назад.

## Что сделано

Чистый вынос по теме «что происходит за один такт» в новый
`takt-sim/src/unit/tick.rs` (`use super::*;` + собственный блок `impl Unit`) —
образец `takt-lang/src/semantic/validate/` (приёмы фичи 0088):

| Перенесено | Назначение |
|---|---|
| `tick`, `tick_soft`, `tick_mode`, `tick_body` | публичный контракт такта и его тело |
| `check_guards` | инварианты (фича 0044) и мягкий режим (фича 0087) |
| `tick_node`, `tick_parallel`, `tick_sequential` | диспетчеризация по форме узла |
| `enter_initial_state` | вход в стартовое состояние (фича 0033) |

В `mod.rs` остались определения (`Predicate`, `Flow`, `Guards`, `TickResult`,
`UnitKind`), `impl Context for Unit`, композиция юнитов (`add`/`union`),
наблюдение значений (`get_qualified`/`set_qualified`) и сбор трасс
(`take_last_transitions`, `take_invariant_violations`, `reachable_from_active`).

**Поведение не менялось**: перенесены целые функции, ни одна строка тела не
правлена. Приватные элементы родителя (`describe`, поля вариантов `UnitKind`)
видны потомку модуля — дополнительной публичности не потребовалось.

Обратная функциональность (правило 11): затронут только внутренний состав
модулей крейта `takt-sim`; публичный API (`Unit::tick`, `Unit::variable`, …)
байт-в-байт прежний, для прочих стеков — **н/п**.

## Проверки

| Что | Результат |
|---|---|
| `cargo build` | без ошибок и предупреждений |
| `cargo fmt` | применён, диффа нет |
| `cargo clippy --all-targets --all-features -- -D warnings` | чисто |
| `cargo test --all-features -- --test-threads=1` | зелёный, провалов нет |
| `scripts/check-module-size.sh` | 272 файла, записей долга 4 (без изменений) |

Размеры: `mod.rs` **988 → 676**, новый `tick.rs` — **328**. Запас под 0181-02 и
0181-03 есть у обоих.
