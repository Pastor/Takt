# Реестр разработки

Стадия 4 жизненного цикла (правило 17 в [../RULE.md](../RULE.md)). Подзадачи
разработки `XXXX-YY-slug.md` (нумерация `YY` внутри фичи). Крупные фичи
декомпозируются на несколько подзадач.

Заготовка создаётся из шаблона [`../templates/development.md`](../templates/development.md).

| Задача | Фича | Заголовок | Документ |
|--------|------|-----------|----------|
| 0018-01 | 0018 | Срезовые типы, `#[non_exhaustive]`, чистка мёртвого кода | [0018-01-slice-types-and-api.md](0018-01-slice-types-and-api.md) |
| 0018-02 | 0018 | Опции генератора (`GenerateOptions`); Builder — не требуется | [0018-02-generate-options.md](0018-02-generate-options.md) |
| 0018-03 | 0018 | `with_capacity` в отступах, `#[non_exhaustive]` на AST/IR, аудит конструкторов | [0018-03-with-capacity-nonexhaustive-ast.md](0018-03-with-capacity-nonexhaustive-ast.md) |
| 0018-04 | 0018 | Компилируемые doctests (P12); возврат владения при Err (P09) | [0018-04-doctests-and-ownership.md](0018-04-doctests-and-ownership.md) |
| 0018-05 | 0018 | Аудит `.clone()` (P08); `mem::take` (P10) | [0018-05-clone-audit.md](0018-05-clone-audit.md) |
| 0019-01 | 0019 | Устранение дубликата LoopCond параметризацией Expression (ВЫПОЛНЕНО) | [0019-01-loopcond-dedup.md](0019-01-loopcond-dedup.md) |
| 0025-01 | 0025 | Ядро вычисления `eval`: значения, операции, приведение типов (ВЫПОЛНЕНО) | [0025-01-eval-core.md](0025-01-eval-core.md) |
| 0025-02a | 0025 | Адаптер выражений `ExpressionNode` — Д1/Д2, старый вычислитель удалён (ВЫПОЛНЕНО) | [0025-02a-expression-adapter.md](0025-02a-expression-adapter.md) |
| 0025-02b-1 | 0025 | Интерпретатор операторов: локальные `var`, `while`/`loop`, `for`, `match` (ВЫПОЛНЕНО) | [0025-02b-1-statement-interpreter.md](0025-02b-1-statement-interpreter.md) |
| 0025-02b-2 | 0025 | Поток управления (`Flow`) и вызовы функций — Д3/Д4, критерий A7 (ВЫПОЛНЕНО) | [0025-02b-2-function-calls.md](0025-02b-2-function-calls.md) |
| 0025-04 | 0025 | `enter` стартового состояния — Д5 (ВЫПОЛНЕНО) | [0025-04-initial-enter.md](0025-04-initial-enter.md) |
| 0025-05 | 0025 | Канал диагностики: `TickResult::Failed` → `RunResult::EvalFailed` → код возврата (R5) (ВЫПОЛНЕНО) | [0025-05-diagnostic-channel.md](0025-05-diagnostic-channel.md) |
| 0025-06 | 0025 | Фикстуры `tests/data/eval/` + интеграционный слой `tests/eval_tests.rs` (ВЫПОЛНЕНО) | [0025-06-fixtures-integration-tests.md](0025-06-fixtures-integration-tests.md) |
| 0025-07 | 0025 | Автоматическая сверка с `lamc -t c` (критерий A8) (ВЫПОЛНЕНО) | [0025-07-c-conformance-test.md](0025-07-c-conformance-test.md) |
| 0025-08 | 0025 | Перенос примеров и контрпримеров в README (правила 15, 16) (ВЫПОЛНЕНО) | [0025-08-readme-examples.md](0025-08-readme-examples.md) |
| 0025-03 | 0025 | Адаптер условий `ConditionNode`, переписывание `flat` — Д6/Д7/Д8, паники сняты (ВЫПОЛНЕНО) | [0025-03-condition-adapter.md](0025-03-condition-adapter.md) |
| 0020-01 | 0020 | Грамматика и AST оператора `address` (ВЫПОЛНЕНО) | [0020-01-address-grammar.md](0020-01-address-grammar.md) |
| 0020-02 | 0020 | Семантика: привязка `address` + диагностики SE-048/049 (ВЫПОЛНЕНО) | [0020-02-semantics-diagnostics.md](0020-02-semantics-diagnostics.md) |
| 0020-03 | 0020 | Внешняя `.ld`-карта: парсер + `--address-map` + оверлей SE-050/051 (ВЫПОЛНЕНО) | [0020-03-external-map.md](0020-03-external-map.md) |
| 0020-04 | 0020 | Полнота адресов по достижимости (SE-052, опора на `unused.rs`) (ВЫПОЛНЕНО) | [0020-04-completeness.md](0020-04-completeness.md) |
| 0020-05 | 0020 | Потребление адреса в C: цель `c-hal` (таблица + дефолтный HAL) (ВЫПОЛНЕНО) | [0020-05-c-consumption.md](0020-05-c-consumption.md) |
| 0021-01 | 0021 | Лексер + грамматика: `:=` присваивание, `=` равенство (ЗАПЛАНИРОВАНО) | [0021-01-lexer-grammar.md](0021-01-lexer-grammar.md) |
| 0021-02 | 0021 | Семантика/C-генератор/LSP под новую семантику (ЗАПЛАНИРОВАНО) | [0021-02-semantics-codegen-lsp.md](0021-02-semantics-codegen-lsp.md) |
| 0021-03 | 0021 | Мигратор `.lam` + миграция фикстур/примеров (ЗАПЛАНИРОВАНО) | [0021-03-migrator.md](0021-03-migrator.md) |
| 0021-04 | 0021 | Документация и повышение версии языка (ЗАПЛАНИРОВАНО) | [0021-04-docs-version.md](0021-04-docs-version.md) |
| 0022-01 | 0022 | Каркас плагина: Gradle, FileType, регистрация `.lam` (**ВЫПОЛНЕНО**) | [0022-01-plugin-skeleton.md](0022-01-plugin-skeleton.md) |
| 0022-02 | 0022 | Лексер (`LexerBase`) + SyntaxHighlighter (**ВЫПОЛНЕНО**) | [0022-02-lexer-highlighter.md](0022-02-lexer-highlighter.md) |
| 0022-03 | 0022 | ColorSettingsPage, commenter, brace matcher, доки (**ВЫПОЛНЕНО**) | [0022-03-color-settings-docs.md](0022-03-color-settings-docs.md) |
| 0023-01 | 0023 | Плагин IntelliJ IDEA — навигация к декларации и include | [0023-01-intellij-navigation-include.md](0023-01-intellij-navigation-include.md) |

