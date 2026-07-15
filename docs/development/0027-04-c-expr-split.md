# Задача 0027-04: Разделение `generator/c/c_expr.rs` (1736 → каталог `c_expr/`)

> Фича: [../features/0027-module-size-split.md](../features/0027-module-size-split.md) · ADR: [../adr/0027-module-size-split.md](../adr/0027-module-size-split.md) · анализ: [../analyze/0027-module-size-split.md](../analyze/0027-module-size-split.md) · тест-план: [../tests/0027-module-size-split.md](../tests/0027-module-size-split.md)

> **Статус: Планируется (разработка не начата).** Разделы «Что сделано» и
> «Проверки» — план; заполняются фактом по ходу выполнения.
>
> **Предусловие:** выполнена [0027-01](0027-01-module-size-split.md) и снят
> эталон «до» (тест-план, T0).

## Что было

*Реальное состояние на 2026-07-15 (ветка `v2`, коммит `6984471`); номера строк
проверены.*

`grammar/src/generator/c/c_expr.rs` — **1736 строк** при лимите ~1000
(`CLAUDE.md:128`), превышение в **1,7 раза**. Цифра кандидата в `FEATURES.md`
(1736) **подтвердилась**.

### Топология файла

**Тестов в файле нет** — `#[cfg(test)]` не встречается ни разу
(`grep -c "cfg(test)" grammar/src/generator/c/c_expr.rs` → **0**). Все 1736
строк — продуктивный код. Тесты C-генератора живут в
`grammar/src/generator/c/mod.rs:228` (`mod tests`) и в
`grammar/tests/codegen_tests.rs` (1203 строки).

Это делает задачу технически **самой простой** из трёх разделений: нечего
разрезать по темам, нет риска потерять тест при переносе (риск Р3 не
применяется).

### Логические группы

| Группа | Строки | Объём | Содержимое |
|---|---|---:|---|
| Шапка + импорты | 1–26 | 26 | `//!`-доки; `use` из `super`, `c_map`, `indent::Printer`, `semantic::*` |
| Имена функций | 27–46 | 20 | `get_function_name` (28) |
| Разрешение путей вложенных моделей | 47–176 | 130 | `find_in_extend` (47), `find_in_concat` (81), `find_in_parallel` (127) |
| Разрешение переменных → C-выражение | 177–361 | 185 | `field_name_in_parent` (177), `resolve_variable_c_expr` (196: Simple/Const/Port, `read_bit`/`read_float`), `resolve_simple_var_in_context` (315) |
| Имена макросов условий | 362–381 | 20 | `condition_macro_name` (362) |
| Таблица приоритетов | 382–427 | 46 | `expr_precedence` (382): Assign=1 … унарные=13, атомы=15 |
| Генерация условий | 428–779 | 352 | `generate_condition_expr` (428): литералы (434–444), Not/Parenthesis (445–451), арифметика (452–461), And/Or (462–471), сравнения (472–491), Equal (492–572) и NotEqual (573–653) со спец-веткой `ConditionNode::Model` (сравнение состояний), Variable (654), EnumVariant (664), ArraySubscript (665), BitAccess (677) |
| Вызовы функций | 780–903 | 124 | `generate_args` (780), `generate_function_call` (803): Local → `Model_name(main,…)`, External, Builtin (min/max/abs/clamp → тернарник; debug/S → ошибка) |
| Генерация выражений | 904–1412 | **509** | `generate_expr` (904) — **крупнейшая функция файла, один `match`**: литералы (923), унарные (940), Power→`pow()` (960), бинарные арифметические (969), сдвиги (998), побитовые (1010), сравнение (1027), логические (1059), специальные — Parenthesis/тернарник/Assign/ArraySubscript/Variable/Condition/Function/Initializer/Array/Cast (1071–1315), неподдерживаемые — ArraySlice/BitAccess/CodeBlock/NamedFunctionBox/List/Type/Address/Model (1316–1412) |
| Операторы и блоки | 1413–1736 | 324 | `generate_stmt_expression` (1413, обёртка над `generate_expr` с `min_prec=0`), `generate_formula_check` (1424: Formulas/Guard→assert/LTL-заглушка), `generate_code_block` (1455: Block/Expression/If/Loop/For/Variable/Return/Continue/Break) |

