# Анализ фичи 0036: Согласование видимости публичного API крейта simulation

> Фича: [../features/0036-sim-visibility.md](../features/0036-sim-visibility.md) · ADR: [../adr/0036-sim-visibility.md](../adr/0036-sim-visibility.md) · тест-план: [../tests/0036-sim-visibility.md](../tests/0036-sim-visibility.md)

## Цель и контекст

Устранить рассогласование видимости в публичном API крейта `simulation`:
перечисление `Unit` объявлено `pub`, а типы его полей (`Context`, `Flow`,
`Predicate`) — `pub(crate)`. Компилятор сообщает об этом 10 предупреждениями
`private_interfaces` при каждой сборке. Решение ([ADR 0036](../adr/0036-sim-visibility.md),
Option B) — инкапсулировать `Unit` (`pub struct` + приватный `enum UnitKind`),
не расширяя публичный API, и закрепить результат точечным линтом
`#![deny(private_interfaces)]`.

Правило 18: язык Lam не затрагивается — фича целиком внутри крейта `simulation`.
Правило 22: версия **языка** не меняется; версия **крейта** `simulation`
поднимается `0.1.0` → `0.2.0` (см. «Особенности по обратной функциональности»).

### Фактическое состояние кода (замер, а не пересказ кандидата)

Команда: `cargo build -p simulation --all-targets` (и `cargo clippy -p simulation
--all-targets` — тот же набор, дополнительных `private_interfaces` не даёт).
Дата замера: 2026-07-15, ветка `v2`, коммит `6984471`.

**Все 10 предупреждений `private_interfaces`** (счётчик подтверждён:
`cargo clippy -p simulation --all-targets 2>&1 | grep -c "more private than"` → `10`):

| # | Текст предупреждения | Место поля |
|---|---|---|
| 1 | trait `context::Context` is more private than the item `unit::Unit::Node::context` | `simulation/src/unit/mod.rs:88` |
| 2 | trait `context::Context` is more private than the item `unit::Unit::Node::state_executions` | `simulation/src/unit/mod.rs:91` |
| 3 | trait `context::Context` is more private than the item `unit::Unit::Node::executions` | `simulation/src/unit/mod.rs:95` |
| 4 | trait `context::Context` is more private than the item `unit::Unit::Parallel::executions` | `simulation/src/unit/mod.rs:109` |
| 5 | trait `context::Context` is more private than the item `unit::Unit::Sequential::executions` | `simulation/src/unit/mod.rs:114` |
| 6 | type `Predicate` is more private than the item `unit::Unit::Node::state_transitions` | `simulation/src/unit/mod.rs:90` |
| 7 | type `Flow` is more private than the item `unit::Unit::Node::state_executions` | `simulation/src/unit/mod.rs:91` |
| 8 | type `Flow` is more private than the item `unit::Unit::Node::executions` | `simulation/src/unit/mod.rs:95` |
| 9 | type `Flow` is more private than the item `unit::Unit::Parallel::executions` | `simulation/src/unit/mod.rs:109` |
| 10 | type `Flow` is more private than the item `unit::Unit::Sequential::executions` | `simulation/src/unit/mod.rs:114` |

Объявления «слишком приватных» типов: `Context` — `simulation/src/context.rs:4`
(`pub(crate) trait`); `Predicate` — `simulation/src/unit/mod.rs:26`
(`pub(crate) struct`); `Flow` — `simulation/src/unit/mod.rs:52`
(`pub(crate) enum`). `Flow`/`Context` утекают через псевдонимы
`Execution` (`unit/mod.rs:65`) и `Executions` (`unit/mod.rs:66`).

> **Замечание о подсчёте.** При сборке `--all-targets` тот же набор печатается
> дважды (цель `lib` и цель тестов), из-за чего «сырой» вывод содержит до 17
> строк `warning:`. Уникальных предупреждений `private_interfaces` — **ровно
> 10**; проверка `cargo build -p simulation` (только `lib`) даёт те же 10.

