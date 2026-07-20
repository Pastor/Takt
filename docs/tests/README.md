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
| 0023 | Плагин IntelliJ IDEA — навигация к декларации и include | [0023-intellij-navigation-include.md](0023-intellij-navigation-include.md) | СОЗДАНА |
| 0025 | Починка вычислителя выражений симулятора | [0025-simulator-expression-eval.md](0025-simulator-expression-eval.md) | ✅ ПРОЙДЕН (отчёт: [reports/0025](../reports/0025-simulator-expression-eval.md)) |
| 0024 | Канонический форматтер .lam (lamc fmt) | [0024-lam-formatter.md](0024-lam-formatter.md) | ✅ ПРОЙДЕН (отчёт: [reports/0024](../reports/0024-lam-formatter.md)) |
| 0026 | Генератор C: typedef корневой структуры для одиночной модели | [0026-c-root-typedef.md](0026-c-root-typedef.md) | СОЗДАНА |
| 0027 | Разделение переросших модулей (validate.rs, lsp.rs, c_expr.rs) | [0027-module-size-split.md](0027-module-size-split.md) | СОЗДАНА |
| 0028 | Заглушки генератора C: диагностика вместо тихого пропуска | [0028-c-generator-stubs.md](0028-c-generator-stubs.md) | СОЗДАНА |
| 0029 | Генератор C: отображение типов Array/Bit/Rational | [0029-c-type-mapping.md](0029-c-type-mapping.md) | СОЗДАНА |
| 0030 | Исправление примера comprehensive.lam (недостижимый сценарий) | [0030-comprehensive-example-fix.md](0030-comprehensive-example-fix.md) | СОЗДАНА |
| 0031 | Вызов функции из тела функции | [0031-fn-calls-fn.md](0031-fn-calls-fn.md) | ✅ ГОТОВО |
| 0032 | Сохранение переменных модели в --save-state/--load-state | [0032-state-io-variables.md](0032-state-io-variables.md) | ✅ ГОТОВО |
| 0033 | Согласование тактов симулятора и порождённого C (INIT-такты) | [0033-init-tick-alignment.md](0033-init-tick-alignment.md) | ✅ ГОТОВО |
| 0034 | Структурные типы в симуляторе | [0034-sim-struct-types.md](0034-sim-struct-types.md) | ГОТОВО |
| 0035 | LTL-формулы в блоках кода: разбор вместо тихой потери | [0035-ltl-in-blocks.md](0035-ltl-in-blocks.md) | ✅ ГОТОВО |
| 0036 | Согласование видимости публичного API крейта simulation | [0036-sim-visibility.md](0036-sim-visibility.md) | СОЗДАНА |
| 0037 | Сбои тестов на Windows (пути include, ресурс viewport) | [0037-windows-test-failures.md](0037-windows-test-failures.md) | СОЗДАНА |
| 0038 | Семантическая подсветка Lam в IntelliJ через lam-lsp | [0038-intellij-semantic-tokens.md](0038-intellij-semantic-tokens.md) | СОЗДАНА |
| 0039 | Действие Reformat Code в плагине IntelliJ | [0039-intellij-reformat.md](0039-intellij-reformat.md) | СОЗДАНА |
| 0040 | Полноценный PSI-парсер плагина IntelliJ | [0040-intellij-psi-parser.md](0040-intellij-psi-parser.md) | СОЗДАНА |
| 0041 | Бэкенд генерации в Structured Text (IEC 61131-3) | [0041-st-backend.md](0041-st-backend.md) | СОЗДАНА |
| 0042 | Инъекция define'ов для адресов (--define) | [0042-address-defines.md](0042-address-defines.md) | СОЗДАНА |
| 0043 | Экспорт карты адресов во внешний формат | [0043-address-map-export.md](0043-address-map-export.md) | ГОТОВО |
| 0044 | Юнит-конструкции языка для симуляции (assert/invariant) | [0044-sim-assert-invariant.md](0044-sim-assert-invariant.md) | ✅ ГОТОВО |
| 0045 | Бэкенд генерации в SystemVerilog | [0045-sv-backend.md](0045-sv-backend.md) | СОЗДАНА |
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
| 0058 | Хвостовой разворот `return` — заход в завершающий `if/else` | [0058-rust-tail-return-if-else.md](0058-rust-tail-return-if-else.md) | СОЗДАНА |
| 0059 | Общие переменные корня → структура `Shared` | [0059-rust-shared-struct.md](0059-rust-shared-struct.md) | СОЗДАНА |
| 0060 | Диапазон и знак перечисления — один расчёт на все цели | [0060-enum-width-shared-layer.md](0060-enum-width-shared-layer.md) | СОЗДАНА |
| 0061 | Fixed-point Q(m.n) как тип языка | [0061-fixed-point-type.md](0061-fixed-point-type.md) | СОЗДАНА |
| 0062 | Цель `sv-mmio` — адреса портов как регистровый файл | [0062-sv-mmio-target.md](0062-sv-mmio-target.md) | СОЗДАНА |
| 0063 | Порт `en` (clock enable) для цели `sv` | [0063-sv-clock-enable.md](0063-sv-clock-enable.md) | СОЗДАНА |
| 0064 | Предупреждение о делителе (`SV-009`) в цели `sv` | [0064-sv-divider-warning.md](0064-sv-divider-warning.md) | СОЗДАНА |
| 0065 | Изоляция пространства имён цели `st` | [0065-st-namespace-isolation.md](0065-st-namespace-isolation.md) | СОЗДАНА |
| 0066 | Литералы по целевому типу в телах цели `st` | [0066-st-bool-literals.md](0066-st-bool-literals.md) | СОЗДАНА |
| 0068 | Верификация свойств над данными | [0068-verify-data-properties.md](0068-verify-data-properties.md) | СОЗДАНА |
| 0069 | Разделение `address_map.rs` | [0069-address-map-eval-split.md](0069-address-map-eval-split.md) | СОЗДАНА |
| 0096 | Q-арифметика через нативный float и флаг генерации (embedded ↔ float) | [0096-fixed-point-native-float.md](0096-fixed-point-native-float.md) | СОЗДАНА |
| 0097 | Пример ПИД-регулятора на языке Lam (fixed-point) | [0097-pid-regulator-example.md](0097-pid-regulator-example.md) | СОЗДАНА |
| 0090 | CI прогоняет весь `precheck.sh` (живые гейты + check-links) | [0090-ci-precheck.md](0090-ci-precheck.md) | ✅ ГОТОВО (T1,T4–T10 локально; T2/T3/T11 — блокер биллинга Actions) |
| 0070 | Инициализатор порта — это адрес, а не значение | [0070-port-initializer-address-role.md](0070-port-initializer-address-role.md) | ✅ ГОТОВО (T1–T11; SE-035 снят с портов; вывод корпуса не изменён) |
| 0071 | Переход на имя состояния в `S(Ping) = End` | [0071-lsp-goto-state-name.md](0071-lsp-goto-state-name.md) | ✅ ГОТОВО (T2/T2b/T4/T7; кросс-модельный `S(Ping)=End` + внутримодельный `x=Done`; кодоген байт-в-байт) |
| 0073 | `Location::filename()` возвращает номер, а не путь | [0073-location-filename-path.md](0073-location-filename-path.md) | ✅ ГОТОВО (T1–T6; метод удалён, покрытие держит `try_file_no`; вывод корпуса не изменён) |