### Внешний контракт — полностью внутренний

Модуль объявлен **приватно** (`grammar/src/generator/c/mod.rs:34`):

```rust
mod c_expr;
```

Реэкспортов `pub use c_expr::…` в `generator/c/mod.rs` **нет**; все элементы —
`pub(super)`, то есть видны только внутри `generator::c`. За пределы крейта не
уходит ничего — ни в `grammar/tests`, ни в `grammar/src/bin`. Фича-гейтов нет.

**Наружу (в пределах `generator/c`) используются 6 имён:**

| Потребитель | Строка импорта | Имена | Вызовы |
|---|---|---|---|
| `generator/c/c_decl.rs` | 6 | `generate_code_block`, `get_function_name` | 157, 166, 192 |
| `generator/c/c_model.rs` | 8 | `generate_code_block`, `generate_condition_expr`, `generate_expr`, `generate_formula_check` | 30, 121, 296, 550, 668 |
| `generator/c/c_source.rs` | 38 | `generate_stmt_expression` | 63 |

**`pub(super)`, но вне файла не используются** (кандидаты на сужение до
`pub(in crate::generator::c::c_expr)`): `field_name_in_parent` (177),
`resolve_variable_c_expr` (196), `resolve_simple_var_in_context` (315),
`condition_macro_name` (362), `expr_precedence` (382).

**Приватные:** `find_in_extend` (47), `find_in_concat` (81), `find_in_parallel`
(127), `generate_args` (780), `generate_function_call` (803).

### Взаимные вызовы между будущими подмодулями

`generate_function_call` (803) → `generate_args` (780) → `generate_stmt_expression`
(1413) → `generate_expr` (904); `generate_condition_expr` (428) ↔ `generate_expr`.
То есть `expr` ↔ `call` ↔ `condition` образуют цикл. **Это не проблема:** Rust
допускает взаимные вызовы между модулями одного дерева; разрывать не требуется
(зафиксировано в ADR, риск Р6).

## Что сделано

> **Планируется (разработка не начата).** Ниже — план реализации.

`c_expr.rs` → каталог `grammar/src/generator/c/c_expr/`. Объявление в
`generator/c/mod.rs:34` (`mod c_expr;`) **не меняется**.

### Целевая раскладка

| Файл | Группа | ≈строк |
|---|---|---:|
| `mod.rs` | `//!`-doc + `pub(super) use` шести имён для `c_decl`/`c_model`/`c_source` | ~40 |
| `names.rs` | `get_function_name` (28), `condition_macro_name` (362) | ~50 |
| `resolve.rs` | `find_in_extend` (47), `find_in_concat` (81), `find_in_parallel` (127), `field_name_in_parent` (177), `resolve_variable_c_expr` (196), `resolve_simple_var_in_context` (315) | ~335 |
| `precedence.rs` | `expr_precedence` (382) | ~50 |
| `condition.rs` | `generate_condition_expr` (428–779) | ~355 |
| `call.rs` | `generate_args` (780), `generate_function_call` (803) | ~125 |
| `expr.rs` | `generate_expr` (904–1412) | ~510 |
| `stmt.rs` | `generate_stmt_expression` (1413), `generate_formula_check` (1424), `generate_code_block` (1455) | ~325 |

Все файлы **≤1000 строк** (R1). Самый крупный — `expr.rs` (~510): это одна
функция `generate_expr`, дробить её на файлы бессмысленно (лимит соблюдён, а
`match` по вариантам `ExpressionNode` — единое целое).

### Реэкспорт в `mod.rs`

```rust
pub(super) use self::names::get_function_name;
pub(super) use self::condition::generate_condition_expr;
pub(super) use self::expr::generate_expr;
pub(super) use self::stmt::{generate_code_block, generate_formula_check, generate_stmt_expression};
```

Ровно 6 имён из таблицы «Что было» — строки импорта в `c_decl.rs:6`,
`c_model.rs:8`, `c_source.rs:38` **не правятся** (T8).

### Правила переноса

