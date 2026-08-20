# Реестр тест-планов

Стадия 5 жизненного цикла (правило 17 в [../RULE.md](../RULE.md)). Тест-план
`XXXX-slug.md` формируется тестировщиком **параллельно** разработке на основе
анализа. Для фич, меняющих синтаксис/семантику языка, тест-план обязан включать
тесты корректности языка и его компилятора/интерпретатора, а также примеры и
контрпримеры (правило 16).

Заготовка создаётся из шаблона [`../templates/tests.md`](../templates/tests.md).

| Фича | Заголовок | Тест-план | Статус |
|------|-----------|-----------|--------|
| 0021 | Смена операторов: `:=` присваивание, `=` сравнение | [0021-swap-assign-compare.md](0021-swap-assign-compare.md) | ✅ ПРОЙДЕН (отчёт: [reports/0021](../reports/0021-swap-assign-compare.md)) |
| 0022 | Плагин IntelliJ IDEA: подсветка синтаксиса Lam | [0022-intellij-syntax-highlight.md](0022-intellij-syntax-highlight.md) | ✅ ПРОЙДЕН (отчёт: [reports/0022](../reports/0022-intellij-syntax-highlight.md)) |
| 0023 | Плагин IntelliJ IDEA — навигация к декларации и include | [0023-intellij-navigation-include.md](0023-intellij-navigation-include.md) | ГОТОВО |
| 0025 | Починка вычислителя выражений симулятора | [0025-simulator-expression-eval.md](0025-simulator-expression-eval.md) | ✅ ПРОЙДЕН (отчёт: [reports/0025](../reports/0025-simulator-expression-eval.md)) |
| 0024 | Канонический форматтер .lam (lamc fmt) | [0024-lam-formatter.md](0024-lam-formatter.md) | ✅ ПРОЙДЕН (отчёт: [reports/0024](../reports/0024-lam-formatter.md)) |
| 0026 | Генератор C: typedef корневой структуры для одиночной модели | [0026-c-root-typedef.md](0026-c-root-typedef.md) | ГОТОВО |
| 0027 | Разделение переросших модулей (validate.rs, lsp.rs, c_expr.rs) | [0027-module-size-split.md](0027-module-size-split.md) | ГОТОВО |
| 0028 | Заглушки генератора C: диагностика вместо тихого пропуска | [0028-c-generator-stubs.md](0028-c-generator-stubs.md) | ГОТОВО |
| 0029 | Генератор C: отображение типов Array/Bit/Rational | [0029-c-type-mapping.md](0029-c-type-mapping.md) | ГОТОВО |
| 0030 | Исправление примера comprehensive.lam (недостижимый сценарий) | [0030-comprehensive-example-fix.md](0030-comprehensive-example-fix.md) | ГОТОВО |
| 0031 | Вызов функции из тела функции | [0031-fn-calls-fn.md](0031-fn-calls-fn.md) | ✅ ГОТОВО |
| 0032 | Сохранение переменных модели в --save-state/--load-state | [0032-state-io-variables.md](0032-state-io-variables.md) | ✅ ГОТОВО |
| 0033 | Согласование тактов симулятора и порождённого C (INIT-такты) | [0033-init-tick-alignment.md](0033-init-tick-alignment.md) | ✅ ГОТОВО |
| 0034 | Структурные типы в симуляторе | [0034-sim-struct-types.md](0034-sim-struct-types.md) | ГОТОВО |
| 0035 | LTL-формулы в блоках кода: разбор вместо тихой потери | [0035-ltl-in-blocks.md](0035-ltl-in-blocks.md) | ✅ ГОТОВО |
| 0036 | Согласование видимости публичного API крейта simulation | [0036-sim-visibility.md](0036-sim-visibility.md) | ГОТОВО |
| 0037 | Сбои тестов на Windows (пути include, ресурс viewport) | [0037-windows-test-failures.md](0037-windows-test-failures.md) | ГОТОВО |
| 0038 | Семантическая подсветка Lam в IntelliJ через lam-lsp | [0038-intellij-semantic-tokens.md](0038-intellij-semantic-tokens.md) | ГОТОВО |
| 0039 | Действие Reformat Code в плагине IntelliJ | [0039-intellij-reformat.md](0039-intellij-reformat.md) | ГОТОВО |
| 0040 | Полноценный PSI-парсер плагина IntelliJ | [0040-intellij-psi-parser.md](0040-intellij-psi-parser.md) | ГОТОВО |
| 0041 | Бэкенд генерации в Structured Text (IEC 61131-3) | [0041-st-backend.md](0041-st-backend.md) | ГОТОВО |
| 0042 | Инъекция define'ов для адресов (--define) | [0042-address-defines.md](0042-address-defines.md) | ГОТОВО |
| 0043 | Экспорт карты адресов во внешний формат | [0043-address-map-export.md](0043-address-map-export.md) | ГОТОВО |
| 0044 | Юнит-конструкции языка для симуляции (assert/invariant) | [0044-sim-assert-invariant.md](0044-sim-assert-invariant.md) | ✅ ГОТОВО |
| 0045 | Бэкенд генерации в SystemVerilog | [0045-sv-backend.md](0045-sv-backend.md) | ГОТОВО |
| 0046 | Устранение всех предупреждений сборки | [0046-build-warnings-cleanup.md](0046-build-warnings-cleanup.md) | ✅ ПРОЙДЕН (отчёт: [reports/0046](../reports/0046-build-warnings-cleanup.md)) |
| 0048 | Детерминированная генерация кода (единый порядок эмиссии) | [0048-deterministic-codegen.md](0048-deterministic-codegen.md) | ✅ ГОТОВО |
| 0049 | Верификация модели (Model Checking) на основе LTL | [0049-model-checking-ltl.md](0049-model-checking-ltl.md) | ГОТОВО (T1–T47 пройдены) |
| 0050 | Бэкенд генерации в Rust | [0050-rust-backend.md](0050-rust-backend.md) | РАЗРАБОТКА |
| 0051 | Область проверки lamc verify (--scope) | [0051-verify-scope.md](0051-verify-scope.md) | ✅ ПРОЙДЕН (отчёт: [reports/0051](../reports/0051-verify-scope.md)) |
| 0052 | Итеративные обходы — снятие потолка стека | [0052-verify-iterative-traversal.md](0052-verify-iterative-traversal.md) | ✅ ПРОЙДЕН (отчёт: [reports/0052](../reports/0052-verify-iterative-traversal.md)) |
| 0053 | Позиции в диагностиках (файл:строка:колонка) и настоящий file_no | [0053-diagnostics-file-id.md](0053-diagnostics-file-id.md) | ✅ ПРОЙДЕН (отчёт: [reports/0053](../reports/0053-diagnostics-file-id.md)) |
| 0054 | Позиции в диагностиках симулятора | [0054-sim-diagnostics-positions.md](0054-sim-diagnostics-positions.md) | ✅ ПРОЙДЕН (отчёт: [reports/0054](../reports/0054-sim-diagnostics-positions.md)) |
| 0055 | Многофайловость LSP: импорты и позиции диагностик | [0055-lsp-multifile.md](0055-lsp-multifile.md) | ✅ ПРОЙДЕН (отчёт: [reports/0055](../reports/0055-lsp-multifile.md)) |
| 0056 | Кросс-файловый переход к декларации (точный путь) | [0056-lsp-goto-exact-file.md](0056-lsp-goto-exact-file.md) | СФОРМИРОВАН (фича в РАЗРАБОТКА) |
| 0057 | Последовательная композиция (`+`) в цели SystemVerilog | [0057-sv-sequential-composition.md](0057-sv-sequential-composition.md) | СФОРМИРОВАН (фича в РАЗРАБОТКА) |
| 0058 | Хвостовой разворот `return` — заход в завершающий `if/else` | [0058-rust-tail-return-if-else.md](0058-rust-tail-return-if-else.md) | ГОТОВО |
| 0059 | Общие переменные корня → структура `Shared` | [0059-rust-shared-struct.md](0059-rust-shared-struct.md) | ГОТОВО |
| 0060 | Диапазон и знак перечисления — один расчёт на все цели | [0060-enum-width-shared-layer.md](0060-enum-width-shared-layer.md) | ГОТОВО |
| 0061 | Fixed-point Q(m.n) как тип языка | [0061-fixed-point-type.md](0061-fixed-point-type.md) | ГОТОВО |
| 0062 | Цель `sv-mmio` — адреса портов как регистровый файл | [0062-sv-mmio-target.md](0062-sv-mmio-target.md) | ГОТОВО |
| 0063 | Порт `en` (clock enable) для цели `sv` | [0063-sv-clock-enable.md](0063-sv-clock-enable.md) | ГОТОВО |
| 0064 | Предупреждение о делителе (`SV-009`) в цели `sv` | [0064-sv-divider-warning.md](0064-sv-divider-warning.md) | ГОТОВО |
| 0065 | Изоляция пространства имён цели `st` | [0065-st-namespace-isolation.md](0065-st-namespace-isolation.md) | ГОТОВО |
| 0066 | Литералы по целевому типу в телах цели `st` | [0066-st-bool-literals.md](0066-st-bool-literals.md) | ГОТОВО |
| 0068 | Верификация свойств над данными | [0068-verify-data-properties.md](0068-verify-data-properties.md) | ГОТОВО |
| 0069 | Разделение `address_map.rs` | [0069-address-map-eval-split.md](0069-address-map-eval-split.md) | ГОТОВО |
| 0075 | эталонная модель порождает компилируемый C | [0075-lib-src-reference-model.md](0075-lib-src-reference-model.md) | ✅ ПРОЙДЕН (отчёт: [reports/0075](../reports/0075-lib-src-reference-model.md)) |
| 0081 | `lamc compile` печатает предупреждения | [0081-lamc-print-warnings.md](0081-lamc-print-warnings.md) | ✅ ПРОЙДЕН (отчёт: [reports/0081](../reports/0081-lamc-print-warnings.md)) |
| 0082 | `unused.rs` обходит формулы | [0082-unused-formulas.md](0082-unused-formulas.md) | ✅ ПРОЙДЕН (отчёт: [reports/0082](../reports/0082-unused-formulas.md)) |
| 0096 | Q-арифметика через нативный float и флаг генерации (embedded ↔ float) | [0096-fixed-point-native-float.md](0096-fixed-point-native-float.md) | ГОТОВО |
| 0097 | Пример ПИД-регулятора на языке Lam (fixed-point) | [0097-pid-regulator-example.md](0097-pid-regulator-example.md) | ГОТОВО |
| 0090 | CI прогоняет весь `precheck.sh` (живые гейты + check-links) | [0090-ci-precheck.md](0090-ci-precheck.md) | ✅ ГОТОВО (T1,T4–T10 локально; T2/T3/T11 — блокер биллинга Actions) |
| 0070 | Инициализатор порта — это адрес, а не значение | [0070-port-initializer-address-role.md](0070-port-initializer-address-role.md) | ✅ ГОТОВО (T1–T11; SE-035 снят с портов; вывод корпуса не изменён) |
| 0071 | Переход на имя состояния в `S(Ping) = End` | [0071-lsp-goto-state-name.md](0071-lsp-goto-state-name.md) | ✅ ГОТОВО (T2/T2b/T4/T7; кросс-модельный `S(Ping)=End` + внутримодельный `x=Done`; кодоген байт-в-байт) |
| 0073 | `Location::filename()` возвращает номер, а не путь | [0073-location-filename-path.md](0073-location-filename-path.md) | ✅ ГОТОВО (T1–T6; метод удалён, покрытие держит `try_file_no`; вывод корпуса не изменён) |
| 0086 | `var q: u8;` без инициализатора → `SIM-009` | [0086-sim-var-without-initializer.md](0086-sim-var-without-initializer.md) | ✅ ГОТОВО (T1–T7; скаляр без init → 0 по типу; регресса портов/констант нет; кодоген неизменен) |
| 0074 | Скобочная форма `S(…)` отвергается семантикой | [0074-parenthesised-state-of.md](0074-parenthesised-state-of.md) | ✅ ГОТОВО (T1–T10; скобки прозрачны, C байт-в-байт = бесскобочной; `SE-025`→`SE-033`; вывод корпуса неизменен) |
| 0083 | Тело `always` на уровне модели не эмитится | [0083-model-always-block.md](0083-model-always-block.md) | ✅ ГОТОВО (T1–T8; потактовая сверка C↔симулятор n=1,2,3,4; эмиссия до диспетчера + компиляция rust/st/sv; корпус неизменен) |
| 0080 | Дефекты генератора C по структурам | [0080-c-struct-defects.md](0080-c-struct-defects.md) | ✅ ГОТОВО (T1–T8; составной литерал + static const компилируются cc; SE-061 на неизвестном поле; корпус неизменен) |
| 0079 | `elevator_mini.lam` не исполняется: порты под-модели композиции | [0079-sim-composition-ports.md](0079-sim-composition-ports.md) | ✅ ГОТОВО (T1–T7; порты под-моделей перечисляются/драйвятся; elevator_mini реагирует на датчик; stacker не сломан) |
| 0072 | LSP не читает initializationOptions (пути поиска импортов) | [0072-lsp-initialization-options.md](0072-lsp-initialization-options.md) | ✅ ГОТОВО |
| 0076 | Симулятор не исполняет массивы вовсе | [0076-sim-arrays.md](0076-sim-arrays.md) | ✅ ГОТОВО (T1–T10; запись элемента + список-init, сверка с C; границы→SIM-010; структуры 0034 не регрессируют; корпус неизменен) |
| 0077 | Реестр кодов диагностик (конфликт `CC-014`) | [0077-diagnostic-code-registry.md](0077-diagnostic-code-registry.md) | ✅ ГОТОВО (T1–T11; гейт зелёный на 171 коде, 4 условия отказа + RESERVED, next-free; вывод байт-в-байт неизменен) |
| 0078 | Семантика `[bit;N]` расходится втрое | [0078-bit-array-semantics.md](0078-bit-array-semantics.md) | ✅ ГОТОВО (T1–T12; упаковка во все цели/симулятор, округление вверх, слова N>64, сверка бит-доступа с C; CC-014 RETIRED; C неизменен) |
| 0085 | Константа версии языка в коде + гейт синхронизации с README | [0085-language-version-constant.md](0085-language-version-constant.md) | ✅ ГОТОВО |
| 0084 | Ключ карты адресов — квалифицированный (модель+порт) | [0084-address-map-qualified-key.md](0084-address-map-qualified-key.md) | ✅ ГОТОВО |
| 0087 | Мягкий режим инвариантов симулятора (записать и продолжить) | [0087-invariant-soft-mode.md](0087-invariant-soft-mode.md) | ✅ ГОТОВО |
| 0094 | Доработка scripts/new-feature.sh (идемпотентность, статус ADR, поздние стадии) | [0094-new-feature-script-fixes.md](0094-new-feature-script-fixes.md) | ✅ ГОТОВО |
| 0093 | Запрет _ => в семантических узлах и разрешение конфликта с #[non_exhaustive] | [0093-wildcard-match-rule.md](0093-wildcard-match-rule.md) | ✅ ГОТОВО |
| 0091 | Правило о размере модуля переносится в docs/CODE.md | [0091-module-size-rule-in-code-md.md](0091-module-size-rule-in-code-md.md) | ✅ ГОТОВО |
| 0067 | Rename и PsiReference для import в плагине IntelliJ | [0067-intellij-rename-psi-import.md](0067-intellij-rename-psi-import.md) | ГОТОВО |
| 0089 | Остаточные проверки плагина IntelliJ (0022/0023) | [0089-intellij-residual-checks.md](0089-intellij-residual-checks.md) | ГОТОВО |
| 0092 | У фичи 0018 нет ADR | [0092-adr-0018-retrofit.md](0092-adr-0018-retrofit.md) | ГОТОВО |
| 0088 | Нарушители лимита размера модуля — безопасная часть | [0088-module-size-remaining.md](0088-module-size-remaining.md) | ✅ ГОТОВО |
| 0098 | диапазон бита адреса порта и безопасный дефолтный HAL | [0098-port-bit-range-safe-hal.md](0098-port-bit-range-safe-hal.md) | ✅ ПРОЙДЕН (отчёт: [reports/0098](../reports/0098-port-bit-range-safe-hal.md)) |
| 0100 | Переименование языка Lam → Takt | [0100-language-rename-takt.md](0100-language-rename-takt.md) | ✅ ГОТОВО |
| 0124 | Экспорт графов верификации (Крипке/Бюхи/произведение) в Graphviz DOT | [0124-verify-graph-export.md](0124-verify-graph-export.md) | ГОТОВО |
| 0125 | Плагин IntelliJ Takt: LSP-проверка, автодополнение, документация и настройки инструментов | [0125-intellij-takt-lsp-tooling.md](0125-intellij-takt-lsp-tooling.md) | ГОТОВО |
| 0139 | Удаление мёртвой конфигурации .travis.yml | [0139-remove-travis-config.md](0139-remove-travis-config.md) | ✅ ГОТОВО |
| 0137 | Фиксация толчейна Rust и MSRV | [0137-toolchain-pin-msrv.md](0137-toolchain-pin-msrv.md) | ✅ ГОТОВО |
| 0128 | Диагностика вместо паники на числовом литерале больше i64::MAX | [0128-lexer-literal-overflow.md](0128-lexer-literal-overflow.md) | ✅ ГОТОВО |
| 0129 | Устранение переполнения стека на глубине выражений и операторов | [0129-semantic-deep-nesting.md](0129-semantic-deep-nesting.md) | ✅ ГОТОВО |
| 0127 | Единая семантика переполнения целых во всех целях | [0127-int-overflow-semantics.md](0127-int-overflow-semantics.md) | ✅ ГОТОВО |
| 0133 | Гейт компиляции и симуляции примеров документа book/ | [0133-book-examples-gate.md](0133-book-examples-gate.md) | ✅ ГОТОВО |
| 0135 | Квалифицированные имена портов в симуляторе | [0135-sim-qualified-port-names.md](0135-sim-qualified-port-names.md) | ✅ ГОТОВО |
| 0131 | LSP: definition, references и rename | [0131-lsp-definition-references-rename.md](0131-lsp-definition-references-rename.md) | ГОТОВО |
| 0130 | Накопление семантических диагностик | [0130-diagnostics-batch.md](0130-diagnostics-batch.md) | ГОТОВО |
| 0132 | Именованные порты в сценариях симулятора | [0132-sim-named-port-scenarios.md](0132-sim-named-port-scenarios.md) | ГОТОВО |
| 0140 | Ревизия витрины кандидатов и разделение ролей README и book | [0140-backlog-revision-doc-split.md](0140-backlog-revision-doc-split.md) | ГОТОВО |
| 0138 | Измерение покрытия тестами | [0138-coverage-measurement.md](0138-coverage-measurement.md) | ГОТОВО |
| 0136 | Бенчмарки производительности | [0136-perf-benchmarks.md](0136-perf-benchmarks.md) | ГОТОВО |
| 0178 | Приведение LSP и плагинов в соответствие языку + сторож | [0178-editor-layer-language-sync.md](0178-editor-layer-language-sync.md) | ✅ ПРОЙДЕН (отчёт: [reports/0178](../reports/0178-editor-layer-language-sync.md)) |
| 0179 | Дочистка URL репозитория после переезда BuT → Takt | [0179-repo-url-cleanup.md](0179-repo-url-cleanup.md) | ✅ ПРОЙДЕН (отчёт: [reports/0179](../reports/0179-repo-url-cleanup.md)) |
| 0144 | Экспонента числового литерала | [0144-int-literal-exponent.md](0144-int-literal-exponent.md) | ✅ ПРОЙДЕН (отчёт: [reports/0144](../reports/0144-int-literal-exponent.md)) |
| 0155 | Семантическое разрешение тел вложенных операторов | [0155-semantic-nested-statement-resolution.md](0155-semantic-nested-statement-resolution.md) | ГОТОВО |
| 0146 | Гейт символов вне шрифта документа book/ | [0146-book-glyph-gate.md](0146-book-glyph-gate.md) | ГОТОВО |
| 0149 | Гейт согласованности живого контекста CLAUDE.md | [0149-claude-md-consistency-gate.md](0149-claude-md-consistency-gate.md) | ГОТОВО |
| 0180 | Сокращение живого контекста CLAUDE.md | [0180-claude-md-context-diet.md](0180-claude-md-context-diet.md) | ГОТОВО |
| 0177 | Гейт согласованности статуса | [0177-features-registry-status-gate.md](0177-features-registry-status-gate.md) | ГОТОВО |
| 0159 | Фиксация требования JDK | [0159-intellij-jdk21-build.md](0159-intellij-jdk21-build.md) | ГОТОВО |
| 0171 | Гейт цели c под -Werror | [0171-c-gate-werror.md](0171-c-gate-werror.md) | ГОТОВО |
| 0181 | Симулятор исполняет реализацию состояния с переходом next | [0181-sim-state-implementation-tick.md](0181-sim-state-implementation-tick.md) | ГОТОВО |
| 0166 | Корпусной SV-транслируемый пример на последовательную композицию + | [0166-sv-example-sequential-composition.md](0166-sv-example-sequential-composition.md) | ГОТОВО |
| 0174 | Цель rust: корневая модель без портов (clippy::new_without_default) | [0174-rust-new-without-default.md](0174-rust-new-without-default.md) | ГОТОВО |
| 0147 | Тесты textDocument/documentSymbol | [0147-lsp-document-symbol-tests.md](0147-lsp-document-symbol-tests.md) | ГОТОВО |
| 0148 | Покрытие печатников цели rust тестами | [0148-rust-printers-coverage.md](0148-rust-printers-coverage.md) | ГОТОВО |
| 0163 | Исчерпывающий разбор узлов во втором вычислителе | [0163-builder-eval-exhaustive.md](0163-builder-eval-exhaustive.md) | ГОТОВО |
| 0143 | `after` принимает константное выражение типа `duration` | [0143-after-const-duration.md](0143-after-const-duration.md) | ГОТОВО |
| 0183 | Тип `duration` в целях и вычисляемая выдержка | [0183-duration-type-in-targets.md](0183-duration-type-in-targets.md) | ГОТОВО |
| 0184 | Общие переменные библиотечного файла в импортёре | [0184-imported-shared-variables.md](0184-imported-shared-variables.md) | ГОТОВО |
| 0182 | Библиотечный ПИД-регулятор и пример его применения | [0182-pid-library-and-application.md](0182-pid-library-and-application.md) | ГОТОВО |
| 0185 | Параметризация моделей (ключевое слово parameter) | [0185-model-parameters.md](0185-model-parameters.md) | ГОТОВО |
| 0186 | Раздел документа «Практический пример: процессор» (шины, кеш, ядра) | [0186-book-processor-example.md](0186-book-processor-example.md) | ГОТОВО |
| 0187 | Пересмотр задания адресов и доступа к портам | [0187-port-io-redesign.md](0187-port-io-redesign.md) | ✅ ПРОЙДЕН (отчёт: [reports/0187](../reports/0187-port-io-redesign.md)) |
| 0190 | Разделение предкоммита на компоненты и выборочный запуск | [0190-precheck-selective-gates.md](0190-precheck-selective-gates.md) | ГОТОВО |
| 0157 | Представление числового литерала: полная маска [bit;64] | [0157-literal-u64-representation.md](0157-literal-u64-representation.md) | ГОТОВО |
| 0156 | Ограничение глубины вложенности на уровне лексера/парсера | [0156-parser-depth-limit.md](0156-parser-depth-limit.md) | ГОТОВО |
| 0176 | Позиция бита у bit-порта с голым адресом | [0176-bit-port-address-position.md](0176-bit-port-address-position.md) | ГОТОВО |
| 0188 | Направление порта проверяется во всех позициях | [0188-port-direction-everywhere.md](0188-port-direction-everywhere.md) | ГОТОВО |
| 0189 | Анонимные порты | [0189-anonymous-ports.md](0189-anonymous-ports.md) | ГОТОВО |
| 0196 | Подсветка имён типов отдельным цветом в LSP и плагинах | [0196-editor-type-highlighting.md](0196-editor-type-highlighting.md) | ГОТОВО |
| 0191 | Цель st: потактовая сверка с эталоном и устранение расхождений | [0191-st-per-tick-conformance.md](0191-st-per-tick-conformance.md) | ГОТОВО |
| 0192 | Константное выражение в инициализаторе объявления | [0192-const-init-fold.md](0192-const-init-fold.md) | ✅ ПРОЙДЕН (отчёт: [reports/0192](../reports/0192-const-init-fold.md)) |
| 0193 | Цели rust и sv: одноимённые константы разных моделей | [0193-shared-const-qualified.md](0193-shared-const-qualified.md) | ✅ ПРОЙДЕН (отчёт: [reports/0193](../reports/0193-shared-const-qualified.md)) |
| 0194 | Симулятор теряет model-level always у модели-композиции | [0194-sim-composition-model-always.md](0194-sim-composition-model-always.md) | ✅ ПРОЙДЕН (отчёт: [reports/0194](../reports/0194-sim-composition-model-always.md)) |
| 0195 | Коллизии имён при отображении в пространство имён цели | [0195-target-name-collisions.md](0195-target-name-collisions.md) | ✅ ПРОЙДЕН (отчёт: [reports/0195](../reports/0195-target-name-collisions.md)) |
| 0198 | Форматтер выносит комментарий из тела блока наружу | [0198-formatter-comment-in-block.md](0198-formatter-comment-in-block.md) | ✅ ПРОЙДЕН (отчёт: [reports/0198](../reports/0198-formatter-comment-in-block.md)) |
| 0199 | Форма model M = A & B { … } не работает ни в одной стороне | [0199-model-implements-brace-form.md](0199-model-implements-brace-form.md) | ✅ ПРОЙДЕН (отчёт: [reports/0199](../reports/0199-model-implements-brace-form.md)) |
| 0200 | Идентификатор с не-ASCII буквами: язык принимает, цели sv и st не выражают | [0200-non-ascii-identifier-targets.md](0200-non-ascii-identifier-targets.md) | ✅ ПРОЙДЕН (отчёт: [reports/0200](../reports/0200-non-ascii-identifier-targets.md)) |
| 0201 | Мёртвая лексика: слова и терминалы, которых грамматика не знает | [0201-dead-lexemes.md](0201-dead-lexemes.md) | ✅ ПРОЙДЕН (отчёт: [reports/0201](../reports/0201-dead-lexemes.md)) |
| 0202 | taktc fmt печатает синтаксическую ошибку Debug-дампом | [0202-fmt-diagnostic-formatting.md](0202-fmt-diagnostic-formatting.md) | ✅ ПРОЙДЕН (отчёт: [reports/0202](../reports/0202-fmt-diagnostic-formatting.md)) |
| 0152 | Восстановление на границе элемента в стадиях построения | [0152-semantic-recovery-element-boundary.md](0152-semantic-recovery-element-boundary.md) | ✅ ПРОЙДЕН (отчёт: [reports/0152](../reports/0152-semantic-recovery-element-boundary.md)) |
| 0197 | Стиль кода языка Takt — свод правил оформления и раздел документа | [0197-language-code-style.md](0197-language-code-style.md) | ✅ ПРОЙДЕН (отчёт: [reports/0197](../reports/0197-language-code-style.md)) |
| 0226 | Канон именования: предупреждение в fmt и LSP | [0226-naming-convention-warning.md](0226-naming-convention-warning.md) | ГОТОВО |
| 0227 | Редактор показывает CS-001 и при ошибках в файле | [0227-lsp-style-warning-with-errors.md](0227-lsp-style-warning-with-errors.md) | ГОТОВО |
| 0228 | Предупреждение taktc compile несёт позицию | [0228-compile-warning-position.md](0228-compile-warning-position.md) | ГОТОВО |
| 0229 | Отказ форматтера — диагностика с позицией | [0229-format-unsupported-position.md](0229-format-unsupported-position.md) | ГОТОВО |
| 0230 | Сторож форматтера: корпус восстановлен, KNOWN_GAPS с ратчетом | [0230-format-corpus-sentinel.md](0230-format-corpus-sentinel.md) | ГОТОВО |
| 0231 | Текст диагностики без внутреннего представления | [0231-diagnostic-text-no-debug.md](0231-diagnostic-text-no-debug.md) | ГОТОВО |
| 0232 | Предупреждение о неявной булевости доезжает до пользователя | [0232-implicit-bool-warning-delivery.md](0232-implicit-bool-warning-delivery.md) | ГОТОВО |
| 0233 | Правило булевости условия — одно | [0233-single-boolean-predicate.md](0233-single-boolean-predicate.md) | ГОТОВО |
| 0151 | Накопление диагностик внутри отдельной проверки validate | [0151-diagnostics-batch-within-check.md](0151-diagnostics-batch-within-check.md) | ГОТОВО |
| 0160 | Синхронизация эталона Takt.ebnf с актуальным синтаксисом | [0160-takt-ebnf-sync.md](0160-takt-ebnf-sync.md) | ГОТОВО |
| 0170 | Насыщение (saturation) для fixed-point q(m, n) | [0170-fixed-point-saturation.md](0170-fixed-point-saturation.md) | ✅ ПРОЙДЕН (отчёт: [reports/0170](../reports/0170-fixed-point-saturation.md)) |
| 0203 | validate не обходит формулы: неизвестное имя в Guard молчит | [0203-validate-formulas-traversal.md](0203-validate-formulas-traversal.md) | ✅ ПРОЙДЕН (отчёт: [reports/0203](../reports/0203-validate-formulas-traversal.md)) |
| 0234 | Профилирование и ускорение предкоммита | [0234-precheck-time-profile.md](0234-precheck-time-profile.md) | ✅ ПРОЙДЕН (отчёт: [reports/0234](../reports/0234-precheck-time-profile.md)) |
| 0235 | Цели st и sv теряют охранную формулу | [0235-guard-formula-in-st-sv.md](0235-guard-formula-in-st-sv.md) | ✅ ПРОЙДЕН (отчёт: [reports/0235](../reports/0235-guard-formula-in-st-sv.md)) |
| 0236 | Печатник цели c печатает пустоту на неразрешённом условии | [0236-c-unresolved-condition-refusal.md](0236-c-unresolved-condition-refusal.md) | ✅ ПРОЙДЕН (отчёт: [reports/0236](../reports/0236-c-unresolved-condition-refusal.md)) |
| 0238 | Живой контекст: раздел критических инвариантов дублирует подводные камни | [0238-claude-md-duplicate-invariants.md](0238-claude-md-duplicate-invariants.md) | ✅ ПРОЙДЕН (отчёт: [reports/0238](../reports/0238-claude-md-duplicate-invariants.md)) |
| 0204 | Вывод типов не протягивает тип через ссылку константа-константа | [0204-const-ref-type-inference.md](0204-const-ref-type-inference.md) | ✅ ПРОЙДЕН (отчёт: [reports/0204](../reports/0204-const-ref-type-inference.md)) |
| 0205 | Приведение as не вычисляется в инициализаторе объявления | [0205-as-in-declaration-initializer.md](0205-as-in-declaration-initializer.md) | ✅ ПРОЙДЕН (отчёт: [reports/0205](../reports/0205-as-in-declaration-initializer.md)) |
| 0206 | Вариант импортированного перечисления не разрешается в образце match | [0206-imported-enum-variant-in-match.md](0206-imported-enum-variant-in-match.md) | ✅ ПРОЙДЕН (отчёт: [reports/0206](../reports/0206-imported-enum-variant-in-match.md)) |
| 0207 | Отрицание ~0 для беззнакового типа: два правила языка столкнулись | [0207-bitwise-not-unsigned-literal.md](0207-bitwise-not-unsigned-literal.md) | ✅ ПРОЙДЕН (отчёт: [reports/0207](../reports/0207-bitwise-not-unsigned-literal.md)) |
| 0208 | Три константных вычислителя компилятора живут порознь | [0208-const-evaluators-unification.md](0208-const-evaluators-unification.md) | ✅ ПРОЙДЕН (отчёт: [reports/0208](../reports/0208-const-evaluators-unification.md)) |
| 0209 | Внешний интерфейс модели: extern fn в симуляторе и агрегат как аргумент | [0209-model-external-interface.md](0209-model-external-interface.md) | ✅ ПРОЙДЕН (отчёт: [reports/0209](../reports/0209-model-external-interface.md)) |
| 0172 | Семантика перечисления без вариантов | [0172-empty-enum-semantics.md](0172-empty-enum-semantics.md) | ✅ ПРОЙДЕН (отчёт: [reports/0172](../reports/0172-empty-enum-semantics.md)) |
| 0168 | Предупреждения генераторов возвращаются вызывающему | [0168-generator-warnings-return.md](0168-generator-warnings-return.md) | ✅ ПРОЙДЕН (отчёт: [reports/0168](../reports/0168-generator-warnings-return.md)) |
| 0167 | Цель c использует объявленные константы перечисления | [0167-c-enum-constants-usage.md](0167-c-enum-constants-usage.md) | ✅ ПРОЙДЕН (отчёт: [reports/0167](../reports/0167-c-enum-constants-usage.md)) |
| 0169 | Адаптеры шин для цели sv-mmio (APB) | [0169-sv-mmio-bus-adapters.md](0169-sv-mmio-bus-adapters.md) | ✅ ПРОЙДЕН (отчёт: [reports/0169](../reports/0169-sv-mmio-bus-adapters.md)) |
| 0210 | Массив как общая переменная в цели st; индекс-выражение | [0210-st-array-shared-and-index.md](0210-st-array-shared-and-index.md) | ✅ ПРОЙДЕН (отчёт: [reports/0210](../reports/0210-st-array-shared-and-index.md)) |
| 0211 | Модель без стартового состояния: цель c отказывает бессодержательно | [0211-c-missing-start-state-diagnostic.md](0211-c-missing-start-state-diagnostic.md) | ✅ ПРОЙДЕН (отчёт: [reports/0211](../reports/0211-c-missing-start-state-diagnostic.md)) |
| 0212 | Диагностика цели c без кода | [0212-c-diagnostic-without-code.md](0212-c-diagnostic-without-code.md) | ✅ ПРОЙДЕН (отчёт: [reports/0212](../reports/0212-c-diagnostic-without-code.md)) |
| 0239 | Скрипт релизной сборки и установки инструментов | [0239-install-script.md](0239-install-script.md) | ✅ ПРОЙДЕН (отчёт: [reports/0239](../reports/0239-install-script.md)) |
| 0241 | Ускорение предкоммита | [0241-precheck-speedup.md](0241-precheck-speedup.md) | ✅ ПРОЙДЕН (отчёт: [reports/0241](../reports/0241-precheck-speedup.md)) |
| 0213 | Цель c печатает лишний break после безусловного перехода | [0213-c-redundant-break.md](0213-c-redundant-break.md) | ✅ ПРОЙДЕН (отчёт: [reports/0213](../reports/0213-c-redundant-break.md)) |
| 0244 | Стоимость тестовых целей | [0244-test-target-build-cost.md](0244-test-target-build-cost.md) | ✅ ПРОЙДЕН (отчёт: [reports/0244](../reports/0244-test-target-build-cost.md)) |
| 0243 | Переопределение типа | [0243-type-redefinition-diagnostic.md](0243-type-redefinition-diagnostic.md) | ✅ ПРОЙДЕН (отчёт: [reports/0243](../reports/0243-type-redefinition-diagnostic.md)) |
| 0214 | Регистровый интерфейс sv-mmio | [0214-sv-mmio-unused-write-signals.md](0214-sv-mmio-unused-write-signals.md) | ✅ ПРОЙДЕН (отчёт: [reports/0214](../reports/0214-sv-mmio-unused-write-signals.md)) |
| 0240 | Перевод документа book/ в формат Typst | [0240-book-typst.md](0240-book-typst.md) | ГОТОВО |
| 0245 | Симулятор исполняет S(Модель) — проверку состояния под-модели | [0245-sim-state-of-model.md](0245-sim-state-of-model.md) | ГОТОВО |
| 0237 | Раздел «Импорты» не описывает S(Модель) | [0237-book-state-of-model-section.md](0237-book-state-of-model-section.md) | ГОТОВО |
| 0246 | Ссылка вперёд в инициализаторе переменной — ошибка компиляции | [0246-init-forward-reference.md](0246-init-forward-reference.md) | ГОТОВО |
| 0222 | Раздел документа о свёртке инициализатора | [0222-book-variables-const-fold.md](0222-book-variables-const-fold.md) | ГОТОВО |
| 0223 | Три примера объясняют выходной порт устаревшей нуждой цели rust | [0223-examples-port-rationale-stale.md](0223-examples-port-rationale-stale.md) | ГОТОВО |
| 0224 | Подъём Kotlin в плагине intellij-takt снимет ограничение на пусковой JDK | [0224-intellij-kotlin-upgrade.md](0224-intellij-kotlin-upgrade.md) | ГОТОВО |
| 0225 | Модуль semantic/statement.rs — 999 строк при пределе 1000 | [0225-statement-module-size.md](0225-statement-module-size.md) | ГОТОВО |
| 0221 | Панель структуры: инвариант состояния символом не становится | [0221-lsp-state-invariant-symbol.md](0221-lsp-state-invariant-symbol.md) | ГОТОВО |
| 0220 | Флаг -Wextra для гейта цели c: 38 предупреждений одного класса | [0220-c-gate-wextra.md](0220-c-gate-wextra.md) | ГОТОВО |
| 0218 | Реестры стадий 5 и 6 хранят заготовочное СОЗДАНА в колонках вердикта | [0218-registry-verdict-placeholder.md](0218-registry-verdict-placeholder.md) | ГОТОВО |
| 0217 | Ветвь-заглушка до будущей задачи переживает саму задачу | [0217-stub-branch-gate.md](0217-stub-branch-gate.md) | ГОТОВО |
| 0248 | Встроенные функции min/max/abs/clamp/debug не исполняются эталоном | [0248-sim-builtin-functions.md](0248-sim-builtin-functions.md) | ГОТОВО |
| 0247 | Голое имя состояния и модели в условии не исполняется эталоном | [0247-sim-bare-state-condition.md](0247-sim-bare-state-condition.md) | ГОТОВО |
| 0249 | Левая часть присваивания — место записи | [0249-assign-to-call-place.md](0249-assign-to-call-place.md) | ГОТОВО |
| 0250 | Запись бита x.N := v | [0250-bit-write-in-targets.md](0250-bit-write-in-targets.md) | ГОТОВО |
| 0145 | Потолок верификации по данным считается по рёбрам | [0145-verify-vertex-budget.md](0145-verify-vertex-budget.md) | ГОТОВО |
| 0153 | Рабочая область: references и rename между файлами | [0153-lsp-workspace-index.md](0153-lsp-workspace-index.md) | ГОТОВО |
| 0251 | Единый каталог сборки | [0251-cargo-target-dir.md](0251-cargo-target-dir.md) | ГОТОВО |
| 0154 | Переименование отдано серверу | [0154-intellij-server-rename.md](0154-intellij-server-rename.md) | ГОТОВО |
| 0150 | Позиционная форма предупреждает о себе | [0150-sim-positional-scenario-deprecation.md](0150-sim-positional-scenario-deprecation.md) | ГОТОВО |
| 0158 | Запуск инструментов из IDE | [0158-intellij-run-configurations.md](0158-intellij-run-configurations.md) | ГОТОВО |
| 0165 | Подкоманда taktc version | [0165-taktc-version-subcommand.md](0165-taktc-version-subcommand.md) | ГОТОВО |
| 0161 | Остаточные старые имена языка в данных и комментариях | [0161-fixture-comments-rename.md](0161-fixture-comments-rename.md) | ГОТОВО |
| 0162 | Пропущенные метки версий языка и сторож правила 22 | [0162-git-tag-v040.md](0162-git-tag-v040.md) | ГОТОВО |
| 0266 | Чтение порта в инициализаторе объявления | [0266-port-in-declaration-initializer.md](0266-port-in-declaration-initializer.md) | ГОТОВО |
| 0291 | Решение «ребро безусловно» — у одного носителя | [0291-rust-sv-unresolved-condition.md](0291-rust-sv-unresolved-condition.md) | ГОТОВО |
| 0300 | Дробная арифметика в инициализаторе объявления | [0300-fractional-init-arithmetic.md](0300-fractional-init-arithmetic.md) | ГОТОВО |
| 0284 | Структура без полей | [0284-empty-struct-semantics.md](0284-empty-struct-semantics.md) | ГОТОВО |
| 0301 | Снятие замера расхождения | [0301-probe-checklist.md](0301-probe-checklist.md) | ГОТОВО |
| 0302 | Релиз и тег при подъёме минорной версии языка | [0302-release-on-language-minor.md](0302-release-on-language-minor.md) | ГОТОВО (проверяемая часть; прогон CI — за 0175) |
| 0285 | Ширина выведенного типа берётся у результата | [0285-inferred-width-from-result.md](0285-inferred-width-from-result.md) | ГОТОВО |
| 0287 | Расширение типов не знает именованных целых | [0287-wider-type-array-literal.md](0287-wider-type-array-literal.md) | ГОТОВО |
| 0262 | Широкий бит-вектор в целях c и rust | [0262-wide-bit-vector-c-rust.md](0262-wide-bit-vector-c-rust.md) | ГОТОВО |
| 0263 | Приведение индекса к usize по нужде | [0263-rust-literal-index-cast.md](0263-rust-literal-index-cast.md) | ГОТОВО |
| 0281 | Сравнение перечисления с числом в цели rust | [0281-rust-enum-compare-literal.md](0281-rust-enum-compare-literal.md) | ГОТОВО |
| 0299 | Не-ASCII имя в нижнем регистре у цели rust | [0299-rust-non-ascii-lowercase-name.md](0299-rust-non-ascii-lowercase-name.md) | ГОТОВО |
| 0295 | Хвостовой комментарий тела и его хозяин | [0295-format-element-comment-binding.md](0295-format-element-comment-binding.md) | ГОТОВО |
| 0279 | Вложенная модель подключённого файла и подсказка | [0279-qualified-import-model-reference.md](0279-qualified-import-model-reference.md) | ГОТОВО |
| 0264 | Координата судей тела — позиция употребления | [0264-body-judge-usage-position.md](0264-body-judge-usage-position.md) | ГОТОВО |
| 0273 | Недостижимое ребро — предупреждение SE-116 | [0273-unreachable-edge-warning.md](0273-unreachable-edge-warning.md) | ГОТОВО |
| 0276 | Диагностика семантики без кода и позиции | [0276-semantic-diagnostics-without-code.md](0276-semantic-diagnostics-without-code.md) | ГОТОВО |
| 0277 | Координата отказа цели — место употребления | [0277-expression-usage-position.md](0277-expression-usage-position.md) | ГОТОВО |
| 0282 | Собственная позиция формулы | [0282-formula-own-location.md](0282-formula-own-location.md) | ГОТОВО |
| 0296 | Порядок стадий построения — один носитель | [0296-semantic-stages-single-source.md](0296-semantic-stages-single-source.md) | ГОТОВО |
| 0278 | Мёртвая упаковка последовательной композиции | [0278-compact-implement-dead-branch.md](0278-compact-implement-dead-branch.md) | ГОТОВО |
| 0260 | Неиспользуемый параметр в порождённом C | [0260-c-unused-struct-parameter.md](0260-c-unused-struct-parameter.md) | ГОТОВО |
| 0267 | Проверка состояния соседней модели в целях | [0267-state-of-model-in-targets.md](0267-state-of-model-in-targets.md) | ГОТОВО |
| 0303 | Условное ребро состояния-композиции | [0303-composition-state-conditional-edge.md](0303-composition-state-conditional-edge.md) | ГОТОВО |
| 0286 | Вычислимое приведение в инициализаторе | [0286-sv-const-initializer-expression.md](0286-sv-const-initializer-expression.md) | ГОТОВО |
| 0293 | Структуры в целях st, rust и sv | [0293-struct-in-st-rust.md](0293-struct-in-st-rust.md) | ГОТОВО |
| 0253 | Старое имя языка в порождаемом коде (lam_q_*, LAM_Q_*) | [0253-legacy-names-in-generated-code.md](0253-legacy-names-in-generated-code.md) | ГОТОВО |
| 0292 | Код CC-022 обещан комментарием, но не эмитируется никем | [0292-cc022-promise-without-emitter.md](0292-cc022-promise-without-emitter.md) | ГОТОВО |
| 0255 | Коды симулятора, вплавленные в текст, невидимы гейту и реестру | [0255-sim-diagnostic-codes-registry.md](0255-sim-diagnostic-codes-registry.md) | ГОТОВО |
| 0256 | Символ формы import { A } объявляется видом Model | [0256-lsp-import-binding-kind.md](0256-lsp-import-binding-kind.md) | ГОТОВО |
| 0258 | Verdict::Unsupported не различает причину отказа | [0258-verify-unsupported-reason.md](0258-verify-unsupported-reason.md) | ГОТОВО |
| 0259 | Встроенные функции языка не описаны в документе book/ | [0259-book-builtin-functions.md](0259-book-builtin-functions.md) | ГОТОВО |
| 0268 | У SE-033 нет описания в приложении «Ошибки» | [0268-se033-appendix-description.md](0268-se033-appendix-description.md) | ГОТОВО |
| 0274 | Снимки порождённого кода в book/ никем не сверяются | [0274-book-generated-snapshots-gate.md](0274-book-generated-snapshots-gate.md) | ГОТОВО |
| 0275 | Команды в README.md никем не проверяются | [0275-readme-commands-gate.md](0275-readme-commands-gate.md) | ГОТОВО |
| 0290 | Приложение «Ошибки» сверяется с реестром диагностик | [0290-book-diagnostics-codes-gate.md](0290-book-diagnostics-codes-gate.md) | Пройден: 9/9 условий, 3 мутации подтверждены |
| 0298 | Списки лексики раздела «Лексика» сверяются с языком | [0298-book-lexicon-lists-sync.md](0298-book-lexicon-lists-sync.md) | Пройден: 11/11 условий, пример и контрпример |
| 0215 | Потактовые сверки длительностей для целей st и sv | [0215-duration-per-tick-conformance-st-sv.md](0215-duration-per-tick-conformance-st-sv.md) | Пройден: 8/8 условий, мутация подтверждена |
| 0216 | Печатник живости цели rust получает сторожа поведения | [0216-rust-live-printer-coverage.md](0216-rust-live-printer-coverage.md) | Пройден: 13/13 условий, 3 мутации подтверждены |
| 0254 | Старое имя изъято из служебных идентификаторов | [0254-legacy-names-internal-identifiers.md](0254-legacy-names-internal-identifiers.md) | Пройден: 12/12 условий, 6 форм ловится |
| 0269 | Подсветка блоков st и ebnf в документе | [0269-book-st-syntax-highlight.md](0269-book-st-syntax-highlight.md) | Пройден: 12/12 условий |
| 0270 | Вес PDF документа: причина найдена, теги отключены | [0270-book-pdf-size.md](0270-book-pdf-size.md) | Пройден: 7/7 условий, контроль возврата тегов |
| 0283 | Печать результата компиляции сведена к одной функции | [0283-cli-report-result-merge.md](0283-cli-report-result-merge.md) | Пройден: 8/8 условий, 2 мутации |
| 0294 | SE-102 называет файл, подключающий библиотеку | [0294-se102-suggest-importer.md](0294-se102-suggest-importer.md) | Пройден: 10/10 условий |
| 0164 | Реестры стадий сверяются с файлами на диске | [0164-registry-rebuild-gate.md](0164-registry-rebuild-gate.md) | Пройден: 9/9 условий, 2 мутации |
| 0173 | Заглушка too_many_arguments снята в цели rust | [0173-rust-generator-arg-count.md](0173-rust-generator-arg-count.md) | Пройден: 6/6 условий, вывод побайтово тот же |
| 0252 | CI и замер покрытия приведены к снятому правилу однопоточности | [0252-ci-windows-test-threads.md](0252-ci-windows-test-threads.md) | Пройден: 5/5 условий (прогон CI недоступен — 0175) |
| 0261 | Жёлтая зона гейта размера модулей и его сторож | [0261-module-size-warning-zone.md](0261-module-size-warning-zone.md) | Пройден: 10/10 условий |
| 0265 | SVG диаграмм не несут версию graphviz | [0265-book-svg-graphviz-version.md](0265-book-svg-graphviz-version.md) | Пройден: 8/8 условий, обе пробы ловятся |
