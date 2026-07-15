# Задача 0034-01: Ядро — `Value::Struct`, реестр структур, `coerce_to_type`

> Фича: [../features/0034-sim-struct-types.md](../features/0034-sim-struct-types.md) · ADR: [../adr/0034-sim-struct-types.md](../adr/0034-sim-struct-types.md) · анализ: [../analyze/0034-sim-struct-types.md](../analyze/0034-sim-struct-types.md)
>
> Покрывает требования **R1** (структурное значение), **R2** (приведение по типу).

## Что было

*Реальное состояние кода на момент постановки (проверено чтением и пробами).*

**`Value` структур не имеет** — `simulation/src/eval/value.rs:14`:

```rust
pub enum Value {
    Number(i64),
    Real(f64),
    Boolean(bool),
    Array(Vec<Value>),
}
```

> Кандидат в `FEATURES.md` указывал файл `simulation/src/value.rs` — такого файла
> **нет**: значение перенесено в `eval/value.rs` фичей 0025.

**`coerce_to_type` отказывает на структурах** — `simulation/src/eval/mod.rs:104`:

```rust
TypeNode::Struct(name) => Err(EvalError::UnsupportedType {
    ty: format!("структура '{name}'"),
}),
```

Код — **SIM-007** (`eval/error.rs:74`). Комментарий над веткой прямо ссылается на
настоящий пробел: «Пробел `Value`: структуры симулятором не представимы. Явная
диагностика вместо тихого `None` — контрпример T22 тест-плана».

**Однако сквозной путь не срабатывает.** Отчёт фичи 0025 сам пометил T22 как ⚠️:
«юнит-тест `coerce_to_type(_, Struct)` → `SIM-007` ✅. **Сквозной путь не
срабатывает:** объявление `var p: Point;` без присваивания диагностики не даёт —
приведение вызывается только при записи. Пробел покрытия». Фикстура
`struct_var.lam` в репозитории **отсутствует** (проверено поиском).

**Состав полей ядру недоступен.** `TypeNode::Struct(String)` несёт **только имя**
(`semantic/type_node.rs:131`). Поля живут в `ModelNode.structs: HashMap<String,
StructDefinitionNode>` и достаются через `ModelNode::search_struct(name)`
(`semantic/mod.rs:408`, ищет вверх по родительским моделям). Сигнатура

```rust
pub(crate) fn coerce_to_type(value: Value, ty: &TypeNode) -> Result<Value, EvalError>
```

принимает лишь `&TypeNode`, поэтому привести `{1, 2}` к `Point` **физически не
может**: ей неизвестны ни число полей, ни их типы, ни порядок.

**Порядок полей семантически значим.** Инициализатор позиционный:
`grammar.lalrpop:535` даёт `Expression::Initializer(loc, Vec<Expression>)`;
синтаксиса `{x: 1, y: 2}` в языке нет.

**Наблюдаемое поведение (пробы на `target/debug/simulation`):**

| Модель | Результат |
|---|---|
| `var p: Point := {1, 2};` | симуляция **проходит молча**, `p` отсутствует в переменных |
| `t := p.x;` | `ОШИБКА вычисления на шаге 1: переменная 'p' не найдена (SIM-009)` |
| `p.x := 7;` | `ОШИБКА вычисления на шаге 1: присваивание не в переменную пока не поддерживается симулятором (SIM-017)` |

Места вызова `coerce_to_type` (все правятся вместе с сигнатурой): `expression.rs:204`,
`unit/statement.rs:204`, `:259`, `:471`, `:483`.

## Что сделано

> **Планируется (разработка не начата).** Ниже — план по ADR 0034 (Option C).

**1. Вариант значения** — `eval/value.rs`:

```rust
pub enum Value {
    Number(i64),
    Real(f64),
    Boolean(bool),
    Array(Vec<Value>),
    /// Структурное значение: имя типа + поля в **объявленном** порядке.
    /// `Vec`, а не `BTreeMap`: инициализатор `{1, 2}` позиционный, а карта
    /// упорядочила бы поля по имени и молча перепутала их (ADR 0034, Option A).
    Struct { name: String, fields: Vec<(String, Value)> },
}
```

