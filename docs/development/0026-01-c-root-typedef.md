# Задача 0026-01: Безусловная эмиссия typedef корня в генераторе C

> Фича: [../features/0026-c-root-typedef.md](../features/0026-c-root-typedef.md) · ADR: [../adr/0026-c-root-typedef.md](../adr/0026-c-root-typedef.md) · анализ: [../analyze/0026-c-root-typedef.md](../analyze/0026-c-root-typedef.md)

> **Статус: ПЛАНИРУЕТСЯ (разработка не начата).** Раздел «Что было» —
> зафиксированное состояние кода на 2026-07-15 (ветка `v2`, проверено чтением и
> пробами). Разделы «Что сделано» и «Проверки» — **план предстоящей работы**, а
> не отчёт; заполняются по факту выполнения.

## Что было

Состояние на 2026-07-15, ветка `v2` (все ссылки на строки проверены).

### Дефект

`grammar/src/generator/c/c_header.rs:511–534`, функция `generate_header` —
эмиссия forward-деклараций разветвлена на три случая:

```rust
if !sorted_models.is_empty() {          // (1) есть под-модели
    printer.print("/* Forward declarations */").nl();
    for element in &sorted_models { … typedef struct {Sub} {Sub}; … }   // :518
    let root_struct = map.root_name().unique_camelcase();
    printer.print(&format!("typedef struct {0} {0};", root_struct)).nl(); // :522
} else if options.hal {                  // (2) под-моделей нет, цель c-hal
    let root_struct = map.root_name().unique_camelcase();
    printer.print(&format!("typedef struct {0} {0};", root_struct)).nl(); // :531
}                                        // (3) под-моделей нет, цель c → НИЧЕГО
```

Комментарий ветки (2), `c_header.rs:526–528`, фиксирует, что правку 0020-05
намеренно ограничили целью `c-hal`: «Обычный режим `c` не трогаем (вывод
байт-в-байт)». Ветка (3) — источник дефекта.

Сопутствующие факты:

- `c_header.rs:333` — структура модели печатается **тегом без typedef**:
  `struct {Root} {`; закрывается `};` (`c_header.rs:483`). Голое имя `{Root}`
  становится типом **только** благодаря forward-`typedef`.
- Прототипы печатаются через голое имя: `void {Root}_init({Root} *main);`.
- `c_header.rs:540` — пользовательские структуры перед эмиссией **явно
  сортируются** (`structs.sort_by_key`), а состояния — нет (см. «Находка»).

### Воспроизведение (проба выполнена)

```sh
cargo build --bin lamc
./target/debug/lamc compile -t c \
  grammar/tests/data/semantic/valid/deterministic_transitions.lam -o /tmp/out
```

Порождённый `/tmp/out/deterministic_transitions.h` — **без** typedef:

```c
struct DeterministicTransitions { … };
void DeterministicTransitions_init(DeterministicTransitions *main);
```

```
$ cc -fsyntax-only deterministic_transitions.c
error: must use 'struct' tag to refer to type 'DeterministicTransitions'
… 8 ошибок (4 в .h + 4 в .c)
```

Та же модель целью `-t c-hal` → `cc -fsyntax-only` даёт **0 ошибок**: ветка (2)
эмитит typedef. Это и есть эталон целевого поведения.

### Почему не поймано до сих пор

- Все **пять** `examples/*.lam`, на которых `precheck.sh` собирает C через
  cmake/ninja, содержат под-модели → попадают в ветку (1). Проверено: в каждом
  порождённом `.h` typedef присутствует (2–10 вхождений).
- `simulation/tests/data/eval/conformance_u8.lam` — единственное место, где
  порождённый C реально компилируется тестом (`conformance_c_tests.rs:141`), —
  **обходит** дефект: модель намеренно обёрнута в `model Conf { … }` +
  `start Entry = Conf;`. В самой фикстуре стоит комментарий: «для **одиночной**
  корневой модели генератор не эмитит `typedef`, и порождённый C не
  компилируется (дефект генератора, бэклог)».
- Ни один из **35** тестов `grammar/tests/codegen_tests.rs` (все зелёные) не
  вызывает `cc` и не проверяет одиночную модель на наличие typedef.

### Находка, зафиксированная при разборе (вне области задачи)

Генерация C **недетерминирована**: пять запусков на одном входе дали разный
порядок вариантов `enum` состояний и веток `switch` (причина — итерация по
`states: HashMap<String, StateNode>`, `grammar/src/semantic/mod.rs:101`).
Следствие для задачи: **проверки только структурные**, побайтовое сравнение
вывода как метод исключено. Сама находка — отдельный кандидат в `FEATURES.md`
(передана координатору), в 0026-01 **не** чинится.

## Что сделано

> **Планируется (разработка не начата).** Ниже — план работ по
> [ADR 0026](../adr/0026-c-root-typedef.md), Option A.

### 1. Слияние путей эмиссии (`grammar/src/generator/c/c_header.rs:511–534`)

Ветвление снимается: секция forward-деклараций печатается **всегда**, цикл по
под-моделям исполняется столько раз, сколько их есть (в том числе ноль), typedef
корня печатается **безусловно**. Ветка `else if options.hal` удаляется — цель
`c-hal` получает typedef тем же общим путём (R1, R3, R6).

