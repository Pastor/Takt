# Отчёт о тестировании фичи 0023: Плагин IntelliJ IDEA — навигация к декларации и include

> Фича: [../features/0023-intellij-navigation-include.md](../features/0023-intellij-navigation-include.md) · тест-план: [../tests/0023-intellij-navigation-include.md](../tests/0023-intellij-navigation-include.md) · анализ: [../analyze/0023-intellij-navigation-include.md](../analyze/0023-intellij-navigation-include.md)

## Резюме

**Вердикт: ✅ ГОТОВО.** Все проверки тест-плана пройдены. Реализован Option A ADR
(лёгкий путь): переход к декларации и навигация по `import` работают на
`GotoDeclarationHandler` поверх плоского PSI и лексера 0022. Дефектов, требующих
`docs/fixes/`, не выявлено. Языковой слой не затронут — примеры/контрпримеры по
синтаксису языка неприменимы (правило 16 н/п).

**Окружение:** IntelliJ Platform IC `2024.1.7`, JDK 17.0.19, Gradle wrapper
8.10.2, `TestFrameworkType.Platform`. Команда: `./gradlew --offline clean
buildPlugin test` → **BUILD SUCCESSFUL**, **47/47 тестов зелёные** (27 регресс
0022 + 20 новых). Собран `intellij-lam-0.2.0.zip`.

## Фактические результаты по проверкам

| # | Проверка | Результат | Комментарий |
|---|---|---|---|
| T1 | Индекс всех форм деклараций | ✅ | `LamSymbolScannerTest` (model/state/start/type/enum/cond/var/const/fn) |
| T2 | Импорт-переименования в индексе | ✅ | `C/P/Q` введены; источник `SharedModel` — нет |
| T3 | Диапазон декларации на имени | ✅ | подстрока диапазона == `Widget` |
| T4 | Использование → декларация | ✅ | `LamGotoDeclarationTest` (model/type) |
| T5 | Переход по псевдониму import | ✅ | цель — алиас `M` |
| T6 | Нет перехода на ключевом слове | ✅ | `mod<caret>el` → null |
| T7 | Нет перехода на самой декларации | ✅ | `model Fo<caret>o` → null |
| T8 | Нет перехода для неизвестного имени | ✅ | `Unkno<caret>wn` → null |
| T9 | Импорт-путь → файл (3 формы) | ✅ | `LamImportReferenceTest` (plain/from/as) |
| T10 | Отсутствующий файл | ✅ | цели нет, без исключений |
| T11 | Строка вне import не навигируется | ✅ | `formula "…"` → null |
| T12 | Сборка + весь набор тестов | ✅ | 47/47, `buildPlugin` OK |

## Результаты по функциональности

- **Плагин `intellij-lam`:** ✅ — 20 новых тестов + регресс 0022 (подсветка,
  эргономика, FileType) зелёные; подсветка не сломана плоским `ParserDefinition`.
- **`grammar`/`simulation`, синтаксис/семантика языка:** — не затронуты (аддитивная
  фича редакторной оснастки).

## Выводы и дальнейшие шаги

Фича готова к закрытию (ГОТОВО). Осознанные ограничения Option A (без областей
видимости; кросс-файловое разрешение имён внутри импортов; find-usages/rename/
структура; настоящие `PsiReference` для путей) вынесены в бэклог `FEATURES.md`
как будущая PSI-фича. Бинарный `verifyPlugin`/визуальная `runIde`-проверка в
headless-среде не запускались (как и в 0022) — остаточный пункт бэклога.
