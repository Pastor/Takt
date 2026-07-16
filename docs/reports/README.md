# Реестр отчётов о тестировании

Стадия 6 жизненного цикла (правило 17 в [../RULE.md](../RULE.md)). Отчёт
`XXXX-slug.md` формируется по результатам тестирования по обязательному формату
(правило 20): сводка прогонов и окружение, сверка с тест-планом, примеры и
контрпримеры, найденные дефекты со ссылками на фиксы, итоговый вердикт.

Заготовка создаётся из шаблона [`../templates/reports.md`](../templates/reports.md).

| Фича | Заголовок | Отчёт | Вердикт |
|------|-----------|-------|---------|
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
| 0028 | Заглушки генератора C: диагностика вместо тихого пропуска | [0028-c-generator-stubs.md](0028-c-generator-stubs.md) | ✅ ГОТОВО (`CC-018`; вскрыл, что `S(Модель) = Состояние` в C не переводится) |
| 0047 | Трансляция `S(Модель) = Состояние` в цель `c` | [0047-c-state-of-model.md](0047-c-state-of-model.md) | ✅ ГОТОВО (эталон `syntax_simple` зелёный по существу) |
| 0048 | Детерминированная генерация кода (единый порядок эмиссии) | [0048-deterministic-codegen.md](0048-deterministic-codegen.md) | ✅ ГОТОВО (10 прогонов → 1 вариант; гейт в `precheck.sh`; ABI стабилен) |
| 0033 | Согласование тактов симулятора и порождённого C (INIT-такты) | [0033-init-tick-alignment.md](0033-init-tick-alignment.md) | ✅ ГОТОВО (тело на такте 1 на любой глубине; потактовая сверка; UB устранён) |
| 0032 | Сохранение переменных модели в --save-state/--load-state | [0032-state-io-variables.md](0032-state-io-variables.md) | ✅ ГОТОВО (Д1/Д2/Д3 закрыты; единое хранилище; `inout` работает; 5/5 stacker) |
| 0031 | Вызов функции из тела функции | [0031-fn-calls-fn.md](0031-fn-calls-fn.md) | ✅ ГОТОВО (композиция `f→g`; рекурсия → SE-053; форвард-прототипы в C) |
| 0044 | Юнит-конструкции языка для симуляции (assert/invariant) | [0044-sim-assert-invariant.md](0044-sim-assert-invariant.md) | ✅ ГОТОВО (`invariant`; атом LTL; симулятор проверяет формулы — SIM-025) |
| 0035 | LTL-формулы в блоках кода: разбор вместо тихой потери | [0035-ltl-in-blocks.md](0035-ltl-in-blocks.md) | ✅ ГОТОВО (паритет уровней; SE-055/SE-056 через `ltl_warnings`; C неизменен) |
| 0045 | Бэкенд генерации в SystemVerilog (FPGA/ASIC) | [0045-sv-backend.md](0045-sv-backend.md) | ✅ ГОТОВО (оба гейта зелёные; сдвиг = 0 на глубинах 1/2/3 против настоящего RTL; `SV-012` найдена сверх реестра; 6 расхождений проработки с фактом) |
