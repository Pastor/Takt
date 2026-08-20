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
| 0024-01 | 0024 | Ядро печати АСД (`format_source`) (ВЫПОЛНЕНО) | [0024-01-format-core.md](0024-01-format-core.md) |
| 0024-02 | 0024 | Печать комментариев (переассоциация по `Location`) — гейт по корпусу включён (ВЫПОЛНЕНО) | [0024-02-comments.md](0024-02-comments.md) |
| 0024-03 | 0024 | Подкоманда `lamc fmt` (`--check`/`--stdin`, обход каталогов) (ВЫПОЛНЕНО) | [0024-03-lamc-fmt.md](0024-03-lamc-fmt.md) |
| 0024-04 | 0024 | LSP `textDocument/formatting` (ВЫПОЛНЕНО); часть про IntelliJ заблокирована плоским PSI | [0024-04-lsp-formatting.md](0024-04-lsp-formatting.md) |
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
| 0026-01 | 0026 | Безусловная эмиссия typedef корня в генераторе C | [0026-01-c-root-typedef.md](0026-01-c-root-typedef.md) |
| 0027-01 | 0027 | Механическая проверка размера модулей (храповик) | [0027-01-module-size-split.md](0027-01-module-size-split.md) |
| 0027-02 | 0027 | Разделение `semantic/validate.rs` (3648 → каталог `validate/`) | [0027-02-validate-split.md](0027-02-validate-split.md) |
| 0027-03 | 0027 | Разделение `lsp.rs` (2163 → каталог `lsp/`) | [0027-03-lsp-split.md](0027-03-lsp-split.md) |
| 0027-04 | 0027 | Разделение `generator/c/c_expr.rs` (1736 → каталог `c_expr/`) | [0027-04-c-expr-split.md](0027-04-c-expr-split.md) |
| 0028-01 | 0028 | Диагностика CC-014 вместо тихого пропуска условного перехода | [0028-01-c-generator-stubs.md](0028-01-c-generator-stubs.md) |
| 0028-02 | 0028 | Удаление недостижимой ветки и исчерпывающее сопоставление | [0028-02-dead-branch-cleanup.md](0028-02-dead-branch-cleanup.md) |
| 0028-03 | 0028 | Фикстуры, тесты и документация диагностики CC-014 | [0028-03-fixtures-tests.md](0028-03-fixtures-tests.md) |
| 0029-01 | 0029 | Отображение `Array` — бит-вектор против настоящего массива | [0029-01-c-type-mapping.md](0029-01-c-type-mapping.md) |
| 0029-02 | 0029 | Отображение `Bit` → `uint8_t` и согласование ширины `c-hal` | [0029-02-c-type-bit.md](0029-02-c-type-bit.md) |
| 0029-03 | 0029 | Отображение `Rational` → `double` и опция `--float-width` | [0029-03-c-type-rational.md](0029-03-c-type-rational.md) |
| 0029-04 | 0029 | Расширение сверки с симулятором и примеры в документации | [0029-04-conformance-docs.md](0029-04-conformance-docs.md) |
| 0029-05 | 0029 | Инициализация массива в `_init` (заведена по ходу разработки) | [0029-05-array-initializer.md](0029-05-array-initializer.md) |
| 0047-01 | 0047 | Трансляция `S(Модель) = Состояние` в цель `c` | [0047-01-state-of-model.md](0047-01-state-of-model.md) |
| 0030-01 | 0030 | Починка модели comprehensive.lam + приёмочный тест сценария | [0030-01-comprehensive-example-fix.md](0030-01-comprehensive-example-fix.md) |
| 0030-02 | 0030 | Корпусной гейт достижимости заявленных сценариев для examples/ | [0030-02-examples-scenario-gate.md](0030-02-examples-scenario-gate.md) |
| 0031-01 | 0031 | Семантика — разрешение вызовов функций из тел функций | [0031-01-fn-calls-fn.md](0031-01-fn-calls-fn.md) |
| 0031-02 | 0031 | Генератор C — форвард-прототипы локальных функций | [0031-02-c-prototypes.md](0031-02-c-prototypes.md) |
| 0031-03 | 0031 | Документация языка, примеры и версия 0.3.0 | [0031-03-docs-version.md](0031-03-docs-version.md) |
| 0032-01 | 0032 | Единое хранилище значений (упразднение `Unit::variables`) | [0032-01-state-io-variables.md](0032-01-state-io-variables.md) |
| 0032-02 | 0032 | Снимок и восстановление через единое хранилище (`Context::dump`) | [0032-02-snapshot-through-context.md](0032-02-snapshot-through-context.md) |
| 0032-03 | 0032 | Интеграционные тесты кругового рейса (`state_io_tests.rs`) | [0032-03-state-io-integration-tests.md](0032-03-state-io-integration-tests.md) |
| 0033-01 | 0033 | Генератор C — вход в стартовое состояние не расходует такт | [0033-01-init-tick-alignment.md](0033-01-init-tick-alignment.md) |
| 0033-02 | 0033 | Потактовая автосверка трасс симулятора и порождённого C | [0033-02-per-tick-conformance.md](0033-02-per-tick-conformance.md) |
| 0033-03 | 0033 | Семантика такта в документации и рост версии языка | [0033-03-language-version-docs.md](0033-03-language-version-docs.md) |
| 0034-01 | 0034 | Ядро — `Value::Struct`, реестр структур, `coerce_to_type` | [0034-01-sim-struct-types.md](0034-01-sim-struct-types.md) |
| 0034-02 | 0034 | Чтение поля и разрешение неоднозначности `BitAccess` | [0034-02-field-read.md](0034-02-field-read.md) |
| 0034-03 | 0034 | Запись в поле — lvalue-путь вместо SIM-017 | [0034-03-field-write.md](0034-03-field-write.md) |
| 0034-04 | 0034 | Инициализаторы структур и устранение второго вычислителя | [0034-04-initializers.md](0034-04-initializers.md) |
| 0034-05 | 0034 | Наблюдаемость значений и сверка с эталоном C | [0034-05-observability-conformance.md](0034-05-observability-conformance.md) |
| 0035-01 | 0035 | Разбор LTL-формул в блоках кода (устранение тихой потери) | [0035-01-ltl-in-blocks.md](0035-01-ltl-in-blocks.md) |
| 0035-02 | 0035 | Явные диагностики SE-053/SE-054 вместо немых веток | [0035-02-ltl-diagnostics.md](0035-02-ltl-diagnostics.md) |
| 0035-03 | 0035 | Форматтер — печать встроенной формулы в блоке кода | [0035-03-ltl-format-stmt.md](0035-03-ltl-format-stmt.md) |
| 0035-04 | 0035 | Примеры, контрпримеры, фикстуры и синхронизация README | [0035-04-ltl-examples-docs.md](0035-04-ltl-examples-docs.md) |
| 0036-01 | 0036 | Инкапсуляция `Unit` (pub struct + приватный UnitKind) | [0036-01-sim-visibility.md](0036-01-sim-visibility.md) |
| 0036-02 | 0036 | Закрепление чистой сборки линтом, чистка и версия крейта | [0036-02-lint-pin.md](0036-02-lint-pin.md) |
| 0037-01 | 0037 | Кросс-платформенные тесты разбора `-I` в `lamc` | [0037-01-windows-test-failures.md](0037-01-windows-test-failures.md) |
| 0037-02 | 0037 | Отладочная запись в `/tmp` в тесте `viewport` | [0037-02-viewport-tmp-write.md](0037-02-viewport-tmp-write.md) |
| 0037-03 | 0037 | Windows в матрице CI | [0037-03-ci-windows-matrix.md](0037-03-ci-windows-matrix.md) |
| 0038-01 | 0038 | Интеграция LSP4IJ — запуск lam-lsp из плагина | [0038-01-intellij-semantic-tokens.md](0038-01-intellij-semantic-tokens.md) |
| 0038-02 | 0038 | Маппинг семантических токенов в цвета редактора | [0038-02-semantic-tokens-colors.md](0038-02-semantic-tokens-colors.md) |
| 0038-03 | 0038 | Тесты классификации токенов lam-lsp и документация | [0038-03-server-tokens-tests.md](0038-03-server-tokens-tests.md) |
| 0039-01 | 0039 | Внешний форматтер — `AsyncDocumentFormattingService` + `lamc fmt --stdin` | [0039-01-intellij-reformat.md](0039-01-intellij-reformat.md) |
| 0039-02 | 0039 | Настройки пути к `lamc` и диагностика отсутствия бинарника | [0039-02-lamc-settings.md](0039-02-lamc-settings.md) |
| 0039-03 | 0039 | Сверка «байт-в-байт» с `lamc fmt` и версия плагина | [0039-03-golden-tests.md](0039-03-golden-tests.md) |
| 0040-01 | 0040 | Структурный PSI-парсер — каркас и узлы деклараций | [0040-01-intellij-psi-parser.md](0040-01-intellij-psi-parser.md) |
| 0040-02 | 0040 | Ссылки и резолв имён на PSI; `PsiReference` для путей `import` | [0040-02-psi-references.md](0040-02-psi-references.md) |
| 0040-03 | 0040 | Find usages поверх PSI | [0040-03-find-usages.md](0040-03-find-usages.md) |
| 0040-04 | 0040 | Rename — штатный рефакторинг IDEA | [0040-04-rename.md](0040-04-rename.md) |
| 0040-05 | 0040 | Арбитраж с LSP4IJ и приёмка R2/R4/R6 от фичи 0038 | [0040-05-lsp-arbitration.md](0040-05-lsp-arbitration.md) |
| 0041-01 | 0041 | Каркас ST-бэкенда (`Language::ST`, `generator/st/`, цели `st`/`st-at`) | [0041-01-st-backend.md](0041-01-st-backend.md) |
| 0041-02 | 0041 | Отображение типов Lam → IEC 61131-3 | [0041-02-type-mapping.md](0041-02-type-mapping.md) |
| 0041-03 | 0041 | Состояния, переходы и композиция моделей в ST | [0041-03-state-mapping.md](0041-03-state-mapping.md) |
| 0041-04 | 0041 | Выражения, условия и функции в ST | [0041-04-expressions-functions.md](0041-04-expressions-functions.md) |
| 0041-05 | 0041 | Карта адресов → `AT %…` (цель `st-at`) | [0041-05-address-at.md](0041-05-address-at.md) |
| 0041-06 | 0041 | Проба-гейт — валидация порождённого ST через MatIEC `iec2c` | [0041-06-matiec-validation.md](0041-06-matiec-validation.md) |
| 0041-07 | 0041 | Примеры, контрпримеры и документация ST-бэкенда | [0041-07-examples-docs.md](0041-07-examples-docs.md) |
| 0042-01 | 0042 | Вычислитель выражений адреса (свёртка констант и разрешение символов) | [0042-01-address-defines.md](0042-01-address-defines.md) |
| 0042-02 | 0042 | Среда символов адреса `AddressEnv` и оверлей над `const` | [0042-02-define-env.md](0042-02-define-env.md) |
| 0042-03 | 0042 | Флаги CLI `--define`/`-D`, документация и версия языка | [0042-03-cli-wiring.md](0042-03-cli-wiring.md) |
| 0043-01 | 0043 | Ядро экспорта — обогащение записи и формат `map` | [0043-01-address-map-export.md](0043-01-address-map-export.md) |
| 0043-02 | 0043 | Эмиттер формата `json` — контракт с внешними инструментами | [0043-02-json-emitter.md](0043-02-json-emitter.md) |
| 0043-03 | 0043 | CLI-подкоманда `lamc address-map --emit` | [0043-03-cli-subcommand.md](0043-03-cli-subcommand.md) |
| 0044-01 | 0044 | Грамматика и АСД конструкции `invariant` | [0044-01-invariant-grammar.md](0044-01-invariant-grammar.md) |
| 0044-02 | 0044 | Семантика `invariant` — десахаризация и диагностики | [0044-02-semantics-desugar.md](0044-02-semantics-desugar.md) |
| 0044-03 | 0044 | Симулятор — проверка формул по шагам | [0044-03-simulator-checks.md](0044-03-simulator-checks.md) |
| 0044-04 | 0044 | Генератор C — нулевой регресс и сверка с симулятором | [0044-04-c-generator-conformance.md](0044-04-c-generator-conformance.md) |
| 0044-05 | 0044 | Форматтер — печать узла `InvariantDefine` | [0044-05-formatter.md](0044-05-formatter.md) |
| 0044-06 | 0044 | Документация языка и версия 0.3.0 | [0044-06-docs-version.md](0044-06-docs-version.md) |
| 0045-01 | 0045 | Каркас бэкенда — `Language::SV`, `generator/sv/`, `SvMap`, цель `-t sv` | [0045-01-sv-backend.md](0045-01-sv-backend.md) |
| 0045-02 | 0045 | Гейт проверяемости — Verilator (линт) + yosys (синтез) | [0045-02-validation.md](0045-02-validation.md) |
| 0045-03 | 0045 | Отображение типов Lam → SystemVerilog + диагностики | [0045-03-type-mapping.md](0045-03-type-mapping.md) |
| 0045-04 | 0045 | Модуль и порты — `clk`/`rst_n`, `in`/`out` → порты модуля | [0045-04-module-ports.md](0045-04-module-ports.md) |
| 0045-05 | 0045 | Автомат — состояния, переходы, сброс, композиция | [0045-05-fsm-time-reset.md](0045-05-fsm-time-reset.md) |
| 0045-06 | 0045 | Выражения, условия, функции | [0045-06-expressions-functions.md](0045-06-expressions-functions.md) |
| 0045-07 | 0045 | Тестбенч и сверка с симулятором Lam | [0045-07-testbench-conformance.md](0045-07-testbench-conformance.md) |
| 0045-08 | 0045 | Примеры, контрпримеры, документация | [0045-08-examples-docs.md](0045-08-examples-docs.md) |
| 0048-01 | 0048 | Упорядоченный общий слой (`ModelNode`, `minimap::Map`, `Ord` для `Name`) | [0048-01-ordered-semantic-layer.md](0048-01-ordered-semantic-layer.md) |
| 0048-02 | 0048 | Детерминированная `topological_sort_models` (цель `c`) | [0048-02-deterministic-topo-sort.md](0048-02-deterministic-topo-sort.md) |
| 0048-03 | 0048 | Гейт воспроизводимости в `precheck.sh` + тесты-сторожа | [0048-03-reproducibility-gate.md](0048-03-reproducibility-gate.md) |
| 0048-04 | 0048 | Перегенерация `examples/generated/` и синхронизация документации | [0048-04-regenerate-examples.md](0048-04-regenerate-examples.md) |
| 0049-01 | 0049 | Структура Крипке из ModelNode (управляющий граф) | [0049-01-kripke.md](0049-01-kripke.md) |
| 0049-02 | 0049 | Произведение автоматов и проверка пустоты (nested-DFS, лассо) | [0049-02-product-emptiness.md](0049-02-product-emptiness.md) |
| 0049-03 | 0049 | Движок верификации `verify_model` (build_buchi(¬φ) → пустота) | [0049-03-verify-engine.md](0049-03-verify-engine.md) |
| 0049-04 | 0049 | Подкоманда `lamc verify` | [0049-04-cli-verify.md](0049-04-cli-verify.md) |
| 0049-05 | 0049 | Документация, примеры, тесты | [0049-05-docs-tests.md](0049-05-docs-tests.md) |
| 0049-06 | 0049 | Область LTL-формулы, объявленной в состоянии (`G (S -> φ)`) | [0049-06-state-formula-scope.md](0049-06-state-formula-scope.md) |
| 0050-01 | 0050 | Каркас бэкенда: `Language::Rust`, `generator/rust/`, цель `-t rust` | [0050-01-scaffold.md](0050-01-scaffold.md) |
| 0050-02 | 0050 | Гейт проверяемости: `rustc` + `clippy` по порождённому коду | [0050-02-gate.md](0050-02-gate.md) |
| 0050-03 | 0050 | Отображение типов Lam → Rust и диагностики | [0050-03-type-mapping.md](0050-03-type-mapping.md) |
| 0050-04 | 0050 | Имена: регистр, raw-идентификаторы, коллизия `Self` | [0050-04-naming.md](0050-04-naming.md) |
| 0050-05 | 0050 | Модель и порты: `struct`, HAL-трейт, `in`/`out`/`inout` | [0050-05-model-ports.md](0050-05-model-ports.md) |
| 0050-06 | 0050 | Автомат: состояния, переходы, контракт такта, композиция | [0050-06-fsm.md](0050-06-fsm.md) |
| 0050-07 | 0050 | Выражения, условия, функции, встроенные и `extern fn` | [0050-07-expr-fn.md](0050-07-expr-fn.md) |
| 0050-08 | 0050 | Примеры, контрпримеры, документация | [0050-08-docs-tests.md](0050-08-docs-tests.md) |
| 0051-01 | 0051 | Признак происхождения модели (`origin`) | [0051-01-verify-scope.md](0051-01-verify-scope.md) |
| 0051-02 | 0051 | Область в `verify_all` и флаг CLI `--scope` | [0051-02-cli-scope.md](0051-02-cli-scope.md) |
| 0052-01 | 0052 | `minimap::visit_state` → итеративный обход (чинит все 5 целей) | [0052-01-minimap-iterative.md](0052-01-minimap-iterative.md) |
| 0052-02 | 0052 | nested-DFS (`check.rs`) → итеративный обход | [0052-02-nested-dfs-iterative.md](0052-02-nested-dfs-iterative.md) |
| 0053-01 | 0053 | Реестр файлов и настоящий `file_no` | [0053-01-file-table.md](0053-01-file-table.md) |
| 0053-02 | 0053 | Печать `путь:строка:колонка` в `lamc` | [0053-02-cli-positions.md](0053-02-cli-positions.md) |
| 0054-01 | 0054 | Печать позиции — в общий слой `grammar::diagnostics` | [0054-01-shared-position-print.md](0054-01-shared-position-print.md) |
| 0054-02 | 0054 | Позиция и код в диагностиках симулятора | [0054-02-sim-diagnostics.md](0054-02-sim-diagnostics.md) |
| 0055-01 | 0055 | Неявный путь импорта и цепочка импорта (ядро) | [0055-01-implicit-import-path.md](0055-01-implicit-import-path.md) |
| 0055-02 | 0055 | Импорты и чужие диагностики в LSP | [0055-02-lsp-foreign-diagnostics.md](0055-02-lsp-foreign-diagnostics.md) |
| 0056-01 | 0056 | Индекс различает файлы (`file_no`, `offset`) | [0056-01-file-aware-index.md](0056-01-file-aware-index.md) |
| 0056-02 | 0056 | Точный путь вместо угадывания (`to_snake_case` удаляется) | [0056-02-goto-exact-path.md](0056-02-goto-exact-path.md) |
| 0056-03 | 0056 | Подключить кросс-файловый переход к серверу | [0056-03-wire-server.md](0056-03-wire-server.md) |
| 0057-01 | 0057 | Регистр шага и его enum в `Fsm`/минимапе | [0057-01-step-register.md](0057-01-step-register.md) |
| 0057-02 | 0057 | `emit_extend` для `Concatenation` — инлайн активного шага | [0057-02-emit-concatenation.md](0057-02-emit-concatenation.md) |
| 0057-03 | 0057 | Вложенная композиция — покрытие или явная диагностика | [0057-03-nesting-diagnostics.md](0057-03-nesting-diagnostics.md) |
| 0057-04 | 0057 | Потактовая сверка и гейт для `+` в SV | [0057-04-conformance-gate.md](0057-04-conformance-gate.md) |
| 0058-01 | 0058 | Предикат сворачиваемости хвоста — единственный источник истины | [0058-01-tail-fold-predicate.md](0058-01-tail-fold-predicate.md) |
| 0058-02 | 0058 | Рекурсивный печатник хвостовой позиции (заход в `if/else`) | [0058-02-recursive-tail-printer.md](0058-02-recursive-tail-printer.md) |
| 0059-01 | 0059 | Состав `Shared` — объединение нужд под-моделей | [0059-01-shared-needs.md](0059-01-shared-needs.md) |
| 0059-02 | 0059 | Эмиссия `Shared` — тип, поле, доступ, снятие заглушки | [0059-02-emit-shared.md](0059-02-emit-shared.md) |
| 0060-01 | 0060 | Общий слой — факт о перечислении (диапазон, знак, ширина) | [0060-01-enum-facts.md](0060-01-enum-facts.md) |
| 0060-02 | 0060 | Миграция четырёх целей + закрытие фикса 0005-01 | [0060-02-migrate-targets.md](0060-02-migrate-targets.md) |
| 0061-01 | 0061 | Синтаксис `q(m, n)`, АСД, вывод типов, форматтер | [0061-01-syntax-and-type.md](0061-01-syntax-and-type.md) |
| 0061-02 | 0061 | Эталонная Q-арифметика в симуляторе | [0061-02-simulator-arithmetic.md](0061-02-simulator-arithmetic.md) |
| 0061-03 | 0061 | Цели `c`, `rust`, `st` — Q-арифметика; ловушка C11 6.5.7p5 | [0061-03-software-targets.md](0061-03-software-targets.md) |
| 0061-04 | 0061 | Цель `sv` — синтезируемый fixed-point | [0061-04-sv-target.md](0061-04-sv-target.md) |
| 0061-05 | 0061 | Пример-регулятор, README, перенос примеров в документацию | [0061-05-example-and-docs.md](0061-05-example-and-docs.md) |
| 0062-01 | 0062 | Регистровый файл и его интерфейс (без протокола) | [0062-01-register-file-interface.md](0062-01-register-file-interface.md) |
| 0062-02 | 0062 | Внешняя карта адресов для `sv-mmio` + потактовая сверка | [0062-02-address-map-and-conformance.md](0062-02-address-map-and-conformance.md) |
| 0063-01 | 0063 | Вход `en` с умолчанием `1'b1` и разрешённый такт | [0063-01-enable-port.md](0063-01-enable-port.md) |
| 0064-01 | 0064 | `SV-009` на переменный делитель + канал предупреждений цели `sv` | [0064-01-divider-warning.md](0064-01-divider-warning.md) |
| 0065-01 | 0065 | Префикс POU именем модели — закрытие фикса 0041-01 (Tier 1) | [0065-01-prefix-pou.md](0065-01-prefix-pou.md) |
| 0065-02 | 0065 | `ST-014` — столкновение имени со стандартной библиотекой IEC | [0065-02-st014-reserved-names.md](0065-02-st014-reserved-names.md) |
| 0065-03 | 0065 | Потактовая сверка цели `st` — отдача долга фичи 0041 | [0065-03-st-conformance.md](0065-03-st-conformance.md) |
| 0066-01 | 0066 | `coerce_to` по целевому типу — `BOOL` и перечисления | [0066-01-coerce-to.md](0066-01-coerce-to.md) |
| 0068-01 | 0068 | `build_kripke` — вершина как (состояние, значения переменных φ) | [0068-01-tracked-vars-kripke.md](0068-01-tracked-vars-kripke.md) |
| 0069-01 | 0069 | `address_map/` — разделение по темам + снятие записи долга | [0069-01-split-by-theme.md](0069-01-split-by-theme.md) |
| 0096-01 | 0096 | Q-арифметика через нативный float и флаг генерации (embedded ↔ float) | [0096-01-fixed-point-native-float.md](0096-01-fixed-point-native-float.md) |
| 0097-01 | 0097 | Пример ПИД-регулятора на языке Lam (fixed-point) | [0097-01-pid-regulator-example.md](0097-01-pid-regulator-example.md) |
| 0090-01 | 0090 | Обобщённый строгий режим `precheck.sh` (`PRECHECK_STRICT=1`) | [0090-01-precheck-strict-mode.md](0090-01-precheck-strict-mode.md) |
| 0090-02 | 0090 | `ci.yml` вызывает `precheck.sh` под строгим режимом | [0090-02-ci-runs-precheck.md](0090-02-ci-runs-precheck.md) |
| 0070-01 | 0070 | Снять `SE-035` с инициализатора порта (это адрес) | [0070-01-skip-port-bit-value-check.md](0070-01-skip-port-bit-value-check.md) |
| 0071-01 | 0071 | use-site `Location` на `ConditionNode::State` + индекс/goto | [0071-01-condition-state-location.md](0071-01-condition-state-location.md) |
| 0073-01 | 0073 | Удалить `Location::filename()`, тесты — на `try_file_no` | [0073-01-remove-filename.md](0073-01-remove-filename.md) |
| 0086-01 | 0086 | Нулевой дефолт по типу для переменной без инициализатора | [0086-01-default-scalar-value.md](0086-01-default-scalar-value.md) |

