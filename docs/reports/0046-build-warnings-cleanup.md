# Отчёт о тестировании — Фича 0046: Устранение всех предупреждений сборки

- **Фича:** [0046](../features/0046-build-warnings-cleanup.md)
- **ADR:** [0046](../adr/0046-build-warnings-cleanup.md) · **Анализ:** [0046](../analyze/0046-build-warnings-cleanup.md) · **Тест-план:** [0046](../tests/0046-build-warnings-cleanup.md)
- **Задачи:** 0046-01 (ужатие `Location`), 0046-02 (механическая чистка), 0046-03 (закрепление)
- **Дата:** 2026-07-19
- **Вердикт:** ✅ **ГОТОВО**. `./scripts/precheck.sh` — EXIT=0; rustc 0 / clippy 0; вывод генераторов байт-в-байт неизменен; ноль-долг закреплён `-D warnings`.

## Сводка

Предупреждения сборки обоих крейтов сведены к нулю и закреплены `-D warnings`
на шаге clippy в `precheck.sh` и CI (Option B ADR). Главная находка разработки:
свежий инвентарь дал **549** clippy (снимок карточки 2026-07-15 — ~106), из них
**414 — один класс `result_large_err`** (стал `#[warn]` по умолчанию в clippy
0.1.99). Разрешено ужатием `Diagnostic` ниже порога 128 байт через `Location`
(решение заказчика).

## Эталон «до» → «после»

| Инструмент | До (свежий, 2026-07-19) | После |
|---|---|---|
| rustc (`build --all-targets --all-features`) | 3 (`unused_imports`, `missing_docs`, `dead_code`) | **0** |
| clippy (`--all-targets --all-features`) | **549** (из них 414 `result_large_err`) | **0** |

⚠️ Инвентарь карточки (2026-07-15: 19 rustc + ~106 clippy) **устарел**: 0036
убрала 11 `private_interfaces`, а clippy 0.1.99 добавил `result_large_err` (414).

## Что сделано

- **0046-01 — `Location::Source(u64,usize,usize) → (u32,u32,u32)`.** Вариант 16
  байт вместо 32 → `Diagnostic` 136 → **120** байт → все **414**
  `result_large_err` исчезли **без** `#[allow]` и **без** правки сигнатур
  `Result`. Публичный API методов `Location` остался в `usize`/`String`: каст
  локализован в аксессорах (`start`/`end`/`range`/`try_*`) и в новом
  хелпере-конструкторе `Location::source(u64, usize, usize)`, которым заменены
  **160** конструирований в `grammar.lalrpop` и лексере (sed); разбор
  (`index.rs`, `docs`, `comments`, `lsp/*`) — `as usize` у места использования.
- **0046-02 — механическая чистка остатка (~135).** `cargo clippy --fix`
  (needless_ref 24, collapsible_if 18, clone_on_copy 16, needless_borrow 6, …) +
  ручные: `too_many_arguments` (7 → `#[allow]` с обоснованием — печатники
  генератора, `SimulationRunner::new`), `large_enum_variant` (`UnitKind` →
  `#[allow]`: `Node` — доминирующий вариант), `type_complexity`
  (`Predicate.func`), `upper_case_acronyms` (`Viewport::SVG`),
  `doc_lazy_continuation` (×3 — `//` между doc-блоками сливал список и прозу,
  разделены пустым `///`), `redundant_guards` (`if b == 0.0` → `#[allow]`: паттерн
  `0.0` дал бы `illegal_floating_point_literal_pattern`),
  `field_reassign_with_default` (struct-update), `assertions_on_constants`
  (`assert!(false, …)` → `panic!`), `needless_range_loop` (→ `enumerate`),
  `get(k).is_none()` → `!contains_key(k)`. rustc: `unused_imports` (clippy --fix),
  `missing_docs` (`collect_diagnostics` — добавлен doc), `dead_code`
  (мёртвое поле `Item.end` — удалено).
- **0046-03 — закрепление.** `cargo clippy --all-targets --all-features -- -D
  warnings` в `precheck.sh` и новый шаг CI. Clippy гоняет и clippy-, и
  rustc-линты — один флаг покрывает оба набора. CLI-уровень (**не** запрещённый
  `#![deny(warnings)]` в коде): обновление компилятора ломает `precheck`/CI, а не
  сборку у пользователя.

## Сверка с тест-планом

| # | Проверка | Результат |
|---|---|---|
| T1 | 0 rustc | ✅ 0 (без мёртвого `src/grammar.rs`) |
| T2 | 0 clippy (`-D warnings`) | ✅ EXIT=0, «No issues found» |
| T3 | Codegen байт-в-байт | ✅ `git diff examples/generated` — **пусто** |
| T4 | Поведение (тесты + conformance) | ✅ `precheck` тесты зелёные |
| T5 | Инвариант 0025 (`wildcard_enum_match_arm`) | ✅ не тронут |
| T6/T7 | Закрепление `-D warnings` | ✅ в `precheck.sh` и CI |
| T8/T9 | `result_large_err` = 0, `Diagnostic` < 128 | ✅ 414 → 0 |
| T10 | Позиции целы (LSP/диагностики) | ✅ `cargo test -p grammar --all-features` зелёные |
| T12 | precheck | ✅ EXIT=0 |

## Находки и отклонения

- **Ловушка кэша clippy.** `cargo clippy 2>&1 | grep -c warning` дал ложный
  **0** — при отсутствии перекомпиляции clippy не переэмитит предупреждения.
  Истинный остаток (25 не-машинных) виден лишь при `-D warnings` (промотирует в
  ошибки) или после `touch`. Урок: считать предупреждения только на свежей
  компиляции.
- **`cargo build --all-targets` НЕ компилирует inline `#[cfg(test)]` юниты lib.**
  8 «ошибок» первого precheck — clippy `-D warnings` на тестовом коде, невидимом
  для `build --all-targets`. Полный охват даёт только `cargo test`/clippy.
- **Мёртвый коммитнутый `grammar/src/grammar.rs`** (~29k строк) не компилируется
  (сборка из `OUT_DIR`) — вне объёма 0046, кандидат на удаление (в ADR).
- **База карточки устарела за 4 дня** (0036 + clippy 0.1.99) — инвентарь
  пересчитан по факту (урок 0036 повторился).

## Дефекты

Не найдено. Фиксы (`docs/fixes/0046-YY-*`) не заводились.

## Итог

Критерии A1–A4 и требования R1–R4 выполнены. Язык **не менялся** (правило 18/22):
`Location` — внутренняя структура диагностик, не конструкция языка. Крейт
`grammar` — минорный бамп **0.6.0 → 0.7.0**: смена типов полей публичного
`Location::Source(u64,usize,usize) → (u32,u32,u32)` — ломающее изменение API по
SemVer 0.x (внешний код, конструирующий/разбирающий вариант, требует правки; в
репозитории — уже поправлен). Фича закрыта.
