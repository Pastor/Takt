# Реестр отчётов о тестировании

Стадия 6 жизненного цикла (правило 17 в [../RULE.md](../RULE.md)). Отчёт
`XXXX-slug.md` формируется по результатам тестирования по обязательному формату
(правило 20): сводка прогонов и окружение, сверка с тест-планом, примеры и
контрпримеры, найденные дефекты со ссылками на фиксы, итоговый вердикт.

Заготовка создаётся из шаблона [`../templates/reports.md`](../templates/reports.md).

| Фича | Заголовок | Отчёт | Вердикт |
|------|-----------|-------|---------|
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