**Сверка утверждений кандидата из `FEATURES.md`:**

| Утверждение кандидата | Вердикт | Факт |
|---|---|---|
| `Context` — `pub(crate)`, шумит | **подтвердилось** | `context.rs:4`, 5 предупреждений |
| `Predicate` — `pub(crate)`, шумит | **подтвердилось** | `unit/mod.rs:26`, 1 предупреждение |
| `Flow` — `pub(crate)`, шумит | **подтвердилось** | `unit/mod.rs:52`, 4 предупреждения |
| `Value` — `pub(crate)`, шумит | **НЕ подтвердилось** | `Value` **уже `pub`** (`eval/value.rs:14`), реэкспорт `lib.rs:29`. Предупреждений по нему **нет**. Модуль `eval::value` — `pub(crate)`, но сам тип публичен через реэкспорт, чего линту достаточно. Из объёма работ исключён. |
| `TickResult` исправлен задачей 0025-05 (стал `pub`) | **подтвердилось** | `unit/mod.rs:73` — `pub enum TickResult`, с комментарием-обоснованием. Правок не требует. |

**Прочие предупреждения сборки (для полноты; вне области фичи):**

| Предупреждение | Место | Решение |
|---|---|---|
| `unused import: StatementNode` | `simulation/src/unit/builder.rs:9` | крейт фичи, тривиально → задача **0036-02** |
| `field 'end' is never read` | `grammar/src/format/comments.rs:27` | **крейт `grammar`**, к фиче не относится → предложить пунктом бэклога (правило 7) |

### Ключевое ограничение языка (проверено компилятором)

Исходная формулировка «сузить поля `Unit` до `pub(crate)`» **невыполнима**:
поля вариантов перечисления всегда наследуют видимость перечисления. Пробный
крейт с `pub enum E { V { pub(crate) x: u8 } }` даёт:

```
error[E0449]: visibility qualifiers are not permitted here
  = note: enum variants and their fields always share the visibility of the enum they are in
```

Отсюда единственная форма инкапсуляции — `pub struct Unit(UnitKind)` с приватным
`enum UnitKind` (ADR 0036, Option B).

## Зависимости фичи (правило 17/19)

- **Зависит от:** **нет**.

  **Обоснование.** Проверены все возможные каналы зависимости:
  - *По контракту.* Фича не потребляет ничего от других незакрытых фич.
    Затрагиваемый код (`Unit`, `Context`, `Flow`, `Predicate`) существует и
    стабилен с момента закрытия 0025; `TickResult`/`Value` уже приведены в
    порядок задачей 0025-05, то есть предпосылка выполнена.
  - *По инфраструктуре.* Нужны только `cargo build`/`cargo test` и
    `scripts/precheck.sh` — всё на месте.
  - *По языку.* Синтаксис/семантика Lam не затрагиваются → зависимости от
    языковых фич (0035, 0041, 0042, 0044) нет.
  - *По процессу.* Нерешённый пункт бэклога «Конфликт правил `#[non_exhaustive]`
    против запрета `_ =>`» **не является зависимостью**: принятый Option B
    публичных перечислений не добавляет, поэтому вопрос не встаёт (в отличие от
    отвергнутого Option A, который сделал бы этот пункт блокирующим — см. ADR).

  Статус `ЗАБЛОКИРОВАНА` **не ставится**; фича может быть взята в работу
  немедленно.

