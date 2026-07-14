# Плагин IntelliJ IDEA — язык Lam

Плагин для JetBrains IntelliJ IDEA (и IDE на IntelliJ Platform) с поддержкой
языка **Lam** (Language of Automata Models): распознавание файлов `.lam`,
**подсветка синтаксиса** и **навигация**. Часть монорепозитория проекта Lam
(фичи [`0022`](../../docs/features/0022-intellij-syntax-highlight.md),
[`0023`](../../docs/features/0023-intellij-navigation-include.md)).

## Возможности

- Распознавание `*.lam` как языка Lam (свой тип файла и иконка).
- Лексическая подсветка: ключевые слова, операторы (пост-0021 `:=` присваивание,
  `=` сравнение, `<=` реляционный; `==` выведен — подсвечивается как ошибка),
  числа, строки, комментарии `//` / `///` / `/* … */`, скобки и пунктуация.
- Страница настройки цветов (**Settings → Editor → Color Scheme → Lam**).
- Комментирование строкой `//` (Ctrl+/) и блоком `/* … */`; подсветка парных
  скобок `{}` `()` `[]`.
- **Переход к декларации** (0023, `Ctrl/⌘+Click`, `Ctrl/⌘+B`) — от использования
  имени к объявлению `model`/`state`/`start`/`type`/`enum`/`cond`/`var`/`const`/
  `fn` и имён, введённых `import … as`.
- **Навигация по `import`** (0023) — переход от строки-пути (`import "файл.lam";`
  и `import { … } from "файл.lam";`) к самому файлу `.lam`.

Подсветка **лексическая** и работает офлайн (без LSP-сервера) в любой редакции,
включая Community. Навигация — лёгкий путь поверх лексера (без полноценного
PSI-парсера): переход к декларации через `GotoDeclarationHandler`, разрешение имён
эвристикой по токенам одного файла. Find usages/rename/структура/инспекции и
семантическая подсветка через `lam-lsp` — задел на будущее.

## Требования

- JDK 17.
- Целевая платформа сборки: IntelliJ IDEA Community **2024.1.x** (`sinceBuild
  241`, верхняя граница **не задана** — открытый диапазон). Версии вынесены в
  [`gradle.properties`](gradle.properties). Совместимость с новыми IDE проверена
  IntelliJ Plugin Verifier (см. ниже).

## Сборка и запуск

```sh
cd extensions/intellij-lam

./gradlew buildPlugin      # собрать плагин → build/distributions/intellij-lam-<версия>.zip
./gradlew test             # юнит-тесты (лексер, highlighter, эргономика, регресс ключевых слов)
./gradlew runIde           # запустить песочницу IDE с установленным плагином
./gradlew verifyPlugin     # проверка совместимости с новыми IDE (Plugin Verifier)
```

`verifyPlugin` качает указанные в `build.gradle.kts`
(`pluginVerification.ides`) сборки IDE и проверяет бинарную совместимость.
Список задан строковой нотацией `ide("IC-2024.3")`, `ide("IC-2025.1")`,
`ide("IC-2025.2")` — строки аккумулируются (форма `ide(type, version)` для одного
типа схлопнулась бы в одну проверку). Последний прогон: **Compatible** для всех
трёх. Список обновляется по мере выхода новых релизов.

## Установка из файла

**Settings → Plugins → ⚙ → Install Plugin from Disk…** и выбрать собранный
`build/distributions/intellij-lam-<версия>.zip`.

## Установка в RustRover скриптом

Скрипт собирает плагин и ставит/обновляет его во всех найденных инсталляциях
RustRover (каталоги `…/JetBrains/RustRover*`), после чего IDE нужно перезапустить.

macOS/Linux — [`../install-rustrover-plugin.sh`](../install-rustrover-plugin.sh):

```sh
extensions/install-rustrover-plugin.sh              # собрать и установить/обновить
extensions/install-rustrover-plugin.sh --skip-build # без пересборки (готовый zip)
```

Windows (PowerShell) — [`../install-rustrover-plugin.ps1`](../install-rustrover-plugin.ps1):

```powershell
extensions\install-rustrover-plugin.ps1            # собрать и установить/обновить
extensions\install-rustrover-plugin.ps1 -SkipBuild # без пересборки (готовый zip)
```

## Структура

```
src/main/kotlin/org/lam/intellij/
  LamLanguage.kt, LamFileType.kt, LamIcons.kt   — язык и тип файла (0022-01)
  psi/          LamTokenType, LamElementType, LamTokenTypes  — токены
  lexer/        LamLexer                          — лексер (0022-02)
  highlight/    LamSyntaxHighlighter(+Factory),
                LamHighlighterColors, LamColorSettingsPage    — подсветка/цвета
  editor/       LamCommenter, LamBraceMatcher     — эргономика (0022-03)
  psi/          LamFile, LamTokenSets             — плоский PSI (0023)
  parser/       LamParserDefinition, LamParser    — плоский разбор (0023)
  navigation/   LamSymbolScanner, LamImports,
                LamGotoDeclarationHandler         — навигация/import (0023)
src/main/resources/META-INF/plugin.xml           — точки расширения
src/main/resources/icons/lam.svg                 — иконка типа файла
```

## Источник истины лексики

Набор ключевых слов и операторов зеркалит Rust-лексер компилятора
`grammar/src/parser/lexer.rs`. Соответствие проверяется регресс-тестом
`LamKeywordSyncTest` (читает таблицу `KEYWORDS` из лексера и сверяет с плагином).
При изменении лексики языка обновите `LamTokenTypes.KEYWORDS`/`LamLexer`.
