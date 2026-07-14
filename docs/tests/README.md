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
| 0025 | Починка вычислителя выражений симулятора | [0025-simulator-expression-eval.md](0025-simulator-expression-eval.md) | СОЗДАН — ожидает реализации (`0025-01…05`); T1–T8 обязаны быть красными на `HEAD` до починки |
