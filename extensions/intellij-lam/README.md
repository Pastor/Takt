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
- **Семантическая подсветка** (0038, через LSP4IJ + `lam-lsp`) — идентификаторы
  окрашиваются по **смыслу**: функции, типы, варианты `enum`, состояния/модели и
  переменные — каждый своим цветом (лексер их красит одинаково). Опциональна и
  деградируема — см. раздел «Семантическая подсветка через LSP4IJ» ниже.

Базовая подсветка **лексическая** и работает офлайн (без LSP-сервера) в любой
редакции, включая Community. Навигация — лёгкий путь поверх лексера (без
полноценного PSI-парсера): переход к декларации через `GotoDeclarationHandler`,
разрешение имён эвристикой по токенам одного файла. Find usages/rename/структура/
инспекции — задел на будущее. **Семантическая подсветка** (0038) уже есть —
надстройка через `lam-lsp`, включается при наличии LSP4IJ и сервера.

## Требования

- **JDK 21** — авто-провизионируется Gradle toolchain (foojay-резолвер в
  [`settings.gradle.kts`](settings.gradle.kts)); отдельно ставить не нужно.
- Целевая платформа сборки: IntelliJ IDEA Community **2024.2.x** (`sinceBuild
  242`, верхняя граница **не задана** — открытый диапазон). ⚠️ Поднято с 2024.1.x
  фичей 0038: современный **LSP4IJ несовместим с 241** (требует 242 → JDK 21).
  Версии вынесены в [`gradle.properties`](gradle.properties). Совместимость с
  новыми IDE проверена IntelliJ Plugin Verifier (см. ниже).

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

## Семантическая подсветка через LSP4IJ (фича 0038)

Лексический слой видит любое имя как идентификатор и красит одинаково. **Смысл**
имени (функция / тип / вариант `enum` / состояние-модель / переменная) знает
только компилятор — плагин получает его от сервера `lam-lsp` по протоколу LSP
через клиент **LSP4IJ**. Единый источник классификации — крейт `grammar`,
дублирования семантики на Kotlin нет.

Слой **опционален и деградируем**: без LSP4IJ или без бинарника `lam-lsp` плагин
работает ровно как базовый (лексическая подсветка + навигация), без ошибок и
модальных диалогов.

**Как включить:**

1. **Установите LSP4IJ** из Marketplace (Settings → Plugins → Marketplace →
   «LSP4IJ»). Требуется IDE **2024.2+** (та же линейка, что и у плагина).
2. **Соберите сервер** `lam-lsp` (флаг `lsp` обязателен —
   `required-features = ["lsp"]`):

   ```sh
   cargo build --features lsp --bin lam-lsp        # → target/debug/lam-lsp
   ```
3. **Укажите путь** к бинарнику: положите `lam-lsp` в `PATH` (тогда плагин найдёт
   его автоматически) **либо** задайте явный путь в настройке плагина
   (`LamLspSettings`). Приоритет: явная настройка → `PATH`.

После этого при открытии `.lam` LSP4IJ поднимает `lam-lsp`, и идентификаторы
перекрашиваются по смыслу поверх лексической подсветки. Цвета настраиваются теми
же ключами Lam (**Settings → Editor → Color Scheme → Lam**: `FUNCTION`, `TYPE`,
`ENUM_MEMBER`, `CLASS` и лексические). Состояние сервера видно в окне **LSP
Consoles** (LSP4IJ).

**Если сервера нет** — семантический слой просто не включается; базовая подсветка
не затронута.

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
