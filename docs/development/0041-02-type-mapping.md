# Задача 0041-02: Отображение типов Lam → IEC 61131-3

> Фича: [../features/0041-st-backend.md](../features/0041-st-backend.md) · ADR: [../adr/0041-st-backend.md](../adr/0041-st-backend.md) (вопрос 2) · анализ: [../analyze/0041-02-type-mapping.md](../analyze/0041-02-type-mapping.md) · тест-план: [../tests/0041-st-backend.md](../tests/0041-st-backend.md)

## Что было

**Состояние кода на 2026-07-15 (проверено).**

### Вход: `TypeNode`

`grammar/src/semantic/type_node.rs:496` — 14 вариантов: `Inference` (`#[default]`),
`Address(u64, Option<u64>)`, `Bit`, `Bool`, `Rational`, `Array(u16, Box<TypeNode>)`,
`Enum(String)`, `Struct(String)`, `Unsupported`, `Unit`, `BuiltinString`,
`BuiltinModel`, `BuiltinState`, `BuiltinNumeric`, `Integer { bits: u8, signed: bool }`.

Перечисление помечено **`#[non_exhaustive]`** (строка 495) — единственное такое
среди семантических узлов. Следствие зафиксировано в процессном бэклоге
(«Конфликт правил: `#[non_exhaustive]` против запрета `_ =>`»): разбор вынужден
иметь ветку `_`, и она обязана возвращать **ошибку**, а не тихий `None` (прецедент
— `eval::coerce_to_type`, `CLAUDE.md`).

### Единственный существующий аналог — `get_c_type`, и он **дефектен**

`grammar/src/generator/c/mod.rs:150-198`. Кандидат фичи **0029** описывает три
дефекта; проверено по коду:

```rust
pub fn get_c_type(typ: &TypeNode, model: &ModelNode) -> Option<String> {
    match typ {
        TypeNode::Bit => Some("int".to_string()),          // Д2: 1 бит → 32-битный знаковый
        TypeNode::Bool => Some("bool".to_string()),
        TypeNode::Rational => Some("float".to_string()),   // Д3: f32 против f64 симулятора
        TypeNode::Array(size, typ) => {
            if let TypeNode::Rational = **typ {
                Some("float *".to_string())                // непоследовательно: [float;N] → указатель
            } else {
                Some(format!("uint{}_t", *size))           // Д1: size — ЧИСЛО ЭЛЕМЕНТОВ → [u8;4] = uint4_t
            }
        }
        …
        TypeNode::BuiltinModel | TypeNode::BuiltinState | TypeNode::BuiltinNumeric
        | TypeNode::Unsupported | TypeNode::Inference
        | TypeNode::Address(_, _) => None,                 // Д4: тихий отказ
    }
}
```

**Д1b (следствие Д1):** переменная-массив **молча исчезает** из структуры. Проба
кандидата 0029: `var data: [u8; 4];` → в `arr.h` только `int flag;`, **без
диагностики**. Класс «тихий пропуск» — тот же, что дал восемь дефектов в фиче 0025.

**Ключевое решение (ADR, вопрос 2): `get_st_type` пишется независимо и с нуля.**
Общего слоя с `get_c_type` **не заводим** — это связало бы аддитивную 0041 с
незакрытой 0029 без выигрыша: системы типов C и IEC различны.

### Чего нет

- Эмиссии объявлений типов (`TYPE … END_TYPE`) — ни в одном бэкенде: C схлопывает
  `enum` в `uint{bits}_t` по максимальному варианту (`c/mod.rs:167-190`), PlantUML
  типы не печатает вовсе. Это **новый пласт вывода**.
- Пространство диагностик `ST-` — **свободно**. Заняты: `SE-*`, `CC-001`…`CC-013`
  (C-генератор), `PU-001` (PlantUML), `AM-001`…`AM-006` (карта адресов).

## Что сделано

> **Реализовано 2026-07-15.** План выполнен по нормативной таблице `T1..T16`
> ([анализ 0041-02](../analyze/0041-02-type-mapping.md)) с **четырьмя правками по
> фактам** — см. «Отклонения от плана».

### Итог