- Функции переносятся **целиком и дословно**; тела не редактируются.
- `super::X` внутри `c_expr.rs` означал `generator::c::X` (в т. ч. `c_map`,
  `Element`, `Printer`); в подмодуле он станет `super::super::X`. **Каждое**
  вхождение `super::` пересматривается вручную — главный источник тихих ошибок
  (риск Р1).
- Сузить видимость пяти элементов, не используемых вне `c_expr`
  (`field_name_in_parent`, `resolve_variable_c_expr`,
  `resolve_simple_var_in_context`, `condition_macro_name`, `expr_precedence`) →
  `pub(in crate::generator::c::c_expr)`. **Не** правка публичного API: модуль
  приватный, вне крейта невидим.

### Сопутствующие правки

- Удалить запись `grammar/src/generator/c/c_expr.rs` из
  `scripts/module-size-baseline.txt`.
- `c_source.rs` (1086) и `c_header.rs` (1070) **остаются** в baseline — их
  деление в 0027 не входит (превышение <1,5×, порог ADR не пройден).

### Статус по функциональности (правило 11)

| Функциональность | Работа | Обоснование |
|---|---|---|
| Язык `.lam` | **н/п** | Не затрагивается; версия языка не растёт (правило 22) |
| **Генератор C** | **да (перенос)** | Порождённый код обязан быть **байт-в-байт** прежним (T3) |
| Генератор PlantUML | **н/п** | Независимый генератор, `c_expr` не потребляет |
| Публичный API крейта | **н/п** | `c_expr` — приватный `mod`, вне крейта невидим |
| Семантика | **н/п** | `c_expr` — потребитель `semantic`, не наоборот |
| Крейт `simulation` | **н/п** | Не зависит от генератора C |

## Проверки

> **Планируется (разработка не начата).** Ожидаемые результаты — из тест-плана,
> блоки 2–4.

| Проверка | Команда | Ожидаемый результат |
|---|---|---|
| T10 размер | `find grammar/src/generator/c/c_expr -name '*.rs' \| xargs wc -l \| awk '$1>1000 && $2!="total"'` | **Пусто**; файла `c_expr.rs` больше нет |
| **T3 порождённый код** | `./scripts/precheck.sh`; затем `git diff --stat examples/generated/` | **Пусто** — C для всех `examples/*.lam` байт-в-байт совпадает с эталоном T0. Ключевая проверка задачи |
| T1 число тестов | `cargo test --features lsp -- --test-threads=1` → `diff` строк `test result:` с эталоном T0 | **Пусто** (тестов внутри `c_expr.rs` нет, но `codegen_tests.rs` и `generator/c/mod.rs:228` покрывают его вывод) |
| T8 импорты | `git diff grammar/src/generator/c/c_decl.rs grammar/src/generator/c/c_model.rs grammar/src/generator/c/c_source.rs` | Правок строк `use` (6, 8, 38) **нет** |
| T2 тесты/примеры не правились | `git diff --name-only \| grep -E '^(grammar/tests\|examples)/'` | **Пусто** |
| T4 clippy | `cargo clippy --all-targets --all-features` | Diff с эталоном пуст |
| T11 по логике | Ревью | Каждый подмодуль = одна фаза печати, несёт `//!`-doc; имён `part1`/`part2` нет |
| T17 baseline | `./scripts/check-module-size.sh` | Код **0**; запись `c_expr.rs` удалена |
| Правило 5 | `cargo clean && cargo build --all-features --all-targets`; `./scripts/precheck.sh` | Успешно, включая сборку порождённого C через `cmake`/`ninja` |

**Критерии приёмки задачи:** A1, A3, A4, A5, A9, A10 анализа.

**Осторожно — известные дефекты рядом.** `c_expr.rs` соседствует с кандидатами
0028 (заглушки `//FIXME` в `c_model.rs:769`, `//TODO` в `c_model.rs:316`) и 0029
(дефектное отображение `Array`/`Bit`/`Rational` в `generator/c/mod.rs:150`).
Оба — **в других файлах** и в 0027 **не чинятся**: задача переносит код, не
исправляя его. Если при переносе обнаружится дефект в самом `c_expr.rs` — он
заводится кандидатом в `FEATURES.md` (правило 7), а не правится здесь: иначе
рушится критерий A3 «поведение не изменилось», и регрессию будет не отличить от
починки.
