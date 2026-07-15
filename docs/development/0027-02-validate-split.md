# Задача 0027-02: Разделение `semantic/validate.rs` (3648 → каталог `validate/`)

> Фича: [../features/0027-module-size-split.md](../features/0027-module-size-split.md) · ADR: [../adr/0027-module-size-split.md](../adr/0027-module-size-split.md) · анализ: [../analyze/0027-module-size-split.md](../analyze/0027-module-size-split.md) · тест-план: [../tests/0027-module-size-split.md](../tests/0027-module-size-split.md)

> **Статус: Планируется (разработка не начата).** Разделы «Что сделано» и
> «Проверки» — план; заполняются фактом по ходу выполнения.
>
> **Предусловие:** выполнена [0027-01](0027-01-module-size-split.md) (есть
> измеритель) и снят эталон «до» (тест-план, T0).

## Что было

*Реальное состояние на 2026-07-15 (ветка `v2`, коммит `6984471`); номера строк
проверены.*

`grammar/src/semantic/validate.rs` — **3648 строк** при лимите ~1000
(`CLAUDE.md:128`), превышение в **3,6 раза**. Худший модуль `src` в проекте.

### Топология файла

| Блок | Строки | Объём |
|---|---|---:|
| Doc-комментарий модуля + `use` | 1–30 | 30 |
| Продуктивный код (часть 1) | 31–1448 | 1418 |
| `#[cfg(test)] mod tests` | 1450–2420 | 971 |
| `#[cfg(test)] mod tests_ce4_declarations` | 2424–2609 | 186 |
| `#[cfg(test)] mod tests_ce15_array_size` | 2613–2692 | 80 |
| Продуктивный код (часть 2) | 2694–3648 | 955 |

Тесты — **1237 строк (33,9%)**, продуктивный код — **2403 (66,1%)**.

Ключевая особенность (усложняет работу): **тесты не в конце файла, а вклиниваются
в середину** (1450–2692), после чего продуктивный код продолжается. Из-за этого
логические группы разорваны: перечисления живут в 847–877 **и** 2947–3296.

### Логические группы (девять тем в одном файле)

| Группа | Ключевые функции (строка) | Коды диагностик | ≈строк |
|---|---|---|---:|
| A. Состояния и переходы | `model_only_one_start_state` (41), `validate_cond` (138), `validate_state_references` (270), `validate_reference` (420), `validate_conditions` (443), `check_transition_completeness` (1281), `collect_transition_completeness` (1288), `check_unreachable_states` (3423), `collect_unreachable_states` (3429), `get_state_name` (3486), `get_state_loc` (3493), `get_reachable_targets` (3500) | SE-010, SE-011, SE-012, SE-025, SE-033, SE-046 | 480 |
| B. Детерминированность (Ce14/NI4) | `check_nondeterministic_transitions` (2706), `enum Constraint` (2718), `extract_simple_constraint` (2741), `constraints_overlap` (2803), `check_nondeterministic_model` (2851) | SE-037, SE-042 | 240 |
| C. Булевость условий (Се11) | `is_boolean_ast_condition` (478), `ast_condition_summary` (544), `emit_implicit_bool_warning` (592), `is_boolean_semantic_condition` (635), `semantic_condition_summary` (666), `check_one_ref` (714), `collect_implicit_bool_warnings` (744), `check_implicit_bool_conditions` (815) | SE-037 | 340 |
| D. Константные условия | `check_constant_conditions` (3526), `collect_constant_condition_warnings` (3532), `eval_condition_const` (3564), `eval_const_value` (3590), `eval_literal_i64` (3641) | SE-047 | 130 |
| E. Типы: массивы, циклы псевдонимов | `MAX_ARRAY_SIZE` (891), `check_type_array_size` (896), `check_array_sizes` (916), `collect_type_deps` (937), `dfs_type_cycle` (956), `check_type_alias_cycles_ast` (994), `check_recursive_type_aliases` (1046) | SE-038, SE-039 | 175 |
| F. Перечисления (Ce4/NI6) | `validate_enum_type_declarations` (847), `is_valid_enum_value` (2954), `check_enum_expr` (2963), `check_enum_stmt` (3064), `check_enum_variable_value` (3120), `validate_enum_values` (3165), `collect_enum_type_safety` (3186), `check_enum_type_safety` (3292) | SE-035, SE-043 | 380 |
| G. Bit-значения переменных | `check_bit_variable_value` (90), `validate_bit_values` (123), `validate_variables` (428) | SE-035 | 60 |
| H. Порты и адреса | `validate_expression` (285), `check_port_address_completeness` (1116), `collect_incomplete_addresses` (1128), `check_port_addresses` (1184), `warn_nested_model_ports` (1235) | SE-026, SE-027, SE-048, SE-049, SE-052 | 210 |
| I. Структуры (Ce17/Ce18) | `check_duplicate_struct_fields` (3321), `check_struct_field_types` (3379) | SE-040, SE-041 | 90 |
| J. Оркестратор | `validate_model` (1062–1100) | — | 40 |