Ожидаемая форма:

```rust
printer.print("/* Forward declarations */").nl();
for element in &sorted_models { … typedef struct {Sub} {Sub}; … }
let root_struct = map.root_name().unique_camelcase();
printer.print(&format!("typedef struct {0} {0};", root_struct)).nl();
printer.nl();
```

### 2. Тесты (`grammar/tests/codegen_tests.rs`)

- **Наличие и единственность** typedef корня для одиночной модели, цель `c`:
  счёт вхождений `== 1` (не `contains` — иначе дубль пройдёт незамеченным).
  Критерий A1, проверки T1, T7.
- **Компиляция порождённого C** настоящим `cc -std=c11 -fsyntax-only` — главная
  проверка задачи (A2, T2, T3). Вызов через `std::process::Command` по образцу
  `simulation/tests/conformance_c_tests.rs:141`, с гейтом доступности `cc`
  (`cc_available()`, там же строка 65): без `cc` тест пропускается, а не падает.
- **Регресс `c-hal`:** typedef корня ровно один после слияния путей (T7).

### 3. Фикстура-обход 0025 (`simulation/tests/data/eval/conformance_u8.lam`)

Комментарий фикстуры, объясняющий обёртку ссылкой на дефект, **обновляется**
(иначе он дезинформирует: дефекта больше нет). Сама обёртка **остаётся** — тест
проверяет сверку значений, а не форму фикстуры; снятие обёртки — лишний риск в
рамках фикса генератора.

### Статус по обратной функциональности (правило 11)

| Функциональность | Статус плана |
|---|---|
| Цель `c`, одиночная модель | **Чинится** — из некомпилируемого вывода в компилируемый (+2 строки) |
| Цель `c`, модель с под-моделями | **Не затрагивается** — ветка (1) уже печатала typedef корня; поведение сохраняется |
| Цель `c-hal` | **Не регрессирует** — typedef приходит общим путём; допустимое отличие: +строка `/* Forward declarations */` (косметика) |
| Генератор PlantUML | **н/п** — правка внутри `generator/c/c_header.rs` |
| Язык `.lam` | **н/п** — синтаксис и семантика не затронуты; версия языка остаётся **0.2.0** (правило 22), мигратор и правка документации языка не нужны |
| Симуляция / сверка «симулятор ↔ C» | **Не регрессирует** — `conformance_c_tests.rs` остаётся зелёным; обёртка в фикстуре становится необязательной |
| LSP | **н/п** — не затронут; прогон с `--features lsp` как страховка |

## Проверки

> **Планируется (разработка не начата).** Условия и ожидаемые результаты
> согласованы с [тест-планом](../tests/0026-c-root-typedef.md) (T1–T14) и
> критериями [анализа](../analyze/0026-c-root-typedef.md) (A1–A9).

### Порядок проверки фикса (обязателен)

Тест на компиляцию (T2) сначала прогоняется на коде **до** фикса и обязан
**упасть** (зафиксировано: 8 ошибок `must use 'struct' tag`), затем — после
фикса и обязан **пройти**. Тест, зелёный до и после, дефект этого класса не
удержит — именно так дефект и дожил до 0026.

### Команды (правило 5)

```sh
cargo build --bin lamc
cargo test --test codegen_tests -- --test-threads=1   # база до фичи: 35 зелёных
cargo test -- --test-threads=1                        # полный прогон (A6, T11)
cargo test --features lsp -- --test-threads=1         # включая LSP (A6, T12)
./scripts/precheck.sh                                 # fmt + clippy + сборка C примеров (A9, T13)
```

### Живая проверка (A7, T14)

```sh
./target/debug/lamc compile -t c \
  grammar/tests/data/semantic/valid/deterministic_transitions.lam -o /tmp/out
grep -c "typedef struct DeterministicTransitions" /tmp/out/deterministic_transitions.h  # ожидается 1
cd /tmp/out && cc -std=c11 -fsyntax-only deterministic_transitions.c                    # ожидается 0 ошибок (было 8)
cc -std=c11 -fsyntax-only -x c deterministic_transitions.h                              # заголовок самодостаточен (T3)
```

### Ревью дифа (A5, A8; T9, T10)

- `grep -n "options.hal" grammar/src/generator/c/c_header.rs` — не должно
  остаться ветки, гейтящей typedef корня по цели (R6).
- В дифе **нет** `grammar.lalrpop`, `parser/lexer.rs`, `parser/ast.rs`,
  `semantic/` — подтверждение, что язык не затронут; `README.md` → «Версия языка:
  0.2.0» без изменений (правило 22).

### Ожидаемый итог

- T1–T3, T7, T14 — новые/живые проверки: зелёные (были бы красными до фикса).
- T4–T6, T8, T11–T13 — регресс: зелёные **без правок существующих тестов**
  (`test_header_has_forward_declarations` `codegen_tests.rs:547`,
  `c_hal_emits_address_table_and_hal` `:1103`, `plain_c_has_no_hal_artifacts`
  `:1141`). Необходимость править существующий тест = сигнал о незапланированном
  изменении поведения, требующий возврата к анализу.