- **Влияние на порядок разработки:**
  - Завершение 0036 **не разблокирует** ни одну фичу формально (никто от неё не
    зависит).
  - **Пересечение по файлам (не зависимость, а очерёдность).** 0036 правит
    `simulation/src/state_io.rs`, `unit/mod.rs`, `unit/builder.rs`,
    `unit/statement.rs`, `unit/viewport.rs`, `runner.rs`,
    `bin/simulation.rs`. Те же файлы трогают **0032** (переменные в
    `--save-state`, `state_io.rs`), **0034** (структурные типы), **0044**
    (assert/invariant). Рекомендация аналитика (правило 19, критерий 4):
    выполнить **0036 раньше** 0032/0034 — она мелкая, механическая и делает
    форму дерева симуляции приватной, после чего те фичи меняют внутренности,
    не задевая публичный API и не переоткрывая вопрос видимости. Обратный
    порядок означал бы правку тех же строк дважды.
  - Приоритет фичи низкий (Tier 3), поэтому в таблице `FEATURES.md` она
    **не** поднимается выше содержательных фич — рекомендация касается лишь
    относительного порядка внутри группы работ по `simulation`.

## Требования и проверяемые условия

- **R1. Ноль предупреждений `private_interfaces`.** `cargo build -p simulation`
  и `cargo build -p simulation --all-targets` не печатают ни одного
  предупреждения `private_interfaces` (эталон «было» — 10 шт., таблица выше).
- **R2. Публичный API не расширяется.** `Context`, `Flow`, `Predicate`,
  `Execution`, `Executions` остаются `pub(crate)`. Множество публичных типов
  крейта после фичи **не больше**, чем до неё: `Unit`, `TickResult`, `Value`
  (+ уже публичные модули `runner`, `state_io`, `graphics_config`,
  `json_input`).
- **R3. `Unit` непрозрачен.** `Unit` — `pub struct` с приватным полем; внутренний
  `enum UnitKind` не экспортируется, извне крейта не достижим и не конструируем.
- **R4. Поведение симулятора не изменяется.** Фича — рефакторинг видимости:
  результаты `tick`, значения переменных, трассы, сохранение/загрузка состояния
  и SVG/GIF-вывод остаются побайтово прежними. Ни одна существующая проверка не
  меняет ожидаемого результата.
- **R5. Закрепление линтом, но не `deny(warnings)`.** В `simulation/src/lib.rs`
  добавлен `#![deny(private_interfaces)]`. `#![deny(warnings)]` **не**
  добавляется — прямой запрет `docs/CODE.md` («ломает сборку при обновлении
  компилятора; настраивай lint-ы точечно и в CI»).
- **R6. Закрепление доказано негативно.** Существует воспроизводимая проверка:
  временное возвращение утечки (`pub`-элемент с `pub(crate)`-типом) **валит**
  сборку крейта. Без этой проверки R5 — необоснованное утверждение.
- **R7. Аксессоры сохранены.** Публичные методы `Unit` (`tick`,
  `take_last_transition`, `take_last_transitions`, `reachable_from_active`,
  `execution`, `variable`, `current_state`, `active_states`, `is_terminal`,
  `union`, `add`) сохраняют сигнатуры и семантику — именно они и есть публичный
  контракт крейта.
- **R8. Версионирование (правило 22).** Версия крейта `simulation` в
  `simulation/Cargo.toml`: `0.1.0` → `0.2.0`. Версия языка Lam **не меняется**.
- **R9. Чистота крейта фичи.** Устранено `unused import: StatementNode`
  (`unit/builder.rs:9`). Предупреждение в крейте `grammar`
  (`format/comments.rs:27`) — **вне области**, не трогается.

## Критерии приёмки и способ проверки

