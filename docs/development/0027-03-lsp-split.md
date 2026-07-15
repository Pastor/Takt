# Задача 0027-03: Разделение `lsp.rs` (2163 → каталог `lsp/`)

> Фича: [../features/0027-module-size-split.md](../features/0027-module-size-split.md) · ADR: [../adr/0027-module-size-split.md](../adr/0027-module-size-split.md) · анализ: [../analyze/0027-module-size-split.md](../analyze/0027-module-size-split.md) · тест-план: [../tests/0027-module-size-split.md](../tests/0027-module-size-split.md)

> **Статус: Планируется (разработка не начата).** Разделы «Что сделано» и
> «Проверки» — план; заполняются фактом по ходу выполнения.
>
> **Предусловие:** выполнена [0027-01](0027-01-module-size-split.md) и снят
> эталон «до» (тест-план, T0).
>
> ⚠️ **Единственная задача фичи, задевающая публичный API крейта** (правило 11).

## Что было

*Реальное состояние на 2026-07-15 (ветка `v2`, коммит `6984471`); номера строк
проверены.*

`grammar/src/lsp.rs` — **2163 строки** при лимите ~1000 (`CLAUDE.md:128`),
превышение в **2,2 раза**.

**Расхождение с текстом кандидата:** в `FEATURES.md` указано «`lsp.rs` (2134)» —
цифра устарела, фактически **2163** (+29 строк).

### Топология файла

| Блок | Строки | Объём |
|---|---|---:|
| Продуктивный код | 1–1582 | 1582 (73%) |
| `#[cfg(test)] mod tests` | 1583–2163 | 581 (27%), 37 тестов |

В отличие от `validate.rs`, тесты — **одним модулем в конце**, продуктивный код
не разорван. Общая фикстура тестов — `VALID_SRC` (1592+).

### Логические группы

| Группа | Строки | Объём | Содержимое |
|---|---|---:|---|
| Шапка + импорты | 1–14 | 14 | `//!`-доки, `use lsp_types::*`, `use crate::semantic` |
| Легенда семантических токенов | 15–39 | 25 | `SEMANTIC_TOKEN_TYPES` (17), `TT_KEYWORD`…`TT_CLASS` (30–39) |
| Словари completion | 41–122 | 82 | `BUT_KEYWORDS` (42–82), `BUT_BUILTIN_TYPES` (88–122) |
| Formatting | 123–156 | 34 | `formatting_edits` (146) |
| Диагностика | 157–252 | 96 | `collect_diagnostics` (157), `grammar_diagnostic_to_lsp` (209) |
| Position/offset-утилиты | 253–434 | 182 | `offset_to_range` (253), `offset_to_position` (291), `utf16_to_byte_offset` (327), `position_to_offset` (368) |
| Goto declaration | 435–682 | 248 | `node_at_position` (435), `struct Location` (451), `goto_declaration` (474), `goto_declaration_with_paths` (508), `to_snake_case` (582), `find_declaration_in_index` (597), `declaration_range_of` (633) |
| Completion | 683–850 | 168 | `completion_items` (683) |
| Hover | 851–1173 | 323 | `word_at_position` (851), `hover_info` (893) |
| Document symbols | 1174–1411 | 238 | `document_symbols` (1174), `loc_to_range` (1183), `make_sym` (1196), `symbols_from_model` (1215) |
| Семантические токены | 1412–1582 | 171 | `semantic_tokens` (1412): лексинг (1423–1521), комментарии (1522), сортировка (1532), дельта-кодирование UTF-16 (1535–1582) |

Транспортной логики LSP в модуле **нет** — `lsp-server`, `main_loop`,
`ServerState` живут в бинарнике `grammar/src/bin/lam_lsp.rs` (280 строк).
`lsp.rs` — чистая библиотека возможностей.

### Внешний контракт — публичный API крейта

Модуль объявлен публично (`grammar/src/lib.rs:46-48`):

```rust
/// Вспомогательные функции LSP-сервера (только при флаге `lsp`).
#[cfg(feature = "lsp")]
pub mod lsp;
```

Реэкспортов в `lib.rs` нет — потребители ходят по полным путям
`grammar::lsp::X`. **16 публичных элементов** (`pub(crate)` в файле нет):

