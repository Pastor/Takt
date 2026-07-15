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
