# Фича 0023: Плагин IntelliJ IDEA — навигация к декларации и include

- **Номер:** 0023
- **Статус:** ГОТОВО
- **Зависит от:** нет (надстройка над готовой фичей [0022](0022-intellij-syntax-highlight.md);
  связь с [0011](0011-lsp-server.md) `lam-lsp` — необязательная, навигация автономна)
- **Связанные issue (анализ):** новая фича (из блока «Кандидаты» `FEATURES.md`:
  «Расширение плагина IntelliJ до навигации/инспекций»)
- **Крейт/подпроект:** существующий подпроект `extensions/intellij-lam/`
  (Kotlin + Gradle IntelliJ Platform Plugin), расширение поверх лексера 0022

## Ссылки на артефакты (жизненный цикл, правило 17)

| Стадия | Артефакт |
|---|---|
| Архитектура (ADR) | [`docs/adr/0023-intellij-navigation-include.md`](../adr/0023-intellij-navigation-include.md) |
| Анализ | [`docs/analyze/0023-intellij-navigation-include.md`](../analyze/0023-intellij-navigation-include.md) |
| Разработка 0023-01 | [`docs/development/0023-01-intellij-navigation-include.md`](../development/0023-01-intellij-navigation-include.md) (**ВЫПОЛНЕНО**) |
| Тест-план | [`docs/tests/0023-intellij-navigation-include.md`](../tests/0023-intellij-navigation-include.md) |
| Отчёт о тестировании | [`docs/reports/0023-intellij-navigation-include.md`](../reports/0023-intellij-navigation-include.md) (✅ ГОТОВО) |
| Исправление 0023-01 | [`docs/fixes/0023-01-verifyplugin-descriptor.md`](../fixes/0023-01-verifyplugin-descriptor.md) (verifyPlugin + валидность дескриптора) |

## Краткое описание

Расширение плагина IntelliJ IDEA для Lam (задел фичи [0022](0022-intellij-syntax-highlight.md))
двумя навигационными возможностями:

1. **Переход к декларации** (Go to Declaration, `Ctrl/⌘+Click`, `Ctrl/⌘+B`) —
   от использования идентификатора к месту его объявления: `model`, `state`/`start`,
   `type`, `enum`, `cond`, `var`, `const`, `fn`, а также имён, введённых `import … as`.
2. **Обработка `import`-директив** — навигация от строкового литерала пути
   (`import "файл.lam";`, `import { … } from "файл.lam";`) к самому файлу
   `.lam` на диске (в т.ч. с учётом относительных путей и корней проекта).

Реализуется **лёгким путём** (ADR, Option A): `GotoDeclarationHandler` +
`PsiReferenceContributor` поверх существующего лексера, **без** полноценного
PSI-парсера. Фича аддитивна: синтаксис/семантика языка и версия языка не
меняются (правило 22 неприменим).

> Фича зарегистрирована по запросу заказчика (2026-07-13) из блока «Кандидаты»
> `FEATURES.md`; далее проходит жизненный цикл по правилу 17.

## Итог (что сделано)

Реализован **Option A** (ADR): навигация в плагине `extensions/intellij-lam`
поверх лексера 0022, **без** полноценного PSI-парсера. Аддитивно: `grammar`/
`simulation`, синтаксис/семантика и **версия языка** не тронуты (правило 22
неприменим). Версия плагина `0.1.1 → 0.2.0`. Одна задача — [0023-01](../development/0023-01-intellij-navigation-include.md).

- **Плоский PSI:** `LamParserDefinition` + `LamParser` + `LamFile` — токены
  лексера кладутся листьями под корень; даёт реальные `PsiElement` под кареткой
  (подсветку 0022 не ломает, разбор всегда успешен).
- **Переход к декларации** (`GotoDeclarationHandler`): `LamSymbolScanner` строит
  индекс `имя → диапазон` для `model`/`state`/`start`/`type`/`enum`/`cond`/`var`/
  `const`/`fn` и локальных имён `import` (алиасы `as`, имена в `{ … }`); переход
  от использования к объявлению в том же файле.
- **Обработка `import`:** `LamImports` резолвит строку-путь (все формы `import`)
  в файл `.lam` относительно каталога файла и корней контента; тот же
  `GotoDeclarationHandler` открывает файл. `PsiReferenceContributor` не применён —
  ссылки не привязываются к листовым токенам (см. [development](../development/0023-01-intellij-navigation-include.md)).

**Проверка:** `./gradlew --offline clean buildPlugin test` → BUILD SUCCESSFUL,
**47/47 тестов зелёные** (27 регресс 0022 + 20 новых: сканер 9, переход 6,
импорт 5). **Совместимость с новыми IDE** подтверждена IntelliJ Plugin Verifier —
**Compatible** для IC 2024.3 / 2025.1 / 2025.2 (фикс [0023-01](../fixes/0023-01-verifyplugin-descriptor.md):
настройка `verifyPlugin`, снят невалидный пустой `until-build`, версия плагина
`0.2.0 → 0.2.1`). Остаточные пункты (find-usages/rename/структура/инспекции,
настоящие `PsiReference` для путей, кросс-файловое разрешение имён, визуальная
`runIde` в GUI) — в [отчёте](../reports/0023-intellij-navigation-include.md) и бэклоге `FEATURES.md`.