### Внешний контракт (что нельзя ломать)

Модуль объявлен `pub(crate) mod validate;` (`grammar/src/semantic/mod.rs:39`) —
**вне крейта невидим**, публичным API не является. Реэкспортов `validate` в
`semantic/mod.rs` нет. Потребители — только внутри крейта:

- `grammar/src/semantic/tree.rs:28-31` импортирует **5 имён**: `validate_model`
  (1062), `check_implicit_bool_conditions` (815), `check_transition_completeness`
  (1281), `check_type_alias_cycles_ast` (994), `warn_nested_model_ports` (1235).
- `grammar/src/lib.rs` вызывает **6 имён** по полному пути
  `semantic::validate::…`: `check_nondeterministic_transitions` (2706 →
  `lib.rs:386`), `check_port_address_completeness` (1116 → `lib.rs:403`),
  `check_recursive_type_aliases` (1046 → `lib.rs:413`), `check_enum_type_safety`
  (3292 → `lib.rs:442`), `check_unreachable_states` (3423 → `lib.rs:452`),
  `check_constant_conditions` (3526 → `lib.rs:462`).
- `grep -rn "validate::" grammar/tests` — **совпадений нет**.

Прочее, проверенное:

- **Общие хелперы** (перекрёстно используются группами A/G/H): `validate_cond`
  (138, ~20 рекурсивных самовызовов внутри 187–241), `validate_expression` (285).
- **Единственная внешняя зависимость:** группа H тянет
  `super::unused::compute_usage` (вызов на 1120).
- **`pub`, но наружу не нужны** (кандидаты на сужение видимости):
  `check_duplicate_struct_fields` (3321) и `check_struct_field_types` (3379) —
  вызываются только из `validate_model` (1080, 1085); `MAX_ARRAY_SIZE` (891) —
  вне модуля не используется.
- **`pub(crate)`, фактически внутренние:** `is_boolean_ast_condition` (478),
  `ast_condition_summary` (544), `check_type_array_size` (896).
- **Intra-doc-ссылка:** `semantic/mod.rs:122` ссылается на
  `validate::check_port_addresses` (сама функция на 1184 приватна) — при
  переносе путь меняется.

## Что сделано

> **Планируется (разработка не начата).** Ниже — план реализации.

`validate.rs` → каталог `grammar/src/semantic/validate/`. Объявление в
`semantic/mod.rs:39` (`pub(crate) mod validate;`) **не меняется** — Rust сам
подхватит каталог вместо файла.

### Целевая раскладка

| Файл | Группа | ≈строк кода | Тесты | Итого |
|---|---|---:|---:|---:|
| `mod.rs` | J: `validate_model` + `pub use` реэкспорт 11 имён + `//!`-doc | 60 | — | ~60 |
| `common.rs` | общие хелперы: `validate_cond`, `validate_expression`, `get_state_*` | 300 | — | ~300 |
| `states.rs` | A (без хелперов) | 250 | часть `mod tests` | ~400 |
| `implicit_bool.rs` | C | 340 | подсекция Се11 (1633–2244) | ~950 |
| `nondeterminism.rs` | B | 240 | — | ~240 |
| `types.rs` | E | 175 | `mod tests_ce15_array_size` (2613–2692) целиком | ~260 |
| `enums.rs` | F + G (обе про допустимые значения) | 440 | `mod tests_ce4_declarations` (2424–2609) целиком + подсекции bit (1524) и NI6 (2245+) | ~950 |
| `ports.rs` | H | 210 | — | ~210 |
| `structs.rs` | I | 90 | — | ~90 |
| `constant_conditions.rs` | D | 130 | — | ~130 |

Все файлы **≤1000 строк** (R1). Наиболее плотные — `implicit_bool.rs` и
`enums.rs` (~950): при выходе за лимит группа F делится на `enums.rs`
(объявления) и `enum_safety.rs` (NI6, `collect_enum_type_safety`).

### Порядок выноса (по возрастанию риска)

Каждый шаг — отдельный коммит с зелёными тестами (правило 3, 5):

