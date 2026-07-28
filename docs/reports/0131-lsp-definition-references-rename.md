# Отчёт о тестировании фичи 0131: LSP — `definition`, `references` и `rename`

> Фича: [../features/0131-lsp-definition-references-rename.md](../features/0131-lsp-definition-references-rename.md) · тест-план: [../tests/0131-lsp-definition-references-rename.md](../tests/0131-lsp-definition-references-rename.md) · анализ: [../analyze/0131-lsp-definition-references-rename.md](../analyze/0131-lsp-definition-references-rename.md)

## Резюме

**✅ ПРОЙДЕН.** Все 17 проверок тест-плана выполнены и зелены. Фича готова к
закрытию (`ГОТОВО`).

Окружение: macOS (darwin 25.5.0), толчейн из `rust-toolchain.toml` (стабильный
`1.97.1`). Прогоны:

| Команда | Результат |
|---|---|
| `cargo test --features lsp -- --test-threads=1` | **2375** тестов, 0 провалов |
| `cargo clippy --all-features --all-targets` | 0 предупреждений |
| `./scripts/check-module-size.sh` | `EXIT=0`, новых нарушителей нет |
| `./scripts/precheck.sh` | **`EXIT=0`** (включая гейты `c`/`c-hal`/`st` (`iec2c` 8/8), `sv` (verilator + yosys), детерминированность, примеры `book/`) |

Новых тестов фичи — **29**: 4 (`lsp_definition_tests`) + 7
(`lsp_references_tests`) + 12 (`lsp_rename_tests`) + 10 юнит-тестов слоя
(`semantic::usages::tests`) − 4 пересечения по нумерации задач (юнит-тесты слоя
считаются отдельно от интеграционных).

## Фактические результаты по проверкам

| # | Проверка | Результат | Комментарий |
|---|---|---|---|
| T1 | Объявлены `declarationProvider` и `definitionProvider` | ✅ | `capabilities::declaration_and_definition_are_both_advertised`; заодно объявлены `referencesProvider` и `renameProvider{prepareProvider: true}` |
| T2 | Прежние возможности не потеряны при выносе списка в библиотеку | ✅ | `previously_advertised_capabilities_are_kept` (hover, completion, symbols, formatting, semanticTokens, sync) |
| T3 | У `definition` и `declaration` один обработчик | ✅ | `server_handles_both_methods_in_one_branch`: ровно одна ветка с обоими методами, второго `GotoDefinition::METHOD` в файле нет |
| T4 | Переход по всем видам ссылок | ✅ | `shared_entry_resolves_every_reference_kind`: переменная в условии, имя состояния, имя модели |
| T5 | Курсор вне текста | ✅ | `cursor_beyond_text_yields_nothing`; ⚠️ курсор **на ключевом слове** `model` штатно даёт переход к модели (поведение с 0056), поэтому сторож стоит за пределами текста |
| T6 | Полнота `references` | ✅ | `usages_cover_block_and_function_bodies` (9 вхождений: объявление, инициализатор, `cond`, тело `fn`, `enter`×2, `always`×2, условие ребра) + `variable_in_ltl_formula_is_a_usage` + `references_include_block_and_function_bodies` |
| T7 | Одноимённая переменная другой модели | ✅ | `same_name_in_two_models_are_distinct_symbols`, `same_name_in_other_model_is_not_returned` |
| T8 | Затеняющая локальная переменная | ✅ | `local_variable_shadows_model_variable`, `shadowing_local_is_a_separate_symbol`: у затенённой переменной модели остаётся только объявление |
| T9 | Точность диапазонов | ✅ | подстрока текста по каждому диапазону равна имени; `prepare_returns_exact_identifier_range` отдельно фиксирует, что правка не покрывает оператор целиком |
| T10 | Полнота `rename` (главный сторож) | ✅ | `rename_is_complete_generated_c_matches_reference`: применённые правки = эталон, **и** порождённый код цели `c` совпадает побайтно |
| T11 | Отказ: объявление вне открытого документа | ✅ | `foreign_symbol_is_refused` → `RenameRefusal::ForeignDeclaration` |
| T12 | Отказ: имя модели | ✅ | `model_name_is_refused` — и на `prepareRename`, и на `rename` |
| T13 | Отказ: новое имя — не идентификатор | ✅ | `non_identifier_new_name_is_refused` (`2speed`, имя с пробелом, пустое, с дефисом) |
| T14 | Отказ: новое имя — ключевое слово | ✅ | `keyword_new_name_is_refused` (`state`, `model`, `var`, `fn`) |
| T15 | Отказ: непокрытый узел | ✅ | проверяется конструктивно: `deny(clippy::wildcard_enum_match_arm)` в `walk.rs` + `examples_corpus_is_fully_covered` (корпус разобран целиком) + `unknown_name_is_reported_unresolved` |
| T16 | Корпус не сдвинулся | ✅ | `precheck.sh`: гейт детерминированности (два прогона × все цели → `diff -r`) и гейты целей зелены — вывод генераторов не изменился |
| T17 | Предкоммит | ✅ | `./scripts/precheck.sh` → `EXIT=0` |

## Результаты по функциональности

| Функциональность | Статус | Комментарий |
|---|---|---|
| LSP `definition` (0131-01) | ✅ | новая возможность; общий обработчик с `declaration` |
| LSP `declaration` (0056, регресс) | ✅ | поведение не изменилось: та же функция, те же тесты 0056 зелены |
| LSP `references` (0131-02) | ✅ | новая возможность поверх слоя `semantic::usages` |
| LSP `rename` (0131-03) | ✅ | новая возможность; принцип «полнота или отказ» |
| Прочие возможности LSP | ✅ | список возможностей вынесен в библиотеку без потерь (T2) |
| Компилятор `taktc` (все цели) | ✅ | ядро не тронуто; вывод байт-в-байт прежний (T16) |
| Симулятор `takt-sim` | ✅ | не затронут; сверки конформности зелены |

## Отклонения от анализа (зафиксированы, не дефекты)

1. **Разрешение имён — собственное, а не через `search_*`.** Анализ
   предполагал переиспользовать поиск семантики; на деле её `search_*` отдают
   узел, чей `loc` покрывает **весь оператор**, а слою нужна позиция имени.
   Согласие со семантикой обеспечено не общим кодом, а тестами на поведение
   (затенение, вложенность, соседние модели).
2. **Слою не нужен `ModelNode`** — он работает по чистому АСД. Побочная польза:
   вхождения находятся и в тексте, который семантика отвергла.
3. **Реестр членов модели** добавлен сверх плана — ради формы `S(Ping) = End`,
   адресующей состояние соседней модели.

## Выводы и дальнейшие шаги

- Главный риск фичи (P3 анализа — частичное переименование) закрыт сторожем,
  который сверяет **порождённый код**, а не факт компиляции: при затенении
  испорченный файл продолжает компилироваться, и проверка «собралось» дефект бы
  пропустила.
- Исправлений (`docs/fixes/0131-YY-*`) не потребовалось.
- Кандидаты, вынесенные из фичи (в блок 2 `FEATURES.md`): индексация рабочей
  области (снимет ограничение `rename` одним документом и даст `references` по
  всем файлам) и снятие дубля rename в плагине IntelliJ (фича 0125 делает его
  собственным PSI).
