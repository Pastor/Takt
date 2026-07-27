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
