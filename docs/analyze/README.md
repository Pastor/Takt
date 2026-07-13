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
| 0020 | Адрес порта отдельно от объявления | [0020-port-address-decl.md](0020-port-address-decl.md) | аддитивно (инлайн-форма сохраняется) |
| 0021 | Смена операторов: `<=` присваивание, `=` сравнение | [0021-swap-assign-compare.md](0021-swap-assign-compare.md) | слом (мажорная версия языка + мигратор) |
| 0022 | Плагин IntelliJ IDEA: подсветка синтаксиса Lam | [0022-intellij-syntax-highlight.md](0022-intellij-syntax-highlight.md) | аддитивно (новый подпроект, язык не тронут) |
| 0023 | Плагин IntelliJ IDEA — навигация к декларации и include | [0023-intellij-navigation-include.md](0023-intellij-navigation-include.md) | — (новая фича) |