| Строка | Элемент |
|---:|---|
| 17 | `pub const SEMANTIC_TOKEN_TYPES: &[SemanticTokenType]` |
| 146 | `formatting_edits(&str) -> Result<Option<Vec<TextEdit>>, crate::format::FormatError>` |
| 157 | `collect_diagnostics(&str) -> Vec<Diagnostic>` |
| 209 | `grammar_diagnostic_to_lsp(&crate::diagnostics::Diagnostic, &str) -> Diagnostic` |
| 253 | `offset_to_range(&str, usize, usize) -> Range` |
| 291 | `offset_to_position(&str, usize) -> Position` |
| 368 | `position_to_offset(&str, Position) -> Option<usize>` |
| 435 | `node_at_position(&str, Position, &Rc<RefCell<ModelNode>>) -> Option<SemanticNodeRef>` |
| 451 | `pub struct Location { pub uri: String, pub range: Range }` |
| 474 | `goto_declaration(&str, Position) -> Option<Range>` |
| 508 | `goto_declaration_with_paths(&str, Position, &[String]) -> Option<Location>` |
| 683 | `completion_items(&str) -> Vec<CompletionItem>` |
| 851 | `word_at_position(&str, Position) -> Option<String>` |
| 893 | `hover_info(&str, Position) -> Option<Hover>` |
| 1174 | `document_symbols(&str) -> Vec<DocumentSymbol>` |
| 1412 | `semantic_tokens(&str) -> SemanticTokens` |

**Потребители:**

- `grammar/src/bin/lam_lsp.rs` — 8 имён: `SEMANTIC_TOKEN_TYPES` (54),
  `completion_items` (137), `formatting_edits` (151), `hover_info` (168),
  `goto_declaration` (179), `document_symbols` (194), `semantic_tokens` (204),
  `collect_diagnostics` (233, 243).
- `grammar/tests/lsp_tests.rs` (1187 строк) — 11 имён: импорт на 11–14
  (`completion_items`, `goto_declaration`, `hover_info`, `node_at_position`,
  `position_to_offset`, `semantic_tokens`), `goto_declaration_with_paths` (602,
  622, 636, 646, 788), импорт на 810–813 (`collect_diagnostics`,
  `grammar_diagnostic_to_lsp`), `formatting_edits` (1135, 1149, 1166, 1177).
- Внутри `grammar/src` обращений к `lsp::` **нет** — модуль листовой.

**Наружу не используются вообще** (но остаются публичными — снаружи крейта их
может звать кто угодно, поэтому убирать нельзя): `offset_to_range`,
`offset_to_position`, `word_at_position`, `Location` (только как тип возврата
`goto_declaration_with_paths`).

## Что сделано

> **Планируется (разработка не начата).** Ниже — план реализации.

`lsp.rs` → каталог `grammar/src/lsp/`. Объявление в `lib.rs:46-48` **не
меняется** (включая `#[cfg(feature = "lsp")]`).

### Целевая раскладка

| Файл | Группа | ≈строк кода | Тесты | Итого |
|---|---|---:|---:|---:|
| `mod.rs` | `//!`-doc + **`pub use` всех 16 имён** | 40 | — | ~40 |
| `position.rs` | position/offset (253–434) | 182 | тесты `position_to_offset` | ~280 |
| `diagnostics.rs` | диагностика (157–252) | 96 | тесты `collect_diagnostics` | ~180 |
| `formatting.rs` | formatting (123–156) | 34 | тесты `formatting_edits` | ~110 |
| `completion.rs` | словари (41–122) + `completion_items` (683–850) | 250 | тесты completion | ~330 |
| `goto.rs` | goto declaration (435–682) | 248 | тесты goto | ~350 |
| `hover.rs` | hover (851–1173) | 323 | тесты hover | ~420 |
| `symbols.rs` | document symbols (1174–1411) | 238 | тесты symbols | ~290 |
| `semantic_tokens.rs` | легенда (15–39) + `semantic_tokens` (1412–1582) | 196 | тесты токенов | ~270 |

Все файлы **≤1000 строк** (R1). `position.rs` — общий для `diagnostics`,
`goto`, `symbols`, `formatting`; его элементы остаются `pub` (они уже часть
публичного API) и реэкспортируются из `mod.rs`.

### Главное требование: `mod.rs` реэкспортирует все 16 имён

```rust
pub use self::position::{offset_to_position, offset_to_range, position_to_offset};
pub use self::goto::{goto_declaration, goto_declaration_with_paths, node_at_position, Location};
pub use self::semantic_tokens::{semantic_tokens, SEMANTIC_TOKEN_TYPES};
// … и т. д. — ровно 16 имён из таблицы «Что было»
```

Забытое имя даёт **ошибку компиляции** неизменённого `lam_lsp.rs` или
`lsp_tests.rs` — то есть контракт защищён конструктивно, а не ревью (T6).

### Правила переноса

- `lam_lsp.rs` и `lsp_tests.rs` **править запрещено** — их успешная компиляция
  и есть доказательство сохранности публичного API (T2, T6).
