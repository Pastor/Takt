# Реестр отчётов о тестировании

Стадия 6 жизненного цикла (правило 17 в [../RULE.md](../RULE.md)). Отчёт
`XXXX-slug.md` формируется по результатам тестирования по обязательному формату
(правило 20): сводка прогонов и окружение, сверка с тест-планом, примеры и
контрпримеры, найденные дефекты со ссылками на фиксы, итоговый вердикт.

Заготовка создаётся из шаблона [`../templates/reports.md`](../templates/reports.md).

| Фича | Заголовок | Отчёт | Вердикт |
|------|-----------|-------|---------|
| 0036 | Согласование видимости публичного API крейта `simulation` | [0036-sim-visibility.md](0036-sim-visibility.md) | ✅ ГОТОВО |
| 0038 | семантическая подсветка Lam в IntelliJ | [0038-intellij-semantic-tokens.md](0038-intellij-semantic-tokens.md) | ✅ ГОТОВО |
| 0046 | Устранение всех предупреждений сборки | [0046-build-warnings-cleanup.md](0046-build-warnings-cleanup.md) | ✅ ГОТОВО |
| 0062 | цель `sv-mmio` — регистровый файл из адресов портов | [0062-sv-mmio-target.md](0062-sv-mmio-target.md) | ✅ ГОТОВО |
| 0063 | порт `en` (clock enable) для цели `sv` | [0063-sv-clock-enable.md](0063-sv-clock-enable.md) | ✅ ГОТОВО |
| 0064 | предупреждение о делителе (`SV-009`) в цели `sv` | [0064-sv-divider-warning.md](0064-sv-divider-warning.md) | ✅ ГОТОВО |
| 0075 | эталонная модель порождает компилируемый C | [0075-lib-src-reference-model.md](0075-lib-src-reference-model.md) | ✅ ГОТОВО |
| 0081 | `lamc compile` печатает предупреждения | [0081-lamc-print-warnings.md](0081-lamc-print-warnings.md) | ✅ ГОТОВО |
| 0082 | `unused.rs` обходит формулы | [0082-unused-formulas.md](0082-unused-formulas.md) | ✅ ГОТОВО |
| 0087 | Мягкий режим инвариантов симулятора (записать и продолжить) | [0087-invariant-soft-mode.md](0087-invariant-soft-mode.md) | ✅ ГОТОВО |
| 0084 | Ключ карты адресов — квалифицированный (модель+порт) | [0084-address-map-qualified-key.md](0084-address-map-qualified-key.md) | ✅ ГОТОВО |
| 0085 | Константа версии языка в коде + гейт синхронизации с README | [0085-language-version-constant.md](0085-language-version-constant.md) | ✅ ГОТОВО |
| 0072 | LSP не читает initializationOptions (пути поиска импортов) | [0072-lsp-initialization-options.md](0072-lsp-initialization-options.md) | ✅ ГОТОВО |
| 0018 | Приведение кода к docs/CODE.md | [0018-code-guidelines.md](0018-code-guidelines.md) | ✅ ГОТОВО |
| 0021 | Смена операторов: `:=` присваивание, `=` сравнение | [0021-swap-assign-compare.md](0021-swap-assign-compare.md) | ✅ ГОТОВО |
| 0022 | Плагин IntelliJ IDEA: подсветка синтаксиса Lam | [0022-intellij-syntax-highlight.md](0022-intellij-syntax-highlight.md) | ✅ ГОТОВО |
| 0023 | Плагин IntelliJ IDEA: навигация к декларации и include | [0023-intellij-navigation-include.md](0023-intellij-navigation-include.md) | ✅ ГОТОВО |
| 0025 | Починка вычислителя выражений симулятора | [0025-simulator-expression-eval.md](0025-simulator-expression-eval.md) | ✅ ПРОЙДЕН (после доработки 0025-06…08) |
| 0019 | Унификация грамматик Condition/Expression | [0019-condition-expression-unification.md](0019-condition-expression-unification.md) | ✅ ПРОЙДЕН |
| 0024 | Канонический форматтер .lam (lamc fmt) | [0024-lam-formatter.md](0024-lam-formatter.md) | ✅ ПРОЙДЕН |
| 0041 | Бэкенд генерации в Structured Text (IEC 61131-3) | [0041-st-backend.md](0041-st-backend.md) | ✅ ГОТОВО (гейт `iec2c` 5/5; поведенческая эквивалентность вне объёма — фича 0033) |
| 0026 | Генератор C: typedef корневой структуры для одиночной модели | [0026-c-root-typedef.md](0026-c-root-typedef.md) | ✅ ГОТОВО (порождённый C компилируется: 8 ошибок → 0) |
| 0029 | Генератор C: отображение типов Array/Bit/Rational | [0029-c-type-mapping.md](0029-c-type-mapping.md) | ✅ ГОТОВО (порождённый C компилируется на всех типах; версия языка **0.3.0**, тег `v0.3.0`) |
| 0030 | Исправление примера comprehensive.lam (недостижимый сценарий) | [0030-comprehensive-example-fix.md](0030-comprehensive-example-fix.md) | ✅ ГОТОВО (T1–T19; сценарий проходит за 172 шага, мёртвых рёбер 0; объём расширен заказчиком на починку генератора `st`; вскрыто, что гейт `rust` закреплял дефект как эталон) |
| 0027 | Разделение переросших модулей (validate/lsp/c_expr) | [0027-module-size-split.md](0027-module-size-split.md) | ✅ ГОТОВО (T12–T18; правило о размере стало исполнимым — храповик в precheck+CI; долг 16953→12049; храповик поймал собственную правку в первый день) |
| 0028 | Заглушки генератора C: диагностика вместо тихого пропуска | [0028-c-generator-stubs.md](0028-c-generator-stubs.md) | ✅ ГОТОВО (`CC-018`; вскрыл, что `S(Модель) = Состояние` в C не переводится) |
| 0047 | Трансляция `S(Модель) = Состояние` в цель `c` | [0047-c-state-of-model.md](0047-c-state-of-model.md) | ✅ ГОТОВО (эталон `syntax_simple` зелёный по существу) |
| 0048 | Детерминированная генерация кода (единый порядок эмиссии) | [0048-deterministic-codegen.md](0048-deterministic-codegen.md) | ✅ ГОТОВО (10 прогонов → 1 вариант; гейт в `precheck.sh`; ABI стабилен) |
| 0033 | Согласование тактов симулятора и порождённого C (INIT-такты) | [0033-init-tick-alignment.md](0033-init-tick-alignment.md) | ✅ ГОТОВО (тело на такте 1 на любой глубине; потактовая сверка; UB устранён) |
| 0032 | Сохранение переменных модели в --save-state/--load-state | [0032-state-io-variables.md](0032-state-io-variables.md) | ✅ ГОТОВО (Д1/Д2/Д3 закрыты; единое хранилище; `inout` работает; 5/5 stacker) |
| 0031 | Вызов функции из тела функции | [0031-fn-calls-fn.md](0031-fn-calls-fn.md) | ✅ ГОТОВО (композиция `f→g`; рекурсия → SE-053; форвард-прототипы в C) |
| 0044 | Юнит-конструкции языка для симуляции (assert/invariant) | [0044-sim-assert-invariant.md](0044-sim-assert-invariant.md) | ✅ ГОТОВО (`invariant`; атом LTL; симулятор проверяет формулы — SIM-025) |
| 0035 | LTL-формулы в блоках кода: разбор вместо тихой потери | [0035-ltl-in-blocks.md](0035-ltl-in-blocks.md) | ✅ ГОТОВО (паритет уровней; SE-055/SE-056 через `ltl_warnings`; C неизменен) |
| 0045 | Бэкенд генерации в SystemVerilog (FPGA/ASIC) | [0045-sv-backend.md](0045-sv-backend.md) | ✅ ГОТОВО (оба гейта зелёные; сдвиг = 0 на глубинах 1/2/3 против настоящего RTL; `SV-012` найдена сверх реестра; 6 расхождений проработки с фактом) |
| 0050 | Бэкенд генерации в Rust (`no_std`) | [0050-rust-backend.md](0050-rust-backend.md) | ✅ ГОТОВО (весь корпус транслируется; rustc+clippy под `-D warnings`; при приёмке заведена потактовая сверка — пробел T23) |
| 0049 | Верификация модели (Model Checking) на основе LTL | [0049-model-checking-ltl.md](0049-model-checking-ltl.md) | ✅ ГОТОВО (T1–T47; два дефекта вердикта вскрыты **не** тестами — фикс 0010-01 и недо-аппроксимация Крипке; область формулы состояния → `G (S -> φ)`) |
| 0051 | Область проверки `lamc verify` (`--scope`) | [0051-verify-scope.md](0051-verify-scope.md) | ✅ ГОТОВО (T1–T16; признак `origin` вместо суррогата; отсечение поддеревом; вскрыт кандидат — `file_no` ≡ 0 в позициях диагностик) |
| 0052 | Итеративные обходы — снятие потолка стека | [0052-verify-iterative-traversal.md](0052-verify-iterative-traversal.md) | ✅ ГОТОВО (T1–T12; 20000 состояний × 5 целей; зонд опроверг постановку — потолок был в генерации, а не в verification/) |
| 0053 | Позиции в диагностиках и настоящий `file_no` | [0053-diagnostics-file-id.md](0053-diagnostics-file-id.md) | ✅ ГОТОВО (T1–T13; зонд опроверг кандидата — дефект был латентным, а боль в отсутствии печати позиций) |
| 0054 | Позиции в диагностиках симулятора | [0054-sim-diagnostics-positions.md](0054-sim-diagnostics-positions.md) | ✅ ГОТОВО (T1–T8; печать вынесена в общий слой, а не скопирована — урок 0028-01; симулятор терял и код диагностики) |
| 0055 | Многофайловость LSP: импорты и позиции диагностик | [0055-lsp-multifile.md](0055-lsp-multifile.md) | ✅ ГОТОВО (T1–T10; зонд вскрыл, что импорт рядом не искал и сам `lamc` — лечение ушло в ядро; чужая ошибка привязана к строке import) |
| 0042 | Инъекция define'ов для адресов (`--define`) | [0042-address-defines.md](0042-address-defines.md) | ✅ ГОТОВО (T1–T24; закрыт тихий пропуск выражений адреса; версия языка НЕ поднималась — правило 22, 0.3.0 уже поднята 0029; 3 находки сверх проработки) |
| 0056 | Кросс-файловый переход к декларации | [0056-lsp-goto-exact-file.md](0056-lsp-goto-exact-file.md) | ✅ ГОТОВО (T1–T12; переход в импортированный файл заработал впервые; зонд опроверг постановку — узла на имени модели не существовало вовсе, объём расширен заказчиком на позицию у ссылок) |
| 0057 | Последовательная композиция (`+`) в цели SystemVerilog | [0057-sv-sequential-composition.md](0057-sv-sequential-composition.md) | ✅ ГОТОВО (A1–A7; сверка SV↔C потактово — симулятор непригоден из-за фикса 0057-01; `extend_complex` не SV-транслируем — extern fn) |
| 0058 | Хвостовой `if/else` с `return` в цели `rust` | [0058-rust-tail-return-if-else.md](0058-rust-tail-return-if-else.md) | ✅ ГОТОВО (A1–A7; свёртка через предикат `tail_foldable` + `print_tail`; `examples/generated/rust` побайтово неизменны; 9 новых тестов) |
| 0066 | Литералы по целевому типу в телах цели `st` (`BOOL`/перечисления) | [0066-st-bool-literals.md](0066-st-bool-literals.md) | ✅ ГОТОВО (A1–A8; `coerce_to` как в rust/sv; дифф `st` только литералы; `iec2c` зелёный; прочие цели побайтово прежние) |
| 0059 | Общие переменные корня → структура `Shared` в цели `rust` | [0059-rust-shared-struct.md](0059-rust-shared-struct.md) | ✅ ГОТОВО (A1–A10; `#[allow]` в выводе исчез — политика (а) без исключений; новый модуль `rust_shared`; conformance не изменился) |
| 0069 | Разделение `address_map.rs` по темам (снятие записи долга) | [0069-address-map-eval-split.md](0069-address-map-eval-split.md) | ✅ ГОТОВО (A1–A8; каталог `address_map/` — `parse`/`env`/`eval`/`resolve` + `mod.rs`-реэкспорт; вывод всех целей побайтово прежний; `apply_binary` одна; долг 22 → 21) |
| 0068 | Верификация свойств над данными (атом-предикат LTL) | [0068-verify-data-properties.md](0068-verify-data-properties.md) | ✅ ГОТОВО (A1–A9, консервативное ядро; `verification/data_kripke.rs` — вершина = состояние × оценка; `Holds` надёжен, направление ошибки под мутационным сторожем; данные полностью недетерминированы, полный Option D отложен; язык не менялся) |
| 0061 | Fixed-point `q(m, n)` как тип языка | [0061-05-example-and-docs.md](0061-05-example-and-docs.md) | ✅ ГОТОВО (задачи 01–05; отчёты [0061-03](0061-03-software-targets.md)/[04](0061-04-sv-target.md)/[05](0061-05-example-and-docs.md); Q-арифметика **побитово** едина у симулятора и 4 целей; ловушка C11 6.5.7p5 закрыта floor-делением; пример-регулятор проходит все гейты корпуса) |
| 0096 | прозрачный `float` через глобальную Q-точность | [0096-fixed-point-native-float.md](0096-fixed-point-native-float.md) | ✅ ГОТОВО |
| 0097 | Пример ПИД-регулятора на Lam (fixed-point) | [0097-pid-regulator-example.md](0097-pid-regulator-example.md) | ✅ ГОТОВО (A1–A6; позиционный ПИД с anti-windup на `q(8,8)`, объект 1-го порядка; сходится без входов; все гейты корпуса; конфликт имён с ФБ `INTEGRAL` IEC → `i_acc`/`meas`; компилятор не изменён) |
| 0039 | Действие Reformat Code в плагине IntelliJ | [0039-intellij-reformat.md](0039-intellij-reformat.md) | ✅ ГОТОВО (развилка → Option B LSP4IJ после 0038; форматирование бесплатно от `lam-lsp`; production-кода нет; приёмка A2 байт-в-байт на всём корпусе автотестом; остаток — визуальный `runIde`) |
| 0043 | Экспорт карты адресов во внешний формат | [0043-address-map-export.md](0043-address-map-export.md) | ✅ ГОТОВО (T1–T22 + K5; `lamc address-map --emit map\|json`; круговой рейс байт-в-байт; сверка с `c-hal`; CLI вынесен в библиотеку — `lamc.rs` пришпилен к baseline; крейт 0.5.0 → 0.6.0) |
| 0034 | Структурные типы в симуляторе | [0034-sim-struct-types.md](0034-sim-struct-types.md) | ✅ ГОТОВО (T1–T22; `Value::Struct` порядок полей; чтение/запись поля точечно с усечением; сверка с C `cc`; SIM-012/026–029; ⚠️ `eval_expr` не слит, `--load-state` структур не реализован; крейт `simulation` 0.2.0 → 0.3.0) |
| 0090 | CI прогоняет весь `precheck.sh` (живые гейты + check-links) | [0090-ci-precheck.md](0090-ci-precheck.md) | ✅ ГОТОВО с оговоркой (T1,T4–T10 локально; строгий `precheck.sh` EXIT=0, 110 строк гейтов, ST 8/8; A5/T2/T3/T11 не проверены — Actions заблокирован по биллингу; закрыта по локальной проверке) |
| 0070 | Инициализатор порта — это адрес, а не значение | [0070-port-initializer-address-role.md](0070-port-initializer-address-role.md) | ✅ ГОТОВО (Option A; VariableNode::Port выведен из-под check_bit_variable_value; 5 тестов в новом port_initializer_tests.rs; вывод корпуса байт-в-байт; версия языка не поднята) |
| 0037 | Сбои тестов на Windows (пути include, ресурс viewport) | [0037-windows-test-failures.md](0037-windows-test-failures.md) | ✅ ГОТОВО с оговоркой (T1–T7,T10,T13 локально; 5 тестов `-I` параметризованы `SEP`, `/tmp` убран из `viewport`, job `windows` = `cargo test` с `continue-on-error`; T8/T9/T11/T12 и A8/A9/A11 не проверены — Actions заблокирован по биллингу; прецедент 0090) |
| 0073 | `Location::filename()` возвращает номер, а не путь | [0073-location-filename-path.md](0073-location-filename-path.md) | ✅ ГОТОВО (T1–T6; ложно названный метод удалён, покрытие держит `try_file_no`; API сужен без потребителей; вывод корпуса не изменён) |
| 0086 | `var q: u8;` без инициализатора → `SIM-009` | [0086-sim-var-without-initializer.md](0086-sim-var-without-initializer.md) | ✅ ГОТОВО (T1–T7; ветка «нет init» → `default_field`, `default_struct` удалён; скаляр → 0 по типу; conformance зелён; массивы — вне объёма, 0076) |
| 0074 | Скобочная форма `S(…)` отвергается семантикой | [0074-parenthesised-state-of.md](0074-parenthesised-state-of.md) | ✅ ГОТОВО (Option C; канонизация в `resolve_condition`; сторож `SE-025` инвертирован; проба нашла 3-ю форму `S(Ping)=(End)`; вывод корпуса байт-в-байт; версия языка не поднята) |
| 0083 | Тело `always` на уровне модели не эмитится | [0083-model-always-block.md](0083-model-always-block.md) | ✅ ГОТОВО (Option B; эталона не было — терялось и в симуляторе; реализовано в симуляторе + c/rust/st/sv; помощники вынесены в *_blocks.rs; корпус байт-в-байт; версия языка не поднята) |
| 0080 | Дефекты генератора C по структурам | [0080-c-struct-defects.md](0080-c-struct-defects.md) | ✅ ГОТОВО (3 дефекта: (Type){…} составной литерал, static const вместо #define, SE-061 на несуществующем поле; SE-061 опережает SIM-027; корпус байт-в-байт; версия языка не поднята) |
| 0079 | `elevator_mini.lam` не исполняется: порты под-модели композиции | [0079-sim-composition-ports.md](0079-sim-composition-ports.md) | ✅ ГОТОВО (симптом SIM-009 замаскирован 0086; первопричина — extract_port_names не рекурсивна; PortNames::from_model вынесена в библиотеку; run_simulations матчинг по длинному префиксу; stacker не задет; версия языка не менялась) |
| 0094 | Доработка scripts/new-feature.sh (идемпотентность, статус ADR, поздние стадии) | [0094-new-feature-script-fixes.md](0094-new-feature-script-fixes.md) | ✅ ГОТОВО |
| 0093 | Запрет _ => в семантических узлах и разрешение конфликта с #[non_exhaustive] | [0093-wildcard-match-rule.md](0093-wildcard-match-rule.md) | ✅ ГОТОВО |
| 0091 | Правило о размере модуля переносится в docs/CODE.md | [0091-module-size-rule-in-code-md.md](0091-module-size-rule-in-code-md.md) | ✅ ГОТОВО |
| 0067 | Rename и PsiReference для import в плагине IntelliJ | [0067-intellij-rename-psi-import.md](0067-intellij-rename-psi-import.md) | ГОТОВО |
| 0089 | Остаточные проверки плагина IntelliJ (0022/0023) | [0089-intellij-residual-checks.md](0089-intellij-residual-checks.md) | ГОТОВО |
| 0092 | У фичи 0018 нет ADR | [0092-adr-0018-retrofit.md](0092-adr-0018-retrofit.md) | ГОТОВО |
| 0088 | Нарушители лимита размера модуля — безопасная часть | [0088-module-size-remaining.md](0088-module-size-remaining.md) | ✅ ГОТОВО |
| 0098 | диапазон бита адреса и безопасный дефолтный HAL | [0098-port-bit-range-safe-hal.md](0098-port-bit-range-safe-hal.md) | ✅ ГОТОВО |
| 0100 | Переименование языка Lam → Takt | [0100-language-rename-takt.md](0100-language-rename-takt.md) | ✅ ГОТОВО |
| 0125 | Плагин IntelliJ Takt: LSP-проверка, автодополнение, документация и настройки инструментов | [0125-intellij-takt-lsp-tooling.md](0125-intellij-takt-lsp-tooling.md) | ГОТОВО |
| 0139 | Удаление мёртвой конфигурации .travis.yml | [0139-remove-travis-config.md](0139-remove-travis-config.md) | ✅ ПРОЙДЕН |
| 0137 | Фиксация толчейна Rust и MSRV | [0137-toolchain-pin-msrv.md](0137-toolchain-pin-msrv.md) | ✅ ПРОЙДЕН |
| 0128 | Диагностика вместо паники на числовом литерале больше i64::MAX | [0128-lexer-literal-overflow.md](0128-lexer-literal-overflow.md) | ✅ ПРОЙДЕН |
| 0129 | Устранение переполнения стека на глубине выражений и операторов | [0129-semantic-deep-nesting.md](0129-semantic-deep-nesting.md) | ✅ ПРОЙДЕН |
| 0127 | Единая семантика переполнения целых во всех целях | [0127-int-overflow-semantics.md](0127-int-overflow-semantics.md) | ✅ ПРОЙДЕН |
| 0133 | Гейт компиляции и симуляции примеров документа book/ | [0133-book-examples-gate.md](0133-book-examples-gate.md) | ✅ ПРОЙДЕН |
| 0135 | Квалифицированные имена портов в симуляторе | [0135-sim-qualified-port-names.md](0135-sim-qualified-port-names.md) | ✅ ПРОЙДЕН |
| 0131 | LSP: definition, references и rename | [0131-lsp-definition-references-rename.md](0131-lsp-definition-references-rename.md) | ГОТОВО |
| 0130 | Накопление семантических диагностик | [0130-diagnostics-batch.md](0130-diagnostics-batch.md) | ГОТОВО |
| 0132 | Именованные порты в сценариях симулятора | [0132-sim-named-port-scenarios.md](0132-sim-named-port-scenarios.md) | ГОТОВО |
| 0140 | Ревизия витрины кандидатов и разделение ролей README и book | [0140-backlog-revision-doc-split.md](0140-backlog-revision-doc-split.md) | ГОТОВО |
| 0138 | Измерение покрытия тестами | [0138-coverage-measurement.md](0138-coverage-measurement.md) | ГОТОВО |
| 0136 | Бенчмарки производительности | [0136-perf-benchmarks.md](0136-perf-benchmarks.md) | ГОТОВО |
| 0178 | Приведение LSP и плагинов в соответствие языку + сторож | [0178-editor-layer-language-sync.md](0178-editor-layer-language-sync.md) | ✅ ПРОЙДЕН (17/17, дефектов нет) |
| 0179 | Дочистка URL репозитория после переезда BuT → Takt | [0179-repo-url-cleanup.md](0179-repo-url-cleanup.md) | ✅ ПРОЙДЕН (11/11, дефектов нет) |
| 0144 | Экспонента числового литерала | [0144-int-literal-exponent.md](0144-int-literal-exponent.md) | ✅ ПРОЙДЕН (15/15, дефектов нет) |
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
| 0143 | `after` принимает константное выражение типа duration | [0143-after-const-duration.md](0143-after-const-duration.md) | ГОТОВО |
| 0183 | Тип `duration` в целях и вычисляемая выдержка | [0183-duration-type-in-targets.md](0183-duration-type-in-targets.md) | ГОТОВО |
| 0184 | Общие переменные библиотечного файла в импортёре | [0184-imported-shared-variables.md](0184-imported-shared-variables.md) | ГОТОВО |
| 0182 | Библиотечный ПИД-регулятор и пример его применения | [0182-pid-library-and-application.md](0182-pid-library-and-application.md) | ГОТОВО |
| 0185 | Параметризация моделей (ключевое слово parameter) | [0185-model-parameters.md](0185-model-parameters.md) | ГОТОВО |
| 0186 | Раздел документа «Практический пример: процессор» (шины, кеш, ядра) | [0186-book-processor-example.md](0186-book-processor-example.md) | ГОТОВО |
| 0187 | Пересмотр задания адресов и доступа к портам | [0187-port-io-redesign.md](0187-port-io-redesign.md) | ✅ ГОТОВО (раздел «Вывод» отчёта) |
| 0190 | Разделение предкоммита на компоненты и выборочный запуск | [0190-precheck-selective-gates.md](0190-precheck-selective-gates.md) | ГОТОВО |
| 0157 | Представление числового литерала: полная маска [bit;64] | [0157-literal-u64-representation.md](0157-literal-u64-representation.md) | ГОТОВО |
| 0156 | Ограничение глубины вложенности на уровне лексера/парсера | [0156-parser-depth-limit.md](0156-parser-depth-limit.md) | ГОТОВО |
| 0176 | Позиция бита у bit-порта с голым адресом | [0176-bit-port-address-position.md](0176-bit-port-address-position.md) | ГОТОВО |
| 0188 | Направление порта проверяется во всех позициях | [0188-port-direction-everywhere.md](0188-port-direction-everywhere.md) | ГОТОВО |
| 0189 | Анонимные порты | [0189-anonymous-ports.md](0189-anonymous-ports.md) | ГОТОВО |
| 0196 | Подсветка имён типов отдельным цветом в LSP и плагинах | [0196-editor-type-highlighting.md](0196-editor-type-highlighting.md) | ГОТОВО |
| 0191 | Цель st: потактовая сверка с эталоном и устранение расхождений | [0191-st-per-tick-conformance.md](0191-st-per-tick-conformance.md) | ГОТОВО |
| 0192 | Константное выражение в инициализаторе объявления | [0192-const-init-fold.md](0192-const-init-fold.md) | ✅ ГОТОВО |
| 0193 | Цели rust и sv: одноимённые константы разных моделей | [0193-shared-const-qualified.md](0193-shared-const-qualified.md) | ✅ ГОТОВО |
| 0194 | Симулятор теряет model-level always у модели-композиции | [0194-sim-composition-model-always.md](0194-sim-composition-model-always.md) | ✅ ГОТОВО |
| 0195 | Коллизии имён при отображении в пространство имён цели | [0195-target-name-collisions.md](0195-target-name-collisions.md) | ✅ ГОТОВО |
| 0198 | Форматтер выносит комментарий из тела блока наружу | [0198-formatter-comment-in-block.md](0198-formatter-comment-in-block.md) | ✅ ГОТОВО |
| 0199 | Форма model M = A & B { … } не работает ни в одной стороне | [0199-model-implements-brace-form.md](0199-model-implements-brace-form.md) | ✅ ГОТОВО |
| 0200 | Идентификатор с не-ASCII буквами: язык принимает, цели sv и st не выражают | [0200-non-ascii-identifier-targets.md](0200-non-ascii-identifier-targets.md) | ✅ ГОТОВО |
| 0201 | Мёртвая лексика: слова и терминалы, которых грамматика не знает | [0201-dead-lexemes.md](0201-dead-lexemes.md) | ✅ ГОТОВО |
| 0202 | taktc fmt печатает синтаксическую ошибку Debug-дампом | [0202-fmt-diagnostic-formatting.md](0202-fmt-diagnostic-formatting.md) | ✅ ГОТОВО |
| 0152 | Восстановление на границе элемента в стадиях построения | [0152-semantic-recovery-element-boundary.md](0152-semantic-recovery-element-boundary.md) | ✅ ГОТОВО |
| 0197 | Стиль кода языка Takt — свод правил оформления и раздел документа | [0197-language-code-style.md](0197-language-code-style.md) | ✅ ГОТОВО |
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
| 0170 | Насыщение (saturation) для fixed-point q(m, n) | [0170-fixed-point-saturation.md](0170-fixed-point-saturation.md) | ✅ ГОТОВО |
| 0203 | validate не обходит формулы: неизвестное имя в Guard молчит | [0203-validate-formulas-traversal.md](0203-validate-formulas-traversal.md) | ✅ ГОТОВО |
| 0234 | Профилирование и ускорение предкоммита | [0234-precheck-time-profile.md](0234-precheck-time-profile.md) | ✅ ГОТОВО |
| 0235 | Цели st и sv теряют охранную формулу | [0235-guard-formula-in-st-sv.md](0235-guard-formula-in-st-sv.md) | ✅ ГОТОВО |
| 0236 | Печатник цели c печатает пустоту на неразрешённом условии | [0236-c-unresolved-condition-refusal.md](0236-c-unresolved-condition-refusal.md) | ✅ ГОТОВО |
| 0238 | Живой контекст: раздел критических инвариантов дублирует подводные камни | [0238-claude-md-duplicate-invariants.md](0238-claude-md-duplicate-invariants.md) | ✅ ГОТОВО |
| 0204 | Вывод типов не протягивает тип через ссылку константа-константа | [0204-const-ref-type-inference.md](0204-const-ref-type-inference.md) | ✅ ГОТОВО |
| 0205 | Приведение as не вычисляется в инициализаторе объявления | [0205-as-in-declaration-initializer.md](0205-as-in-declaration-initializer.md) | ✅ ГОТОВО |
| 0206 | Вариант импортированного перечисления не разрешается в образце match | [0206-imported-enum-variant-in-match.md](0206-imported-enum-variant-in-match.md) | ✅ ГОТОВО |
| 0207 | Отрицание ~0 для беззнакового типа: два правила языка столкнулись | [0207-bitwise-not-unsigned-literal.md](0207-bitwise-not-unsigned-literal.md) | ✅ ГОТОВО |
| 0208 | Три константных вычислителя компилятора живут порознь | [0208-const-evaluators-unification.md](0208-const-evaluators-unification.md) | ✅ ГОТОВО |
| 0209 | Внешний интерфейс модели: extern fn в симуляторе и агрегат как аргумент | [0209-model-external-interface.md](0209-model-external-interface.md) | ✅ ГОТОВО |
| 0172 | Семантика перечисления без вариантов | [0172-empty-enum-semantics.md](0172-empty-enum-semantics.md) | ✅ ГОТОВО |
| 0168 | Предупреждения генераторов возвращаются вызывающему | [0168-generator-warnings-return.md](0168-generator-warnings-return.md) | ✅ ГОТОВО |
| 0167 | Цель c использует объявленные константы перечисления | [0167-c-enum-constants-usage.md](0167-c-enum-constants-usage.md) | ✅ ГОТОВО |
| 0169 | Адаптеры шин для цели sv-mmio (APB) | [0169-sv-mmio-bus-adapters.md](0169-sv-mmio-bus-adapters.md) | ✅ ГОТОВО |
| 0210 | Массив как общая переменная в цели st; индекс-выражение | [0210-st-array-shared-and-index.md](0210-st-array-shared-and-index.md) | ✅ ГОТОВО |
| 0211 | Модель без стартового состояния: цель c отказывает бессодержательно | [0211-c-missing-start-state-diagnostic.md](0211-c-missing-start-state-diagnostic.md) | ✅ ГОТОВО |
| 0212 | Диагностика цели c без кода | [0212-c-diagnostic-without-code.md](0212-c-diagnostic-without-code.md) | ✅ ГОТОВО |
| 0239 | Скрипт релизной сборки и установки инструментов | [0239-install-script.md](0239-install-script.md) | ✅ ГОТОВО |
| 0241 | Ускорение предкоммита | [0241-precheck-speedup.md](0241-precheck-speedup.md) | ✅ ГОТОВО |
| 0213 | Цель c печатает лишний break после безусловного перехода | [0213-c-redundant-break.md](0213-c-redundant-break.md) | ✅ ГОТОВО |
| 0244 | Стоимость тестовых целей | [0244-test-target-build-cost.md](0244-test-target-build-cost.md) | ✅ ГОТОВО |
| 0243 | Переопределение типа | [0243-type-redefinition-diagnostic.md](0243-type-redefinition-diagnostic.md) | ✅ ГОТОВО |
| 0214 | Регистровый интерфейс sv-mmio | [0214-sv-mmio-unused-write-signals.md](0214-sv-mmio-unused-write-signals.md) | ✅ ГОТОВО |
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
| 0290 | Приложение «Ошибки» сверяется с реестром диагностик | [0290-book-diagnostics-codes-gate.md](0290-book-diagnostics-codes-gate.md) | Готово: расхождение 32 кода + 1 обратное устранено |
| 0298 | Списки лексики раздела «Лексика» сверяются с языком | [0298-book-lexicon-lists-sync.md](0298-book-lexicon-lists-sync.md) | Готово: 4 слова добавлены, ложь о словах LTL снята |
| 0215 | Потактовые сверки длительностей для целей st и sv | [0215-duration-per-tick-conformance-st-sv.md](0215-duration-per-tick-conformance-st-sv.md) | Готово: дефектов нет, сверки заведены сторожем |
| 0216 | Печатник живости цели rust получает сторожа поведения | [0216-rust-live-printer-coverage.md](0216-rust-live-printer-coverage.md) | Готово: два дефекта найдены и исправлены |
| 0254 | Старое имя изъято из служебных идентификаторов | [0254-legacy-names-internal-identifiers.md](0254-legacy-names-internal-identifiers.md) | Готово: 114 вхождений, гейт расширен |
| 0269 | Подсветка блоков st и ebnf в документе | [0269-book-st-syntax-highlight.md](0269-book-st-syntax-highlight.md) | Готово: 9 блоков, гейт языков заведён |
| 0270 | Вес PDF документа: причина найдена, теги отключены | [0270-book-pdf-size.md](0270-book-pdf-size.md) | Готово: 3.08 МБ → 947 КБ |
| 0283 | Печать результата компиляции сведена к одной функции | [0283-cli-report-result-merge.md](0283-cli-report-result-merge.md) | Готово: --verbose действует у всех целей |
| 0294 | SE-102 называет файл, подключающий библиотеку | [0294-se102-suggest-importer.md](0294-se102-suggest-importer.md) | Готово: подсказка у taktc и takt-sim |
| 0164 | Реестры стадий сверяются с файлами на диске | [0164-registry-rebuild-gate.md](0164-registry-rebuild-gate.md) | Готово: 42 записи восполнены, гейт заведён |
| 0173 | Заглушка too_many_arguments снята в цели rust | [0173-rust-generator-arg-count.md](0173-rust-generator-arg-count.md) | Готово: 7 заглушек снято, вывод не изменился |
| 0252 | CI и замер покрытия приведены к снятому правилу однопоточности | [0252-ci-windows-test-threads.md](0252-ci-windows-test-threads.md) | Готово: флаг снят в двух местах |
| 0261 | Жёлтая зона гейта размера модулей и его сторож | [0261-module-size-warning-zone.md](0261-module-size-warning-zone.md) | Готово: зона 970…1000, сторож заведён |
| 0265 | SVG диаграмм не несут версию graphviz | [0265-book-svg-graphviz-version.md](0265-book-svg-graphviz-version.md) | Готово: молчаливая поломка регенерации устранена |
| 0271 | Устройство интеграционных тестов сторожится машиной | [0271-test-target-gate.md](0271-test-target-gate.md) | Готово: T1 и T2 ловятся пробой |
| 0272 | Обвязка замера: прогрев, два прогона, вердикт | [0272-build-measurement-harness.md](0272-build-measurement-harness.md) | Готово: вердикт по разбросу, сторож на быстрых командах |
| 0288 | Сторожа фикстур проверяют обещание, а не разбор | [0288-fixture-guards-audit.md](0288-fixture-guards-audit.md) | Готово: 5 усилено, 28 в ратчете |
| 0289 | Чек-лист инвариантов сверяется с подробными пунктами | [0289-claude-md-invariant-checklist-gate.md](0289-claude-md-invariant-checklist-gate.md) | Готово: класс 5 гейта живого контекста |
| 0297 | Гейт живого контекста читает абзацы, а не строки | [0297-check-claude-md-line-blindness.md](0297-check-claude-md-line-blindness.md) | Готово: сверка версий заработала, бутафория снята |
| 0304 | Локальное объявление в теле блока получает тип | [0304-local-declaration-type-inference.md](0304-local-declaration-type-inference.md) | Готово: один вход — один ответ у девяти потребителей |
| 0305 | Вызов внешней функции в инициализаторе — SE-084 | [0305-extern-call-in-initializer.md](0305-extern-call-in-initializer.md) | Готово: девять потребителей отвечают одинаково |
| 0306 | Невычислимый вызов функции в инициализаторе | [0306-unfoldable-call-in-initializer.md](0306-unfoldable-call-in-initializer.md) | Готово: девять потребителей отвечают одинаково, причина названа |
| 0307 | Текст SIM-011 называет ширину значения | [0307-sim-bit-range-text.md](0307-sim-bit-range-text.md) | Готово: сообщение описывает разрядность значения |
| 0308 | Координата отказа целей rust, st, sv | [0308-target-refusal-position.md](0308-target-refusal-position.md) | Готово: место отказа указывают все четыре цели |
| 0309 | Массив с агрегатом в цели sv | [0309-sv-array-initializer.md](0309-sv-array-initializer.md) | Готово: восемь целей переводят, трасса совпала |
| 0310 | Общий носитель правила приведения | [0310-int-cast-shared-layer.md](0310-int-cast-shared-layer.md) | Готово: компилятор и эталон считают одними формулами |
| 0311 | Описания записей реестра диагностик | [0311-diagnostic-descriptions.md](0311-diagnostic-descriptions.md) | Готово: все 229 записей называют смысл |
| 0312 | Релизный режим в гейтах цели c | [0312-c-gate-release-mode.md](0312-c-gate-release-mode.md) | Готово: оба режима проверяются, харнесс исправлен |
| 0313 | Арность вызова функции | [0313-call-arity-check.md](0313-call-arity-check.md) | Готово: вызов не по объявлению отвергается компилятором |
| 0314 | Предупреждение о выброшенном вызове | [0314-c-dropped-builtin-warning.md](0314-c-dropped-builtin-warning.md) | Готово: вывод прежний, молчание устранено |
| 0315 | Сторожа гейтов предкоммита | [0315-gate-guards.md](0315-gate-guards.md) | Готово: гейтов без прикрытия не осталось |
| 0316 | Пересказ правила в комментарии шага | [0316-precheck-comment-duplication.md](0316-precheck-comment-duplication.md) | Готово: четыре пересказа сняты, новый ловится |
| 0317 | Представление q в общем слое | [0317-fixed-cast-shared-layer.md](0317-fixed-cast-shared-layer.md) | Готово: sv принял приведение, c не зовёт floor |
| 0318 | Мост «число ↔ длительность» | [0318-duration-cast-folding.md](0318-duration-cast-folding.md) | Готово: обе стороны считает компилятор |
| 0319 | Приведение агрегата к массиву | [0319-array-cast-folding.md](0319-array-cast-folding.md) | Готово: девять потребителей согласны |
| 0320 | Длина агрегата | [0320-aggregate-length-check.md](0320-aggregate-length-check.md) | Готово: девять потребителей отвечают одинаково |
| 0321 | Разворот for в цели sv | [0321-sv-for-unroll.md](0321-sv-for-unroll.md) | Готово: статический цикл переводят все восемь целей |
| 0322 | match в цели sv | [0322-sv-match-case.md](0322-sv-match-case.md) | Готово: match переводят все восемь целей |
| 0323 | Приведение as в цели sv | [0323-sv-integer-cast.md](0323-sv-integer-cast.md) | Готово: as переводят все восемь целей |
| 0324 | Арифметический сдвиг вправо | [0324-arithmetic-shift-right.md](0324-arithmetic-shift-right.md) | Готово: девять потребителей дают −4 |
| 0325 | Семантика сдвигов в документе | [0325-book-shift-semantics.md](0325-book-shift-semantics.md) | Готово: норма 0324 описана в языке |
| 0326 | Сдвиг на ширину типа в цели rust | [0326-rust-shift-width.md](0326-rust-shift-width.md) | Готово: вывод собирается, значение совпало |
| 0327 | Описание SV-002 | [0327-sv002-description.md](0327-sv002-description.md) | Готово: описание называет признак, а не список форм |
| 0328 | Целая степень | [0328-integer-power.md](0328-integer-power.md) | Готово: значение совпало, ST принят арбитром |
| 0329 | Степень в целях rust и sv | [0329-power-in-rust-sv.md](0329-power-in-rust-sv.md) | Готово: степень переводят все восемь целей |
| 0330 | Присваивание агрегата | [0330-aggregate-assignment.md](0330-aggregate-assignment.md) | Готово: переводят все восемь целей |
| 0331 | Именованное условие в теле | [0331-named-condition-in-body.md](0331-named-condition-in-body.md) | Готово: девять потребителей согласны |
| 0332 | Обещание задачи в диагностике | [0332-stub-task-promise.md](0332-stub-task-promise.md) | Готово: заглушка с обещанием ловится |
| 0333 | Раздел об ошибках программы | [0333-book-runtime-errors.md](0333-book-runtime-errors.md) | Готово: разница прогона и прошивки описана |
| 0334 | Сдвиг на величину не меньше ширины типа | [0334-rust-variable-shift-width.md](0334-rust-variable-shift-width.md) | Готово: цель `rust` считает как эталон |
| 0335 | Разряд в позиции числового значения | [0335-bit-value-in-targets.md](0335-bit-value-in-targets.md) | Готово: три цели чинены, значения сверены |
| 0336 | Приведение в аргументе и возврате | [0336-call-return-coercion.md](0336-call-return-coercion.md) | Готово: три позиции приёмника, значения сверены |
| 0337 | Неиспользуемый параметр функции | [0337-unused-function-parameter.md](0337-unused-function-parameter.md) | Готово: три цели проходят свои гейты |
| 0338 | Перечисление внутри функции | [0338-enum-in-function.md](0338-enum-in-function.md) | Готово: две цели чинены, кандидат исправлен |
| 0339 | Шаблон отказа цели `sv` | [0339-sv-refusal-template.md](0339-sv-refusal-template.md) | Готово: носитель один, текст читается |
| 0340 | Место записи агрегата | [0340-aggregate-assign-place.md](0340-aggregate-assign-place.md) | Готово: три цели, общий носитель |
| 0341 | Вложенная структура | [0341-nested-struct-order.md](0341-nested-struct-order.md) | Готово: четыре цели переводят |
| 0342 | Зарезервированные имена IEC | [0342-st-reserved-names.md](0342-st-reserved-names.md) | Готово: список обоснован прогоном |
| 0343 | Инициализатор массива | [0343-array-initializer.md](0343-array-initializer.md) | Готово: потеря значения устранена |
| 0344 | Порядок функций у цели `st` | [0344-st-call-order.md](0344-st-call-order.md) | Готово: порядок следует вызовам |
| 0345 | Агрегат в локальном объявлении | [0345-local-aggregate.md](0345-local-aggregate.md) | Готово: отказ был пробелом печати |
| 0346 | Индексация параметра-массива | [0346-array-param-index.md](0346-array-param-index.md) | Готово: ломался язык, а не цель |
| 0347 | Константа-агрегат | [0347-const-aggregate.md](0347-const-aggregate.md) | Готово: правка обнажила второй дефект |
| 0348 | Массив в параметре функции | [0348-st-array-parameter.md](0348-st-array-parameter.md) | Готово: разбор — не доказательство |
| 0349 | Длительность в поле структуры | [0349-duration-field.md](0349-duration-field.md) | Готово: одна запись, две причины |
| 0350 | Порт составного типа | [0350-port-composite-type.md](0350-port-composite-type.md) | Готово: согласие достигнуто |
| 0351 | Значение по умолчанию у цели rust: структура, длительность и q | [0351-rust-default-value-types.md](0351-rust-default-value-types.md) | ГОТОВО |
| 0352 | Имя типа видно во всём файле; цикл структур — SE-124 | [0352-struct-field-forward-reference.md](0352-struct-field-forward-reference.md) | ГОТОВО |
| 0353 | Цель c обнуляет переменные без инициализатора | [0353-c-default-init.md](0353-c-default-init.md) | ГОТОВО |
| 0354 | Умолчание duration у эталона несёт свой вид значения | [0354-sim-duration-default.md](0354-sim-duration-default.md) | ГОТОВО |
| 0355 | Срез массива переводят четыре цели | [0355-array-slice-in-targets.md](0355-array-slice-in-targets.md) | ГОТОВО |
| 0356 | Разряд участвует в арифметике как 0/1 | [0356-sim-bit-in-arithmetic.md](0356-sim-bit-in-arithmetic.md) | ГОТОВО |
| 0357 | Умолчание общей переменной строит один носитель | [0357-rust-shared-default-value.md](0357-rust-shared-default-value.md) | ГОТОВО |
| 0358 | Индексация применима к выражению, а не только к имени | [0358-postfix-index-on-expression.md](0358-postfix-index-on-expression.md) | ГОТОВО |
| 0359 | Сравнение знакового с беззнаковым | [0359-mixed-sign-comparison.md](0359-mixed-sign-comparison.md) | ГОТОВО |
| 0360 | Арифметика операндов разной знаковости | [0360-mixed-sign-arithmetic.md](0360-mixed-sign-arithmetic.md) | ГОТОВО |
| 0361 | Приведение к тому же типу опускается | [0361-same-type-cast.md](0361-same-type-cast.md) | ГОТОВО |
| 0362 | Проба прогоняет инструменты целей | [0362-probe-target-tools.md](0362-probe-target-tools.md) | ГОТОВО |
| 0363 | Индексация многомерного массива у цели st | [0363-st-multidim-subscript.md](0363-st-multidim-subscript.md) | ГОТОВО |
| 0364 | Вложенный массив у цели c | [0364-c-nested-array.md](0364-c-nested-array.md) | ГОТОВО |
| 0365 | Распакованный массив у цели sv | [0365-sv-unpacked-array.md](0365-sv-unpacked-array.md) | ГОТОВО |
| 0366 | Раскрытие вложенного агрегата — общий носитель | [0366-nested-aggregate-carrier.md](0366-nested-aggregate-carrier.md) | ГОТОВО |
| 0367 | Массив структур у цели sv синтезируется | [0367-sv-struct-array.md](0367-sv-struct-array.md) | ГОТОВО |
| 0368 | Элемент агрегата печатается по типу элемента | [0368-aggregate-element-type.md](0368-aggregate-element-type.md) | ГОТОВО |
| 0369 | Массив в параметре функции у цели sv | [0369-sv-array-parameter.md](0369-sv-array-parameter.md) | ГОТОВО |
| 0370 | Понижение q-литерала доходит до полей структуры | [0370-struct-field-fixed.md](0370-struct-field-fixed.md) | ГОТОВО |
| 0371 | Приведение q из поля структуры масштабируется | [0371-fixed-cast-from-field.md](0371-fixed-cast-from-field.md) | ГОТОВО |
| 0372 | Составной элемент массива в параметре функции | [0372-composite-array-parameter.md](0372-composite-array-parameter.md) | ГОТОВО |
| 0373 | Локальная переменная структурного типа у цели sv | [0373-sv-struct-local.md](0373-sv-struct-local.md) | ГОТОВО |
