# Отчёт о тестировании — Фича 0036: Согласование видимости публичного API крейта `simulation`

- **Фича:** [0036](../features/0036-sim-visibility.md)
- **ADR:** [0036](../adr/0036-sim-visibility.md) · **Анализ:** [0036](../analyze/0036-sim-visibility.md) · **Тест-план:** [0036](../tests/0036-sim-visibility.md)
- **Задачи:** [0036-01](../development/0036-01-sim-visibility.md) (инкапсуляция), [0036-02](../development/0036-02-lint-pin.md) (защёлка/версия)
- **Дата:** 2026-07-19
- **Вердикт:** ✅ **ГОТОВО**. `./scripts/precheck.sh` — EXIT=0; `private_interfaces` 16 → 0; линт-защёлка доказана пробой утечки (`warning` → `error`).

## Сводка

`Unit` инкапсулирован (ADR 0036, Option B): `pub enum Unit` заменён на
непрозрачный `pub struct Unit(UnitKind)` с **приватным** полем и приватным
`pub(crate) enum UnitKind`. Публичный API крейта не пополнился ни одним новым
типом — внутренние типы (`Context`/`Flow`/`Predicate`/`Guards`/`Execution`)
честно остались `pub(crate)`, а рассогласование, дававшее предупреждения
`private_interfaces`, устранено в корне. Защёлка `#![deny(private_interfaces)]`
в `lib.rs` держит это механически: следующая утечка `pub(crate)`-типа наружу
валит сборку.

## Окружение

| Компонент | Значение |
|---|---|
| Тип | `simulation/src/unit/mod.rs` — `pub struct Unit(UnitKind)` + приватный `enum UnitKind`; хелперы `from_kind`/`kind`/`kind_mut` |
| Потребители | `state_io.rs` (через `kind()`/`kind_mut()`/`from_kind`), `unit/builder.rs`, `unit/viewport.rs` (потомки `unit` — через те же аксессоры) |
| Защёлка | `simulation/src/lib.rs` — `#![deny(private_interfaces)]` (точечный линт, **не** `deny(warnings)`) |
| Тесты | Юнит-тесты `Unit` вынесены в новый `simulation/src/unit/tests.rs` (`mod tests;`) — 43 теста, утверждения не менялись |
| Версия | крейт `simulation` 0.3.0 → 0.4.0 (слом формы публичного `Unit`; язык Lam не менялся) |

## Эталон «до»

`cargo clean -p simulation && cargo build -p simulation --all-targets`:
**16** предупреждений `private_interfaces` (не 10, как в базе задачи от коммита
`6984471`: с тех пор фича 0044 добавила поле `guards: Guards` с приватным
`Predicate` — +6). Разбивка: `Context`×5, `Flow`×8, `Predicate`×2, `Guards`×1.
Плюс одно предупреждение вне области — `field 'end' is never read`
(`grammar/src/format/comments.rs`).

## Сверка с тест-планом

| # | Проверка | Результат |
|---|---|---|
| T1 (эталон «до») | `grep -c "more private than"` | ✅ **16** (база задачи 10 устарела — врезка выше) |
| T1 (ключевая) | то же после правки | ✅ **0** |
| T2 | Сборка крейта чистая: `grep -c "^warning"` (без `grammar`) | ✅ **0** (ушли 16 `private_interfaces`; `unused import` не было) |
| T3 | Публичный API не расширился; внутренние типы `pub(crate)` | ✅ `pub use unit::{TickResult, Unit}` неизменен; `Context`/`Flow`/`Predicate`/`Execution`/`UnitKind` — `pub(crate)`, `UnitKind` **не** реэкспортирован |
| T4 | Форма типа | ✅ `pub struct Unit(UnitKind)`, `enum UnitKind` без `pub` и без реэкспорта |
| T5 | Проба: внешний `match unit { Unit::Node {..} }` не компилируется | ✅ вариантов у `Unit` больше нет — конструкция невыразима вне крейта |
| T6 | `deny(private_interfaces)` в `lib.rs` | ✅ строка на месте |
| T7 | `deny(warnings)` отсутствует | ✅ `grep -rn "deny(warnings)"` — пусто |
| T8 (защёлка) | Проба утечки `pub fn leak() -> Flow` | ✅ `error: type Flow is more private than … leak_probe` (именно **error**, не warning) — проба откачена |
| T9/T12 | `cargo test -p simulation -- --test-threads=1` | ✅ 336 passed, 1 ignored; `git diff` по `simulation/tests/` пуст |
| T10 | `cargo test --features lsp -- --test-threads=1` | ✅ (в составе precheck) |
| T11 | `./scripts/run_simulations.sh` | ✅ (в составе precheck) — вывод совпадает с «до» |
| T13 | Набор аксессоров `pub fn` неизменен | ✅ `tick`/`variable`/`current_state`/`active_states`/`is_terminal`/`take_last_transition(s)`/`reachable_from_active`/`execution`/`union`/`add` |
| T14 | Версия крейта | ✅ `0.4.0` (см. «Отклонения»); версия языка Lam не менялась |
| T15 | `./scripts/precheck.sh` | ✅ EXIT=0 |
| T16 | Сборка workspace `--all-features --all-targets` | ✅ `private_interfaces` отсутствуют; единственное постороннее — `field 'end'` в `grammar` (вне области) |