| Файл | Содержание |
|---|---|
| `generator/st/st_type.rs` (новый) | `get_st_type(&TypeNode, &ModelNode) -> Result<String, Diagnostic>`; таблица T1..T16; 16 юнит-тестов |
| `generator/st/st_decl.rs` (новый) | `emit_struct_types` (`TYPE … END_TYPE`), `emit_declarations` (`VAR_INPUT`/`VAR_OUTPUT`/`VAR_IN_OUT`/`VAR`/`VAR CONSTANT`); 10 юнит-тестов |
| `generator/st/mod.rs` | Секции объявлений в каждом `FUNCTION_BLOCK`; `TYPE` — раньше блоков |
| `generator/st/st_map.rs` | `raw_model_at`: код `ST-007` → `ST-012` (см. ниже) |
| `tests/st_tests.rs` (новый) | 6 интеграционных тестов через `compile_to_st` |
| `tests/data/st/valid/{array_var,types_all,enum_struct}.lam`, `invalid/array_zero.lam` | Фикстуры |

Тесты: **1549 зелёных**, `precheck.sh` проходит.

### Отклонения от плана (по фактам, не по вкусу)

Четыре нормы плана **опровергнуты проверкой** и исправлены:

| # | План говорил | Факт | Как теперь |
|---|---|---|---|
| **О1** | `Array` рекурсивно: `ARRAY [0..2] OF ARRAY [0..1] OF USINT` (T12/A4.2) | `iec2c`: `error: invalid item data type in array specification`. **Массивы в IEC не вкладываются** | Уплощение в многомерный `ARRAY [0..2, 0..1] OF USINT` (проба ✅). Порядок размерностей — внешняя первой |
| **О2** | Ветка `_` вынужденна (`#[non_exhaustive]`) → `ST-003` | `#[non_exhaustive]` действует **только на внешние крейты**; `generator/st` — тот же крейт, что `semantic`. Компилятор: `unreachable_patterns` | Ветки `_` **нет**, разбор исчерпывающий. Новый вариант `TypeNode` **завалит сборку** — гарантия строже, чем `ST-003` в рантайме. **`ST-003` не занят** |
| **О3** | Откат Option C: перечисление → плоский `USINT` | `enum Action { Idle = 670, Closing }` (`elevator.lam:121`) в `USINT` **не влезает** — было бы тихое усечение (класс дефекта, против которого написана фича) | Разрядность **по фактическому диапазону** вариантов (как в `c/mod.rs:167-190`); отрицательные варианты → знаковый тип. `Action_Idle : UINT := 670` (проба ✅) |
| **О4** | (не обсуждалось) | Lam разрешает `var data: [u8; 4] := 0;` — так объявлен **весь корпус**; `iec2c`: `error: invalid initial value in array specification with initialization` | Составные типы инициализатор **не получают**; обнуление по умолчанию IEC совпадает с намерением `:= 0`. Агрегатная форма `:= [0,0,0,0]` — 0041-04 |

### Конфликт кодов диагностик (исправлен)

Каркас 0041-01 занял `ST-007` и `ST-008` под **внутренние** ошибки («модель не
найдена», «корень карты не модель») — а нормативная таблица отводит их под
`Array` нулевого размера и неразрешимый `Enum`/`Struct`. Оба внутренних случая
переведены на **`ST-012`** (свободен: `ST-001`…`ST-011` расписаны задачами
01/02/04/05).

### План

1. **`st/st_type.rs`** — `get_st_type(typ: &TypeNode, model: &ModelNode) ->
   Result<String, Diagnostic>`.

   **Сигнатура принципиальна:** `Result`, а не `Option`. `Option::None` позволяет
   вызывающему «просто пропустить» переменную — именно так возник Д1b. `Err`
   обязывает обработать.

2. **Таблица (нормативно):** `Bit`/`Bool`→`BOOL` (исправляет Д2);
   `Integer{8..64, signed}`→`USINT/UINT/UDINT/ULINT` и `SINT/INT/DINT/LINT`;
   `Rational`→**`LREAL`** (f64 — как симулятор; исправляет Д3);
   `Array(N,T)`→**`ARRAY [0..N-1] OF <T>`** рекурсивно (исправляет Д1/Д1b);
   `Enum`→`TYPE … : (…); END_TYPE`; `Struct`→`TYPE … : STRUCT … END_STRUCT;
   END_TYPE`.
