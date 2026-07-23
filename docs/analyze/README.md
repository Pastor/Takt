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
