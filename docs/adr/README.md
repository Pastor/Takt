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
| [0039](./0039-intellij-reformat.md) | Действие Reformat Code в плагине IntelliJ | Accepted | фича 0039 |
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