`#[non_exhaustive]` **не** ставится — обоснование в ADR (прецедент `TypeNode`
вынудил ветку `_` в самом `coerce_to_type`).

**2. Реестр структур** — новый трейт в `eval/`:

```rust
pub(crate) trait StructRegistry {
    fn find(&self, name: &str) -> Option<StructDefinitionNode>;
}
```

Реализация поверх `ModelNode::search_struct` (учитывает поиск по родителям).

**3. Сигнатура приведения**:

```rust
pub(crate) fn coerce_to_type(
    value: Value,
    ty: &TypeNode,
    structs: &dyn StructRegistry,
) -> Result<Value, EvalError>
```

Ветка `TypeNode::Struct(name)`: найти определение → сверить арность → привести
поля **по позиции** в объявленном порядке (рекурсивно через `coerce_to_type`) →
`Ok(Value::Struct{…})`. `EvalError::UnsupportedType` для структур **удаляется**.

**4. Новые варианты `EvalError`** (коды из свободного диапазона `SIM-0xx`):
несовпадение арности инициализатора; неизвестное поле; несовпадение имени
структурного типа. `value_kind` (`error.rs:87`) получает ветку `Struct` → «структура».

**5. Ветки в операциях.** Компилятор потребует обработать `Value::Struct` в каждом
`match` в `eval/ops.rs` и `eval/mod.rs` — это по замыслу (`deny(clippy::wildcard_enum_match_arm)`).
Подавляющее большинство — честный `TypeMismatch`: арифметика, сдвиги и сравнения
над структурой не определены. **Сравнение `p = q` — тоже `TypeMismatch`**, и это
осознанно: C запрещает `==` на структурах (драйвер 3 ADR).

### Статус по функциональности (правило 11)

| Функциональность | Статус |
|---|---|
| Язык `.lam`, грамматика, АСД | **н/п** — не трогаются; версия языка не растёт (R10) |
| Генератор C (`grammar`) | **н/п** — крейт не затрагивается; вывод C байт-в-байт неизменен |
| `simulation::Value` (публичный) | **аддитивно** (+`Struct`); внешних потребителей вне workspace нет |
| `coerce_to_type` | **слом внутреннего API** (`pub(crate)`): +3-й параметр, 5 мест вызова |
| Диагностика SIM-007 | **снимается** для структур; прочие типы (`Inference`, `Unit`, `BuiltinString`…) не задеты |
| Модели без структур | **не задеты** — новых веток не проходят |

## Проверки

> **Планируется (разработка не начата).**

| Что | Как | Ожидаемо |
|---|---|---|
| T6 (A5) | юнит-тест ядра: `coerce_to_type(Array([1,2]), Struct("Point"), registry)` | `Ok(Value::Struct{…})`; `UnsupportedType` не возвращается |
| T2 (A2) | `struct S { b: u8, a: u8 }` + `{1, 2}` | `b = 1, a = 2` — **по позиции**. Тест обязан падать на `BTreeMap` |
| T11 (A7) | `{1}` при двух полях | диагностика арности, **не** дополнение нулями |
| T22 (A9) | `p.y := 300` при `y: u8` | `44` — усечение S9 действует внутри поля |
| T18 (A10) | `grep` + сборка | `Value` не `#[non_exhaustive]`; `deny(clippy::wildcard_enum_match_arm)` в `eval/` на месте |
| A11 (R10) | `git diff --stat grammar/src/grammar.lalrpop grammar/src/parser/ast.rs` | пусто |

```sh
cargo test --all-features -- --test-threads=1     # правило 5, однопоточно
./scripts/precheck.sh                             # fmt + check + clippy + test + примеры
grep -rn "UnsupportedType" simulation/src/eval/   # структур в списке быть не должно
```

**Соответствие анализу:** R1 (порядок полей — A1, A2), R2 (приведение — A5).
Контрпримеры арности и типа — R7 (A7). Тесты пишутся **на значения**
(`Unit::variable`), а не на факт перехода (инвариант `CLAUDE.md`); сперва зонд для
захвата реального вывода, затем assertions против захваченных значений.
