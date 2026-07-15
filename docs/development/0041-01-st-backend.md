# Задача 0041-01: Каркас ST-бэкенда (`Language::ST`, `generator/st/`, цели `st`/`st-at`)

> Фича: [../features/0041-st-backend.md](../features/0041-st-backend.md) · ADR: [../adr/0041-st-backend.md](../adr/0041-st-backend.md) · анализ: [../analyze/0041-st-backend.md](../analyze/0041-st-backend.md) · тест-план: [../tests/0041-st-backend.md](../tests/0041-st-backend.md)

## Что было

**Состояние кода на 2026-07-15 (проверено, не по памяти).**

Слой генерации — `grammar/src/generator/`, 6248 строк в 11 файлах:

| Файл | Строк | Роль |
|---|---|---|
| `mod.rs` | 96 | `Language`, `GenerateOptions`, трейт `Generator`, свободная `generate()` |
| `indent.rs` | 123 | `Printer` — печать с отступами (`ident`/`nl`/`up`/`down`) |
| `plantuml/mod.rs` | 258 | Генератор PlantUML — **ближайший образец «второго бэкенда»** |
| `plantuml/puml_map.rs` | 66 | Тонкая обёртка-снимок над `semantic::minimap::Map` |
| `c/mod.rs` | 558 | Точка входа C-бэкенда, `get_c_type`, `PortClass`, `PortDirection` |
| `c/c_map.rs` | 128 | `CMap` — снимок + `UsageSet` + `guard_enable` |
| `c/c_header.rs` | 1070 | Заголовок; `generate_hal` (0020-05) |
| `c/c_expr.rs` | 1736 | Выражения (кандидат 0027 на дробление) |
| `c/c_source.rs` | 1086 | Тело `.c` |
| `c/c_model.rs` | 907 | Модели, `switch (model->state)` (строка 554) |
| `c/c_decl.rs` | 220 | Объявления |

### Точка расширения — уже спроектирована

`generator/mod.rs:8-19`:

```rust
/// Поддерживаемые языки генерации кода.
///
/// Помечен `#[non_exhaustive]`: список целевых языков будет расширяться, и
/// добавление вариантов не должно ломать обратную совместимость (правило 11).
#[derive(Debug)]
#[non_exhaustive]
pub enum Language {
    /// Генерация C-кода.
    C,
    /// Генерация диаграммы состояний PlantUML.
    PlantUML,
}
```

То есть **ровно два варианта**, и комментарий прямо предусматривает то, что делает
эта задача.

### Трейт и диспетчер

`generator/mod.rs:66-96`:

```rust
pub trait Generator {
    fn generate(&self, model: &ModelNode, output_path: &str,
                options: &GenerateOptions) -> Result<(), Diagnostic>;
}

pub fn generate(l: Language, model: &ModelNode, output_path: &str,
                options: &GenerateOptions) -> Result<(), Diagnostic> {
    match l {
        Language::C        => { let generator = c::Generator {};        generator.generate(model, output_path, options) }
        Language::PlantUML => { let generator = plantuml::Generator {}; generator.generate(model, output_path, options) }
    }
}
```

`match` — **внутри** крейта, где `#[non_exhaustive]` не действует → добавление
`ST` **завалит сборку**, пока ветка не написана. Это желаемое поведение.

### `GenerateOptions` — канал для карты адресов уже есть

`generator/mod.rs:28-42` (тоже `#[non_exhaustive]`): `guard_enable: bool`,
`hal: bool`, `address_map: HashMap<String, ResolvedAddress>`. Поля `hal`/
`address_map` заведены задачей 0020-05 для режима `c-hal`. **Новых полей для ST не
нужно** — `hal` переиспользуется как «адрес-потребляющий режим».

### Образец: PlantUML-бэкенд

`plantuml/mod.rs` — структура, которую надо повторить:

1. `pub struct Generator {}` + `impl AsGenerator for Generator` (строки 26-51):
   строит `PumlMap`, зовёт чистую `generate_diagram(&map) -> Result<String,
   Diagnostic>`, пишет файл, ошибку записи оборачивает в `Diagnostic` с кодом
   (`PU-001`).
2. `puml_map.rs` — тонкая обёртка над `semantic::minimap::Map` (`Map::create` от
   `model.copy(None, None)`), отдающая ровно то, что нужно генератору:
   `root_name()`, `model()`, `states()`, `state_at()`, `using_models()`.
3. Юнит-тесты — в самом модуле (`#[cfg(test)] mod tests`, строки 153-257),
   assertions на **подстроки** через хелпер `make_map(src, name)`.

`CMap` (`c_map.rs`) — то же плюс `usage: UsageSet` (`compute_usage`) и
`guard_enable`.

### CLI

`bin/lamc.rs:551` — `match options.target.as_str()`: **три** цели — `"c-hal"`,
`"c"`, `"plantuml"`; `t => { eprintln!("Ошибка: неизвестная цель '{}'.
Поддерживается: c, plantuml", t); process::exit(1); }`.

**Найденный попутно дефект:** сообщение перечисляет `c, plantuml`, **умалчивая про
`c-hal`**, который реально поддерживается (строка 552). Строка всё равно меняется
этой задачей → чинится здесь.

Публичный API крейта (`lib.rs`): `compile_to_c` (209), `compile_to_c_hal` (258),
`compile_to_plantuml` (316) — образцы сигнатур.

### Чего нет

Ни строки про Structured Text / IEC 61131-3 во всём репозитории (проверено
`grep -rni 'structured text|iec 61131|matiec|codesys'` → пусто). Задача создаёт
слой с нуля.

