# Задача 0036-01: Инкапсуляция `Unit` (pub struct + приватный UnitKind)

> Фича: [../features/0036-sim-visibility.md](../features/0036-sim-visibility.md) · ADR: [../adr/0036-sim-visibility.md](../adr/0036-sim-visibility.md) · анализ: [../analyze/0036-sim-visibility.md](../analyze/0036-sim-visibility.md) · тест-план: [../tests/0036-sim-visibility.md](../tests/0036-sim-visibility.md)

## Что было

**Реальное состояние кода на момент постановки** (ветка `v2`, коммит `6984471`,
замер `cargo build -p simulation --all-targets`, 2026-07-15).

Крейт `simulation` собирается с **10 предупреждениями `private_interfaces`**.
Причина — в `simulation/src/unit/mod.rs:84`: `Unit` объявлен как `pub enum`, а
типы, которыми набиты его варианты, — `pub(crate)`:

```rust
pub enum Unit {
    None,
    Node {
        context: Option<Rc<RefCell<dyn Context>>>,                     // :88
        state_transitions: HashMap<String, Vec<(String, Predicate)>>,  // :90
        state_executions: HashMap<String, Executions>,                 // :91
        state: Option<String>,
        variables: HashMap<String, Value>,
        executions: Executions,                                        // :95
        last_transition: Option<(String, String, String)>,
        entered_initial: bool,
    },
    Parallel   { units: Vec<Rc<RefCell<Unit>>>, executions: Executions },            // :109
    Sequential { units: Vec<Rc<RefCell<Unit>>>, index: usize, executions: Executions }, // :114
}
```

Объявления «слишком приватных» типов:

- `pub(crate) trait Context` — `simulation/src/context.rs:4` (5 предупреждений);
- `pub(crate) struct Predicate` — `simulation/src/unit/mod.rs:26` (1);
- `pub(crate) enum Flow` — `simulation/src/unit/mod.rs:52` (4).

`Flow`/`Context` утекают не напрямую, а через псевдонимы (`unit/mod.rs:65–66`):

```rust
pub(crate) type Execution = Rc<dyn Fn(&mut dyn Context) -> Result<Flow, Diagnostic>>;
type Executions = HashMap<String, Vec<Execution>>;
```

Полный список 10 предупреждений с путями/строками — в
[анализе](../analyze/0036-sim-visibility.md).

**Уже в порядке, правок не требует** (проверено по коду):

- `TickResult` — `pub` (`unit/mod.rs:73`), исправлен попутно 0025-05
  (утверждение бэклога **подтвердилось**);
- `Value` — `pub` (`eval/value.rs:14`), реэкспорт `lib.rs:29`; предупреждения
  по нему **нет** (утверждение бэклога **не подтвердилось**).

**Почему нельзя «просто сузить поля».** Проверено компилятором: поля вариантов
перечисления всегда наследуют его видимость —

```
error[E0449]: visibility qualifiers are not permitted here
  = note: enum variants and their fields always share the visibility of the enum they are in
```

**Кто разбирает `Unit` (объём правки), проверено `grep`:**
`simulation/src/unit/builder.rs`, `simulation/src/unit/statement.rs`,
`simulation/src/unit/viewport.rs`, `simulation/src/state_io.rs` (9 мест:
строки 53, 62, 65, 78, 99, 105, 156, 173, 190), `simulation/src/runner.rs`,
`simulation/src/bin/simulation.rs`. **Вне крейта — никто**: интеграционные
тесты (`simulation/tests/eval_tests.rs:26`,
`simulation/tests/conformance_c_tests.rs:43`) импортируют только
`{TickResult, Unit, Value, build_unit}` и работают через аксессоры
(`tick`, `variable`); крейт `grammar` от `simulation` не зависит.

## Что сделано

> **Планируется (разработка не начата).** Ниже — план по ADR 0036, Option B.

1. **`simulation/src/unit/mod.rs`** — заменить `pub enum Unit` на непрозрачный
   newtype с приватным внутренним перечислением:

   ```rust
   #[derive(Clone, Default)]
   pub struct Unit(UnitKind);          // поле приватное (docs/CODE.md: приватные поля)

   #[derive(Clone, Default)]
   enum UnitKind {                     // приватный: наружу не виден
       #[default]
       None,
       Node { … },                     // поля — без изменений
       Parallel { … },
       Sequential { … },
   }
   ```

   `#[default]` переезжает на `UnitKind::None`; `Unit` получает `Default` через
   derive по newtype (R-2 анализа).