1. `constant_conditions.rs` (D) — нулевые связи.
2. `implicit_bool.rs` (C) — изолирована, кроме пары `pub(crate)`.
3. `nondeterminism.rs` (B) — вместе с `enum Constraint`.
4. `structs.rs` (I).
5. `types.rs` (E) + `MAX_ARRAY_SIZE`.
6. `enums.rs` (F+G) — требует склейки двух разнесённых кусков (847–877 и
   2947–3296).
7. `ports.rs` (H) — тянет `super::unused` → станет `super::super::unused`.
8. `states.rs` (A) + `common.rs` — последними, они опорные.

### Правила переноса (защита от тихой регрессии)

- Функции переносятся **целиком и дословно**; тела не редактируются. Допустимы
  только: правка `use`-путей, сужение видимости, добавление `//!`-доков.
- `super::X` внутри `validate.rs` означал `semantic::X`; в подмодуле того же
  уровня он станет `super::super::X`. **Каждое** вхождение `super::`
  пересматривается вручную — это главный источник тихих ошибок (риск Р1).
- Тесты переносятся дословно, каждый к своей теме. Число тестов до и после
  обязано совпасть (T1) — иначе тест молча выпал (риск Р3).

### Сопутствующие правки

- Сузить видимость: `check_duplicate_struct_fields`, `check_struct_field_types`,
  `MAX_ARRAY_SIZE` → `pub(super)`; `is_boolean_ast_condition`,
  `ast_condition_summary`, `check_type_array_size` → `pub(super)`. **Не** правка
  публичного API: модуль `pub(crate)`, вне крейта невидим.
- Проверить intra-doc-ссылку `semantic/mod.rs:122` на
  `validate::check_port_addresses` — путь стал `validate::ports::check_port_addresses`.
  Правка одной строки ссылки допустима (не логика).
- Удалить запись `grammar/src/semantic/validate.rs` из
  `scripts/module-size-baseline.txt` (иначе 0027-01 упадёт по условию
  «незакрытая запись» — T15).

### Статус по функциональности (правило 11)

| Функциональность | Работа | Обоснование |
|---|---|---|
| Язык `.lam` | **н/п** | Грамматика/лексер/AST не правятся; версия языка не растёт (правило 22) |
| Семантические проверки | **да (перенос)** | Логика, тексты и коды диагностик (SE-010…SE-052, Ce4…Ce18) неизменны |
| Публичный API крейта | **н/п** | `validate` — `pub(crate)`, вне крейта невидим |
| Генераторы C/PlantUML | **н/п** | Не потребляют `validate` напрямую |
| Крейт `simulation` | **н/п** | Не зависит от `semantic::validate` |

## Проверки

> **Планируется (разработка не начата).** Ожидаемые результаты — из тест-плана,
> блоки 2–4.

| Проверка | Команда | Ожидаемый результат |
|---|---|---|
| T10 размер | `find grammar/src/semantic/validate -name '*.rs' \| xargs wc -l \| awk '$1>1000 && $2!="total"'` | **Пусто**; файла `validate.rs` больше нет |
| T1 число тестов | `cargo test --features lsp -- --test-threads=1` → `diff` строк `test result:` с эталоном T0 | **Пусто** — ни один тест не выпал и не добавился |
| T5 диагностики | `cargo test --test semantic_tests -- --test-threads=1` | Зелёный **без правок** `semantic_tests.rs` (4766 строк покрывают SE-0xx/Ce-xx) |
| T8 импорты | `git diff grammar/src/semantic/tree.rs grammar/src/lib.rs` | Правок строк `use` (`tree.rs:28-31`) и вызовов `semantic::validate::…` в `lib.rs` **нет** |
| T2 тесты не правились | `git diff --name-only \| grep -E '^grammar/tests/'` | **Пусто** |
| T4 clippy/doc | `cargo clippy --all-targets --all-features`; `cargo doc --no-deps` | Diff с эталоном пуст; нет новых предупреждений о битой ссылке `semantic/mod.rs:122` |
| T11 по логике | Ревью | Каждый подмодуль = одна группа таблицы «Что было», несёт `//!`-doc; имён `part1`/`part2` нет |
| T17 baseline | `./scripts/check-module-size.sh` | Код **0**; запись `validate.rs` из реестра удалена |
| Правило 5 | `cargo clean && cargo build --all-features --all-targets`; `./scripts/precheck.sh` | Успешно |

**Критерии приёмки задачи:** A1, A3, A4, A5, A9, A10 анализа.
