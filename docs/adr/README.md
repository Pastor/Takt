# Реестр ADR (архитектурные решения)

Стадия 2 жизненного цикла (правило 17 в [../RULE.md](../RULE.md)). Каждая фича,
меняющая архитектуру/синтаксис/семантику языка, получает ADR `XXXX-slug.md`.
Если ADR меняет синтаксис или семантику — в него добавляется диаграмма
активности или EBNF на PlantUML (правило 18).

Заготовка создаётся из шаблона [`../templates/adr.md`](../templates/adr.md).

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