| 0074-01 | 0074 | Канонизация скобок паттерна `S(Модель)` в `resolve_condition` | [0074-01-canonicalize-state-of-parens.md](0074-01-canonicalize-state-of-parens.md) |
| 0083-01 | 0083 | Model-level `always` в симуляторе и всех целях + вынос помощников блоков | [0083-01-model-always-all-targets.md](0083-01-model-always-all-targets.md) |
| 0080-01 | 0080 | Три дефекта C по структурам: составной литерал, static const, SE-061 | [0080-01-c-struct-defects.md](0080-01-c-struct-defects.md) |
| 0079-01 | 0079 | Рекурсивное перечисление портов композиции + матчинг run_simulations | [0079-01-sim-composition-ports.md](0079-01-sim-composition-ports.md) |
| 0072-01 | 0072 | LSP не читает initializationOptions (пути поиска импортов) | [0072-01-lsp-initialization-options.md](0072-01-lsp-initialization-options.md) |
| 0076-01 | 0076 | Симулятор не исполняет массивы вовсе | [0076-01-sim-arrays.md](0076-01-sim-arrays.md) |
| 0077-01 | 0077 | Реестр кодов диагностик + гейт check-diagnostic-codes.sh | [0077-01-diagnostic-code-registry.md](0077-01-diagnostic-code-registry.md) |
| 0078-01 | 0078 | Семантика `[bit;N]`: единый слой bit_vector + упаковка во все цели | [0078-01-bit-array-semantics.md](0078-01-bit-array-semantics.md) |
| 0085-01 | 0085 | Константа версии языка в коде + гейт синхронизации с README | [0085-01-language-version-constant.md](0085-01-language-version-constant.md) |
| 0084-01 | 0084 | Ключ карты адресов — квалифицированный (модель+порт) | [0084-01-address-map-qualified-key.md](0084-01-address-map-qualified-key.md) |
| 0087-01 | 0087 | Мягкий режим инвариантов симулятора (записать и продолжить) | [0087-01-invariant-soft-mode.md](0087-01-invariant-soft-mode.md) |
| 0094-01 | 0094 | Доработка scripts/new-feature.sh (идемпотентность, статус ADR, поздние стадии) | [0094-01-new-feature-script-fixes.md](0094-01-new-feature-script-fixes.md) |
| 0093-01 | 0093 | Запрет _ => в семантических узлах и разрешение конфликта с #[non_exhaustive] | [0093-01-wildcard-match-rule.md](0093-01-wildcard-match-rule.md) |
| 0091-01 | 0091 | Правило о размере модуля переносится в docs/CODE.md | [0091-01-module-size-rule-in-code-md.md](0091-01-module-size-rule-in-code-md.md) |
| 0067-01 | 0067 | Rename и PsiReference для import в плагине IntelliJ | [0067-01-intellij-rename-psi-import.md](0067-01-intellij-rename-psi-import.md) |
| 0067-02 | 0067 | Rename и PsiReference для import в плагине IntelliJ | [0067-02-intellij-rename-psi-import.md](0067-02-intellij-rename-psi-import.md) |
| 0067-03 | 0067 | Rename и PsiReference для import в плагине IntelliJ | [0067-03-intellij-rename-psi-import.md](0067-03-intellij-rename-psi-import.md) |
| 0088-01 | 0088 | Остальные нарушители лимита размера модуля | [0088-01-module-size-remaining.md](0088-01-module-size-remaining.md) |
| 0089-01 | 0089 | Остаточные проверки плагина IntelliJ (0022/0023) | [0089-01-intellij-residual-checks.md](0089-01-intellij-residual-checks.md) |
| 0092-01 | 0092 | У фичи 0018 нет ADR | [0092-01-adr-0018-retrofit.md](0092-01-adr-0018-retrofit.md) |
| 0088-02 | 0088 | Остальные нарушители лимита размера модуля | [0088-02-module-size-remaining.md](0088-02-module-size-remaining.md) |
| 0088-03 | 0088 | Остальные нарушители лимита размера модуля | [0088-03-module-size-remaining.md](0088-03-module-size-remaining.md) |
| 0088-04 | 0088 | Остальные нарушители лимита размера модуля | [0088-04-module-size-remaining.md](0088-04-module-size-remaining.md) |
| 0088-05 | 0088 | Остальные нарушители лимита размера модуля | [0088-05-module-size-remaining.md](0088-05-module-size-remaining.md) |
| 0088-06 | 0088 | Остальные нарушители лимита размера модуля | [0088-06-module-size-remaining.md](0088-06-module-size-remaining.md) |
| 0088-07 | 0088 | Остальные нарушители лимита размера модуля | [0088-07-module-size-remaining.md](0088-07-module-size-remaining.md) |
| 0088-08 | 0088 | Остальные нарушители лимита размера модуля | [0088-08-module-size-remaining.md](0088-08-module-size-remaining.md) |
| 0088-09 | 0088 | Остальные нарушители лимита размера модуля | [0088-09-module-size-remaining.md](0088-09-module-size-remaining.md) |
| 0088-10 | 0088 | Остальные нарушители лимита размера модуля | [0088-10-module-size-remaining.md](0088-10-module-size-remaining.md) |
| 0088-11 | 0088 | Остальные нарушители лимита размера модуля | [0088-11-module-size-remaining.md](0088-11-module-size-remaining.md) |
| 0088-12 | 0088 | Остальные нарушители лимита размера модуля | [0088-12-module-size-remaining.md](0088-12-module-size-remaining.md) |
| 0100-01 | 0100 | Переименование языка Lam → Takt | [0100-01-language-rename-takt.md](0100-01-language-rename-takt.md) |
| 0100-02 | 0100 | Переименование языка Lam → Takt | [0100-02-language-rename-takt.md](0100-02-language-rename-takt.md) |
| 0100-03 | 0100 | Переименование языка Lam → Takt | [0100-03-language-rename-takt.md](0100-03-language-rename-takt.md) |
| 0100-04 | 0100 | Переименование языка Lam → Takt | [0100-04-language-rename-takt.md](0100-04-language-rename-takt.md) |
| 0100-05 | 0100 | Переименование языка Lam → Takt | [0100-05-language-rename-takt.md](0100-05-language-rename-takt.md) |
| 0100-06 | 0100 | Переименование языка Lam → Takt | [0100-06-language-rename-takt.md](0100-06-language-rename-takt.md) |
| 0124-01 | 0124 | Экспорт графов верификации (Крипке/Бюхи/произведение) в Graphviz DOT | [0124-01-verify-graph-export.md](0124-01-verify-graph-export.md) |
| 0125-01 | 0125 | Плагин IntelliJ Takt: LSP-проверка, автодополнение, документация и настройки инструментов | [0125-01-intellij-takt-lsp-tooling.md](0125-01-intellij-takt-lsp-tooling.md) |
| 0139-01 | 0139 | Удаление мёртвой конфигурации .travis.yml | [0139-01-remove-travis-config.md](0139-01-remove-travis-config.md) |
| 0137-01 | 0137 | Фиксация толчейна Rust и MSRV | [0137-01-toolchain-pin-msrv.md](0137-01-toolchain-pin-msrv.md) |
| 0128-01 | 0128 | Диагностика вместо паники на числовом литерале больше i64::MAX | [0128-01-lexer-literal-overflow.md](0128-01-lexer-literal-overflow.md) |
| 0129-01 | 0129 | Предел вложенности семантических обходов | [0129-01-semantic-deep-nesting.md](0129-01-semantic-deep-nesting.md) |
| 0127-01 | 0127 | Единая семантика переполнения целых во всех целях | [0127-01-int-overflow-semantics.md](0127-01-int-overflow-semantics.md) |
| 0133-01 | 0133 | Гейт компиляции и симуляции примеров документа book/ | [0133-01-book-examples-gate.md](0133-01-book-examples-gate.md) |
| 0135-01 | 0135 | Квалифицированные имена портов в симуляторе | [0135-01-sim-qualified-port-names.md](0135-01-sim-qualified-port-names.md) |
| 0131-01 | 0131 | LSP: definition, references и rename | [0131-01-lsp-definition-references-rename.md](0131-01-lsp-definition-references-rename.md) |
| 0131-02 | 0131 | LSP: definition, references и rename | [0131-02-lsp-definition-references-rename.md](0131-02-lsp-definition-references-rename.md) |
| 0131-03 | 0131 | LSP: definition, references и rename | [0131-03-lsp-definition-references-rename.md](0131-03-lsp-definition-references-rename.md) |
| 0130-01 | 0130 | Накопление семантических диагностик | [0130-01-diagnostics-batch.md](0130-01-diagnostics-batch.md) |
| 0130-02 | 0130 | Накопление семантических диагностик | [0130-02-diagnostics-batch.md](0130-02-diagnostics-batch.md) |
| 0130-03 | 0130 | Накопление семантических диагностик | [0130-03-diagnostics-batch.md](0130-03-diagnostics-batch.md) |
| 0132-01 | 0132 | Именованные порты в сценариях симулятора | [0132-01-sim-named-port-scenarios.md](0132-01-sim-named-port-scenarios.md) |
| 0132-02 | 0132 | Именованные порты в сценариях симулятора | [0132-02-sim-named-port-scenarios.md](0132-02-sim-named-port-scenarios.md) |
| 0132-03 | 0132 | Именованные порты в сценариях симулятора | [0132-03-sim-named-port-scenarios.md](0132-03-sim-named-port-scenarios.md) |
| 0140-01 | 0140 | Ревизия витрины кандидатов и разделение ролей README и book | [0140-01-backlog-revision-doc-split.md](0140-01-backlog-revision-doc-split.md) |
| 0140-02 | 0140 | Ревизия витрины кандидатов и разделение ролей README и book | [0140-02-backlog-revision-doc-split.md](0140-02-backlog-revision-doc-split.md) |
| 0140-03 | 0140 | Ревизия витрины кандидатов и разделение ролей README и book | [0140-03-backlog-revision-doc-split.md](0140-03-backlog-revision-doc-split.md) |
| 0138-02 | 0138 | Измерение покрытия тестами | [0138-02-coverage-measurement.md](0138-02-coverage-measurement.md) |
| 0136-01 | 0136 | Бенчмарки производительности | [0136-01-perf-benchmarks.md](0136-01-perf-benchmarks.md) |
| 0136-02 | 0136 | Бенчмарки производительности | [0136-02-perf-benchmarks.md](0136-02-perf-benchmarks.md) |
| 0136-03 | 0136 | Бенчмарки производительности | [0136-03-perf-benchmarks.md](0136-03-perf-benchmarks.md) |
| 0134-01 | 0134 | Лексика, грамматика и АСД: литерал длительности, `clock`, `after`/`every` | [0134-01-lexis-grammar-ast.md](0134-01-lexis-grammar-ast.md) |
| 0134-02 | 0134 | Тип `duration`, слой `semantic::duration` и диагностики | [0134-02-type-and-lowering.md](0134-02-type-and-lowering.md) |
| 0134-03 | 0134 | Симулятор: виртуальные часы, значение длительности, `after` | [0134-03-simulator-clock.md](0134-03-simulator-clock.md) |
| 0134-04 | 0134 | Цель `c`: выдержка `after` в профиле «такты» | [0134-04-target-c.md](0134-04-target-c.md) |
| 0134-04b | 0134 | Цель `c`/`c-hal`: профиль «часы» (внешний источник `now_ms`) | [0134-04b-target-c-clock-profile.md](0134-04b-target-c-clock-profile.md) |
| 0134-05 | 0134 | Частота — контракт сборки + CLI-часть 0134-08 | [0134-05-clock-contract.md](0134-05-clock-contract.md) |
| 0134-06 | 0134 | Цель `rust`: модель времени (оба профиля) | [0134-06-target-rust.md](0134-06-target-rust.md) |
| 0134-07 | 0134 | Цель `st`/`st-at`: модель времени (`TON` и счётчик) | [0134-07-target-st.md](0134-07-target-st.md) |
| 0134-08 | 0134 | Цель `sv`: модель времени (вход `time_ms` и счётчик) | [0134-08-target-sv.md](0134-08-target-sv.md) |
| 0134-09 | 0134 | Периодический блок `every` — все цели и симулятор | [0134-09-every.md](0134-09-every.md) |
| 0142-01 | 0142 | Верхний колонтитул документа с названием раздела | [0142-01-running-header.md](0142-01-running-header.md) |
| 0178-01 | 0178 | Исчерпывающий разбор токена и догон подсветки | [0178-01-exhaustive-token-highlight.md](0178-01-exhaustive-token-highlight.md) |
| 0178-02 | 0178 | Догон списка автодополнения и сторож вложения | [0178-02-completion-keywords-guard.md](0178-02-completion-keywords-guard.md) |
| 0178-03 | 0178 | Исполнение тестов LSP в предкоммите | [0178-03-precheck-lsp-tests.md](0178-03-precheck-lsp-tests.md) |
| 0179-01 | 0179 | Правка 16 мест со старым адресом | [0179-01-url-replacements.md](0179-01-url-replacements.md) |
| 0179-02 | 0179 | Гейт единственности адреса репозитория | [0179-02-repo-url-gate.md](0179-02-repo-url-gate.md) |
| 0179-03 | 0179 | Пересборка и переустановка плагинов | [0179-03-plugins-reinstall.md](0179-03-plugins-reinstall.md) |
| 0144-01 | 0144 | Вычисление экспоненты в лексере | [0144-01-lexer-exponent.md](0144-01-lexer-exponent.md) |
| 0144-02 | 0144 | Понижение float в q с экспонентой | [0144-02-fixed-lowering-exponent.md](0144-02-fixed-lowering-exponent.md) |
| 0144-03 | 0144 | Документ и версия языка | [0144-03-book-and-version.md](0144-03-book-and-version.md) |
| 0155-01 | 0155 | Семантическое разрешение тел вложенных операторов | [0155-01-semantic-nested-statement-resolution.md](0155-01-semantic-nested-statement-resolution.md) |
| 0155-02 | 0155 | Семантическое разрешение тел вложенных операторов | [0155-02-semantic-nested-statement-resolution.md](0155-02-semantic-nested-statement-resolution.md) |
| 0155-03 | 0155 | Семантическое разрешение тел вложенных операторов | [0155-03-semantic-nested-statement-resolution.md](0155-03-semantic-nested-statement-resolution.md) |
| 0146-01 | 0146 | Гейт символов вне шрифта документа book/ | [0146-01-book-glyph-gate.md](0146-01-book-glyph-gate.md) |
| 0146-02 | 0146 | Гейт символов вне шрифта документа book/ | [0146-02-book-glyph-gate.md](0146-02-book-glyph-gate.md) |
| 0149-01 | 0149 | Гейт согласованности живого контекста CLAUDE.md | [0149-01-claude-md-consistency-gate.md](0149-01-claude-md-consistency-gate.md) |
| 0149-02 | 0149 | Гейт согласованности живого контекста CLAUDE.md | [0149-02-claude-md-consistency-gate.md](0149-02-claude-md-consistency-gate.md) |
| 0180-01 | 0180 | Сокращение живого контекста CLAUDE.md | [0180-01-claude-md-context-diet.md](0180-01-claude-md-context-diet.md) |
| 0177-01 | 0177 | Гейт согласованности статуса в реестре и в карточке фичи | [0177-01-features-registry-status-gate.md](0177-01-features-registry-status-gate.md) |
| 0159-01 | 0159 | Фиксация требования JDK для сборки плагина | [0159-01-intellij-jdk21-build.md](0159-01-intellij-jdk21-build.md) |
| 0171-01 | 0171 | Гейт цели c под -Werror | [0171-01-c-gate-werror.md](0171-01-c-gate-werror.md) |
| 0181-01 | 0181 | Деление takt-sim/src/unit/mod.rs — вынос такта в unit/tick.rs | [0181-01-unit-module-split.md](0181-01-unit-module-split.md) |
| 0181-02 | 0181 | Реализация состояния строится дочерним юнитом с общим контекстом | [0181-02-state-implementation-child-unit.md](0181-02-state-implementation-child-unit.md) |
| 0181-03 | 0181 | Такт узла тикает реализацию; переход берётся по её завершении | [0181-03-tick-node-implementation.md](0181-03-tick-node-implementation.md) |
| 0181-04 | 0181 | Тройная сверка sim-C-SV и значенческие тесты композиции | [0181-04-triple-conformance.md](0181-04-triple-conformance.md) |
| 0166-01 | 0166 | Пример batch_cycle и его обвязка (SV_TRANSLATABLE, тестбенч, контракт, харнесс C) | [0166-01-batch-cycle-example.md](0166-01-batch-cycle-example.md) |
| 0174-01 | 0174 | Эмиссия impl Default и сторож на гейте | [0174-01-rust-default-impl.md](0174-01-rust-default-impl.md) |
| 0147-01 | 0147 | Тесты состава, вложенности и диапазонов символов | [0147-01-document-symbol-tests.md](0147-01-document-symbol-tests.md) |
| 0148-01 | 0148 | Тесты ветвей печатников и фикс сравнения bit | [0148-01-rust-printer-branch-tests.md](0148-01-rust-printer-branch-tests.md) |
| 0163-01 | 0163 | Вынос вычислителя в unit/initial.rs с модульным deny и расширение гейта | [0163-01-initial-module-deny.md](0163-01-initial-module-deny.md) |
| 0143-01 | 0143 | Грамматика, узел АСД и исчерпывающие обходы | [0143-01-after-const-duration.md](0143-01-after-const-duration.md) |
| 0143-02 | 0143 | Вычислитель константной выдержки и диагностика `SE-072` | [0143-02-after-const-duration.md](0143-02-after-const-duration.md) |
| 0143-03 | 0143 | Документ: раздел времени и приложение диагностик | [0143-03-after-const-duration.md](0143-03-after-const-duration.md) |
| 0143-04 | 0143 | Ревизия объёма: константное выражение в after | [0143-04-after-const-duration.md](0143-04-after-const-duration.md) |
| 0183-01 | 0183 | Общий слой представления и цель c | [0183-01-duration-type-in-targets.md](0183-01-duration-type-in-targets.md) |
| 0183-02 | 0183 | Цель `rust` | [0183-02-duration-type-in-targets.md](0183-02-duration-type-in-targets.md) |
| 0183-03 | 0183 | Цель `st` | [0183-03-duration-type-in-targets.md](0183-03-duration-type-in-targets.md) |
| 0183-04 | 0183 | Цель `sv` | [0183-04-duration-type-in-targets.md](0183-04-duration-type-in-targets.md) |
| 0183-05 | 0183 | Вычисляемая выдержка `after (v + 1s)` | [0183-05-duration-type-in-targets.md](0183-05-duration-type-in-targets.md) |
| 0183-06 | 0183 | Документ и значение порта `duration` | [0183-06-duration-type-in-targets.md](0183-06-duration-type-in-targets.md) |
| 0184-01 | 0184 | Общие переменные библиотечного файла в импортёре | [0184-01-imported-shared-variables.md](0184-01-imported-shared-variables.md) |
| 0184-02 | 0184 | Общие переменные библиотечного файла в импортёре | [0184-02-imported-shared-variables.md](0184-02-imported-shared-variables.md) |
| 0182-01 | 0182 | Библиотечный ПИД-регулятор и пример его применения | [0182-01-pid-library-and-application.md](0182-01-pid-library-and-application.md) |
| 0182-02 | 0182 | Библиотечный ПИД-регулятор и пример его применения | [0182-02-pid-library-and-application.md](0182-02-pid-library-and-application.md) |
| 0182-03 | 0182 | Библиотечный ПИД-регулятор и пример его применения | [0182-03-pid-library-and-application.md](0182-03-pid-library-and-application.md) |
| 0185-01 | 0185 | Лексика, грамматика и АСД объявления parameter | [0185-01-model-parameters.md](0185-01-model-parameters.md) |
| 0185-02 | 0185 | Аргументы инстанцирования и их диагностики | [0185-02-model-parameters.md](0185-02-model-parameters.md) |
| 0185-03 | 0185 | Константный вычислитель и константные функции | [0185-03-model-parameters.md](0185-03-model-parameters.md) |
| 0185-04 | 0185 | Режим assign: применение аргументов пятью потребителями и флаг CLI | [0185-04-model-parameters.md](0185-04-model-parameters.md) |
| 0185-05 | 0185 | Режим specialize: копия модели, детерминированные имена, дедупликация | [0185-05-model-parameters.md](0185-05-model-parameters.md) |
| 0185-06 | 0185 | Параметризация моделей (ключевое слово parameter) | [0185-06-model-parameters.md](0185-06-model-parameters.md) |
| 0185-07 | 0185 | Параметризация моделей (ключевое слово parameter) | [0185-07-model-parameters.md](0185-07-model-parameters.md) |
| 0185-08 | 0185 | Параметризация моделей (ключевое слово parameter) | [0185-08-model-parameters.md](0185-08-model-parameters.md) |
| 0185-09 | 0185 | Параметризация моделей (ключевое слово parameter) | [0185-09-model-parameters.md](0185-09-model-parameters.md) |
| 0186-01 | 0186 | Раздел документа «Практический пример: процессор» (шины, кеш, ядра) | [0186-01-book-processor-example.md](0186-01-book-processor-example.md) |
| 0186-02 | 0186 | Раздел документа «Практический пример: процессор» (шины, кеш, ядра) | [0186-02-book-processor-example.md](0186-02-book-processor-example.md) |
| 0186-03 | 0186 | Раздел документа «Практический пример: процессор» (шины, кеш, ядра) | [0186-03-book-processor-example.md](0186-03-book-processor-example.md) |
| 0186-04 | 0186 | Раздел документа «Практический пример: процессор» (шины, кеш, ядра) | [0186-04-book-processor-example.md](0186-04-book-processor-example.md) |
| 0186-05 | 0186 | Раздел документа «Практический пример: процессор» (шины, кеш, ядра) | [0186-05-book-processor-example.md](0186-05-book-processor-example.md) |
| 0186-06 | 0186 | Раздел документа «Практический пример: процессор» (шины, кеш, ядра) | [0186-06-book-processor-example.md](0186-06-book-processor-example.md) |
| 0157-01 | 0157 | Представление числового литерала: полная маска [bit;64] | [0157-01-literal-u64-representation.md](0157-01-literal-u64-representation.md) |
| 0157-03 | 0157 | Представление числового литерала: полная маска [bit;64] | [0157-03-literal-u64-representation.md](0157-03-literal-u64-representation.md) |
| 0157-04 | 0157 | Представление числового литерала: полная маска [bit;64] | [0157-04-literal-u64-representation.md](0157-04-literal-u64-representation.md) |
| 0157-02 | 0157 | Представление числового литерала: полная маска [bit;64] | [0157-02-literal-u64-representation.md](0157-02-literal-u64-representation.md) |
| 0157-05 | 0157 | Представление числового литерала: полная маска [bit;64] | [0157-05-literal-u64-representation.md](0157-05-literal-u64-representation.md) |
| 0157-06 | 0157 | Представление числового литерала: полная маска [bit;64] | [0157-06-literal-u64-representation.md](0157-06-literal-u64-representation.md) |
| 0156-01 | 0156 | Снятие клона поддерева в действии грамматики Precedence0 | [0156-01-parser-depth-limit.md](0156-01-parser-depth-limit.md) |
| 0156-02 | 0156 | Ограничение глубины вложенности на уровне лексера/парсера | [0156-02-parser-depth-limit.md](0156-02-parser-depth-limit.md) |
| 0156-03 | 0156 | Ограничение глубины вложенности на уровне лексера/парсера | [0156-03-parser-depth-limit.md](0156-03-parser-depth-limit.md) |
| 0156-04 | 0156 | Ограничение глубины вложенности на уровне лексера/парсера | [0156-04-parser-depth-limit.md](0156-04-parser-depth-limit.md) |
| 0156-05 | 0156 | Ограничение глубины вложенности на уровне лексера/парсера | [0156-05-parser-depth-limit.md](0156-05-parser-depth-limit.md) |
| 0156-06 | 0156 | Ограничение глубины вложенности на уровне лексера/парсера | [0156-06-parser-depth-limit.md](0156-06-parser-depth-limit.md) |
| 0176-01 | 0176 | Позиция бита у bit-порта с голым адресом | [0176-01-bit-port-address-position.md](0176-01-bit-port-address-position.md) |
| 0176-02 | 0176 | Позиция бита у bit-порта с голым адресом | [0176-02-bit-port-address-position.md](0176-02-bit-port-address-position.md) |
| 0176-03 | 0176 | Позиция бита у bit-порта с голым адресом | [0176-03-bit-port-address-position.md](0176-03-bit-port-address-position.md) |
| 0176-04 | 0176 | Позиция бита у bit-порта с голым адресом | [0176-04-bit-port-address-position.md](0176-04-bit-port-address-position.md) |
| 0188-01 | 0188 | Направление порта проверяется во всех позициях | [0188-01-port-direction-everywhere.md](0188-01-port-direction-everywhere.md) |
| 0187-07 | 0187 | Пересмотр задания адресов и доступа к портам | [0187-07-port-io-redesign.md](0187-07-port-io-redesign.md) |
| 0187-02 | 0187 | Пересмотр задания адресов и доступа к портам | [0187-02-port-io-redesign.md](0187-02-port-io-redesign.md) |
| 0187-03 | 0187 | Пересмотр задания адресов и доступа к портам | [0187-03-port-io-redesign.md](0187-03-port-io-redesign.md) |
| 0187-04 | 0187 | начальное значение порта в целях sv, sv-mmio, st и st-at | [0187-04-port-io-redesign.md](0187-04-port-io-redesign.md) |
| 0187-05 | 0187 | симулятор и потактовая сверка начального значения порта | [0187-05-port-io-redesign.md](0187-05-port-io-redesign.md) |
| 0187-06 | 0187 | нормированная модель доступа к порту (ось 4) | [0187-06-port-io-redesign.md](0187-06-port-io-redesign.md) |
| 0187-08 | 0187 | редакторский слой: LSP и плагин IntelliJ (правило 29) | [0187-08-port-io-redesign.md](0187-08-port-io-redesign.md) |
| 0190-01 | 0190 | параллельные тесты: каталог теста уникален по тесту | [0190-01-precheck-selective-gates.md](0190-01-precheck-selective-gates.md) |
| 0196-01 | 0196 | Подсветка имён типов отдельным цветом в LSP и плагинах | [0196-01-editor-type-highlighting.md](0196-01-editor-type-highlighting.md) |
| 0191-01 | 0191 | Цель st: потактовая сверка с эталоном и устранение расхождений | [0191-01-st-per-tick-conformance.md](0191-01-st-per-tick-conformance.md) |
| 0191-02 | 0191 | Цель st: потактовая сверка с эталоном и устранение расхождений | [0191-02-st-per-tick-conformance.md](0191-02-st-per-tick-conformance.md) |
| 0191-03 | 0191 | Цель st: потактовая сверка с эталоном и устранение расхождений | [0191-03-st-per-tick-conformance.md](0191-03-st-per-tick-conformance.md) |
| 0192-01 | 0192 | Константное выражение в инициализаторе объявления | [0192-01-const-init-fold.md](0192-01-const-init-fold.md) |
| 0192-02 | 0192 | Константное выражение в инициализаторе объявления | [0192-02-const-init-fold.md](0192-02-const-init-fold.md) |
| 0193-01 | 0193 | Цели rust и sv: одноимённые константы разных моделей | [0193-01-shared-const-qualified.md](0193-01-shared-const-qualified.md) |
| 0193-02 | 0193 | Цели rust и sv: одноимённые константы разных моделей | [0193-02-shared-const-qualified.md](0193-02-shared-const-qualified.md) |
| 0194-01 | 0194 | Симулятор теряет model-level always у модели-композиции | [0194-01-sim-composition-model-always.md](0194-01-sim-composition-model-always.md) |
| 0194-02 | 0194 | Симулятор теряет model-level always у модели-композиции | [0194-02-sim-composition-model-always.md](0194-02-sim-composition-model-always.md) |
| 0195-01 | 0195 | Коллизии имён при отображении в пространство имён цели | [0195-01-target-name-collisions.md](0195-01-target-name-collisions.md) |
| 0195-02 | 0195 | Коллизии имён при отображении в пространство имён цели | [0195-02-target-name-collisions.md](0195-02-target-name-collisions.md) |
| 0198-01 | 0198 | Форматтер выносит комментарий из тела блока наружу | [0198-01-formatter-comment-in-block.md](0198-01-formatter-comment-in-block.md) |
| 0198-02 | 0198 | Форматтер выносит комментарий из тела блока наружу | [0198-02-formatter-comment-in-block.md](0198-02-formatter-comment-in-block.md) |
| 0199-01 | 0199 | Форма model M = A & B { … } не работает ни в одной стороне | [0199-01-model-implements-brace-form.md](0199-01-model-implements-brace-form.md) |
| 0199-02 | 0199 | Форма model M = A & B { … } не работает ни в одной стороне | [0199-02-model-implements-brace-form.md](0199-02-model-implements-brace-form.md) |
| 0200-01 | 0200 | Идентификатор с не-ASCII буквами: язык принимает, цели sv и st не выражают | [0200-01-non-ascii-identifier-targets.md](0200-01-non-ascii-identifier-targets.md) |
| 0200-02 | 0200 | Идентификатор с не-ASCII буквами: язык принимает, цели sv и st не выражают | [0200-02-non-ascii-identifier-targets.md](0200-02-non-ascii-identifier-targets.md) |
| 0201-01 | 0201 | Изъятие string и template из лексера и редакторского слоя | [0201-01-dead-lexemes.md](0201-01-dead-lexemes.md) |
| 0201-02 | 0201 | Мёртвый механизм pragma и терминалы extern-блока | [0201-02-dead-lexemes.md](0201-02-dead-lexemes.md) |
| 0201-03 | 0201 | Сторож согласованности лексера и грамматики; документ | [0201-03-dead-lexemes.md](0201-03-dead-lexemes.md) |
| 0202-01 | 0202 | Диагностика разбора доезжает до вызывающего структурой | [0202-01-fmt-diagnostic-formatting.md](0202-01-fmt-diagnostic-formatting.md) |
| 0152-01 | 0152 | Накопление диагностик по соседям в стадиях 4-6 | [0152-01-semantic-recovery-element-boundary.md](0152-01-semantic-recovery-element-boundary.md) |
| 0197-01 | 0197 | Раздел документа о стиле кода | [0197-01-language-code-style.md](0197-01-language-code-style.md) |
| 0226-01 | 0226 | Слой стиля: предупреждение CS-001 в fmt и LSP | [0226-01-naming-convention-warning.md](0226-01-naming-convention-warning.md) |
| 0226-02 | 0226 | Корпус приведён к канону именования | [0226-02-corpus-naming-canon.md](0226-02-corpus-naming-canon.md) |
| 0226-03 | 0226 | Тесты проводки CS-001 в fmt и LSP | [0226-03-naming-warning-wiring-tests.md](0226-03-naming-warning-wiring-tests.md) |
| 0226-04 | 0226 | Раздел «Стиль кода» и запись CS-001 в приложение | [0226-04-code-style-doc.md](0226-04-code-style-doc.md) |
| 0227-01 | 0227 | Редактор показывает CS-001 и при ошибках в файле | [0227-01-lsp-style-warning-with-errors.md](0227-01-lsp-style-warning-with-errors.md) |
| 0228-01 | 0228 | Предупреждение taktc compile несёт позицию | [0228-01-compile-warning-position.md](0228-01-compile-warning-position.md) |
| 0229-01 | 0229 | Отказ форматтера — диагностика с позицией | [0229-01-format-unsupported-position.md](0229-01-format-unsupported-position.md) |
| 0230-01 | 0230 | Сторож форматтера: корпус восстановлен, KNOWN_GAPS с ратчетом | [0230-01-format-corpus-sentinel.md](0230-01-format-corpus-sentinel.md) |
| 0231-01 | 0231 | Текст диагностики без внутреннего представления | [0231-01-diagnostic-text-no-debug.md](0231-01-diagnostic-text-no-debug.md) |
| 0232-01 | 0232 | Предупреждение о неявной булевости доезжает до пользователя | [0232-01-implicit-bool-warning-delivery.md](0232-01-implicit-bool-warning-delivery.md) |
| 0233-01 | 0233 | Правило булевости условия — одно | [0233-01-single-boolean-predicate.md](0233-01-single-boolean-predicate.md) |
| 0151-01 | 0151 | Накопление диагностик внутри отдельной проверки validate | [0151-01-diagnostics-batch-within-check.md](0151-01-diagnostics-batch-within-check.md) |
| 0160-01 | 0160 | Синхронизация эталона Takt.ebnf с актуальным синтаксисом | [0160-01-takt-ebnf-sync.md](0160-01-takt-ebnf-sync.md) |
| 0160-02 | 0160 | Синхронизация эталона Takt.ebnf с актуальным синтаксисом | [0160-02-takt-ebnf-sync.md](0160-02-takt-ebnf-sync.md) |
| 0160-03 | 0160 | Синхронизация эталона Takt.ebnf с актуальным синтаксисом | [0160-03-takt-ebnf-sync.md](0160-03-takt-ebnf-sync.md) |
| 0170-01 | 0170 | Насыщение (saturation) для fixed-point q(m, n) | [0170-01-fixed-point-saturation.md](0170-01-fixed-point-saturation.md) |
| 0170-02 | 0170 | Насыщение (saturation) для fixed-point q(m, n) | [0170-02-fixed-point-saturation.md](0170-02-fixed-point-saturation.md) |
| 0170-03 | 0170 | Насыщение (saturation) для fixed-point q(m, n) | [0170-03-fixed-point-saturation.md](0170-03-fixed-point-saturation.md) |
| 0170-04 | 0170 | Насыщение (saturation) для fixed-point q(m, n) | [0170-04-fixed-point-saturation.md](0170-04-fixed-point-saturation.md) |
| 0170-05 | 0170 | Насыщение (saturation) для fixed-point q(m, n) | [0170-05-fixed-point-saturation.md](0170-05-fixed-point-saturation.md) |
| 0203-01 | 0203 | validate не обходит формулы: неизвестное имя в Guard молчит | [0203-01-validate-formulas-traversal.md](0203-01-validate-formulas-traversal.md) |
| 0203-02 | 0203 | validate не обходит формулы: неизвестное имя в Guard молчит | [0203-02-validate-formulas-traversal.md](0203-02-validate-formulas-traversal.md) |
| 0203-03 | 0203 | validate не обходит формулы: неизвестное имя в Guard молчит | [0203-03-validate-formulas-traversal.md](0203-03-validate-formulas-traversal.md) |
| 0234-01 | 0234 | Профилирование и ускорение предкоммита | [0234-01-precheck-time-profile.md](0234-01-precheck-time-profile.md) |
| 0234-02 | 0234 | Профилирование и ускорение предкоммита | [0234-02-precheck-time-profile.md](0234-02-precheck-time-profile.md) |
| 0234-03 | 0234 | Профилирование и ускорение предкоммита | [0234-03-precheck-time-profile.md](0234-03-precheck-time-profile.md) |
| 0235-01 | 0235 | Цели st и sv теряют охранную формулу | [0235-01-guard-formula-in-st-sv.md](0235-01-guard-formula-in-st-sv.md) |
| 0235-02 | 0235 | Цели st и sv теряют охранную формулу | [0235-02-guard-formula-in-st-sv.md](0235-02-guard-formula-in-st-sv.md) |
| 0235-03 | 0235 | Цели st и sv теряют охранную формулу | [0235-03-guard-formula-in-st-sv.md](0235-03-guard-formula-in-st-sv.md) |
| 0236-01 | 0236 | Печатник цели c печатает пустоту на неразрешённом условии | [0236-01-c-unresolved-condition-refusal.md](0236-01-c-unresolved-condition-refusal.md) |
| 0238-01 | 0238 | Живой контекст: раздел критических инвариантов дублирует подводные камни | [0238-01-claude-md-duplicate-invariants.md](0238-01-claude-md-duplicate-invariants.md) |
| 0204-01 | 0204 | Вывод типов не протягивает тип через ссылку константа-константа | [0204-01-const-ref-type-inference.md](0204-01-const-ref-type-inference.md) |
| 0205-01 | 0205 | Приведение as не вычисляется в инициализаторе объявления | [0205-01-as-in-declaration-initializer.md](0205-01-as-in-declaration-initializer.md) |
| 0206-01 | 0206 | Вариант импортированного перечисления не разрешается в образце match | [0206-01-imported-enum-variant-in-match.md](0206-01-imported-enum-variant-in-match.md) |
| 0207-01 | 0207 | Отрицание ~0 для беззнакового типа: два правила языка столкнулись | [0207-01-bitwise-not-unsigned-literal.md](0207-01-bitwise-not-unsigned-literal.md) |
| 0208-01 | 0208 | Три константных вычислителя компилятора живут порознь | [0208-01-const-evaluators-unification.md](0208-01-const-evaluators-unification.md) |
| 0209-01 | 0209 | Внешний интерфейс модели: extern fn в симуляторе и агрегат как аргумент | [0209-01-model-external-interface.md](0209-01-model-external-interface.md) |
| 0209-02 | 0209 | Внешний интерфейс модели: extern fn в симуляторе и агрегат как аргумент | [0209-02-model-external-interface.md](0209-02-model-external-interface.md) |
| 0172-01 | 0172 | Семантика перечисления без вариантов | [0172-01-empty-enum-semantics.md](0172-01-empty-enum-semantics.md) |
| 0172-02 | 0172 | Семантика перечисления без вариантов | [0172-02-empty-enum-semantics.md](0172-02-empty-enum-semantics.md) |
| 0168-01 | 0168 | Предупреждения генераторов возвращаются вызывающему | [0168-01-generator-warnings-return.md](0168-01-generator-warnings-return.md) |
| 0168-02 | 0168 | Предупреждения генераторов возвращаются вызывающему | [0168-02-generator-warnings-return.md](0168-02-generator-warnings-return.md) |
| 0167-01 | 0167 | Цель c использует объявленные константы перечисления | [0167-01-c-enum-constants-usage.md](0167-01-c-enum-constants-usage.md) |
| 0169-01 | 0169 | Адаптер шины APB | [0169-01-sv-mmio-bus-adapters.md](0169-01-sv-mmio-bus-adapters.md) |
| 0210-01 | 0210 | Массив как общая переменная в цели st; индекс-выражение | [0210-01-st-array-shared-and-index.md](0210-01-st-array-shared-and-index.md) |
| 0211-01 | 0211 | Модель без стартового состояния: цель c отказывает бессодержательно | [0211-01-c-missing-start-state-diagnostic.md](0211-01-c-missing-start-state-diagnostic.md) |
| 0212-01 | 0212 | Диагностика цели c без кода | [0212-01-c-diagnostic-without-code.md](0212-01-c-diagnostic-without-code.md) |
| 0239-01 | 0239 | Скрипт релизной сборки и установки инструментов | [0239-01-install-script.md](0239-01-install-script.md) |
| 0241-01 | 0241 | Ускорение предкоммита | [0241-01-precheck-speedup.md](0241-01-precheck-speedup.md) |
| 0213-01 | 0213 | Цель c печатает лишний break после безусловного перехода | [0213-01-c-redundant-break.md](0213-01-c-redundant-break.md) |
| 0244-01 | 0244 | Отладочная информация тестовых целей — line-tables-only | [0244-01-test-target-build-cost.md](0244-01-test-target-build-cost.md) |
| 0244-02 | 0244 | Слияние 147 тестовых целей в 12 | [0244-02-test-target-build-cost.md](0244-02-test-target-build-cost.md) |
| 0243-01 | 0243 | Воронка занятия имени типа | [0243-01-type-redefinition-diagnostic.md](0243-01-type-redefinition-diagnostic.md) |
| 0214-01 | 0214 | Сигналы записи по составу портов | [0214-01-sv-mmio-unused-write-signals.md](0214-01-sv-mmio-unused-write-signals.md) |
| 0240-01 | 0240 | Перевод документа book/ в формат Typst | [0240-01-book-typst.md](0240-01-book-typst.md) |
| 0240-02 | 0240 | Перевод документа book/ в формат Typst | [0240-02-book-typst.md](0240-02-book-typst.md) |
| 0240-03 | 0240 | Перевод документа book/ в формат Typst | [0240-03-book-typst.md](0240-03-book-typst.md) |
| 0240-04 | 0240 | Перевод документа book/ в формат Typst | [0240-04-book-typst.md](0240-04-book-typst.md) |
| 0245-01 | 0245 | Симулятор исполняет S(Модель) — проверку состояния под-модели | [0245-01-sim-state-of-model.md](0245-01-sim-state-of-model.md) |
| 0237-01 | 0237 | Раздел «Импорты» не описывает S(Модель) | [0237-01-book-state-of-model-section.md](0237-01-book-state-of-model-section.md) |
| 0246-01 | 0246 | Ссылка вперёд в инициализаторе переменной — ошибка компиляции | [0246-01-init-forward-reference.md](0246-01-init-forward-reference.md) |
| 0222-01 | 0222 | Раздел документа о свёртке инициализатора | [0222-01-book-variables-const-fold.md](0222-01-book-variables-const-fold.md) |
| 0223-01 | 0223 | Три примера объясняют выходной порт устаревшей нуждой цели rust | [0223-01-examples-port-rationale-stale.md](0223-01-examples-port-rationale-stale.md) |
| 0224-01 | 0224 | Подъём Kotlin в плагине intellij-takt снимет ограничение на пусковой JDK | [0224-01-intellij-kotlin-upgrade.md](0224-01-intellij-kotlin-upgrade.md) |
| 0225-01 | 0225 | Модуль semantic/statement.rs — 999 строк при пределе 1000 | [0225-01-statement-module-size.md](0225-01-statement-module-size.md) |
| 0221-01 | 0221 | Панель структуры: инвариант состояния символом не становится | [0221-01-lsp-state-invariant-symbol.md](0221-01-lsp-state-invariant-symbol.md) |
| 0220-01 | 0220 | Флаг -Wextra для гейта цели c: 38 предупреждений одного класса | [0220-01-c-gate-wextra.md](0220-01-c-gate-wextra.md) |
| 0218-01 | 0218 | Реестры стадий 5 и 6 хранят заготовочное СОЗДАНА в колонках вердикта | [0218-01-registry-verdict-placeholder.md](0218-01-registry-verdict-placeholder.md) |
| 0217-01 | 0217 | Ветвь-заглушка до будущей задачи переживает саму задачу | [0217-01-stub-branch-gate.md](0217-01-stub-branch-gate.md) |
| 0248-01 | 0248 | Встроенные функции min/max/abs/clamp/debug не исполняются эталоном | [0248-01-sim-builtin-functions.md](0248-01-sim-builtin-functions.md) |
| 0247-01 | 0247 | Голое имя состояния и модели в условии не исполняется эталоном | [0247-01-sim-bare-state-condition.md](0247-01-sim-bare-state-condition.md) |
| 0249-01 | 0249 | Судья места записи: SE-111 и SE-112 | [0249-01-assign-to-call-place.md](0249-01-assign-to-call-place.md) |
| 0250-01 | 0250 | Запись разряда | [0250-01-bit-write-in-targets.md](0250-01-bit-write-in-targets.md) |
| 0250-02 | 0250 | Запись разряда | [0250-02-bit-write-in-targets.md](0250-02-bit-write-in-targets.md) |
| 0250-03 | 0250 | Запись разряда | [0250-03-bit-write-in-targets.md](0250-03-bit-write-in-targets.md) |
| 0145-01 | 0145 | Метрика потолка: рёбра вместо вершин | [0145-01-verify-vertex-budget.md](0145-01-verify-vertex-budget.md) |
| 0145-02 | 0145 | Бенч роста, документ и контекст | [0145-02-verify-vertex-budget.md](0145-02-verify-vertex-budget.md) |
| 0153-01 | 0153 | Слой рабочей области: обход, граф импортов, связывание | [0153-01-lsp-workspace-index.md](0153-01-lsp-workspace-index.md) |
| 0153-02 | 0153 | references и rename по рабочей области | [0153-02-lsp-workspace-index.md](0153-02-lsp-workspace-index.md) |
| 0153-03 | 0153 | Бинарник, контекст и документ | [0153-03-lsp-workspace-index.md](0153-03-lsp-workspace-index.md) |
| 0251-01 | 0251 | Настройка, документация и проверки | [0251-01-cargo-target-dir.md](0251-01-cargo-target-dir.md) |
| 0154-01 | 0154 | Снятие PSI-переименования и сторожа | [0154-01-intellij-server-rename.md](0154-01-intellij-server-rename.md) |
| 0150-01 | 0150 | SIM-037, гейт репозитория и перевод примера документа | [0150-01-sim-positional-scenario-deprecation.md](0150-01-sim-positional-scenario-deprecation.md) |
| 0158-01 | 0158 | Конфигурации запуска, командная строка и фильтр вывода | [0158-01-intellij-run-configurations.md](0158-01-intellij-run-configurations.md) |
| 0165-01 | 0165 | Подкоманда version и её синонимы | [0165-01-taktc-version-subcommand.md](0165-01-taktc-version-subcommand.md) |
| 0161-01 | 0161 | Ренейм старых имён и гейт запрета | [0161-01-fixture-comments-rename.md](0161-01-fixture-comments-rename.md) |
| 0162-01 | 0162 | Метки версий и сторож правила 22 | [0162-01-git-tag-v040.md](0162-01-git-tag-v040.md) |
| 0266-01 | 0266 | SE-113 в общей воронке с SE-099 | [0266-01-port-in-declaration-initializer.md](0266-01-port-in-declaration-initializer.md) |
| 0291-01 | 0291 | Предикат is_unconditional и сторож | [0291-01-rust-sv-unresolved-condition.md](0291-01-rust-sv-unresolved-condition.md) |
| 0300-01 | 0300 | Точная десятичная свёртка и SE-114 | [0300-01-fractional-init-arithmetic.md](0300-01-fractional-init-arithmetic.md) |
| 0284-01 | 0284 | SE-115 на объявлении структуры | [0284-01-empty-struct-semantics.md](0284-01-empty-struct-semantics.md) |
| 0301-01 | 0301 | probe.sh, target-dir.sh и правило 30 | [0301-01-probe-checklist.md](0301-01-probe-checklist.md) |
| 0302-01 | 0302 | release-check.sh, release-notes.sh и workflow | [0302-01-release-on-language-minor.md](0302-01-release-on-language-minor.md) |
| 0285-01 | 0285 | Расширение выведенного типа по результату | [0285-01-inferred-width-from-result.md](0285-01-inferred-width-from-result.md) |
| 0287-01 | 0287 | Расширение типов не знает именованных целых | [0287-01-wider-type-array-literal.md](0287-01-wider-type-array-literal.md) |
| 0262-01 | 0262 | Широкий бит-вектор в цели c | [0262-01-wide-bit-vector-c-rust.md](0262-01-wide-bit-vector-c-rust.md) |
| 0262-02 | 0262 | Широкий бит-вектор в цели rust | [0262-02-wide-bit-vector-c-rust.md](0262-02-wide-bit-vector-c-rust.md) |
| 0263-01 | 0263 | Приведение индекса к usize по нужде | [0263-01-rust-literal-index-cast.md](0263-01-rust-literal-index-cast.md) |
| 0281-01 | 0281 | Сравнение перечисления с числом в цели rust | [0281-01-rust-enum-compare-literal.md](0281-01-rust-enum-compare-literal.md) |
| 0299-01 | 0299 | Не-ASCII имя в нижнем регистре у цели rust | [0299-01-rust-non-ascii-lowercase-name.md](0299-01-rust-non-ascii-lowercase-name.md) |
| 0295-01 | 0295 | Хвостовой комментарий тела и его хозяин | [0295-01-format-element-comment-binding.md](0295-01-format-element-comment-binding.md) |
| 0279-01 | 0279 | Подсказка о выборочном импорте | [0279-01-qualified-import-model-reference.md](0279-01-qualified-import-model-reference.md) |
| 0279-02 | 0279 | Печать примечаний в takt-sim | [0279-02-qualified-import-model-reference.md](0279-02-qualified-import-model-reference.md) |
| 0264-01 | 0264 | Координата судей тела — позиция употребления | [0264-01-body-judge-usage-position.md](0264-01-body-judge-usage-position.md) |
| 0273-01 | 0273 | Недостижимое ребро — предупреждение SE-116 | [0273-01-unreachable-edge-warning.md](0273-01-unreachable-edge-warning.md) |
| 0276-01 | 0276 | Диагностика семантики без кода и позиции | [0276-01-semantic-diagnostics-without-code.md](0276-01-semantic-diagnostics-without-code.md) |
| 0277-01 | 0277 | Координата отказа цели — место употребления | [0277-01-expression-usage-position.md](0277-01-expression-usage-position.md) |
| 0282-01 | 0282 | Собственная позиция формулы | [0282-01-formula-own-location.md](0282-01-formula-own-location.md) |
| 0296-01 | 0296 | Одна воронка стадий для пути импорта | [0296-01-semantic-stages-single-source.md](0296-01-semantic-stages-single-source.md) |
| 0296-02 | 0296 | Подъём частоты подключённого файла | [0296-02-semantic-stages-single-source.md](0296-02-semantic-stages-single-source.md) |
| 0296-03 | 0296 | SE-120 — специализация модели из другого файла | [0296-03-semantic-stages-single-source.md](0296-03-semantic-stages-single-source.md) |
| 0278-01 | 0278 | Снятие мёртвой упаковки последовательной композиции | [0278-01-compact-implement-dead-branch.md](0278-01-compact-implement-dead-branch.md) |
| 0278-02 | 0278 | Публичность перестаёт прятать мёртвое | [0278-02-compact-implement-dead-branch.md](0278-02-compact-implement-dead-branch.md) |
| 0260-01 | 0260 | Заглушка неиспользуемого параметра | [0260-01-c-unused-struct-parameter.md](0260-01-c-unused-struct-parameter.md) |
| 0267-01 | 0267 | Перевод формы в целях sv | [0267-01-state-of-model-in-targets.md](0267-01-state-of-model-in-targets.md) |
| 0267-02 | 0267 | Точные отказы rust и st | [0267-02-state-of-model-in-targets.md](0267-02-state-of-model-in-targets.md) |
| 0303-01 | 0303 | Рёбра состояния-композиции в целях | [0303-01-composition-state-conditional-edge.md](0303-01-composition-state-conditional-edge.md) |
| 0286-01 | 0286 | Вычислимое приведение в общем слое и цели sv | [0286-01-sv-const-initializer-expression.md](0286-01-sv-const-initializer-expression.md) |
| 0293-01 | 0293 | Структуры в цели sv | [0293-01-struct-in-st-rust.md](0293-01-struct-in-st-rust.md) |
| 0293-02 | 0293 | Инициализатор структуры в цели st | [0293-02-struct-in-st-rust.md](0293-02-struct-in-st-rust.md) |
| 0293-03 | 0293 | Структуры в цели rust | [0293-03-struct-in-st-rust.md](0293-03-struct-in-st-rust.md) |
| 0253-01 | 0253 | Старое имя языка в порождаемом коде (lam_q_*, LAM_Q_*) | [0253-01-legacy-names-in-generated-code.md](0253-01-legacy-names-in-generated-code.md) |
| 0253-02 | 0253 | Старое имя языка в порождаемом коде (lam_q_*, LAM_Q_*) | [0253-02-legacy-names-in-generated-code.md](0253-02-legacy-names-in-generated-code.md) |
| 0292-01 | 0292 | Код CC-022 обещан комментарием, но не эмитируется никем | [0292-01-cc022-promise-without-emitter.md](0292-01-cc022-promise-without-emitter.md) |
| 0255-01 | 0255 | Коды симулятора, вплавленные в текст, невидимы гейту и реестру | [0255-01-sim-diagnostic-codes-registry.md](0255-01-sim-diagnostic-codes-registry.md) |
| 0255-02 | 0255 | Коды симулятора, вплавленные в текст, невидимы гейту и реестру | [0255-02-sim-diagnostic-codes-registry.md](0255-02-sim-diagnostic-codes-registry.md) |
| 0256-01 | 0256 | Символ формы import { A } объявляется видом Model | [0256-01-lsp-import-binding-kind.md](0256-01-lsp-import-binding-kind.md) |
| 0258-01 | 0258 | Verdict::Unsupported не различает причину отказа | [0258-01-verify-unsupported-reason.md](0258-01-verify-unsupported-reason.md) |
| 0259-01 | 0259 | Встроенные функции языка не описаны в документе book/ | [0259-01-book-builtin-functions.md](0259-01-book-builtin-functions.md) |
| 0268-01 | 0268 | У SE-033 нет описания в приложении «Ошибки» | [0268-01-se033-appendix-description.md](0268-01-se033-appendix-description.md) |
| 0274-01 | 0274 | Снимки порождённого кода в book/ никем не сверяются | [0274-01-book-generated-snapshots-gate.md](0274-01-book-generated-snapshots-gate.md) |
| 0275-01 | 0275 | Команды в README.md никем не проверяются | [0275-01-readme-commands-gate.md](0275-01-readme-commands-gate.md) |
| 0275-02 | 0275 | Команды в README.md никем не проверяются | [0275-02-readme-commands-gate.md](0275-02-readme-commands-gate.md) |
| 0290-01 | 0290 | Гейт сверки кодов документа с реестром | [0290-01-book-diagnostics-codes-gate.md](0290-01-book-diagnostics-codes-gate.md) |
| 0290-02 | 0290 | Приложение дополнено недостающими кодами | [0290-02-book-diagnostics-codes-gate.md](0290-02-book-diagnostics-codes-gate.md) |
| 0298-01 | 0298 | Гейт лексики расширен разделом «Лексика» | [0298-01-book-lexicon-lists-sync.md](0298-01-book-lexicon-lists-sync.md) |
| 0298-02 | 0298 | Таблица ключевых слов дополнена словами времени и at | [0298-02-book-lexicon-lists-sync.md](0298-02-book-lexicon-lists-sync.md) |
| 0215-01 | 0215 | Сверка значений duration цели st | [0215-01-duration-per-tick-conformance-st-sv.md](0215-01-duration-per-tick-conformance-st-sv.md) |
| 0215-02 | 0215 | Сверка значений duration цели sv | [0215-02-duration-per-tick-conformance-st-sv.md](0215-02-duration-per-tick-conformance-st-sv.md) |
| 0216-01 | 0216 | Сторож поведения печатника живости | [0216-01-rust-live-printer-coverage.md](0216-01-rust-live-printer-coverage.md) |
| 0216-02 | 0216 | match и for-init признаются перезаписью | [0216-02-rust-live-printer-coverage.md](0216-02-rust-live-printer-coverage.md) |
| 0254-01 | 0254 | Переименование служебных идентификаторов | [0254-01-legacy-names-internal-identifiers.md](0254-01-legacy-names-internal-identifiers.md) |
| 0254-02 | 0254 | Гейт ловит служебные имена | [0254-02-legacy-names-internal-identifiers.md](0254-02-legacy-names-internal-identifiers.md) |
| 0269-01 | 0269 | Определения подсветки ST и EBNF | [0269-01-book-st-syntax-highlight.md](0269-01-book-st-syntax-highlight.md) |
| 0269-02 | 0269 | Гейт языков блоков кода | [0269-02-book-st-syntax-highlight.md](0269-02-book-st-syntax-highlight.md) |
| 0270-01 | 0270 | Сборка PDF без тегов доступности | [0270-01-book-pdf-size.md](0270-01-book-pdf-size.md) |
| 0283-01 | 0283 | Слияние report_simple_result и report_hal_result | [0283-01-cli-report-result-merge.md](0283-01-cli-report-result-merge.md) |