- `SEMANTIC_TOKEN_TYPES` (17) переносится **с сохранением порядка элементов**:
  кодирование токенов завязано на индекс в легенде, перестановка сломала бы
  клиента молча (T7).
- Приватные хелперы едут строго за своей группой: `utf16_to_byte_offset` (327)
  → `position.rs`; `to_snake_case` (582), `find_declaration_in_index` (597),
  `declaration_range_of` (633) → `goto.rs`; `loc_to_range` (1183), `make_sym`
  (1196), `symbols_from_model` (1215) → `symbols.rs`; `TT_*` (30–39) →
  `semantic_tokens.rs`; `BUT_KEYWORDS`/`BUT_BUILTIN_TYPES` (42–122) →
  `completion.rs`.
- Единый `mod tests` (1583–2163, 37 тестов) режется по темам; общая фикстура
  `VALID_SRC` (1592) **дублируется** в тех подмодулях, где нужна (дублирование
  константы-фикстуры дешевле нового общего тест-модуля), либо выносится в
  `mod.rs` под `#[cfg(test)] pub(super) const`. Число тестов до и после обязано
  совпасть (T1).
- Функции переносятся дословно; тела не редактируются.

### Сопутствующие правки

- Удалить запись `grammar/src/lsp.rs` из `scripts/module-size-baseline.txt`.
- `grammar/tests/lsp_tests.rs` (1187) **остаётся** в baseline — его деление в
  0027 не входит.

### Статус по функциональности (правило 11)

| Функциональность | Работа | Обоснование |
|---|---|---|
| Язык `.lam` | **н/п** | Не затрагивается; версия языка не растёт (правило 22) |
| **Публичный API крейта `grammar`** | **да (сохраняется)** | Все 16 путей `grammar::lsp::X` сохраняются реэкспортом. API **не ломается** — в отличие от 0018, где слом был осознанным |
| LSP-сервер `lam-lsp` | **да (перенос)** | Поведение неизменно; бинарник не правится |
| Плагин IntelliJ (0038/0039) | **н/п** | Потребляет `lam-lsp` по протоколу LSP, а не Rust-API |
| Семантика/генераторы | **н/п** | `lsp.rs` — листовой модуль, внутри `grammar/src` его никто не зовёт |

## Проверки

> **Планируется (разработка не начата).** Ожидаемые результаты — из тест-плана,
> блоки 2–4.

| Проверка | Команда | Ожидаемый результат |
|---|---|---|
| T10 размер | `find grammar/src/lsp -name '*.rs' \| xargs wc -l \| awk '$1>1000 && $2!="total"'` | **Пусто**; файла `lsp.rs` больше нет |
| **T6 публичный API** | `cargo build --features lsp --bin lam-lsp`; `cargo test --features lsp --test lsp_tests -- --test-threads=1` | Проходят при **неизменённых** `lam_lsp.rs` и `lsp_tests.rs`. Ошибка компиляции = забытый `pub use` |
| T7 порядок легенды | Тесты `semantic_tokens` в `lsp_tests.rs` | Зелёные — порядок `SEMANTIC_TOKEN_TYPES` сохранён |
| T1 число тестов | `cargo test --features lsp -- --test-threads=1` → `diff` строк `test result:` с эталоном T0 | **Пусто** — все 37 тестов `lsp.rs` на месте |
| T2 потребители не правились | `git diff --name-only \| grep -E '^(grammar/tests\|grammar/src/bin)/'` | **Пусто** |
| T4 clippy/doc | `cargo clippy --all-targets --all-features`; `cargo doc --no-deps --features lsp` | Diff с эталоном пуст; новых предупреждений нет |
| T11 по логике | Ревью | Каждый подмодуль = одна возможность LSP, несёт `//!`-doc; имён `part1`/`part2` нет |
| T17 baseline | `./scripts/check-module-size.sh` | Код **0**; запись `lsp.rs` удалена |
| Правило 5 | `cargo clean && cargo build --all-features --all-targets`; `./scripts/precheck.sh` | Успешно (в `precheck.sh` уже есть `cargo build --features lsp --bin lam-lsp`) |

**Критерии приёмки задачи:** A1, A2, A3, A4, A5, A9 анализа.

**Особое внимание:** фича сборки. Тесты `lsp_tests.rs` и сам модуль — под
`#[cfg(feature = "lsp")]`; прогон **без** `--features lsp` их не соберёт и
регрессию не покажет. Все проверки этой задачи — только с `--features lsp`
(ср. коммит `6984471`: «тесты LSP-форматирования не собирались без
`--features lsp`» — эта грабля уже стреляла).
