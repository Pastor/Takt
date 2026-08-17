# Реестр ADR (архитектурные решения)

Стадия 2 жизненного цикла (правило 17 в [../RULE.md](../RULE.md)). Каждая фича,
меняющая архитектуру/синтаксис/семантику языка, получает ADR `XXXX-slug.md`.
Если ADR меняет синтаксис или семантику — в него добавляется диаграмма
активности или EBNF на PlantUML (правило 18).

Заготовка создаётся из шаблона [`../templates/adr.md`](../templates/adr.md).

> **Фичи без ADR — норма, а не долг** (фича 0092). До-процессные фичи (0001–0018,
> заведённые до оформления стадии 2) и любые фичи, **не меняющие** архитектуру/
> синтаксис/семантику языка (например [0018](../features/0018-code-guidelines.md)
> — рефакторинг под `docs/CODE.md`), ADR **не имеют** — по правилу выше это
> корректно. Ссылаться на такую фичу как на **образец ADR** нельзя: образец
> структуры ADR — [0024](./0024-lam-formatter.md); образцом карточки/анализа сама
> 0018 служить может. Ретроспективные ADR постфактум не заводятся (ADR 0092).

| ADR | Заголовок | Статус | Фича |
|-----|-----------|--------|------|
| [0019](./0019-condition-expression-unification.md) | Унификация грамматик Condition/Expression | Accepted | фича 0019 |
| [0020](./0020-port-address-decl.md) | Адрес порта: размещение + потребление (карта адресов) | Accepted (пересм. 2026-07-14) | фича 0020 |
| [0021](./0021-swap-assign-compare.md) | Смена операторов: `<=` присваивание, `=` сравнение | Accepted | фича 0021 |
| [0022](./0022-intellij-syntax-highlight.md) | Плагин IntelliJ IDEA: подсветка синтаксиса Lam | Accepted | фича 0022 |
| [0023](./0023-intellij-navigation-include.md) | Плагин IntelliJ IDEA — навигация к декларации и include | Accepted | фича 0023 |
| [0024](./0024-lam-formatter.md) | Канонический форматтер .lam (lamc fmt) | Accepted | фича 0024 |
| [0025](./0025-simulator-expression-eval.md) | Починка вычислителя выражений симулятора | Accepted | фича 0025 |
| [0026](./0026-c-root-typedef.md) | Генератор C: typedef корневой структуры для одиночной модели | Accepted | фича 0026 |
| [0027](./0027-module-size-split.md) | Разделение переросших модулей (validate.rs, lsp.rs, c_expr.rs) | Accepted | фича 0027 |
| [0028](./0028-c-generator-stubs.md) | Заглушки генератора C: диагностика вместо тихого пропуска | Accepted | фича 0028 |
| [0029](./0029-c-type-mapping.md) | Генератор C: отображение типов Array/Bit/Rational | Accepted | фича 0029 |
| [0030](./0030-comprehensive-example-fix.md) | Исправление примера comprehensive.lam (недостижимый сценарий) | Accepted | фича 0030 |
| [0031](./0031-fn-calls-fn.md) | Вызов функции из тела функции — композиция без рекурсии | Accepted | фича 0031 |
| [0032](./0032-state-io-variables.md) | Единый источник истины для значений переменных симулятора | Accepted | фича 0032 |
| [0033](./0033-init-tick-alignment.md) | Согласование тактов симулятора и порождённого C (INIT-такты) | Accepted | фича 0033 |
| [0034](./0034-sim-struct-types.md) | Структурное значение в симуляторе | Accepted | фича 0034 |
| [0035](./0035-ltl-in-blocks.md) | LTL в блоках кода — разбор с явной диагностикой вместо тихой потери | Accepted | фича 0035 |
| [0036](./0036-sim-visibility.md) | Согласование видимости публичного API крейта simulation | Accepted | фича 0036 |
| [0037](./0037-windows-test-failures.md) | Кросс-платформенность тестов и Windows в матрице CI | Accepted | фича 0037 |
| [0038](./0038-intellij-semantic-tokens.md) | Семантическая подсветка Lam в IntelliJ через lam-lsp | Accepted | фича 0038 |
| [0039](./0039-intellij-reformat.md) | Действие Reformat Code в плагине IntelliJ | Accepted (обновление 2026-07-19: развилка → Option B, LSP4IJ) | фича 0039 |
| [0040](./0040-intellij-psi-parser.md) | Полноценный PSI-парсер плагина IntelliJ — собственный PSI, LSP4IJ или гибрид | Accepted | фича 0040 |
| [0041](./0041-st-backend.md) | Бэкенд генерации в Structured Text (IEC 61131-3) | Accepted | фича 0041 |
| [0042](./0042-address-defines.md) | Инъекция define'ов для адресов — среда символов адреса (`--define`) | Accepted | фича 0042 |
| [0043](./0043-address-map-export.md) | Формат и форма экспорта карты адресов портов | Accepted | фича 0043 |
| [0044](./0044-sim-assert-invariant.md) | Именованный инвариант `invariant` и оживление Guard-формул в симуляторе | Accepted | фича 0044 |
| [0045](./0045-sv-backend.md) | Бэкенд генерации в SystemVerilog | Accepted | фича 0045 |
| [0046](./0046-build-warnings-cleanup.md) | Устранение всех предупреждений сборки (rustc + clippy) | Proposed | фича 0046 |
| [0047](0047-c-state-of-model.md) | Трансляция `S(Модель) = Состояние` в цель `c` | Accepted |
| [0048](./0048-deterministic-codegen.md) | Детерминированная генерация кода (единый порядок эмиссии) | Accepted | фича 0048 |
| [0049](./0049-model-checking-ltl.md) | Верификация модели (Model Checking) на основе LTL | Draft | фича 0049 |
| [0050](./0050-rust-backend.md) | Бэкенд генерации в Rust | Accepted | фича 0050 |
| [0051](./0051-verify-scope.md) | Область проверки lamc verify (--scope) | Accepted | фича 0051 |
| [0052](./0052-verify-iterative-traversal.md) | Итеративные обходы в verification/ (снятие потолка стека) | Accepted | фича 0052 |
| [0053](./0053-diagnostics-file-id.md) | Идентификатор файла в позициях диагностик (file_no) | Accepted | фича 0053 |
| [0054](./0054-sim-diagnostics-positions.md) | Позиции в диагностиках симулятора | Accepted | фича 0054 |
| [0055](./0055-lsp-multifile.md) | Многофайловость LSP: импорты и позиции диагностик | Accepted | фича 0055 |
| [0056](./0056-lsp-goto-exact-file.md) | Точный путь вместо угадывания в goto_declaration | Accepted | фича 0056 |
| [0057](./0057-sv-sequential-composition.md) | Последовательная композиция (`+`) в цели SystemVerilog | Accepted | фича 0057 |
| [0058](./0058-rust-tail-return-if-else.md) | Хвостовой разворот `return` в цели `rust` — заход в завершающий `if/else` | Accepted | фича 0058 |
| [0059](./0059-rust-shared-struct.md) | Общие переменные корня — структура `Shared` вместо параметров по одной | Accepted | фича 0059 |
| [0060](./0060-enum-width-shared-layer.md) | Диапазон и знак перечисления — один расчёт на все цели | Accepted | фича 0060 |
| [0061](./0061-fixed-point-type.md) | Fixed-point Q(m.n) как тип языка (закрывает отложенный Option R-B ADR 0045) | Accepted | фича 0061 |
| [0062](./0062-sv-mmio-target.md) | Цель `sv-mmio` — адреса портов как регистровый файл (шинно-агностичный) | Accepted | фича 0062 |
| [0063](./0063-sv-clock-enable.md) | Порт `en` (clock enable) для цели `sv` — пересмотр Option C ADR 0045 | Accepted | фича 0063 |
| [0064](./0064-sv-divider-warning.md) | Предупреждение о делителе (`SV-009`) — только на переменный делитель | Accepted | фича 0064 |
| [0065](./0065-st-namespace-isolation.md) | Изоляция пространства имён цели `st` (префикс POU + `ST-014`) | Accepted | фича 0065 |
| [0066](./0066-st-bool-literals.md) | Литералы по целевому типу в телах `st` (`BOOL` и перечисления) | Accepted | фича 0066 |
| [0068](./0068-verify-data-properties.md) | Верификация свойств над данными — абстракция по формуле | Accepted | фича 0068 |
| [0069](./0069-address-map-eval-split.md) | Разделение `address_map.rs` — по темам, а не по лимиту | Accepted | фича 0069 |
| [0096](./0096-fixed-point-native-float.md) | Q-арифметика через нативный float и флаг генерации (embedded ↔ float) | Accepted | фича 0096 |
| [0097](./0097-pid-regulator-example.md) | Пример ПИД-регулятора на языке Lam (fixed-point) | Accepted | фича 0097 |
| [0090](./0090-ci-precheck.md) | CI прогоняет весь `precheck.sh` — единый источник истины гейтов | Accepted | фича 0090 |
| [0070](./0070-port-initializer-address-role.md) | Инициализатор порта — это адрес, а не значение (SE-035 снимается с портов) | Accepted | фича 0070 |
| [0071](./0071-lsp-goto-state-name.md) | Переход на имя состояния в условии — `ConditionNode::State` несёт use-site | Accepted | фича 0071 |
| [0073](./0073-location-filename-path.md) | `Location::filename()` — удалить, а не «чинить путь» | Accepted | фича 0073 |
| [0086](./0086-sim-var-without-initializer.md) | `var` без инициализатора — нулевое значение по типу, а не `SIM-009` | Accepted | фича 0086 |