## Что сделано

> **Планируется (разработка не начата).** Ниже — план по ADR 0041 и анализу.

### План

1. **`Language::ST`** — новый вариант в `generator/mod.rs:14` + ветка в
   `generate()` (строка 86). Сборка завалится, пока ветка не написана, — это
   проверка того, что диспетчер полон.
2. **`generator/st/`** — новый модуль. Деление **сразу** по логике (R11: лимит
   ~1000 строк на файл; не повторять `validate.rs` — 3648 строк, кандидат 0027):

   | Файл | Роль | Задача |
   |---|---|---|
   | `st/mod.rs` | `Generator`, `impl AsGenerator`, запись файла, `ST-001` | 0041-01 |
   | `st/st_map.rs` | `StMap` — снимок + `UsageSet` (по образцу `c_map.rs`) | 0041-01 |
   | `st/st_type.rs` | `get_st_type` (таблица T1..T14), `TYPE … END_TYPE` | 0041-02 |
   | `st/st_decl.rs` | `VAR`/`VAR_INPUT`/`VAR_OUTPUT`/`VAR_GLOBAL` | 0041-02, 0041-05 |
   | `st/st_model.rs` | `FUNCTION_BLOCK`, `CASE state OF`, композиция | 0041-03 |
   | `st/st_expr.rs` | Выражения, условия, операторы | 0041-04 |

3. **`StMap`** (`st/st_map.rs`) — по образцу `CMap`: `Map::create(Rc::new(
   RefCell::new(model.copy(None, None))))` + `compute_usage`. `guard_enable` в ST
   — **открытый вопрос**: guard-формулы (`verification/`) в ST не транслируются;
   вероятно, поле игнорируется + предупреждение при `--guard`. Решить при
   реализации.
4. **Публичный API** (`lib.rs`): `compile_to_st` (по образцу `compile_to_c:209`),
   `compile_to_st_at` (по образцу `compile_to_c_hal:258` — с `external_entries` и
   `resolve_addresses`).
5. **CLI** (`bin/lamc.rs`): ветки `"st"` и `"st-at"` в `match` (строка 551);
   `st-at` — по образцу ветки `"c-hal"` (552), включая проверку
   `options.target != "c-hal"` на строке 532 (её условие расширяется:
   адрес-потребляющие цели теперь `c-hal` **и** `st-at`).
   **Попутно:** сообщение о неизвестной цели → «Поддерживается: c, c-hal,
   plantuml, st, st-at».
6. **Минимальный вывод** — на этом этапе достаточно `FUNCTION_BLOCK <Имя>` /
   `END_FUNCTION_BLOCK` с пустым телом: задача 0041-01 закрывает **каркас и
   диспетчеризацию**, наполнение — задачи 02-05.
7. **Версия крейта:** `grammar/Cargo.toml` `0.2.0` → `0.3.0`. **Версию языка не
   трогать** (`README.md:1007` — «Версия языка: 0.2.0»): язык не меняется
   (правило 22).

### Статус по функциональности (правило 11)

| Функциональность | Работа | Обоснование |
|---|---|---|
| `generator` (`Language`, диспетчер) | **Требуется** | Ядро задачи |
| `generator/st/` | **Требуется** (новый) | Ядро задачи |
| `bin/lamc.rs` | **Требуется** | Две новые цели + фикс справки |
| `lib.rs` (публичный API) | **Требуется** | `compile_to_st`, `compile_to_st_at` |
| `generator/c/`, `generator/plantuml/` | **н/п** | Не трогаются: R3/A1 требует байт-в-байт совпадения вывода |
| Язык (`lexer`, `grammar.lalrpop`, `parser/ast.rs`) | **н/п** | Правило 22: язык не меняется |
| `semantic/` | **н/п** | ST — потребитель готового дерева |
| `simulation` | **н/п** | Другой крейт; бэкенд генерации его не задевает |
| `lsp.rs` | **н/п** | Не связан с целями генерации |
| `format/` (0024) | **н/п** | Новых узлов АСД не добавляется |

## Проверки

> **Планируется (разработка не начата).**

```sh
cargo build --bin lamc
cargo check
cargo clippy --all-targets --all-features
cargo test -- --test-threads=1
cargo test --features lsp -- --test-threads=1
./scripts/precheck.sh
```

Соответствие требованиям анализа:

| Требование | Проверка | Как |
|---|---|---|
| **R1** (`Language::ST`) | T7 | `cargo check`; `#[non_exhaustive]` на месте |
| **R2** (цели `st`/`st-at`) | T8, T9, T10 | `lamc compile -t st examples/stacker.lam -o out` → файл создан, код 0 |
| **R3** (регресс = 0) | T1–T4 | **Снапшоты `c`/`c-hal`/`plantuml` снять ДО правок**, сверить байт-в-байт после |
| **R11** (размер) | T11 | `wc -l grammar/src/generator/st/*.rs` — каждый ≤ ~1000 |
| **R12** (версии) | T5, T6 | Диффа в языке нет; `Cargo.toml` = 0.3.0; `README.md:1007` не тронут |

**Порядок:** снапшоты `c`/`c-hal`/`plantuml` (T1–T3) снимаются **первым делом** —
без базовой линии доказать регресс = 0 невозможно.

**Следующая задача — [0041-06](0041-06-matiec-validation.md) (проба-гейт MatIEC),
а не 0041-02.** Её исход меняет проектные решения (`--st-configuration`,
перечислимые `TYPE`, `VAR_IN_OUT`) — узнать это после написания 02-05 значит
переписывать сделанное.