3. **Разбор без `_`** — все 14 вариантов явно; вынужденная ветка `_` (следствие
   `#[non_exhaustive]`) → **`ST-003`** (ошибка).
4. **Сбор объявлений `TYPE`** — `enum`/`struct` объявляются **до** первого
   использования; дедупликация по имени. Живёт в `st/st_decl.rs` или `StMap`.
5. **Диагностики:**

   | Код | Уровень | Условие |
   |---|---|---|
   | `ST-002` | ошибка | Тип без представления в IEC: `Inference`, `Unsupported`, `Address`, `Builtin*` (исправляет Д4) |
   | `ST-003` | ошибка | Неизвестный вариант `TypeNode` (вынужденная `_`) |
   | `ST-007` | ошибка | `Array` нулевого размера (`ARRAY [0..-1]` невыразим) |
   | `ST-008` | ошибка | `Enum`/`Struct` не разрешается в модели |

6. **Фикстуры:** `grammar/tests/data/st/valid/{types_all,array_var}.lam`;
   `invalid/{unmapped_type,array_zero,unresolved_struct}.lam`.

### Открытые вопросы — **ЗАКРЫТЫ пробой [0041-06](0041-06-matiec-validation.md)** (2026-07-15)

- **П4 — ❌ красная ⇒ откат Option C обязателен.** `iec2c` **отвергает** явные
  значения вариантов: `TYPE Floor : (Bottom := 80, Top);` →
  `error: ')' missing at the end of enumerated specification`. Уточняющие пробы:
  перечисление **без** значений (`(Bottom, Middle, Top)`) принимается, **с**
  значениями — нет: явные значения появились в **3-й редакции** IEC 61131-3, а
  MatIEC её не знает. Перечисления Lam значения **имеют**
  (`tests/data/semantic/valid/enum_with_values`), поэтому прямое отображение
  непригодно.
  **Принято: `USINT` + именованные константы.** Форма проверена пробой (`✅`):
  ```
  FUNCTION_BLOCK Demo
  VAR CONSTANT
      Floor_Bottom : USINT := 80;
      Floor_Top : USINT := 81;
  END_VAR
  ```
  ⚠ **Не** `VAR_GLOBAL CONSTANT`, как предполагал ADR: `VAR_GLOBAL` вне
  `CONFIGURATION` **недопустим** (`error: unknown syntax error`) — цель `st`
  `CONFIGURATION` не эмитит (П2). Константы объявляются `VAR CONSTANT` **внутри**
  FB.
- **П5 — ✅.** `ARRAY [0..3] OF USINT` в `VAR` блока FB принимается. T12
  подтверждён.
- **П6 — ✅.** `LREAL` принимается; совпадает с f64 симулятора. Ограничение малых
  ПЛК остаётся вопросом платформы — документируется в 0041-07.

### Критерий приёмки, добавленный гейтом 0041-06 — **предпосылка опровергнута**

Критерий гласил: «`iec2c` обязан принимать вывод по всему корпусу `examples/`;
0041-02 обязана эмитить хотя бы `VAR … END_VAR`, иначе вывод остаётся
невалидным». Он опирался на предположение, что **объявлений достаточно**.

**Проверка (2026-07-15) предположение опровергла.** `VAR … END_VAR` —
необходимое, но **не достаточное** условие: `iec2c` требует от `FUNCTION_BLOCK`
ещё и **тело**.

| Форма | Исход `iec2c` |
|---|---|
| `FUNCTION_BLOCK` + комментарий (каркас 0041-01) | ❌ `FUNCTION_BLOCK with no variable declarations and no body` |
| `FUNCTION_BLOCK` + `VAR … END_VAR`, тело — комментарий | ❌ `no body defined in function block declaration` |
| `FUNCTION_BLOCK` + `VAR … END_VAR` + пустой оператор `;` | ❌ `too many consecutive syntax errors` |
| `FUNCTION_BLOCK` + `VAR … END_VAR` + любой оператор | ✅ |