2. **Внутрикрейтовый конструктор.** Добавить `pub(crate)`-хелперы
   (`Unit::from_kind`/`kind`/`kind_mut`) либо использовать `Unit(UnitKind::…)`
   напрямую внутри крейта — так, чтобы `union`/`add` (`unit/mod.rs:517,570`),
   возвращающие `Self`, строились без публикации формы (R-3 анализа).
3. **`impl Context for Unit`** и все методы `Unit` — перевести разбор с
   `match self { Unit::Node { … } }` на `match self.0 { UnitKind::Node { … } }`.
   **Тела не трогать** — только форма сопоставления.
4. **Потребители (6 файлов)** — механическая замена `Unit::Node|Parallel|
   Sequential` → `UnitKind::…` с доступом через `.0`/хелпер:
   `unit/builder.rs`, `unit/statement.rs`, `unit/viewport.rs`, `state_io.rs`,
   `runner.rs`, `bin/simulation.rs`.
5. **Проверить, что `Context`, `Flow`, `Predicate`, `Execution`, `Executions`
   остались `pub(crate)`** — публичный API не должен пополниться ничем (R2).
6. **Тесты и фикстуры не трогать.** Необходимость правки ожидаемого результата
   = нарушение R4 и сигнал остановиться (см. риск R-1 анализа).

Статус по затрагиваемой обратной функциональности (правило 11):

| Функциональность | Работа | Комментарий |
|---|---|---|
| Публичный API `simulation` | **да** | Форма `Unit` ломается намеренно; потребителей нет (обоснование — в анализе) |
| Исполнение модели (`tick`/переходы/значения) | **н/п** | Тела не меняются; поведение обязано остаться прежним (R4) |
| `state_io`, `viewport`, `runner`, CLI | **да** (механически) | Только форма разбора `Unit`; поведение и форматы прежние |
| Крейт `grammar`, язык Lam | **н/п** | Не зависит от `simulation`; язык не затронут (правило 18) |

Версия крейта, линт-закрепление и чистка `unused import` — **вне этой задачи**,
они в [0036-02](0036-02-lint-pin.md).

## Проверки

> **Планируется (разработка не начата).** Соответствие тест-плану
> [`../tests/0036-sim-visibility.md`](../tests/0036-sim-visibility.md).

1. **Эталон «до» — снять первым делом** (иначе доказывать будет нечего):
   `cargo clean -p simulation && cargo build -p simulation --all-targets 2>&1 | grep -c "more private than"` → ожидается **`10`**.
2. **Ключевая (T1 / R1, A1):** та же команда после правки → **`0`**.
3. **T3 / R2, A3:** `grep -rn "^pub \(enum\|struct\|trait\|fn\|type\)\|^pub use" simulation/src/`
   — список публичных имён совпадает с «до»; `Context`/`Flow`/`Predicate`/
   `Execution`/`Executions` — `pub(crate)`.
4. **T4 / R3, A4:** `grep -n "pub struct Unit\|enum UnitKind" simulation/src/unit/mod.rs`
   → `pub struct Unit(UnitKind)`, `enum UnitKind` без `pub` и без реэкспорта.
5. **T5 / R3, A5:** временная проба в `simulation/tests/` с
   `match unit { Unit::Node { .. } => … }` → **ошибка компиляции**; вывод — в
   отчёт, проба откатывается.
6. **T13 / R7, A9:** `grep -n "pub fn " simulation/src/unit/mod.rs` — набор
   аксессоров совпадает с «до» (`tick`, `take_last_transition`,
   `take_last_transitions`, `reachable_from_active`, `execution`, `variable`,
   `current_state`, `active_states`, `is_terminal`, `union`, `add`).
7. **T9, T12 / R4, A6 — главная защита от регресса:**
   `cargo test -- --test-threads=1` (правило 5, однопоточно) — всё зелёное,
   **при пустом `git diff` по `simulation/tests/` и `grammar/tests/`**.
8. **T10 / R4:** `cargo test --features lsp -- --test-threads=1` — зелёные.
9. **T11 / R4, A6:** `./scripts/run_simulations.sh` — вывод совпадает с «до».
10. **T15 / правило 5:** `./scripts/precheck.sh` — успешно.