| [0074](./0074-parenthesised-state-of.md) | Скобочная форма `S(Модель)` — канонизация в единой воронке `resolve_condition` | Accepted | фича 0074 |
| [0083](./0083-model-always-block.md) | Model-level `always` — исполнять каждый такт во всех целях (Option B) | Accepted | фича 0083 |
| [0080](./0080-c-struct-defects.md) | Дефекты C по структурам: составной литерал, static const, SE-061 | Accepted | фича 0080 |
| [0079](./0079-sim-composition-ports.md) | Порты под-модели композиции — перечислять рекурсивно (PortNames::from_model) | Accepted | фича 0079 |
| [0072](./0072-lsp-initialization-options.md) | LSP не читает initializationOptions (пути поиска импортов) | Accepted | фича 0072 |
| [0076](./0076-sim-arrays.md) | Симулятор не исполняет массивы вовсе | Accepted | фича 0076 |
| [0077](./0077-diagnostic-code-registry.md) | Реестр кодов диагностик — единый источник + машинный гейт (Option A) | Accepted | фича 0077 |
| [0078](./0078-bit-array-semantics.md) | Семантика `[bit;N]` — упакованный бит-вектор в родных типах (Option A) | Accepted | фича 0078 |
| [0085](./0085-language-version-constant.md) | Константа версии языка в коде + гейт синхронизации с README | Accepted | фича 0085 |
| [0084](./0084-address-map-qualified-key.md) | Ключ карты адресов — квалифицированный (модель+порт) | Accepted | фича 0084 |
| [0087](./0087-invariant-soft-mode.md) | Мягкий режим инвариантов симулятора (записать и продолжить) | Accepted | фича 0087 |
| [0094](./0094-new-feature-script-fixes.md) | Доработка scripts/new-feature.sh (идемпотентность, статус ADR, поздние стадии) | Accepted | фича 0094 |
| [0093](./0093-wildcard-match-rule.md) | Запрет _ => в семантических узлах и разрешение конфликта с #[non_exhaustive] | Accepted | фича 0093 |
| [0091](./0091-module-size-rule-in-code-md.md) | Правило о размере модуля переносится в docs/CODE.md | Accepted | фича 0091 |
| [0067](./0067-intellij-rename-psi-import.md) | Rename и PsiReference для import в плагине IntelliJ (Option B: хирургический структурный PSI) | Accepted | фича 0067 |
| [0088](./0088-module-size-remaining.md) | Остальные нарушители лимита размера модуля | Accepted | фича 0088 |
| [0089](./0089-intellij-residual-checks.md) | Остаточные проверки плагина IntelliJ (0022/0023) | Accepted | фича 0089 |
| [0092](./0092-adr-0018-retrofit.md) | У фичи 0018 нет ADR | Accepted | фича 0092 |
| [0100](./0100-language-rename-takt.md) | Переименование языка Lam → Takt | Accepted | фича 0100 |
| [0101](./0101-language-book.md) | Документ описания языка Takt | Accepted | фича 0101 |
| [0117](./0117-book-tools.md) | Раздел документа «Инструментарий» | Draft | фича 0117 |
| [0118](./0118-book-showcase.md) | Раздел документа «Развёрнутый пример» | Draft | фича 0118 |
| [0119](./0119-book-appendices.md) | Приложения документа + приложение «Ошибки» | Draft | фича 0119 |
| [0120](./0120-book-error-notes.md) | Заметки о возможных ошибках в разделах документа | Draft | фича 0120 |
| [0121](./0121-book-example-walkthrough.md) | Разбор примеров в разделах с упором на тему | Draft | фича 0121 |
| [0122](./0122-book-pdf-latexmk.md) | Сборка PDF через latexmk (корректные кросс-ссылки) + Makefile | Draft | фича 0122 |
| [0123](./0123-book-keyword-highlight.md) | Подсветка ключевых слов языка в тексте документа | Draft | фича 0123 |
| [0124](./0124-verify-graph-export.md) | Экспорт графов верификации (Крипке/Бюхи/произведение) в Graphviz DOT | Accepted | фича 0124 |
| [0125](./0125-intellij-takt-lsp-tooling.md) | Плагин IntelliJ Takt: LSP-проверка, автодополнение, документация и настройки инструментов | Accepted | фича 0125 |
| [0126](./0126-language-comparison-diff.md) | Сравнительный анализ языка Takt с родственными языками (отчёт docs/DIFF.md) | Accepted | фича 0126 |
| [0139](./0139-remove-travis-config.md) | Удаление мёртвой конфигурации .travis.yml | Accepted | фича 0139 |
| [0137](./0137-toolchain-pin-msrv.md) | Фиксация толчейна Rust и MSRV | Accepted | фича 0137 |
| [0128](./0128-lexer-literal-overflow.md) | Диагностика вместо паники на числовом литерале больше i64::MAX | Accepted | фича 0128 |
| [0129](./0129-semantic-deep-nesting.md) | Устранение переполнения стека на глубине выражений и операторов | Accepted | фича 0129 |
| [0127](./0127-int-overflow-semantics.md) | Единая семантика переполнения целых во всех целях | Accepted | фича 0127 |
| [0133](./0133-book-examples-gate.md) | Гейт компиляции и симуляции примеров документа book/ | Accepted | фича 0133 |
| [0135](./0135-sim-qualified-port-names.md) | Квалифицированные имена портов в симуляторе | Accepted | фича 0135 |
| [0131](./0131-lsp-definition-references-rename.md) | LSP: definition, references и rename | Accepted | фича 0131 |
| [0130](./0130-diagnostics-batch.md) | Накопление диагностик — несколько ошибок за прогон | Accepted | фича 0130 |
| [0132](./0132-sim-named-port-scenarios.md) | Именованные порты в сценариях симулятора | Accepted | фича 0132 |
| [0140](./0140-backlog-revision-doc-split.md) | Ревизия витрины кандидатов и разделение ролей README и book | Draft | фича 0140 |
| [0138](./0138-coverage-measurement.md) | Измерение покрытия тестами | Draft | фича 0138 |
| [0136](./0136-perf-benchmarks.md) | Бенчмарки производительности | Draft | фича 0136 |
| [0134](./0134-language-time-model.md) | Модель времени в языке: литерал длительности, внешний источник времени и частота такта | Accepted | фича 0134 |
| [0178](./0178-editor-layer-language-sync.md) | Синхронизация редакторского слоя с языком | Accepted | фича 0178 |
| [0179](./0179-repo-url-cleanup.md) | Один адрес репозитория и гейт его единственности | Accepted | фича 0179 |
| [0144](./0144-int-literal-exponent.md) | Экспонента числового литерала вычисляется | Accepted | фича 0144 |
| [0155](./0155-semantic-nested-statement-resolution.md) | Семантическое разрешение тел вложенных операторов | Draft | фича 0155 |
| [0146](./0146-book-glyph-gate.md) | Гейт символов вне шрифта документа book/ | Draft | фича 0146 |
| [0149](./0149-claude-md-consistency-gate.md) | Гейт согласованности живого контекста CLAUDE.md | Draft | фича 0149 |
| [0180](./0180-claude-md-context-diet.md) | Сокращение живого контекста CLAUDE.md | Draft | фича 0180 |
| [0177](./0177-features-registry-status-gate.md) | Гейт согласованности статуса в реестре и в карточке фичи | Draft | фича 0177 |
| [0159](./0159-intellij-jdk21-build.md) | Фиксация требования JDK 21 для сборки плагина intellij-takt | Draft | фича 0159 |
| [0171](./0171-c-gate-werror.md) | Гейт цели c под -Werror | Draft | фича 0171 |
| [0181](./0181-sim-state-implementation-tick.md) | Симулятор исполняет реализацию состояния с переходом next | Accepted | фича 0181 |
| [0182](./0182-pid-library-and-application.md) | Библиотечный ПИД-регулятор и пример его применения | Accepted | фича 0182 |
| [0166](./0166-sv-example-sequential-composition.md) | Корпусной SV-транслируемый пример на последовательную композицию + | Accepted | фича 0166 |
| [0174](./0174-rust-new-without-default.md) | Цель rust: корневая модель без портов (clippy::new_without_default) | Accepted | фича 0174 |
| [0147](./0147-lsp-document-symbol-tests.md) | Тесты textDocument/documentSymbol | Accepted | фича 0147 |
| [0148](./0148-rust-printers-coverage.md) | Покрытие печатников цели rust тестами | Accepted | фича 0148 |
| [0163](./0163-builder-eval-exhaustive.md) | Исчерпывающий разбор узлов во втором вычислителе | Accepted | фича 0163 |
| [0143](./0143-after-const-duration.md) | `after` принимает константное выражение типа `duration`, а не только литерал | Accepted | фича 0143 |
| [0183](./0183-duration-type-in-targets.md) | Тип `duration` в целях генерации и вычисляемая выдержка | Accepted | фича 0183 |
| [0184](./0184-imported-shared-variables.md) | Общие переменные библиотечного файла в импортёре | Accepted | фича 0184 |
| [0185](./0185-model-parameters.md) | Параметризация моделей (ключевое слово `parameter`) | Accepted | фича 0185 |
| [0186](./0186-book-processor-example.md) | Раздел документа «Практический пример: процессор» (шины, кеш, ядра) | Draft | фича 0186 |
| [0157](./0157-literal-u64-representation.md) | Представление числового литерала: полная маска [bit;64] | Draft | фича 0157 |
| [0156](./0156-parser-depth-limit.md) | Ограничение глубины вложенности на уровне лексера/парсера | Draft | фича 0156 |
| [0176](./0176-bit-port-address-position.md) | Позиция бита у bit-порта с голым адресом | Draft | фича 0176 |
| [0187](./0187-port-io-redesign.md) | Пересмотр задания адресов и доступа к портам | Draft | фича 0187 |
| [0188](./0188-port-direction-everywhere.md) | Направление порта проверяется во всех позициях | Draft | фича 0188 |
| [0190](./0190-precheck-selective-gates.md) | Разделение предкоммита на компоненты и выборочный запуск | Accepted | фича 0190 |
| [0189](./0189-anonymous-ports.md) | Анонимные порты | Accepted | фича 0189 |
| [0196](./0196-editor-type-highlighting.md) | Подсветка имён типов отдельным цветом в LSP и плагинах | Accepted | фича 0196 |
| [0192](./0192-const-init-fold.md) | Константное выражение в инициализаторе объявления | Accepted | фича 0192 |
| [0193](./0193-shared-const-qualified.md) | Цели rust и sv: одноимённые константы разных моделей | Accepted | фича 0193 |
| [0194](./0194-sim-composition-model-always.md) | Симулятор теряет model-level always у модели-композиции | Accepted | фича 0194 |
| [0195](./0195-target-name-collisions.md) | Коллизии имён при отображении в пространство имён цели | Accepted | фича 0195 |
| [0198](./0198-formatter-comment-in-block.md) | Форматтер выносит комментарий из тела блока наружу | Accepted | фича 0198 |
| [0199](./0199-model-implements-brace-form.md) | Форма model M = A & B { … } не работает ни в одной стороне | Draft | фича 0199 |
| [0200](./0200-non-ascii-identifier-targets.md) | Идентификатор с не-ASCII буквами: язык принимает, цели sv и st не выражают | Accepted | фича 0200 |
| [0201](./0201-dead-lexemes.md) | Мёртвая лексика: слова и терминалы, которых грамматика не знает | Accepted | фича 0201 |
| [0202](./0202-fmt-diagnostic-formatting.md) | taktc fmt печатает синтаксическую ошибку Debug-дампом | Accepted | фича 0202 |
| [0152](./0152-semantic-recovery-element-boundary.md) | Восстановление на границе элемента в стадиях построения | Accepted | фича 0152 |
| [0197](./0197-language-code-style.md) | Стиль кода языка Takt — свод правил оформления и раздел документа | Accepted | фича 0197 |
| [0226](./0226-naming-convention-warning.md) | Канон именования: предупреждение в fmt и LSP | Accepted | фича 0226 |
| [0227](./0227-lsp-style-warning-with-errors.md) | Редактор показывает CS-001 и при ошибках в файле | Draft | фича 0227 |
| [0228](./0228-compile-warning-position.md) | Предупреждение taktc compile несёт позицию | Draft | фича 0228 |
| [0229](./0229-format-unsupported-position.md) | Отказ форматтера — диагностика с позицией | Draft | фича 0229 |
| [0230](./0230-format-corpus-sentinel.md) | Сторож форматтера: корпус восстановлен, KNOWN_GAPS с ратчетом | Draft | фича 0230 |
| [0231](./0231-diagnostic-text-no-debug.md) | Текст диагностики без внутреннего представления | Draft | фича 0231 |
| [0232](./0232-implicit-bool-warning-delivery.md) | Предупреждение о неявной булевости доезжает до пользователя | Draft | фича 0232 |
| [0233](./0233-single-boolean-predicate.md) | Правило булевости условия — одно | Draft | фича 0233 |
| [0151](./0151-diagnostics-batch-within-check.md) | Накопление диагностик внутри отдельной проверки validate | Draft | фича 0151 |
| [0160](./0160-takt-ebnf-sync.md) | Синхронизация эталона Takt.ebnf с актуальным синтаксисом | Accepted | фича 0160 |
| [0170](./0170-fixed-point-saturation.md) | Насыщение (saturation) для fixed-point q(m, n) | Accepted | фича 0170 |
| [0203](./0203-validate-formulas-traversal.md) | validate не обходит формулы: неизвестное имя в Guard молчит | Accepted | фича 0203 |
| [0234](./0234-precheck-time-profile.md) | Профилирование и ускорение предкоммита | Accepted (Option C + D) | фича 0234 |
| [0235](./0235-guard-formula-in-st-sv.md) | Цели st и sv теряют охранную формулу | Accepted (Option C) | фича 0235 |
| [0236](./0236-c-unresolved-condition-refusal.md) | Печатник цели c печатает пустоту на неразрешённом условии | Accepted (Option B) | фича 0236 |
| [0237](./0237-book-state-of-model-section.md) | Раздел «Импорты» не описывает S(Модель) | Draft | фича 0237 |
| [0238](./0238-claude-md-duplicate-invariants.md) | Живой контекст: раздел критических инвариантов дублирует подводные камни | Accepted (Option B) | фича 0238 |
| [0204](./0204-const-ref-type-inference.md) | Вывод типов не протягивает тип через ссылку константа-константа | Accepted (Option C) | фича 0204 |
| [0205](./0205-as-in-declaration-initializer.md) | Приведение as не вычисляется в инициализаторе объявления | Accepted (Option A) | фича 0205 |
| [0206](./0206-imported-enum-variant-in-match.md) | Вариант импортированного перечисления не разрешается в образце match | Accepted (Option B) | фича 0206 |
| [0207](./0207-bitwise-not-unsigned-literal.md) | Отрицание ~0 для беззнакового типа: два правила языка столкнулись | Accepted (Option B) | фича 0207 |
| [0208](./0208-const-evaluators-unification.md) | Три константных вычислителя компилятора живут порознь | Accepted (Option B) | фича 0208 |
| [0209](./0209-model-external-interface.md) | Внешний интерфейс модели: extern fn в симуляторе и агрегат как аргумент | Accepted | фича 0209 |
| [0172](./0172-empty-enum-semantics.md) | Семантика перечисления без вариантов | Accepted (Option B) | фича 0172 |
| [0168](./0168-generator-warnings-return.md) | Предупреждения генераторов возвращаются вызывающему | Accepted (Option C) | фича 0168 |
| [0167](./0167-c-enum-constants-usage.md) | Цель c использует объявленные константы перечисления | Accepted (Option C) | фича 0167 |
| [0169](./0169-sv-mmio-bus-adapters.md) | Адаптеры шин для цели sv-mmio (APB) | Accepted (Option C) | фича 0169 |
| [0210](./0210-st-array-shared-and-index.md) | Массив как общая переменная в цели st; индекс-выражение | Accepted (A1+B1+C1) | фича 0210 |
| [0211](./0211-c-missing-start-state-diagnostic.md) | Модель без состояний в реализации: отказ семантики `SE-106` | Accepted (Option B) | фича 0211 |
| [0212](./0212-c-diagnostic-without-code.md) | Отказ цели `c` с кодом: воронки `CC-022` и `CC-023` | Accepted (Option B) | фича 0212 |
| [0213](./0213-c-redundant-break.md) | Цель c печатает лишний break после безусловного перехода | Accepted (Option B) | фича 0213 |
| [0240](./0240-book-typst.md) | Перевод документа book/ в формат Typst | Draft | фича 0240 |
| [0241](./0241-precheck-speedup.md) | Ускорение предкоммита: конфигурации cargo, параллельный verilator, пути гейта | Accepted (Option B) | фича 0241 |
| [0242](./0242-grammar-crate-split.md) | Вынос грамматики в отдельный крейт takt-grammar | Accepted (Option B; фича ОТМЕНЕНА) | фича 0242 |
| [0244](./0244-test-target-build-cost.md) | Стоимость тестовых целей: 147 бинарников в предкоммите | Accepted (B, затем A) | фича 0244 |
