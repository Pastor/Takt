---
okf_version: 0.2
format: 2
type: Task
key: TAKT-36
list: ТАКТ
title: "[0036] Согласование видимости публичного API крейта simulation"
status: done
priority: none
author: pastor
model: gpt-6
links:
  blocks:
  - TAKT-46
generated:
  by: agent/gpt-6
  for: human:pastor
  at: 2026-11-11T07:08:42Z
published: 2026-11-11T07:08:42Z
created: 2026-11-11T07:08:41Z
updated: 2026-11-11T07:08:42Z
anal:
  project: 22
  id: 1090
  version: 2
checksum: sha256:a28d164c536eb2c8bf34d4328ac05e32312a008da94a5b0c5beb9ea7d3077145
---

Источник: [docs/features/0036-sim-visibility.md](https://github.com/Pastor/Takt/blob/3c154a13d979b18312a1250ebfbe3db2fa5247d3/docs/features/0036-sim-visibility.md)

Полный текст по ссылке. Ниже начало карточки.

# Фича: Согласование видимости публичного API крейта simulation

- **Номер:** 0036
- **Статус:** ГОТОВО (закрыта 2026-07-19)
- **Зависит от:** нет
- **Приоритет / Tier:** низкий (Tier 3) — гигиена сборки, наблюдаемое поведение
  не меняется
- **Связанные issue (анализ):** — (новая фича; из процессного бэклога
  `FEATURES.md` — «Предупреждения `private_interfaces` в `simulation`»)

## Стадии жизненного цикла

Все стадии — **разделы этой карточки**: «Архитектура (ADR)»,
«Анализ», «Разработка», «Тест-план», «Отчёт о тестировании», «Итог».
Отдельным артефактом остаются только исправления —
[`docs/fixes/`](../fixes/README.md) (при необходимости `0036-YY-*`).

## Краткое описание

Сборка крейта `simulation` даёт **10 предупреждений `private_interfaces`**:
перечисление `Unit` объявлено `pub`, а типы, которыми набиты его варианты
(трейт `Context`, перечисление `Flow`, структура `Predicate`), остаются
`pub(crate)`. Компилятор говорит правду: поля вариантов `Unit` достижимы снаружи
на видимости `pub`, но назвать их типы внешний код не может — API рассогласован.

Фича согласует видимость и **механически закрепляет** отсутствие предупреждений
точечным линтом, чтобы рассогласование не вернулось молча. Решение
([ADR](0036-sim-visibility.md#архитектура-adr)) — **инкапсулировать `Unit`**:
превратить его в `pub struct` с приватным внутренним `enum UnitKind`, оставив
публичным уже существующий набор методов-аксессоров (`tick`, `variable`,
`current_state`, `active_states`, `is_terminal`, …). Публичный API крейта при
этом **не расширяется** ни одним новым типом, внутренние типы честно остаются
внутренними, а вопрос `#[non_exhaustive]` (правило `docs/CODE.md`, конфликтующее
с инвариантом фичи) не встаёт.

> **Уточнение фактов кандидата.** Текст кандидата перечислял четыре типа —
> `Context`, `Value`, `Predicate`, `Flow`. Проверка по коду:
> **`Value` уже `pub`** (`simulation/src/eval/value.rs:14`, реэкспорт
> `simulation/src/lib.rs:29`), предупреждения по нему **нет** — факт не
> подтвердился, в работах он не участвует. Заявление про `TickResult`
> **подтвердилось**: он `pub` (`simulation/src/unit/mod.rs:73`), исправлен
> попутно задачей. Фактический список предупреждений — в
> [анализе](0036-sim-visibility.md#анализ).

> Фича зарегистрирована из процессного бэклога `FEATURES.md`; далее проходит
> жизненный цикл по своду.

## Архитектура (ADR)

- **Status:** Accepted
- **Date:** 2026-07-15
- **Authors:** Архитектор + Системный аналитик
- **Related issues:** [Фича](0036-sim-visibility.md); опирается
  на правила [`docs/CODE.md`](../CODE.md) (приватные поля, Newtype,
  `#[non_exhaustive]`, запрет `deny(warnings)`) и их применение в фиче
  [приведение кода к требованиям docs/CODE.md](0018-code-guidelines.md) ([анализ](0018-code-guidelines.md#анализ);
  собственного ADR у 0018 нет); учитывает инвариант
  [ADR](0025-simulator-expression-eval.md#архитектура-adr)
  (`deny(clippy::wildcard_enum_match_arm)` в `simulation/src/eval/`)

### Context

Крейт `simulation` при каждой сборке печатает **10 предупреждений
`private_interfaces`** (замер — `cargo build -p simulation`; полный список с
путями и строками см. [анализ](0036-sim-visibility.md#анализ)):

| Тип, «который приватнее» | Где объявлен | Через какие поля `Unit` утекает | Шт. |
|---|---|---|---|
| трейт `Context` | `simulation/src/context.rs:4` | `Node::context`, `Node::executions`, `Node::state_executions`, `Parallel::executions`, `Sequential::executions` | 5 |
| `Flow` | `simulation/src/unit/mod.rs:52` | `Node::executions`, `Node::state_executions`, `Parallel::executions`, `Sequential::executions` | 4 |
| `Predicate` | `simulation/src/unit/mod.rs:26` | `Node::state_transitions` | 1 |

Корень — в объявлении `Unit` (`simulation/src/unit/mod.rs:84`):

```rust
pub enum Unit {
    None,
    Node {
        context: Option<Rc<RefCell<dyn Context>>>,                    // Context: pub(crate)
        state_transitions: HashMap<String, Vec<(String, Predicate)>>, // Predicate: pub(crate)
        state_executions: HashMap<String, Executions>,                // Executions -> Flow, Context
        executions: Executions,
        …
    },
    Parallel   { units: Vec<Rc<RefCell<Unit>>>, executions: Executions },
    Sequential { units: Vec<Rc<RefCell<Unit>>>, index: usize, executions: Executions },
}
```

`Flow` и `Context` утекают не напрямую, а через псевдоним
`type Execution = Rc<dyn Fn(&mut dyn Context) -> Result<Flow, Diagnostic>>`
(`unit/mod.rs:65`) внутри `type Executions = HashMap<String, Vec<Execution>>`
(`unit/mod.rs:66`).

**Ключевой факт о языке (проверен компилятором).** Варианты перечисления и их
поля **всегда** наследуют видимость самого перечисления; попытка написать
`pub(crate)` у поля варианта — ошибка `E0449`: «visibility qualifiers are not
permitted here… enum variants and their fields always share the visibility of
the enum they are in». Значит, пока `Unit` — `pub enum`, его поля **обязаны**
быть `pub`, и рассогласование неустранимо «на месте».

**Что уже верно и трогать не нужно** (проверено по коду — вопреки тексту
кандидата в `FEATURES.md`):

- `Value` — **уже** `pub` (`eval/value.rs:14`) и реэкспортирован (`lib.rs:29`);
  предупреждения по нему нет. Утверждение кандидата **не подтвердилось**.
- `TickResult` — **уже** `pub` (`unit/mod.rs:73`), исправлен попутно задачей. Утверждение кандидата **подтвердилось**.

**Кто реально потребляет `Unit`.** Варианты `Unit::{Node,Parallel,Sequential}`
конструируются и разбираются **только внутри крейта** — `unit/builder.rs`,
`unit/statement.rs`, `unit/viewport.rs`, `state_io.rs`, `runner.rs`,
`bin/simulation.rs`. Интеграционные тесты (`simulation/tests/eval_tests.rs`,
`simulation/tests/conformance_c_tests.rs`) работают **исключительно через
методы-аксессоры**: `build_unit`, `Unit::tick`, `Unit::variable`, `TickResult`,
`Value`. То есть публичность формы перечисления сегодня **никем не
используется** — она случайна, а не спроектирована.

свод: язык Lam (синтаксис/семантика) не затрагивается, потоки данных между
компонентами не меняются — диаграмма не требуется и не добавляется.

### Decision Drivers

1. **Честность API важнее тишины.** Предупреждение указывает на реальный дефект
   проектирования, а не на каприз компилятора: наружу торчит форма внутреннего
   дерева симуляции. Заглушить — значит зафиксировать дефект.
2. **Не расширять публичный API без нужды (YAGNI).** Всё, что стало
   `pub`, попадает под семантическое версионирование и связывает руки. Внешних
   потребителей `Context`/`Flow`/`Predicate` нет и никто их не запрашивал.
3. **`docs/CODE.md` прямо предписывает** «использовать приватные поля, чтобы
   запретить внешнее конструирование через литерал структуры и сохранить свободу
   изменений» и применять **Newtype** для сокрытия реализации.
4. **Не разбудить конфликт `#[non_exhaustive]` ↔ инвариант.**
   `docs/CODE.md` требует помечать публичные `enum` атрибутом
   `#[non_exhaustive]`; конфликт этого правила с механизмом 0025 зафиксирован в
   процессном бэклоге `FEATURES.md` как **нерешённый**. Любое решение,
   добавляющее публичные перечисления, обязано этот вопрос открыть (см. Option A).
5. **Механическое закрепление.** Результат должен держаться линтом, а не
   дисциплиной: иначе предупреждения вернутся с первым же новым полем.
6. **Соразмерность.** Фича гигиеническая (Tier 3) и не имеет права менять
   наблюдаемое поведение симулятора.

### Considered Options

#### Option A. Поднять `Context`, `Flow`, `Predicate` до `pub`

Минимальная правка: три `pub(crate)` → `pub`, плюс `pub` для
`Execution`/`Executions` (они входят в сигнатуры полей) и реэкспорт из `lib.rs`.

**Pros:**

- Самое дешёвое по объёму (≈5 строк), нулевой риск задеть поведение.
- Внешние потребители смогли бы конструировать `Unit` вручную и подменять
  `Context` — гипотетическая точка расширения для сторонних симуляторов.
- Симметрично тому, как уже поступили с `TickResult` (0025-05) и `Value`.

**Cons:**

- **Расширяет публичный API крейта пятью типами**, которые являются
  реализацией, а не контрактом. по своду (SemVer) добавление `pub` —
  аддитивное изменение (`0.1.0` → `0.1.1`), но **обратный** ход (спрятать их
  назад) будет уже сломом: решение фактически необратимо.
- Наружу уезжает `Context::set_value` — возможность писать в переменные модели
  **мимо** `eval::coerce_to_type`, то есть мимо приведения по типу. Это прямо
  подрывает инвариант фичи («семантика вычислений живёт только в `eval/`»):
  не гигиена, а дыра в инварианте.
- `Predicate` содержит `Rc<dyn Fn(&mut dyn Context) -> Result<bool, Diagnostic>>`,
  `Execution` — `Rc<dyn Fn(&mut dyn Context) -> Result<Flow, Diagnostic>>`.
  Публикация замыканий фиксирует внутреннюю сигнатуру исполнителя как контракт
  крейта.
- **Открывает вопрос `#[non_exhaustive]` ребром** (драйвер 4). `docs/CODE.md`
  предписал бы пометить публичный `Flow` (и `Unit`, и уже публичный `Value`)
  атрибутом `#[non_exhaustive]`. Разбор нюанса: сам по себе атрибут **безвреден
  для механизма 0025** — внутри крейта-объявителя он не действует, а `Flow` и
  `Value` разбираются только внутри `simulation`, поэтому
  `deny(clippy::wildcard_enum_match_arm)` в `eval/` продолжит валить сборку на
  забытом варианте. **Но**: (а) вопрос требует отдельного решения по каждому
  типу и тянет за собой нерешённый пункт процессного бэклога «Конфликт правил:
  `#[non_exhaustive]` против запрета `_ =>`»; (б) `#[non_exhaustive]` лишает
  внешних потребителей исчерпывающего разбора, обесценивая тот самый доступ,
  ради которого типы и публиковались. Гигиеническая фича Tier 3 — не место, где
  этот конфликт решается впопыхах.
- **Не решает корень**: форма `Unit` остаётся публичной, и любое новое поле с
  внутренним типом снова потребует его публикации — предупреждение вернётся.

#### Option B. Инкапсулировать `Unit`: `pub struct` + приватный `enum UnitKind`

> **Формулировка уточнена против исходной.** Кандидат предлагал «сузить поля
> `Unit` до `pub(crate)`» — так **нельзя**: `E0449`, поля вариантов всегда
> наследуют видимость перечисления (см. Context). Единственная работающая форма
> этой идеи — перестать быть публичным перечислением:

```rust
pub struct Unit(UnitKind);   // публичный newtype, поле приватное
enum UnitKind {              // приватный: из крейта не виден
    None,
    Node { context: Option<Rc<RefCell<dyn Context>>>, … },
    Parallel { … },
    Sequential { … },
}
```

Публичным остаётся уже существующий набор аксессоров: `tick`,
`take_last_transition(s)`, `reachable_from_active`, `execution`, `variable`,
`current_state`, `active_states`, `is_terminal`, `union`, `add`.
Внутрикрейтовые потребители переписываются механически:
`Unit::Node { … }` → `UnitKind::Node { … }` (6 файлов).

**Pros:**

- **Устраняет корень**: поля перестают быть достижимыми на видимости `pub` —
  все 10 предупреждений исчезают разом, и **будущие** поля с внутренними типами
  их не вернут.
- **Публичный API не расширяется ни на один тип.** `Context`, `Flow`,
  `Predicate`, `Execution`, `Executions` остаются `pub(crate)` — тем, чем они и
  являются.
- Прямо следует `docs/CODE.md` (приватные поля ради свободы изменений; Newtype
  для сокрытия реализации).
- **Вопрос `#[non_exhaustive]` не встаёт** (драйвер 4): публичных перечислений
  не прибавляется, `UnitKind` приватен — конфликт CODE.md ↔ 0025 остаётся
  нетронутым в бэклоге, где ему и место.
- Инвариант усиливается: `set_value` недостижим извне, запись в переменные
  идёт только через `eval`.
- Форма дерева симуляции освобождается для будущих фич (0032, 0034) без слома
  внешних потребителей.

**Cons:**

- **Технически слом публичного API**: `Unit` перестаёт быть перечислением —
  внешний код, разбиравший `Unit::Node { .. }`, сломается. По SemVer для `0.x` это мажорный шаг: `0.1.0` → `0.2.0`.
  *Смягчение:* таких потребителей **нет** — ни в workspace, ни в тестах (см.
  Context); крейт не опубликован. Слом теоретический.
- Объём больше, чем у Option A: правки в 6 файлах (`builder.rs`, `statement.rs`,
  `viewport.rs`, `state_io.rs`, `runner.rs`, `bin/simulation.rs`) —
  механические, но не пятистрочные. Риск конфликтов слияния с фичами, трогающими
  те же файлы (0032 — `state_io.rs`; 0034/0044 — дерево симуляции).
- `Unit::None` как `#[default]` и внутренние `matches!(unit, Unit::None)`
  требуют аккуратного переноса на `UnitKind` (риск R-2 в анализе).

#### Option C. Подавить предупреждение `#[allow(private_interfaces)]`

**Pros:**

- Одна строка, ноль риска, ноль изменений API и поведения.
- Сборка немедленно чистая.

**Cons:**

- **Лечит симптом, а не болезнь.** Рассогласование остаётся: поля публичны,
  типы — нет; внешний код по-прежнему не может ни сконструировать `Unit`, ни
  назвать типы его полей. Это ровно то состояние, которое линт придуман
  обнаруживать.
- Противоречит духу `docs/CODE.md` («настраивай lint-ы точечно» — а не глуши
  диагностику о дефекте) и драйверу 1.
- **Скрывает будущие рассогласования**: следующий утёкший тип не будет замечен
  вовсе — `allow` действует на весь элемент.
- Прямо конфликтует с требованием закрепить чистоту линтом (драйвер 5): `deny`
  и `allow` одного линта взаимно бессмысленны.
- Приемлем лишь как временная мера с `TODO`, чего фича не требует: корень
  устраним за обозримый объём.

### Decision

Принимается **Option B** — инкапсуляция `Unit` в `pub struct` с приватным
`enum UnitKind`, с дополнительным **закреплением** результата точечным линтом
`#![deny(private_interfaces)]` в `simulation/src/lib.rs`.

Обоснование:

1. **Это единственная опция, устраняющая причину.** A и C оставляют публичную
   форму `Unit` и потому лишь откладывают возврат предупреждений.
2. **Цена слома нулевая, выгода постоянная.** Формально Option B ломает API, но
   ломать нечего: потребителей формы `Unit` вне крейта не существует, а тесты
   давно живут на аксессорах. Взамен крейт получает нормальную границу
   инкапсуляции.
3. **Не расширяем публичный API** (драйвер 2) и **не будим спор о
   `#[non_exhaustive]`** (драйвер 4) — важное свойство для фичи Tier 3, у
   которой нет мандата решать процессный конфликт CODE.md ↔ 0025.
4. **Поддерживает инвариант**: `Context::set_value` остаётся внутренним.

Закрепление именно `#![deny(private_interfaces)]`, а **не** `#![deny(warnings)]`:
последнее прямо запрещено `docs/CODE.md` («Не используй `#![deny(warnings)]` в
публичных крейтах — это ломает сборку при обновлении компилятора; настраивай
lint-ы точечно и в CI»). Точечный `deny` конкретного линта — ровно предписанный
способ; в проекте уже есть прецедент: `#![deny(clippy::wildcard_enum_match_arm)]`
в `simulation/src/eval/mod.rs`.

### Consequences

#### Положительные

- `cargo build -p simulation` — **0 предупреждений `private_interfaces`**
  (было 10); сборка крейта чистая.
- Публичный API `simulation` сокращается до осмысленного контракта:
  `build_unit`, непрозрачный `Unit` + его аксессоры, `TickResult`, `Value`,
  модули `runner`, `state_io`, `graphics_config`, `json_input`.
- Регресс невозможен молча: `#![deny(private_interfaces)]` валит сборку при
  появлении нового утёкшего типа.
- Инвариант (запись значений только через `eval::coerce_to_type`) защищён
  границей видимости, а не соглашением.
- Внутренняя форма дерева симуляции освобождается для фич (переменные в
  `state_io`) и 0034 (структурные типы) — их правки больше не будут задевать
  публичный API.

#### Отрицательные / Action items

- **Версия крейта `simulation`: `0.1.0` → `0.2.0`** (SemVer для
  `0.x`: слом формы публичного `Unit`). Версия **языка Lam не меняется** — язык
  не затронут.
- Механическая правка 6 файлов-потребителей внутри крейта; координировать с
  фичами по очерёдности, чтобы не плодить конфликты слияния.
- `Unit::None`/`Default` и внутренние `matches!` переносятся на `UnitKind` —
  требуют внимания при ревью.
- **Вне области фичи** (зафиксировать в бэклоге отдельными пунктами —
  свод):
  - `warning: unused import: StatementNode` — `simulation/src/unit/builder.rs:9`.
    Устраняется попутно задачей: тривиально и внутри крейта фичи.
  - `warning: field 'end' is never read` — `grammar/src/format/comments.rs:27`.
    Это **крейт `grammar`**, к фиче отношения не имеет; чинится отдельно.
- Вопрос **`#[non_exhaustive]` для уже публичных `Value` и `TickResult`**
  настоящим ADR **не решается** — он часть нерешённого процессного пункта
  «Конфликт правил: `#[non_exhaustive]` против запрета `_ =>`». Вывод для
  будущего решения фиксируем здесь: пометка этих двух типов `#[non_exhaustive]`
  **не сломала бы** механизм 0025 (внутри крейта-объявителя атрибут не
  действует, а разбираются они только внутри `simulation`), но лишила бы внешних
  потребителей исчерпывающего разбора. Решать — отдельной процессной фичей.

#### Acceptance criteria

1. `cargo build -p simulation --all-targets` не выдаёт ни одного предупреждения
   `private_interfaces` (было 10).
2. `Context`, `Flow`, `Predicate`, `Execution`, `Executions` остаются
   `pub(crate)`; публичный API крейта **не пополнился** ни одним типом (сверка
   `pub use`/`pub enum`/`pub struct`/`pub trait` в `simulation/src/`).
3. `Unit` — `pub struct` с приватным полем; `UnitKind` не экспортируется и не
   достижим извне крейта.
4. `simulation/src/lib.rs` содержит `#![deny(private_interfaces)]`;
   `#![deny(warnings)]` **не** добавлен (запрет `docs/CODE.md`).
5. Негативная проверка линта: временный `pub`-элемент с `pub(crate)`-типом валит
   сборку (доказательство, что закрепление работает).
6. Все тесты `simulation` и `grammar` зелёные, поведение симулятора не
   изменилось: `cargo test -- --test-threads=1`.
7. Версия крейта `simulation` поднята до `0.2.0`; версия языка Lam не изменена.

## Анализ

### Цель и контекст

Устранить рассогласование видимости в публичном API крейта `simulation`:
перечисление `Unit` объявлено `pub`, а типы его полей (`Context`, `Flow`,
`Predicate`) — `pub(crate)`. Компилятор сообщает об этом 10 предупреждениями
`private_interfaces` при каждой сборке. Решение ([ADR](0036-sim-visibility.md#архитектура-adr),
Option B) — инкапсулировать `Unit` (`pub struct` + приватный `enum UnitKind`),
не расширяя публичный API, и закрепить результат точечным линтом
`#![deny(private_interfaces)]`.

свод: язык Lam не затрагивается — фича целиком внутри крейта `simulation`.
свод: версия **языка** не меняется; версия **крейта** `simulation`
поднимается `0.1.0` → `0.2.0` (см. «Особенности по обратной функциональности»).

#### Фактическое состояние кода (а не пересказ кандидата)

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
| `Value` — `pub(crate)`, шумит | **Не подтвердилось** | `Value` **уже `pub`** (`eval/value.rs:14`), реэкспорт `lib.rs:29`. Предупреждений по нему **нет**. Модуль `eval::value` — `pub(crate)`, но сам тип публичен через реэкспорт, чего линту достаточно. Из объёма работ исключён. |
| `TickResult` исправлен задачей (стал `pub`) | **подтвердилось** | `unit/mod.rs:73` — `pub enum TickResult`, с комментарием-обоснованием. Правок не требует. |

**Прочие предупреждения сборки (для полноты; вне области фичи):**

| Предупреждение | Место | Решение |
|---|---|---|
| `unused import: StatementNode` | `simulation/src/unit/builder.rs:9` | крейт фичи, тривиально → задача **0036-02** |
| `field 'end' is never read` | `grammar/src/format/comments.rs:27` | **крейт `grammar`**, к фиче не относится → предложить пунктом бэклога |

#### Ключевое ограничение языка (проверено компилятором)

Исходная формулировка «с