| # | Критерий | Способ проверки |
|---|---|---|
| A1 | Предупреждений `private_interfaces` — 0 (было 10) | `cargo build -p simulation --all-targets 2>&1 \| grep -c "more private than"` → `0` (R1) |
| A2 | Сборка крейта чистая целиком | `cargo build -p simulation --all-targets 2>&1 \| grep -c "^warning"` → `0` (R1, R9) |
| A3 | Публичный API не расширен | `grep -rn "^pub \(enum\|struct\|trait\|fn\|type\)\|^pub use" simulation/src/` — сверка списка «до/после»: новых имён нет; `Context`/`Flow`/`Predicate`/`Execution`/`Executions` — `pub(crate)` (R2) |
| A4 | `Unit` непрозрачен, `UnitKind` приватен | `grep -n "pub struct Unit\|enum UnitKind" simulation/src/unit/mod.rs`; `UnitKind` без `pub` и без реэкспорта в `lib.rs` (R3) |
| A5 | Внешний код не может разобрать `Unit` по вариантам | компиляционная проверка: тест-проба с `match unit { Unit::Node { .. } => … }` в `simulation/tests/` **не компилируется** (R3) |
| A6 | Поведение не изменилось | `cargo test -- --test-threads=1` и `cargo test --features lsp -- --test-threads=1` — все зелёные, **ни один ожидаемый результат не правился** (R4); `./scripts/run_simulations.sh` отрабатывает как прежде |
| A7 | Линт `private_interfaces` включён точечно | `grep -n "deny(private_interfaces)" simulation/src/lib.rs` → есть; `grep -rn "deny(warnings)" simulation/` → пусто (R5) |
| A8 | Закрепление действительно работает | негативный прогон: временный `pub fn leak() -> Flow` валит `cargo build -p simulation` с `error: private_interfaces`; правка откатывается (R6) |
| A9 | Аксессоры целы | `grep -n "pub fn " simulation/src/unit/mod.rs` — список совпадает с зафиксированным «до»; тесты `eval_tests.rs`/`conformance_c_tests.rs` не правились (R7) |
| A10 | Версия крейта поднята | `grep -n '^version' simulation/Cargo.toml` → `0.2.0`; версия языка Lam в `grammar` не изменена (R8) |
| A11 | Предкоммит-проверка проходит | `./scripts/precheck.sh` — успешно (правило 5) |

## Особенности по обратной функциональности

**Строка для реестра `docs/analyze/README.md`:**
`слом публичного API крейта simulation (Unit: pub enum → непрозрачный pub struct); язык не тронут, потребителей формы Unit нет — фактических регрессий ноль`

Развёрнуто (правило 11 — обратная совместимость обязательна к рассмотрению):

- **Что ломается формально.** `Unit` перестаёт быть перечислением. Внешний код
  вида `match unit { Unit::Node { state, .. } => … }` или конструирование
  `Unit::Node { … }` литералом — перестанут компилироваться. Это **слом
  публичного API крейта**.
- **Обоснование допустимости слома** (требование правила 11 — обосновать, если
  совместимость невозможна):
  1. **Потребителей нет.** Проверено `grep` по всему репозиторию: варианты
     `Unit::{Node,Parallel,Sequential}` упоминаются **только** внутри крейта
     `simulation` (`unit/builder.rs`, `unit/statement.rs`, `unit/viewport.rs`,
     `state_io.rs`, `runner.rs`, `bin/simulation.rs`). Крейт `grammar` от
     `simulation` не зависит вовсе.
  2. **Тесты уже на аксессорах.** `simulation/tests/eval_tests.rs` и
     `simulation/tests/conformance_c_tests.rs` импортируют только
     `{TickResult, Unit, Value, build_unit}` и работают через `tick`/`variable`
     — форму `Unit` не разбирают. Правок в тестах не требуется (и это
     подтверждает: реальный контракт — аксессоры, а не форма).
  3. **Крейт не опубликован** (не на crates.io), внешних пользователей вне
     репозитория нет. `version = "0.1.0"` в `simulation/Cargo.toml` — стадия
     `0.x`, где SemVer прямо допускает слом с ростом минорной версии.
  4. **Совместимость сохранить и нельзя, и не нужно.** Сохранить `Unit` как
     `pub enum` — значит сохранить причину предупреждений (см. `E0449`: поля
     варианта неизбежно `pub`). Цель фичи и обратная совместимость формы `Unit`
     логически несовместимы; выбирается цель, поскольку цена слома доказуемо
     нулевая.