## Отклонения от плана задач (база устарела)

База задач снята на коммите `6984471` (2026-07-15); к 2026-07-19 её опередили
закрытые фичи. Три пункта скорректированы **по факту кода**, не по плану:

1. **`private_interfaces` 10 → 16.** Фича 0044 (`guards`) добавила утечки
   `Predicate`/`Guards`. Направление правки не изменилось.
2. **Версия крейта 0.1.0 → ~~0.2.0~~ → 0.4.0.** База задачи 0036-02 знала версию
   `0.1.0`; фактически фича 0034 уже подняла её до `0.3.0`. Слом формы публичного
   `Unit` (0.x SemVer: ломающее → минор) даёт **0.3.0 → 0.4.0**.
3. **`unused import: StatementNode` (0036-02, п.2) — уже вычищен** до 0036
   (`grep StatementNode simulation/src/unit/builder.rs` — пусто). Правка не
   потребовалась; чистить нечего.

## Находки

- **Лимит размера модуля вынудил вынести тесты.** `unit/mod.rs` был в реестре
  долга (1331). Newtype + хелперы файл растят, а храповик
  `check-module-size.sh` рост записи запрещает. Правило само предписывает
  «вынести новое в отдельный модуль» — тест-модуль (43 теста) переехал в
  `unit/tests.rs` **без изменения утверждений** (лишь конструкторы адаптированы
  под newtype: `Unit(UnitKind::…)`, `Unit::default()` вместо `Unit::None`).
  `mod.rs` ужат до 761 строки → запись **удалена** из реестра (долг 21 → 20,
  строк 12049 → 11775). Строго лучше: рассогласование убрано, а долг уменьшен.
- **Диспетчер `tick` без удержания заимствования.** `match &self.0 { … =>
  self.tick_node() }` дал бы конфликт (`&self.0` жив, а ветвь просит `&mut
  self`) — там, где `pub enum` матчился по `self` без живого заимствования.
  Заменено на последовательность `matches!(self.0, UnitKind::…)` + ранний
  `return`: `matches!` борроу не удерживает.
- **Граница «потомок / не потомок».** `builder`/`statement`/`viewport` —
  подмодули `unit`, им приватная форма доступна напрямую; `state_io` — сиблинг,
  для него и заведены `pub(crate)`-хелперы `from_kind`/`kind`/`kind_mut` (ADR,
  R3). `statement.rs` варианты `Unit` не разбирает — правок не потребовал.

## Дефекты

Не найдено. Фиксы (`docs/fixes/0036-YY-*`) не заводились.

## Итог

Критерии A1–A11 и требования R1–R9 выполнены: `private_interfaces` устранены
(A1), сборка крейта чистая (A2), публичный API не расширен (A3), форма `Unit`
непрозрачна (A4/A5), защёлка доказана пробой (A7/A8), поведение симулятора
неизменно (A6 — 336 тестов + `run_simulations.sh`). Версия языка не менялась
(правило 18); крейт `simulation` — минорный бамп `0.3.0 → 0.4.0` (слом формы
публичного `Unit`). Фича закрыта.

Побочный кандидат (вне области, правило 7): предупреждение
`field 'end' is never read` в `grammar/src/format/comments.rs` — мешает
включить глобальный `-D warnings` в CI (пункт для координатора).
