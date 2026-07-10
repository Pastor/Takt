# Реестр фич (карточки)

Стадия 1 жизненного цикла (правило 17 в [../RULE.md](../RULE.md)). Здесь
перечислены **все** карточки фич `XXXX-slug.md` (включая закрытые). Витрина
**незакрытых** фич — в корневом [FEATURES.md](../../FEATURES.md).

Заготовка карточки создаётся из шаблона [`../templates/feature.md`](../templates/feature.md)
(вручную или генератором [`scripts/new-feature.sh`](../../scripts/new-feature.sh)).

| Фича | Наименование | Артефакты | Статус |
|------|--------------|-----------|--------|
| [0001](./0001-core-language-c.md) | Ядро языка Lam и компилятор в C | — | ГОТОВО |
| [0002](./0002-type-system.md) | Система типов и вывод типов | — | ГОТОВО |
| [0003](./0003-composite-states.md) | Составные состояния, композиция, импорты | — | ГОТОВО |
| [0004](./0004-control-flow.md) | Управляющие конструкции и match/switch | — | ГОТОВО |
| [0005](./0005-enums.md) | Перечисления (enum) | — | ГОТОВО |
| [0006](./0006-structs.md) | Структуры и typedef struct в C | — | ГОТОВО |
| [0007](./0007-ports.md) | Порты и индексный доступ | — | ГОТОВО |
| [0008](./0008-diagnostics.md) | Семантические диагностики | — | ГОТОВО |
| [0009](./0009-plantuml-generator.md) | Генератор диаграмм PlantUML | — | ГОТОВО |
| [0010](./0010-verification-ltl.md) | Верификация свойств (LTL/Бюхи) | — | ГОТОВО |
| [0011](./0011-lsp-server.md) | LSP-сервер lam-lsp | — | ГОТОВО |
| [0012](./0012-simulation-core.md) | Крейт simulation — симуляция моделей | — | ГОТОВО |
| [0013](./0013-gif-visualization.md) | GIF-визуализация симуляции | — | ГОТОВО |
| [0014](./0014-state-io.md) | Сохранение/загрузка состояния модели | — | ГОТОВО |
| [0015](./0015-stage-templates.md) | Шаблоны стадий и генератор new-feature.sh | — | ГОТОВО |
| [0016](./0016-public-api-docs.md) | Документирование публичного API | — | ГОТОВО |
| [0017](./0017-lifecycle-process.md) | Внедрение процесса разработки | — | ГОТОВО |
| [0018](./0018-code-guidelines.md) | Приведение кода к docs/CODE.md | [анализ](../analyze/0018-code-guidelines.md) · [отчёт](../reports/0018-code-guidelines.md) · dev 01–05 | ГОТОВО |
| [0019](./0019-condition-expression-unification.md) | Унификация грамматик Condition/Expression | [ADR](../adr/0019-condition-expression-unification.md) · [анализ](../analyze/0019-condition-expression-unification.md) · [dev 01](../development/0019-01-loopcond-dedup.md) | РАЗРАБОТКА |
| [0020](./0020-port-address-decl.md) | Адрес порта отдельно от объявления | [ADR](../adr/0020-port-address-decl.md) · [анализ](../analyze/0020-port-address-decl.md) · [dev 01](../development/0020-01-address-grammar.md) | РАЗРАБОТКА |
