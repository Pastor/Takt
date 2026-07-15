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

> **Планируется (разработка не начата).** План — по нормативной таблице `T1..T16`
> ([анализ 0041-02](../analyze/0041-02-type-mapping.md)).

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

### Открытые вопросы — решаются **пробой 0041-06** до начала работы

- **П4:** принимает ли `iec2c` перечислимые `TYPE Floor : (Bottom := 80, Top);`?
  Если нет — **откат Option C** ADR: `TYPE Floor : USINT; END_TYPE` +
  `VAR_GLOBAL CONSTANT Floor_Bottom : USINT := 80;`.
- **П5:** `ARRAY [0..3] OF USINT` внутри `VAR` блока FB.
- **П6:** `LREAL` (часть малых ПЛК не поддерживает f64 — ограничение платформы, не
  генератора; документируется в 0041-07).

### Статус по функциональности (правило 11)

| Функциональность | Работа | Обоснование |
|---|---|---|
| `generator/st/st_type.rs` | **Требуется** (новый) | Ядро задачи |
| `generator/st/st_decl.rs` | **Требуется** | `TYPE … END_TYPE` |
| `generator/c/mod.rs` (`get_c_type`) | **н/п — НЕ трогать** | Это территория **фичи 0029**. R3/A1: вывод `c` байт-в-байт неизменен |
| `semantic/type_node.rs` | **н/п** | `TypeNode` — вход обеих фич; остаётся стабильным |
| Язык | **н/п** | Правило 22 |

## Проверки

> **Планируется (разработка не начата).**

```sh
cargo test st_type -- --test-threads=1
cargo test -- --test-threads=1
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