- **Что НЕ ломается (гарантируется R4/R7):** `build_unit`, все методы `Unit`,
  `TickResult`, `Value`, модули `runner`, `state_io`, `graphics_config`,
  `json_input`, CLI `simulation`, формат сохранения состояния, SVG/GIF-вывод,
  язык Lam и его версия, крейт `grammar` — не затронуты.
- **Версионирование (правило 22):** `simulation` `0.1.0` → `0.2.0`. Язык Lam —
  без изменений (фича не языковая).

## Риски и зависимости

- **R-1. Правка «по дороге» изменит поведение симулятора.** Перенос `Unit::X` →
  `UnitKind::X` в 6 файлах — механический, но объёмный; легко «заодно»
  поправить логику. *Снижение:* задача 0036-01 выполняется строго как
  переименование формы без правки тел; критерий A6 требует, чтобы **ни один
  ожидаемый результат в тестах не правился** — любая необходимость тронуть
  тест-ожидание есть сигнал нарушения R4 и повод остановиться.
- **R-2. `Default`/`Unit::None`.** `Unit` имеет `#[derive(Clone, Default)]` с
  `#[default] None`. При переносе вариантов в `UnitKind` дериву `Default` нужно
  переехать на `UnitKind`, а `Unit` — получить `Default` через newtype.
  *Снижение:* явная проверка в 0036-01; `state_io`/`viewport` опираются на
  `Unit::None` — покрыто существующими тестами.
- **R-3. `union`/`add` возвращают `Self`.** Методы строят новые `Unit` из
  вариантов; после инкапсуляции конструирование идёт через `Unit(UnitKind::…)`.
  *Снижение:* внутрикрейтовый конструктор-хелпер; поведение покрыто тестами.
- **R-4. Конфликты слияния с 0032/0034/0044.** Те же файлы. *Снижение:*
  рекомендованная очерёдность (0036 раньше) — см. «Влияние на порядок
  разработки»; фича мелкая, окно конфликта короткое.
- **R-5. `#![deny(private_interfaces)]` может завалить сборку на новом
  компиляторе,** если линт расширят. *Снижение:* риск принят осознанно — это
  **точечный** линт одного правила (в отличие от запрещённого `deny(warnings)`,
  который ловит все будущие линты); прецедент в проекте —
  `#![deny(clippy::wildcard_enum_match_arm)]` в `eval/mod.rs`. При проблеме
  правится одной строкой.
- **R-6. Соблазн решить попутно вопрос `#[non_exhaustive]`.** `docs/CODE.md`
  предписывает атрибут для публичных `enum` (`Value`, `TickResult` — уже
  публичны и не помечены). *Снижение:* **вне области 0036** (см. ADR,
  Action items): вопрос — часть нерешённого процессного конфликта CODE.md ↔
  0025 и требует отдельной процессной фичи. Option B выбран в том числе потому,
  что новых публичных перечислений не создаёт и этот вопрос не обостряет.
- **Зависимости:** нет (см. раздел «Зависимости фичи»).

## Подзадачи (декомпозиция для стадии 4)

| Задача | Файл | Содержание |
|---|---|---|
| **0036-01** | [`../development/0036-01-sim-visibility.md`](../development/0036-01-sim-visibility.md) | Инкапсуляция `Unit`: `pub struct Unit(UnitKind)` + приватный `enum UnitKind`; перевод 6 внутрикрейтовых потребителей. Закрывает R1–R4, R7 |
| **0036-02** | [`../development/0036-02-lint-pin.md`](../development/0036-02-lint-pin.md) | Закрепление: `#![deny(private_interfaces)]` в `lib.rs`, негативная проверка линта, чистка `unused import`, версия крейта `0.2.0`. Закрывает R5, R6, R8, R9 |

Объём аналитики мал — декомпозиция самого анализа на `0036-YY-*.md` не
требуется (правило 17).
