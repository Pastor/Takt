# Тест-план фичи 0221: Панель структуры: инвариант состояния символом не становится

> Фича: [../features/0221-lsp-state-invariant-symbol.md](../features/0221-lsp-state-invariant-symbol.md) · ADR: [../adr/0221-lsp-state-invariant-symbol.md](../adr/0221-lsp-state-invariant-symbol.md) · анализ: [../analyze/0221-lsp-state-invariant-symbol.md](../analyze/0221-lsp-state-invariant-symbol.md)

## Область и цель

Фича меняет **состав ответа `documentSymbol`**: инвариант состояния становится
символом. Проверки закрывают три вопроса:

1. **новое поведение** — символ есть, вид верный, диапазоны по контракту 0147
   (R1–R3);
2. **ничего лишнего** — состав панели больше не изменился: безымянные и
   ссылающиеся элементы символами не стали (R4);
3. **сторож на будущее** — разбор `StateElement` исчерпывающий, пропуск нового
   вида останавливает сборку (R5).

⚠️ Тесты LSP живут под `#[cfg(feature = "lsp")]`: обычная `cargo test` их не
видит, гоняет `cargo test --all-features` в предкоммите.

## Проверки (условие → ожидаемый результат)

| # | Проверка | Предусловие | Ожидаемый результат | Ссылка на R/A |
|---|---|---|---|---|
| T1 | `invariant_is_a_symbol_at_both_levels` | правка внесена | `Sane` — ребёнок `Idle`, вид `CONSTANT`; `Safe` — символ верхнего уровня того же вида | R1, R3 / A1, A3 |
| T2 | `state_invariant_ranges_follow_the_contract` | то же | `selection_range` = `Sane`; `range` начинается с `invariant` и содержит `selection_range` | R2 / A2 |
| T3 | `nameless_and_referencing_state_elements_are_not_symbols` | вход с `every`, `: [Guard] …;`, `ref` | дети состояния — ровно `["enter", "Sane"]` | R4 / A4 |
| T4 | `state_owns_its_named_blocks` (обновлён) | правка внесена | дети `Idle` = `["enter", "always", "Sane"]`, блоки — `EVENT` | R1, R4 |
| T5 | Мутация: лишний вариант в `ast::StateElement` | временная правка АСД | сборка падает `E0004` с указанием на `lsp/symbols.rs`; после снятия мутации собирается | R5 / A5 |
| T6 | `grep` по тестам | правка внесена | теста `state_level_invariant_is_not_a_symbol_yet` нет | R6 / A6 |
| T7 | Чтение кода новой ветви | то же | имя берётся через `?` (`Option<Identifier>`), `unwrap` отсутствует | R7 / A7 |
| T8 | `./scripts/precheck.sh` | то же | код 0, включая `cargo test --all-features` | A8 |
| T9 | `git status --porcelain examples/generated/` после T8 | precheck прогнан | пусто — генерация не затронута | обратная функциональность |

## Разбивка проверок по функциональности

Единые условия и ожидаемые результаты прогоняются против каждой задеваемой
обратной функциональности (правило 11); фиксируется статус.

| Функциональность | Статус | Замечание |
|---|---|---|
| `documentSymbol` (панель структуры) | ✅ | T1–T4 — предмет фичи |
| Прочие возможности LSP (hover, goto, rename, диагностики) | ✅ | не затронуты; полный прогон `--all-features` в T8 |
| Ядро (`takt-lang` без фичи `lsp`) | ✅ | правка внутри `src/lsp/`; T8, T9 |
| Генерация целей и симулятор | — | не затронуты; подтверждается T9 |
| Плагин IntelliJ | — | панель строит LSP4IJ по ответу сервера; своего сбора символов плагин не ведёт |
| Плагин Zed | — | списков не ведёт (`language_servers = ["takt-lsp"]`) |
| Документ `book/` (правило 24) | — | не требуется: состав панели редактора — свойство инструментария |

<!-- Легенда: ✅ пройдено · ❌ провалено · ⬜ не проверялось · — не применимо -->

## Тестовые данные и окружение

- Тесты: `takt-lang/tests/lsp/lsp_document_symbol_tests.rs` (константа `SRC`
  уже содержит `invariant Sane` внутри `start Idle`; для T3 заведён свой вход
  с `every`, формулой и `ref`).
- Модуль под правкой: `takt-lang/src/lsp/symbols.rs`.
- Окружение: толчейн Rust `1.97.1`, `cargo test --all-features`, обычный набор
  инструментов `precheck.sh`.