Тело — `CASE state OF` — предмет задачи **0041-03**. Заглушку-тело генератор
**намеренно не эмитит**: она уехала бы в ПЛК под видом логики (тот же класс, что
`ST-009` в 0041-04, но там заглушка хотя бы сопровождается предупреждением).

**Что 0041-02 доказала фактически.** Объявления, которые она эмитит, — валидный
ST по всему корпусу. Проверено подстановкой фиктивного тела в порождённые файлы:

```sh
# все 5 примеров: comprehensive, elevator, elevator_mini, extend_complex, stacker
lamc compile -t st examples/<пример>.lam -o out
# в каждый FUNCTION_BLOCK подставлено `_probe : USINT;` + `_probe := 1;`
iec2c -I <matiec>/lib -T out/gen out/<пример>.st   # ✅ 5 из 5
```

Без подстановки — ❌ 5 из 5, но **диагностика сместилась**: три примера
(`comprehensive`, `elevator`, `elevator_mini`) теперь дают `no body defined`
вместо `no variable declarations and no body`, то есть объявления доехали. Два
(`stacker`, `extend_complex`) сохраняют старый текст: у их блоков нет **ни одной**
используемой переменной — секций для них не существует, и это корректно.

**Вывод (правило 20):** критерий приёмки 0041-02 в исходной формулировке
**недостижим** и закрывается задачей 0041-03. Строка «`iec2c` принимает вывод по
корпусу» остаётся **открытой**; здесь она заменена на проверяемое подмножество —
«объявления валидны». Это записано честно, а не замаскировано.

### Статус по функциональности (правило 11)

| Функциональность | Работа | Обоснование |
|---|---|---|
| `generator/st/st_type.rs` | **Требуется** (новый) | Ядро задачи |
| `generator/st/st_decl.rs` | **Требуется** | `TYPE … END_TYPE` |
| `generator/c/mod.rs` (`get_c_type`) | **н/п — НЕ трогать** | Это территория **фичи 0029**. R3/A1: вывод `c` байт-в-байт неизменен |
| `semantic/type_node.rs` | **н/п** | `TypeNode` — вход обеих фич; остаётся стабильным |
| Язык | **н/п** | Правило 22 |

## Проверки

> **Исполнено 2026-07-15.** 1549 тестов зелёные, `precheck.sh` проходит.

```sh
cargo test -p grammar --lib -- --test-threads=1 generator::st   # 34 юнит-теста
cargo test -p grammar --test st_tests -- --test-threads=1       # 6 интеграционных
cargo test -- --test-threads=1                                  # 1549 зелёных
./scripts/precheck.sh
```

| Требование | Проверка | Как |
|---|---|---|
| **R5.1** (таблица) | T12–T18 / A3.1 | 14 юнит-тестов в `st_type.rs` — по одному на T1..T14 |
| **R5.2** (`ARRAY`) | **T15** / A4.1 | `var data: [u8; 4];` → `data : ARRAY [0..3] OF USINT;`. **Тот же вход, что в пробе кандидата 0029**, где C молча теряет переменную — прямой контрпример |
| **R5.2** (вложенность) | T16 / A4.2 | `[[u8;2];3]` → `ARRAY [0..2] OF ARRAY [0..1] OF USINT` |
| **R5.3** (`LREAL`) | T14 / A4.3 | `var x: float;` → `x : LREAL;`, **не** `REAL` |
| **R5.4** (`TYPE`) | T17, T18 / A5.2 | **Зонд первым:** значения вариантов `enum Floor { Bottom = 80, Top }` (`elevator.lam:117`) снять из семантики — `Top` наследует `81`, но проверить, а не угадать (`CLAUDE.md`) |
| **R4.1** (`Result`) | — | Ревью сигнатуры |
| **R4.2** (`ST-003`) | T21 / A3 | Тест на **код** диагностики |
| **R4.3** (не теряется) | T15, T19, T20 / A5 | Контрпримеры → `ST-002`/`ST-007`; вывод **не создаётся** |

**Правило `CLAUDE.md`:** «Новые тесты — сперва зонд для захвата реального вывода,
затем assertions против захваченных значений (не угадывать строки/адреса)».
Особенно для T17 (значения enum) и T18 (структуры).
