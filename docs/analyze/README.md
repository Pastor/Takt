# Реестр аналитики

Стадия 3 жизненного цикла (правило 17 в [../RULE.md](../RULE.md)). Анализ фичи
`XXXX-slug.md`; при большом объёме декомпозируется на `XXXX-YY-slug.md`, а
базовый документ служит обзором/индексом. На этой стадии аналитик **обязан**
проставить параметр «Зависит от» в карточке фичи и в [FEATURES.md](../../FEATURES.md).

Заготовка создаётся из шаблона [`../templates/analyze.md`](../templates/analyze.md).

| Фича | Заголовок | Документ | Связь с обратной функциональностью |
|------|-----------|----------|------------------------------------|
| 0018 | Приведение кода к docs/CODE.md | [0018-code-guidelines.md](0018-code-guidelines.md) | нет (рефакторинг без слома API) |
| 0019 | Унификация грамматик Condition/Expression | [0019-condition-expression-unification.md](0019-condition-expression-unification.md) | нет (внутренний рефактор грамматики) |
| 0020 | Адрес порта: размещение + потребление (карта адресов) | [0020-port-address-decl.md](0020-port-address-decl.md) | аддитивно для `.lam` (инлайн-форма сохраняется); + внешняя `.ld`-карта, C-эмиссия |
| 0021 | Смена операторов: `<=` присваивание, `=` сравнение | [0021-swap-assign-compare.md](0021-swap-assign-compare.md) | слом (мажорная версия языка + мигратор) |
| 0022 | Плагин IntelliJ IDEA: подсветка синтаксиса Lam | [0022-intellij-syntax-highlight.md](0022-intellij-syntax-highlight.md) | аддитивно (новый подпроект, язык не тронут) |
| 0023 | Плагин IntelliJ IDEA — навигация к декларации и include | [0023-intellij-navigation-include.md](0023-intellij-navigation-include.md) | — (новая фича) |
| 0024 | Канонический форматтер .lam (lamc fmt) | [0024-lam-formatter.md](0024-lam-formatter.md) | аддитивно (только раскладка; язык не тронут) |
| 0025 | Починка вычислителя выражений симулятора | [0025-simulator-expression-eval.md](0025-simulator-expression-eval.md) | язык не тронут (чинится исполнение); меняются результаты симуляции — в сторону спецификации |
| 0026 | Генератор C: typedef корневой структуры для одиночной модели | [0026-c-root-typedef.md](0026-c-root-typedef.md) | — (новая фича) |
| 0027 | Разделение переросших модулей (validate.rs, lsp.rs, c_expr.rs) | [0027-module-size-split.md](0027-module-size-split.md) | — (новая фича) |
| 0028 | Заглушки генератора C: диагностика вместо тихого пропуска | [0028-c-generator-stubs.md](0028-c-generator-stubs.md) | — (новая фича) |
| 0029 | Генератор C: отображение типов Array/Bit/Rational | [0029-c-type-mapping.md](0029-c-type-mapping.md) | — (новая фича) |
| 0030 | Исправление примера comprehensive.lam (недостижимый сценарий) | [0030-comprehensive-example-fix.md](0030-comprehensive-example-fix.md) | — (новая фича) |
| 0031 | Вызов функции из тела функции | [0031-fn-calls-fn.md](0031-fn-calls-fn.md) | — (новая фича) |
| 0032 | Сохранение переменных модели в --save-state/--load-state | [0032-state-io-variables.md](0032-state-io-variables.md) | — (новая фича) |
| 0033 | Согласование тактов симулятора и порождённого C (INIT-такты) | [0033-init-tick-alignment.md](0033-init-tick-alignment.md) | — (новая фича) |
| 0034 | Структурные типы в симуляторе | [0034-sim-struct-types.md](0034-sim-struct-types.md) | — (новая фича) |
| 0035 | LTL-формулы в блоках кода: разбор вместо тихой потери | [0035-ltl-in-blocks.md](0035-ltl-in-blocks.md) | — (новая фича) |
| 0036 | Согласование видимости публичного API крейта simulation | [0036-sim-visibility.md](0036-sim-visibility.md) | — (новая фича) |
| 0037 | Сбои тестов на Windows (пути include, ресурс viewport) | [0037-windows-test-failures.md](0037-windows-test-failures.md) | — (новая фича) |
| 0038 | Семантическая подсветка Lam в IntelliJ через lam-lsp | [0038-intellij-semantic-tokens.md](0038-intellij-semantic-tokens.md) | — (новая фича) |
| 0039 | Действие Reformat Code в плагине IntelliJ | [0039-intellij-reformat.md](0039-intellij-reformat.md) | — (новая фича) |
| 0040 | Полноценный PSI-парсер плагина IntelliJ | [0040-intellij-psi-parser.md](0040-intellij-psi-parser.md) | — (новая фича) |
| 0041 | Бэкенд генерации в Structured Text (IEC 61131-3) | [0041-st-backend.md](0041-st-backend.md) | — (новая фича) |
| 0041 | Анализ 0041-02: Отображение типов Lam → типы IEC 61131-3 (подзадача аналитики) | [0041-02-type-mapping.md](0041-02-type-mapping.md) | см. обзор 0041 |
| 0041 | Анализ 0041-03: Состояния, переходы и композиция моделей в ST (подзадача аналитики) | [0041-03-state-mapping.md](0041-03-state-mapping.md) | см. обзор 0041 |
| 0041 | Анализ 0041-05: Потребление карты адресов — `AddressMap` → `AT %…` (подзадача аналитики) | [0041-05-address-at.md](0041-05-address-at.md) | см. обзор 0041 |
| 0041 | Анализ 0041-06: Проверяемость порождённого ST (проба-гейт MatIEC) (подзадача аналитики) | [0041-06-matiec-validation.md](0041-06-matiec-validation.md) | см. обзор 0041 |
| 0042 | Инъекция define'ов для адресов (--define) | [0042-address-defines.md](0042-address-defines.md) | — (новая фича) |
| 0043 | Экспорт карты адресов во внешний формат | [0043-address-map-export.md](0043-address-map-export.md) | — (новая фича) |
| 0044 | Юнит-конструкции языка для симуляции (assert/invariant) | [0044-sim-assert-invariant.md](0044-sim-assert-invariant.md) | — (новая фича) |
| 0045 | Бэкенд генерации в SystemVerilog | [0045-sv-backend.md](0045-sv-backend.md) | — (новая фича) |
| 0045 | Анализ 0045-02: Проверяемость порождённого SystemVerilog (подзадача аналитики) | [0045-02-validation.md](0045-02-validation.md) | см. обзор 0045 |
| 0045 | Анализ 0045-03: Отображение типов Lam → SystemVerilog (подзадача аналитики) | [0045-03-type-mapping.md](0045-03-type-mapping.md) | см. обзор 0045 |
| 0045 | Анализ 0045-05: Модель времени, сброс, автомат и композиция (подзадача аналитики) | [0045-05-fsm-time-reset.md](0045-05-fsm-time-reset.md) | см. обзор 0045 |
| 0046 | Устранение всех предупреждений сборки (rustc + clippy) | [0046-build-warnings-cleanup.md](0046-build-warnings-cleanup.md) | — (новая фича) |
| 0048 | Детерминированная генерация кода (единый порядок эмиссии) | [0048-deterministic-codegen.md](0048-deterministic-codegen.md) | нет |
| 0049 | Верификация модели (Model Checking) на основе LTL | [0049-model-checking-ltl.md](0049-model-checking-ltl.md) | нет (аддитивно: `lamc verify` — новая оснастка, синтаксис и версия языка не тронуты, вывод генераторов неизменен; зависит от закрытых 0010/0035) |
| 0050 | Бэкенд генерации в Rust | [0050-rust-backend.md](0050-rust-backend.md) | — (новая фича) |
| 0051 | Область проверки lamc verify (--scope) | [0051-verify-scope.md](0051-verify-scope.md) | — (новая фича) |
| 0052 | Итеративные обходы в verification/ (снятие потолка стека) | [0052-verify-iterative-traversal.md](0052-verify-iterative-traversal.md) | — (новая фича) |
| 0053 | Идентификатор файла в позициях диагностик (file_no) | [0053-diagnostics-file-id.md](0053-diagnostics-file-id.md) | — (новая фича) |
| 0054 | Позиции в диагностиках симулятора | [0054-sim-diagnostics-positions.md](0054-sim-diagnostics-positions.md) | — (новая фича) |
| 0055 | Многофайловость LSP: импорты и позиции диагностик | [0055-lsp-multifile.md](0055-lsp-multifile.md) | — (новая фича) |
| 0056 | Точный путь вместо угадывания в goto_declaration | [0056-lsp-goto-exact-file.md](0056-lsp-goto-exact-file.md) | — (новая фича) |
| 0057 | Последовательная композиция (`+`) в цели SystemVerilog | [0057-sv-sequential-composition.md](0057-sv-sequential-composition.md) | аддитивно (SV-002 → поддержано; язык неизменен) |
| 0058 | Хвостовой разворот `return` — заход в завершающий `if/else` | [0058-rust-tail-return-if-else.md](0058-rust-tail-return-if-else.md) | — (новая фича) |
| 0059 | Общие переменные корня → структура `Shared` | [0059-rust-shared-struct.md](0059-rust-shared-struct.md) | — (новая фича) |
| 0060 | Диапазон и знак перечисления — один расчёт на все цели | [0060-enum-width-shared-layer.md](0060-enum-width-shared-layer.md) | — (новая фича) |
| 0061 | Fixed-point Q(m.n) как тип языка | [0061-fixed-point-type.md](0061-fixed-point-type.md) | — (новая фича) |
| 0062 | Цель `sv-mmio` — адреса портов как регистровый файл | [0062-sv-mmio-target.md](0062-sv-mmio-target.md) | — (новая фича) |
| 0063 | Порт `en` (clock enable) для цели `sv` | [0063-sv-clock-enable.md](0063-sv-clock-enable.md) | — (новая фича) |
| 0064 | Предупреждение о делителе (`SV-009`) в цели `sv` | [0064-sv-divider-warning.md](0064-sv-divider-warning.md) | — (новая фича) |
| 0065 | Изоляция пространства имён цели `st` | [0065-st-namespace-isolation.md](0065-st-namespace-isolation.md) | — (новая фича) |
| 0066 | Литералы по целевому типу в телах цели `st` | [0066-st-bool-literals.md](0066-st-bool-literals.md) | — (новая фича) |
| 0068 | Верификация свойств над данными | [0068-verify-data-properties.md](0068-verify-data-properties.md) | — (новая фича) |
| 0069 | Разделение `address_map.rs` | [0069-address-map-eval-split.md](0069-address-map-eval-split.md) | — (новая фича) |
| 0096 | Q-арифметика через нативный float и флаг генерации (embedded ↔ float) | [0096-fixed-point-native-float.md](0096-fixed-point-native-float.md) | — (новая фича) |
| 0097 | Пример ПИД-регулятора на языке Lam (fixed-point) | [0097-pid-regulator-example.md](0097-pid-regulator-example.md) | — (новая фича) |
| 0090 | CI прогоняет весь `precheck.sh` (живые гейты + check-links) | [0090-ci-precheck.md](0090-ci-precheck.md) | — (новая фича) |
| 0070 | Инициализатор порта — это адрес, а не значение | [0070-port-initializer-address-role.md](0070-port-initializer-address-role.md) | — (новая фича) |
| 0071 | Переход на имя состояния в `S(Ping) = End` | [0071-lsp-goto-state-name.md](0071-lsp-goto-state-name.md) | — (новая фича) |
| 0073 | `Location::filename()` возвращает номер, а не путь | [0073-location-filename-path.md](0073-location-filename-path.md) | — (новая фича) |
| 0086 | `var q: u8;` без инициализатора → `SIM-009` | [0086-sim-var-without-initializer.md](0086-sim-var-without-initializer.md) | — (новая фича) |

