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