| 0074 | Скобочная форма `S(…)` отвергается семантикой | [0074-parenthesised-state-of.md](0074-parenthesised-state-of.md) | — (новая фича) |
| 0083 | Тело `always` на уровне модели не эмитится | [0083-model-always-block.md](0083-model-always-block.md) | — (новая фича) |
| 0080 | Дефекты генератора C по структурам | [0080-c-struct-defects.md](0080-c-struct-defects.md) | — (новая фича) |
| 0079 | `elevator_mini.lam` не исполняется: порты под-модели композиции | [0079-sim-composition-ports.md](0079-sim-composition-ports.md) | — (новая фича) |
| 0072 | LSP не читает initializationOptions (пути поиска импортов) | [0072-lsp-initialization-options.md](0072-lsp-initialization-options.md) | — (новая фича) |
| 0076 | Симулятор не исполняет массивы вовсе | [0076-sim-arrays.md](0076-sim-arrays.md) | — (новая фича) |
| 0077 | Реестр кодов диагностик (конфликт `CC-014`) | [0077-diagnostic-code-registry.md](0077-diagnostic-code-registry.md) | — (новая фича) |
| 0078 | Семантика `[bit;N]` расходится втрое | [0078-bit-array-semantics.md](0078-bit-array-semantics.md) | — (новая фича) |
| 0085 | Константа версии языка в коде + гейт синхронизации с README | [0085-language-version-constant.md](0085-language-version-constant.md) | нет (самодостаточна) |
| 0084 | Ключ карты адресов — квалифицированный (модель+порт) | [0084-address-map-qualified-key.md](0084-address-map-qualified-key.md) | нет (аддитивно для `.lam` и корпуса; ключ карты + поле API) |
| 0087 | Мягкий режим инвариантов симулятора (записать и продолжить) | [0087-invariant-soft-mode.md](0087-invariant-soft-mode.md) | аддитивно (умолчание — жёсткий режим 0044 неизменно; opt-in флаг) |
| 0094 | Доработка scripts/new-feature.sh (идемпотентность, статус ADR, поздние стадии) | [0094-new-feature-script-fixes.md](0094-new-feature-script-fixes.md) | нет (инфраструктура; дефолт неизменен) |
| 0093 | Запрет _ => в семантических узлах и разрешение конфликта с #[non_exhaustive] | [0093-wildcard-match-rule.md](0093-wildcard-match-rule.md) | аддитивно (правка свода + гейт; код не тронут) |
| 0091 | Правило о размере модуля переносится в docs/CODE.md | [0091-module-size-rule-in-code-md.md](0091-module-size-rule-in-code-md.md) | нет (документация; код не тронут) |
| 0067 | Rename и PsiReference для import в плагине IntelliJ | [0067-intellij-rename-psi-import.md](0067-intellij-rename-psi-import.md) | — (новая фича) |
| 0088 | Остальные нарушители лимита размера модуля | [0088-module-size-remaining.md](0088-module-size-remaining.md) | — (новая фича) |
| 0089 | Остаточные проверки плагина IntelliJ (0022/0023) | [0089-intellij-residual-checks.md](0089-intellij-residual-checks.md) | — (новая фича) |
| 0092 | У фичи 0018 нет ADR | [0092-adr-0018-retrofit.md](0092-adr-0018-retrofit.md) | — (новая фича) |
| 0100 | Переименование языка Lam → Takt | [0100-language-rename-takt.md](0100-language-rename-takt.md) | — (новая фича) |
| 0101 | Документ описания языка Takt | [0101-language-book.md](0101-language-book.md) | — (новая фича) |
| 0117 | Раздел документа «Инструментарий» | [0117-book-tools.md](0117-book-tools.md) | — (новая фича) |
| 0118 | Раздел документа «Развёрнутый пример» | [0118-book-showcase.md](0118-book-showcase.md) | — (новая фича) |
| 0119 | Приложения документа + приложение «Ошибки» | [0119-book-appendices.md](0119-book-appendices.md) | — (новая фича) |
| 0120 | Заметки о возможных ошибках в разделах документа | [0120-book-error-notes.md](0120-book-error-notes.md) | — (новая фича) |
| 0121 | Разбор примеров в разделах с упором на тему | [0121-book-example-walkthrough.md](0121-book-example-walkthrough.md) | — (новая фича) |
| 0122 | Сборка PDF через latexmk (корректные кросс-ссылки) + Makefile | [0122-book-pdf-latexmk.md](0122-book-pdf-latexmk.md) | — (новая фича) |
| 0123 | Подсветка ключевых слов языка в тексте документа | [0123-book-keyword-highlight.md](0123-book-keyword-highlight.md) | — (новая фича) |
| 0124 | Экспорт графов верификации (Крипке/Бюхи/произведение) в Graphviz DOT | [0124-verify-graph-export.md](0124-verify-graph-export.md) | — (новая фича) |
| 0125 | Плагин IntelliJ Takt: LSP-проверка, автодополнение, документация и настройки инструментов | [0125-intellij-takt-lsp-tooling.md](0125-intellij-takt-lsp-tooling.md) | — (новая фича) |
| 0126 | Сравнительный анализ языка Takt с родственными языками (отчёт docs/DIFF.md) | [0126-language-comparison-diff.md](0126-language-comparison-diff.md) | — (новая фича) |
| 0139 | Удаление мёртвой конфигурации .travis.yml | [0139-remove-travis-config.md](0139-remove-travis-config.md) | — (новая фича) |
| 0137 | Фиксация толчейна Rust и MSRV | [0137-toolchain-pin-msrv.md](0137-toolchain-pin-msrv.md) | — (новая фича) |
| 0128 | Диагностика вместо паники на числовом литерале больше i64::MAX | [0128-lexer-literal-overflow.md](0128-lexer-literal-overflow.md) | — (новая фича) |
| 0129 | Устранение переполнения стека на глубине выражений и операторов | [0129-semantic-deep-nesting.md](0129-semantic-deep-nesting.md) | — (новая фича) |
| 0127 | Единая семантика переполнения целых во всех целях | [0127-int-overflow-semantics.md](0127-int-overflow-semantics.md) | — (новая фича) |
| 0133 | Гейт компиляции и симуляции примеров документа book/ | [0133-book-examples-gate.md](0133-book-examples-gate.md) | — (новая фича) |
| 0135 | Квалифицированные имена портов в симуляторе | [0135-sim-qualified-port-names.md](0135-sim-qualified-port-names.md) | — (новая фича) |
| 0131 | LSP: definition, references и rename | [0131-lsp-definition-references-rename.md](0131-lsp-definition-references-rename.md) | — (новая фича) |
| 0130 | Накопление семантических диагностик | [0130-diagnostics-batch.md](0130-diagnostics-batch.md) | — (новая фича) |
| 0132 | Именованные порты в сценариях симулятора | [0132-sim-named-port-scenarios.md](0132-sim-named-port-scenarios.md) | — (новая фича) |
| 0140 | Ревизия витрины кандидатов и разделение ролей README и book | [0140-backlog-revision-doc-split.md](0140-backlog-revision-doc-split.md) | — (новая фича) |
| 0138 | Измерение покрытия тестами | [0138-coverage-measurement.md](0138-coverage-measurement.md) | — (новая фича) |
| 0136 | Бенчмарки производительности | [0136-perf-benchmarks.md](0136-perf-benchmarks.md) | — (новая фича) |
| 0134 | Модель времени в языке: литерал длительности, внешний источник времени и частота такта | [0134-language-time-model.md](0134-language-time-model.md) | аддитивно (проба: `3s` сегодня даёт `SY-002`) |
| 0134 | Анализ 0134-01: Лексика, грамматика и АСД — литерал, `clock`, `after`/`every` (подзадача аналитики) | [0134-01-lexis-grammar-ast.md](0134-01-lexis-grammar-ast.md) | см. обзор 0134 |
| 0134 | Анализ 0134-02: Тип `duration`, слой пересчёта и диагностики (подзадача аналитики) | [0134-02-type-and-lowering.md](0134-02-type-and-lowering.md) | см. обзор 0134 |
| 0134 | Анализ 0134-03: Симулятор — виртуальные часы, вычислитель, сценарий, трасса (подзадача аналитики) | [0134-03-simulator-clock.md](0134-03-simulator-clock.md) | см. обзор 0134 |
| 0134 | Анализ 0134-04: Цели `c` и `c-hal` — колбэк `now_ms` и счётчики (подзадача аналитики) | [0134-04-target-c.md](0134-04-target-c.md) | см. обзор 0134 |
| 0134 | Анализ 0134-05: Цель `rust` — метод трейта `Hal` (подзадача аналитики) | [0134-05-target-rust.md](0134-05-target-rust.md) | см. обзор 0134 |
| 0134 | Анализ 0134-06: Цели `st` и `st-at` — штатный `TON` (подзадача аналитики) | [0134-06-target-st.md](0134-06-target-st.md) | см. обзор 0134 |
| 0134 | Анализ 0134-07: Цели `sv` и `sv-mmio` — вход `time_ms` и профиль тактов (подзадача аналитики) | [0134-07-target-sv.md](0134-07-target-sv.md) | см. обзор 0134 |
| 0134 | Анализ 0134-08: CLI, сопровождающие слои, документация и свод (подзадача аналитики) | [0134-08-cli-tooling-docs.md](0134-08-cli-tooling-docs.md) | см. обзор 0134 |
| 0178 | Приведение LSP и плагинов в соответствие языку + сторож | [0178-editor-layer-language-sync.md](0178-editor-layer-language-sync.md) | правило 29 свода |
| 0179 | Дочистка URL репозитория после переезда BuT → Takt | [0179-repo-url-cleanup.md](0179-repo-url-cleanup.md) | процессный бэклог |
| 0144 | Экспонента целочисленного литерала: считать или отвергнуть, но не терять молча | [0144-int-literal-exponent.md](0144-int-literal-exponent.md) | кандидат блока 2 FEATURES.md |
| 0155 | Семантическое разрешение тел вложенных операторов | [0155-semantic-nested-statement-resolution.md](0155-semantic-nested-statement-resolution.md) | — (новая фича) |
| 0146 | Гейт символов вне шрифта документа book/ | [0146-book-glyph-gate.md](0146-book-glyph-gate.md) | — (новая фича) |
| 0149 | Гейт согласованности живого контекста CLAUDE.md | [0149-claude-md-consistency-gate.md](0149-claude-md-consistency-gate.md) | — (новая фича) |
| 0180 | Сокращение живого контекста CLAUDE.md | [0180-claude-md-context-diet.md](0180-claude-md-context-diet.md) | — (новая фича) |
| 0177 | Гейт согласованности статуса в реестре и в карточке фичи | [0177-features-registry-status-gate.md](0177-features-registry-status-gate.md) | — (новая фича) |
| 0159 | Фиксация требования JDK 21 для сборки плагина intellij-takt | [0159-intellij-jdk21-build.md](0159-intellij-jdk21-build.md) | — (новая фича) |
| 0171 | Гейт цели c под -Werror | [0171-c-gate-werror.md](0171-c-gate-werror.md) | — (новая фича) |
| 0181 | Симулятор исполняет реализацию состояния с переходом next | [0181-sim-state-implementation-tick.md](0181-sim-state-implementation-tick.md) | — (новая фича) |
| 0182 | Библиотечный ПИД-регулятор и пример его применения | [0182-pid-library-and-application.md](0182-pid-library-and-application.md) | — (новая фича) |
| 0166 | Корпусной SV-транслируемый пример на последовательную композицию + | [0166-sv-example-sequential-composition.md](0166-sv-example-sequential-composition.md) | — (новая фича) |
| 0174 | Цель rust: корневая модель без портов (clippy::new_without_default) | [0174-rust-new-without-default.md](0174-rust-new-without-default.md) | — (новая фича) |
| 0147 | Тесты textDocument/documentSymbol | [0147-lsp-document-symbol-tests.md](0147-lsp-document-symbol-tests.md) | — (новая фича) |
| 0148 | Покрытие печатников цели rust тестами | [0148-rust-printers-coverage.md](0148-rust-printers-coverage.md) | — (новая фича) |
| 0163 | Исчерпывающий разбор узлов во втором вычислителе | [0163-builder-eval-exhaustive.md](0163-builder-eval-exhaustive.md) | — (новая фича) |
| 0143 | `after` принимает константу типа `duration`, а не только литерал | [0143-after-const-duration.md](0143-after-const-duration.md) | — (новая фича) |
| 0183 | Тип `duration` в целях генерации и вычисляемая выдержка | [0183-duration-type-in-targets.md](0183-duration-type-in-targets.md) | — (новая фича) |
| 0184 | Общие переменные библиотечного файла в импортёре | [0184-imported-shared-variables.md](0184-imported-shared-variables.md) | — (новая фича) |
| 0185 | Параметризация моделей (var/const parameter) | [0185-model-parameters.md](0185-model-parameters.md) | — (новая фича) |
| 0186 | Раздел документа «Практический пример: процессор» (шины, кеш, ядра) | [0186-book-processor-example.md](0186-book-processor-example.md) | — (новая фича) |
| 0157 | Представление числового литерала: полная маска [bit;64] | [0157-literal-u64-representation.md](0157-literal-u64-representation.md) | — (новая фича) |
| 0156 | Ограничение глубины вложенности на уровне лексера/парсера | [0156-parser-depth-limit.md](0156-parser-depth-limit.md) | — (новая фича) |
| 0176 | Позиция бита у bit-порта с голым адресом | [0176-bit-port-address-position.md](0176-bit-port-address-position.md) | — (новая фича) |
| 0187 | Пересмотр задания адресов и доступа к портам | [0187-port-io-redesign.md](0187-port-io-redesign.md) | — (новая фича) |
| 0188 | Направление порта проверяется во всех позициях | [0188-port-direction-everywhere.md](0188-port-direction-everywhere.md) | — (новая фича) |
| 0190 | Разделение предкоммита на компоненты и выборочный запуск | [0190-precheck-selective-gates.md](0190-precheck-selective-gates.md) | — (новая фича) |
| 0189 | Анонимные порты | [0189-anonymous-ports.md](0189-anonymous-ports.md) | [ADR 0189](../adr/0189-anonymous-ports.md) |
| 0196 | Подсветка имён типов отдельным цветом в LSP и плагинах | [0196-editor-type-highlighting.md](0196-editor-type-highlighting.md) | — (новая фича) |
| 0192 | Константное выражение в инициализаторе объявления | [0192-const-init-fold.md](0192-const-init-fold.md) | — (новая фича) |
| 0193 | Цели rust и sv: одноимённые константы разных моделей | [0193-shared-const-qualified.md](0193-shared-const-qualified.md) | меняется вывод целей `rust`/`sv` (имена констант); язык и API крейта — нет |
| 0194 | Симулятор теряет model-level always у модели-композиции | [0194-sim-composition-model-always.md](0194-sim-composition-model-always.md) | — (новая фича) |
| 0195 | Коллизии имён при отображении в пространство имён цели | [0195-target-name-collisions.md](0195-target-name-collisions.md) | — (новая фича) |
| 0198 | Форматтер выносит комментарий из тела блока наружу | [0198-formatter-comment-in-block.md](0198-formatter-comment-in-block.md) | — (новая фича) |
| 0199 | Форма model M = A & B { … } не работает ни в одной стороне | [0199-model-implements-brace-form.md](0199-model-implements-brace-form.md) | — (новая фича) |
| 0200 | Идентификатор с не-ASCII буквами: язык принимает, цели sv и st не выражают | [0200-non-ascii-identifier-targets.md](0200-non-ascii-identifier-targets.md) | — (новая фича) |
| 0201 | Мёртвая лексика: слова и терминалы, которых грамматика не знает | [0201-dead-lexemes.md](0201-dead-lexemes.md) | — (новая фича) |
| 0202 | taktc fmt печатает синтаксическую ошибку Debug-дампом | [0202-fmt-diagnostic-formatting.md](0202-fmt-diagnostic-formatting.md) | — (новая фича) |
| 0152 | Восстановление на границе элемента в стадиях построения | [0152-semantic-recovery-element-boundary.md](0152-semantic-recovery-element-boundary.md) | — (новая фича) |
| 0197 | Стиль кода языка Takt — свод правил оформления и раздел документа | [0197-language-code-style.md](0197-language-code-style.md) | — (новая фича) |
| 0226 | Канон именования: предупреждение в fmt и LSP | [0226-naming-convention-warning.md](0226-naming-convention-warning.md) | — (новая фича) |
| 0227 | Редактор показывает CS-001 и при ошибках в файле | [0227-lsp-style-warning-with-errors.md](0227-lsp-style-warning-with-errors.md) | — (новая фича) |
| 0228 | Предупреждение taktc compile несёт позицию | [0228-compile-warning-position.md](0228-compile-warning-position.md) | — (новая фича) |
| 0229 | Отказ форматтера — диагностика с позицией | [0229-format-unsupported-position.md](0229-format-unsupported-position.md) | — (новая фича) |
| 0230 | Сторож форматтера: корпус восстановлен, KNOWN_GAPS с ратчетом | [0230-format-corpus-sentinel.md](0230-format-corpus-sentinel.md) | — (новая фича) |
| 0231 | Текст диагностики без внутреннего представления | [0231-diagnostic-text-no-debug.md](0231-diagnostic-text-no-debug.md) | — (новая фича) |
| 0232 | Предупреждение о неявной булевости доезжает до пользователя | [0232-implicit-bool-warning-delivery.md](0232-implicit-bool-warning-delivery.md) | — (новая фича) |
| 0233 | Правило булевости условия — одно | [0233-single-boolean-predicate.md](0233-single-boolean-predicate.md) | — (новая фича) |
| 0151 | Накопление диагностик внутри отдельной проверки validate | [0151-diagnostics-batch-within-check.md](0151-diagnostics-batch-within-check.md) | — (новая фича) |
| 0160 | Синхронизация эталона Takt.ebnf с актуальным синтаксисом | [0160-takt-ebnf-sync.md](0160-takt-ebnf-sync.md) | — (новая фича) |
| 0170 | Насыщение (saturation) для fixed-point q(m, n) | [0170-fixed-point-saturation.md](0170-fixed-point-saturation.md) | — (новая фича) |
| 0203 | validate не обходит формулы: неизвестное имя в Guard молчит | [0203-validate-formulas-traversal.md](0203-validate-formulas-traversal.md) | — (новая фича) |
| 0234 | Профилирование и ускорение предкоммита | [0234-precheck-time-profile.md](0234-precheck-time-profile.md) | — (новая фича) |
| 0235 | Цели st и sv теряют охранную формулу | [0235-guard-formula-in-st-sv.md](0235-guard-formula-in-st-sv.md) | — (новая фича) |
| 0236 | Печатник цели c печатает пустоту на неразрешённом условии | [0236-c-unresolved-condition-refusal.md](0236-c-unresolved-condition-refusal.md) | — (новая фича) |
| 0237 | Раздел «Импорты» не описывает S(Модель) | [0237-book-state-of-model-section.md](0237-book-state-of-model-section.md) | — (новая фича) |
| 0238 | Живой контекст: раздел критических инвариантов дублирует подводные камни | [0238-claude-md-duplicate-invariants.md](0238-claude-md-duplicate-invariants.md) | — (новая фича) |
| 0204 | Вывод типов не протягивает тип через ссылку константа-константа | [0204-const-ref-type-inference.md](0204-const-ref-type-inference.md) | — (новая фича) |
| 0205 | Приведение as не вычисляется в инициализаторе объявления | [0205-as-in-declaration-initializer.md](0205-as-in-declaration-initializer.md) | — (новая фича) |
| 0206 | Вариант импортированного перечисления не разрешается в образце match | [0206-imported-enum-variant-in-match.md](0206-imported-enum-variant-in-match.md) | — (новая фича) |
| 0207 | Отрицание ~0 для беззнакового типа: два правила языка столкнулись | [0207-bitwise-not-unsigned-literal.md](0207-bitwise-not-unsigned-literal.md) | — (новая фича) |
| 0208 | Три константных вычислителя компилятора живут порознь | [0208-const-evaluators-unification.md](0208-const-evaluators-unification.md) | — (новая фича) |
| 0209 | Внешний интерфейс модели: extern fn в симуляторе и агрегат как аргумент | [0209-model-external-interface.md](0209-model-external-interface.md) | — (новая фича) |
| 0172 | Семантика перечисления без вариантов | [0172-empty-enum-semantics.md](0172-empty-enum-semantics.md) | — (новая фича) |
| 0168 | Предупреждения генераторов возвращаются вызывающему | [0168-generator-warnings-return.md](0168-generator-warnings-return.md) | — (новая фича) |
| 0167 | Цель c использует объявленные константы перечисления | [0167-c-enum-constants-usage.md](0167-c-enum-constants-usage.md) | — (новая фича) |
| 0169 | Адаптеры шин для цели sv-mmio (APB) | [0169-sv-mmio-bus-adapters.md](0169-sv-mmio-bus-adapters.md) | — (новая фича) |
| 0210 | Массив как общая переменная в цели st; индекс-выражение | [0210-st-array-shared-and-index.md](0210-st-array-shared-and-index.md) | — (новая фича) |
| 0211 | Модель без стартового состояния: цель c отказывает бессодержательно | [0211-c-missing-start-state-diagnostic.md](0211-c-missing-start-state-diagnostic.md) | — (новая фича) |
| 0212 | Диагностика цели c без кода | [0212-c-diagnostic-without-code.md](0212-c-diagnostic-without-code.md) | — (новая фича) |
| 0239 | Скрипт релизной сборки и установки инструментов | [0239-install-script.md](0239-install-script.md) | — (новая фича) |
| 0240 | Перевод документа book/ в формат Typst | [0240-book-typst.md](0240-book-typst.md) | — (новая фича) |
| 0241 | Ускорение предкоммита | [0241-precheck-speedup.md](0241-precheck-speedup.md) | — (новая фича) |
| 0213 | Цель c печатает лишний break после безусловного перехода | [0213-c-redundant-break.md](0213-c-redundant-break.md) | — (новая фича) |
| 0244 | Стоимость тестовых целей: 147 бинарников | [0244-test-target-build-cost.md](0244-test-target-build-cost.md) | — (новая фича) |
| 0243 | Переопределение типа | [0243-type-redefinition-diagnostic.md](0243-type-redefinition-diagnostic.md) | — (новая фича) |
| 0214 | Регистровый интерфейс sv-mmio | [0214-sv-mmio-unused-write-signals.md](0214-sv-mmio-unused-write-signals.md) | — (новая фича) |
| 0245 | Симулятор исполняет S(Модель) — проверку состояния под-модели | [0245-sim-state-of-model.md](0245-sim-state-of-model.md) | — (новая фича) |
| 0246 | Ссылка вперёд в инициализаторе переменной — ошибка компиляции | [0246-init-forward-reference.md](0246-init-forward-reference.md) | — (новая фича) |
| 0223 | Три примера объясняют выходной порт устаревшей нуждой цели rust | [0223-examples-port-rationale-stale.md](0223-examples-port-rationale-stale.md) | не затронута (правятся комментарии примеров) |
| 0224 | Подъём Kotlin в плагине intellij-takt снимет ограничение на пусковой JDK | [0224-intellij-kotlin-upgrade.md](0224-intellij-kotlin-upgrade.md) | расширяется: пусковой JDK 17…26 (было ≤ 21), совместимость плагина с IDE не менялась |
| 0225 | Модуль semantic/statement.rs — 999 строк при пределе 1000 | [0225-statement-module-size.md](0225-statement-module-size.md) | не затронута (перемещение приватного модуля внутри крейта) |
| 0221 | Панель структуры: инвариант состояния символом не становится | [0221-lsp-state-invariant-symbol.md](0221-lsp-state-invariant-symbol.md) | наблюдаемо: в панели структуры появляется новый узел; язык не тронут |
| 0220 | Флаг -Wextra для гейта цели c: 38 предупреждений одного класса | [0220-c-gate-wextra.md](0220-c-gate-wextra.md) | не затронута (флаги предкоммит-гейта, не поставляемый CMakeLists) |
| 0219 | Сверки через mmap стоят около 90 секунд каждая | [0219-mmap-conformance-cost.md](0219-mmap-conformance-cost.md) | не затронута (фича отменена: замер опровергнут) |
| 0218 | Реестры стадий 5 и 6 хранят заготовочное СОЗДАНА в колонках вердикта | [0218-registry-verdict-placeholder.md](0218-registry-verdict-placeholder.md) | не затронута (документы и скрипты процесса) |
| 0217 | Ветвь-заглушка до будущей задачи переживает саму задачу | [0217-stub-branch-gate.md](0217-stub-branch-gate.md) | не затронута (реестр, гейт и правила процесса) |
| 0248 | Встроенные функции min/max/abs/clamp/debug не исполняются эталоном | [0248-sim-builtin-functions.md](0248-sim-builtin-functions.md) | расширяется: эталон исполняет встроенные; язык не тронут |
| 0247 | Голое имя состояния и модели в условии не исполняется эталоном | [0247-sim-bare-state-condition.md](0247-sim-bare-state-condition.md) | ужесточение: запись, которую не принимал ни один потребитель, отвергает компилятор (SE-110) |
| 0249 | Левая часть присваивания — место записи | [0249-assign-to-call-place.md](0249-assign-to-call-place.md) | — (новая фича) |
| 0250 | Запись бита x.N := v работает или отказывает по названной причине | [0250-bit-write-in-targets.md](0250-bit-write-in-targets.md) | — (новая фича) |
| 0145 | Потолок верификации по данным: бюджет вместо VERTEX_LIMIT | [0145-verify-vertex-budget.md](0145-verify-vertex-budget.md) | — (новая фича) |
| 0153 | Индексация рабочей области для LSP (references/rename между файлами) | [0153-lsp-workspace-index.md](0153-lsp-workspace-index.md) | — (новая фича) |
| 0251 | Единый каталог сборки для всех потребителей cargo | [0251-cargo-target-dir.md](0251-cargo-target-dir.md) | — (новая фича) |
| 0154 | Перевод плагина IntelliJ на серверный rename | [0154-intellij-server-rename.md](0154-intellij-server-rename.md) | — (новая фича) |
| 0150 | Признание позиционной формы сценария устаревшей | [0150-sim-positional-scenario-deprecation.md](0150-sim-positional-scenario-deprecation.md) | — (новая фича) |
| 0158 | Запуск компилятора и симулятора из IntelliJ | [0158-intellij-run-configurations.md](0158-intellij-run-configurations.md) | — (новая фича) |
| 0165 | Подкоманда taktc version | [0165-taktc-version-subcommand.md](0165-taktc-version-subcommand.md) | — (новая фича) |
| 0161 | Остаточные старые имена языка в данных и комментариях | [0161-fixture-comments-rename.md](0161-fixture-comments-rename.md) | — (новая фича) |
| 0162 | Пропущенные метки версий языка и сторож правила 22 | [0162-git-tag-v040.md](0162-git-tag-v040.md) | — (новая фича) |
| 0266 | Порт в инициализаторе объявления | [0266-port-in-declaration-initializer.md](0266-port-in-declaration-initializer.md) | — (новая фича) |
| 0291 | Решение «ребро безусловно» у одного носителя | [0291-rust-sv-unresolved-condition.md](0291-rust-sv-unresolved-condition.md) | — (новая фича) |
| 0300 | Дробная арифметика в инициализаторе объявления | [0300-fractional-init-arithmetic.md](0300-fractional-init-arithmetic.md) | — (новая фича) |
| 0284 | Структура без полей | [0284-empty-struct-semantics.md](0284-empty-struct-semantics.md) | — (новая фича) |
| 0301 | Снятие замера расхождения: инструмент и чек-лист | [0301-probe-checklist.md](0301-probe-checklist.md) | — (новая фича) |
| 0302 | Релиз и тег при подъёме минорной версии языка | [0302-release-on-language-minor.md](0302-release-on-language-minor.md) | — (новая фича) |
| 0285 | Ширина выведенного типа | [0285-inferred-width-from-result.md](0285-inferred-width-from-result.md) | — (новая фича) |
| 0287 | Расширение типов не знает именованных целых | [0287-wider-type-array-literal.md](0287-wider-type-array-literal.md) | — (новая фича) |
| 0262 | Широкий бит-вектор в целях c и rust | [0262-wide-bit-vector-c-rust.md](0262-wide-bit-vector-c-rust.md) | — (новая фича) |
| 0263 | Приведение индекса к usize по нужде | [0263-rust-literal-index-cast.md](0263-rust-literal-index-cast.md) | — (новая фича) |
| 0281 | Сравнение перечисления с числом в цели rust | [0281-rust-enum-compare-literal.md](0281-rust-enum-compare-literal.md) | — (новая фича) |
| 0299 | Не-ASCII имя в нижнем регистре у цели rust | [0299-rust-non-ascii-lowercase-name.md](0299-rust-non-ascii-lowercase-name.md) | — (новая фича) |
| 0295 | Хвостовой комментарий тела и его хозяин | [0295-format-element-comment-binding.md](0295-format-element-comment-binding.md) | — (новая фича) |
| 0279 | Вложенная модель подключённого файла и подсказка | [0279-qualified-import-model-reference.md](0279-qualified-import-model-reference.md) | — (новая фича) |
| 0264 | Координата судей тела — позиция употребления | [0264-body-judge-usage-position.md](0264-body-judge-usage-position.md) | — (новая фича) |
| 0273 | Недостижимое ребро — предупреждение SE-116 | [0273-unreachable-edge-warning.md](0273-unreachable-edge-warning.md) | — (новая фича) |
| 0276 | Диагностика семантики без кода и позиции | [0276-semantic-diagnostics-without-code.md](0276-semantic-diagnostics-without-code.md) | — (новая фича) |
| 0277 | Координата отказа цели — место употребления | [0277-expression-usage-position.md](0277-expression-usage-position.md) | — (новая фича) |
| 0282 | Собственная позиция формулы | [0282-formula-own-location.md](0282-formula-own-location.md) | — (новая фича) |
| 0296 | Порядок стадий построения — один носитель | [0296-semantic-stages-single-source.md](0296-semantic-stages-single-source.md) | зависимостей нет; Tier исправлен 3 → 2 по замеру |
| 0278 | Мёртвая упаковка последовательной композиции | [0278-compact-implement-dead-branch.md](0278-compact-implement-dead-branch.md) | зависимостей нет; путь отвергнут ADR 0057 — код снят |
| 0260 | Неиспользуемый параметр в порождённом C | [0260-c-unused-struct-parameter.md](0260-c-unused-struct-parameter.md) | зависимостей нет; путь кандидата отвергнут по цене |
| 0267 | Проверка состояния соседней модели в целях | [0267-state-of-model-in-targets.md](0267-state-of-model-in-targets.md) | зависимостей нет; замер поправил кандидата дважды |
| 0303 | Условное ребро состояния-композиции теряется целями | [0303-composition-state-conditional-edge.md](0303-composition-state-conditional-edge.md) | зависимостей нет; взята вне очереди (Tier 1) |
| 0286 | Вычислимое приведение в инициализаторе | [0286-sv-const-initializer-expression.md](0286-sv-const-initializer-expression.md) | зависимостей нет; замер расширил предмет до общего вычислителя |
| 0293 | Структуры в целях st, rust и sv | [0293-struct-in-st-rust.md](0293-struct-in-st-rust.md) | зависимостей нет; замер поправил поведение st |
| 0253 | Старое имя языка в порождаемом коде (lam_q_*, LAM_Q_*) | [0253-legacy-names-in-generated-code.md](0253-legacy-names-in-generated-code.md) | зависимостей нет; имя для IEC выбрано пробой iec2c |
| 0292 | Код CC-022 обещан комментарием, но не эмитируется никем | [0292-cc022-promise-without-emitter.md](0292-cc022-promise-without-emitter.md) | зависимостей нет; замер кандидата опровергнут прогоном |
| 0255 | Коды симулятора, вплавленные в текст, невидимы гейту и реестру | [0255-sim-diagnostic-codes-registry.md](0255-sim-diagnostic-codes-registry.md) | зависимостей нет; смежность с 0290 (документ) |
| 0256 | Символ формы import { A } объявляется видом Model | [0256-lsp-import-binding-kind.md](0256-lsp-import-binding-kind.md) | зависимостей нет; замер уточнил числа кандидата |
| 0258 | Verdict::Unsupported не различает причину отказа | [0258-verify-unsupported-reason.md](0258-verify-unsupported-reason.md) | зависимостей нет; замер нашёл недостижимую ветвь причины |
| 0259 | Встроенные функции языка не описаны в документе book/ | [0259-book-builtin-functions.md](0259-book-builtin-functions.md) | зависимостей нет; прогон нашёл два дефекта поведения (кандидаты) |
| 0268 | У SE-033 нет описания в приложении «Ошибки» | [0268-se033-appendix-description.md](0268-se033-appendix-description.md) | зависимостей нет; замер нашёл ещё два места |
| 0274 | Снимки порождённого кода в book/ никем не сверяются | [0274-book-generated-snapshots-gate.md](0274-book-generated-snapshots-gate.md) | зависимостей нет; замер нашёл три отставших снимка вместо названного одного |
| 0275 | Команды в README.md никем не проверяются | [0275-readme-commands-gate.md](0275-readme-commands-gate.md) | зависимостей нет; гейт нашёл панику компилятора на команде документа |
| 0290 | Приложение «Ошибки» сверяется с реестром диагностик | [0290-book-diagnostics-codes-gate.md](0290-book-diagnostics-codes-gate.md) | — (новая фича) |
| 0298 | Списки лексики раздела «Лексика» сверяются с языком | [0298-book-lexicon-lists-sync.md](0298-book-lexicon-lists-sync.md) | — (новая фича) |
