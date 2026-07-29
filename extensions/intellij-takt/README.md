# Плагин IntelliJ IDEA — язык Takt

Плагин для JetBrains IntelliJ IDEA (и IDE на IntelliJ Platform) с поддержкой
языка **Takt** (Typed, Automata, Known Timing): распознавание файлов `.takt`,
**подсветка синтаксиса**, **навигация**, **rename** и **ссылки на `import`**.
Часть монорепозитория проекта Takt (фичи
[`0022`](../../docs/features/0022-intellij-syntax-highlight.md),
[`0023`](../../docs/features/0023-intellij-navigation-include.md),
[`0038`](../../docs/features/0038-intellij-semantic-tokens.md),
[`0039`](../../docs/features/0039-intellij-reformat.md),
[`0067`](../../docs/features/0067-intellij-rename-psi-import.md)).

## Возможности

- Распознавание `*.takt` как языка Takt (свой тип файла и иконка).
- Лексическая подсветка: ключевые слова, операторы (пост-0021 `:=` присваивание,
  `=` сравнение, `<=` реляционный; `==` выведен — подсвечивается как ошибка),
  числа, строки, комментарии `//` / `///` / `/* … */`, скобки и пунктуация.
- Страница настройки цветов (**Settings → Editor → Color Scheme → Takt**).
- Комментирование строкой `//` (Ctrl+/) и блоком `/* … */`; подсветка парных
  скобок `{}` `()` `[]`.
- **Переход к декларации** (0023, `Ctrl/⌘+Click`, `Ctrl/⌘+B`) — от использования
  имени к объявлению `model`/`state`/`start`/`type`/`enum`/`cond`/`var`/`const`/
  `fn` и имён, введённых `import … as`.
- **Навигация по `import`** (0023) — переход от строки-пути (`import "файл.takt";`
  и `import { … } from "файл.takt";`) к самому файлу `.takt`.
- **Настоящая ссылка на файл `import`** (0067, R5) — строка-путь несёт
  `PsiReference` (`FileReference`): не только `Ctrl/⌘+Click`, но и
  **автообновление пути при переименовании/перемещении файла** средствами
  рефакторинга IDEA (rename-on-move). Работает офлайн, без `takt-lsp`.
- **Rename имён Takt** (0067, R3) — штатный рефакторинг **Rename** (`Shift+F6`)
  для `model`/`state`/`start`/`type`/`enum` и его вариантов/`cond`/`var`/`const`/
  `fn`/портов/алиасов `import`: переименовывает декларацию и её использования **в
  файле**; одноимённые подстроки в комментариях и строках не задеваются;
  ключевые слова Takt как имена отвергаются. Кросс-файловый rename — за LSP (0038).
  Работает офлайн, без `takt-lsp`.
- **Семантическая подсветка** (0038, через LSP4IJ + `takt-lsp`) — идентификаторы
  окрашиваются по **смыслу**: функции, типы, варианты `enum`, состояния/модели и
  переменные — каждый своим цветом (лексер их красит одинаково). Опциональна и
  деградируема — см. раздел «Семантическая подсветка через LSP4IJ» ниже.
- **Форматирование `.takt`** (0039, `Reformat Code` — `Ctrl+Alt+L`, через тот же
  LSP4IJ + `takt-lsp`) — приводит файл к канону тем же ядром печати, что и
  `taktc fmt`, поэтому байт-в-байт совпадает с CI-проверкой `taktc fmt --check`.
  Тот же слой, что и семантическая подсветка (см. раздел ниже).

Базовая подсветка **лексическая** и работает офлайн (без LSP-сервера) в любой
редакции, включая Community. Навигация — лёгкий путь поверх лексера: переход к
декларации через `GotoDeclarationHandler`, разрешение имён эвристикой по токенам
одного файла. **Rename и ссылки на файлы `import`** (0067) — поверх
**хирургического** структурного PSI: парсер оборачивает в композитные узлы
**только** строки-пути `import` (`IMPORT_PATH`) и идентификаторы деклараций/
использований (`NAME_DECL`/`NAME_REF`), остальное остаётся плоскими листьями —
грамматика выражений/типов не дублируется (единый источник форм —
`TaktSymbolScanner`, антидивергенция сверяется round-trip-тестом по всему корпусу).
Структура/инспекции/кросс-файловость — от `takt-lsp` (0038). **Семантическая
подсветка** (0038) — надстройка через `takt-lsp`, включается при наличии LSP4IJ и
сервера.

## Требования

- **JDK компиляции — 21**, авто-провизионируется Gradle toolchain
  (foojay-резолвер в [`settings.gradle.kts`](settings.gradle.kts)); отдельно
  ставить не нужно.
- **Пусковой JDK — не новее 21.** ⚠️ Это **другой** JDK: на нём работает демон
  Gradle и Kotlin-плагин, и берётся он из окружения — `jvmToolchain` его **не
  меняет**. Таблица версий Java внутри `kotlin-gradle-plugin` 2.0.21 обрывается
  на 21, поэтому под более новым пусковым JDK `compileKotlin` падает с
  `IllegalArgumentException: <версия>`. Сборка проверяет это сама и объясняет,
  что делать (`build.gradle.kts`); лечение — запустить Gradle под подходящим JDK:

  ```sh
  JAVA_HOME=$(/usr/libexec/java_home -v 21) ./gradlew test
  ```

  Измерено: до 21 включительно работает, 25 ломается; 22–24 не проверялись.
- Целевая платформа сборки: IntelliJ IDEA Community **2024.2.x** (`sinceBuild
  242`, верхняя граница **не задана** — открытый диапазон). ⚠️ Поднято с 2024.1.x
  фичей 0038: современный **LSP4IJ несовместим с 241** (требует 242 → JDK 21).
  Версии вынесены в [`gradle.properties`](gradle.properties). Совместимость с
  новыми IDE проверена IntelliJ Plugin Verifier (см. ниже).

## Сборка и запуск

```sh
cd extensions/intellij-takt

./gradlew buildPlugin      # собрать плагин → build/distributions/intellij-takt-<версия>.zip
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
`build/distributions/intellij-takt-<версия>.zip`.

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
только компилятор — плагин получает его от сервера `takt-lsp` по протоколу LSP
через клиент **LSP4IJ**. Единый источник классификации — крейт `takt-lang`,
дублирования семантики на Kotlin нет.

Слой **опционален и деградируем**: без LSP4IJ или без бинарника `takt-lsp` плагин
работает ровно как базовый (лексическая подсветка + навигация), без ошибок и
модальных диалогов.

**Как включить:**

1. **Установите LSP4IJ** из Marketplace (Settings → Plugins → Marketplace →
   «LSP4IJ»). Требуется IDE **2024.2+** (та же линейка, что и у плагина).
2. **Соберите сервер** `takt-lsp` (флаг `lsp` обязателен —
   `required-features = ["lsp"]`):

   ```sh
   cargo build --features lsp --bin takt-lsp        # → target/debug/takt-lsp
   ```
3. **Укажите путь** к бинарнику: положите `takt-lsp` в `PATH` (тогда плагин найдёт
   его автоматически) **либо** задайте явный путь в настройке плагина
   (`TaktLspSettings`). Приоритет: явная настройка → `PATH`.

После этого при открытии `.takt` LSP4IJ поднимает `takt-lsp`, и идентификаторы
перекрашиваются по смыслу поверх лексической подсветки. Цвета настраиваются теми
же ключами Takt (**Settings → Editor → Color Scheme → Takt**: `FUNCTION`, `TYPE`,
`ENUM_MEMBER`, `CLASS` и лексические). Состояние сервера видно в окне **LSP
Consoles** (LSP4IJ).

**Если сервера нет** — семантический слой просто не включается; базовая подсветка
не затронута.

## Форматирование `.takt` через LSP4IJ (фича 0039)

Действие **Reformat Code** (`Ctrl+Alt+L`) для файлов `.takt` работает через тот же
слой LSP4IJ + `takt-lsp`, что и семантическая подсветка: IDE отправляет
`textDocument/formatting`, сервер отвечает канонически отформатированным текстом.
Дублирования стиля на Kotlin нет — форматирует то же ядро печати
`takt-lang/src/format/`, что и `taktc fmt`, поэтому результат **байт-в-байт** равен
выводу `taktc fmt` (и, значит, проверке `taktc fmt --check` в CI) — не по
договорённости, а по построению (один вызов `format_source`). Приёмка A2 закрыта
автотестом `a2_reformat_matches_lamc_fmt_over_corpus` (`takt-lang/tests/lsp_tests.rs`),
сверяющим это на всём корпусе `examples/`.

Ничего дополнительно включать не нужно: LSP4IJ регистрирует форматирование
автоматически, как только сервер объявляет `documentFormattingProvider` (а
`takt-lsp` его объявляет). Если сервер не поднят (нет LSP4IJ или бинарника
`takt-lsp`) — `Ctrl+Alt+L` для `.takt` не форматирует, остальное не затронуто.

> Исторически ADR 0039 планировал внешний форматтер `taktc fmt --stdin` (Option C,
> самодостаточность без стороннего плагина). После принятия LSP4IJ в фиче 0038 та
> посылка отпала (LSP4IJ и так жёсткая зависимость), и развилка разрешена в пользу
> LSP-пути (Option B): форматирование приходит бесплатно от уже работающего
> сервера. Подробности — в ADR 0039, раздел «Обновление решения».

## Структура

```
src/main/kotlin/org/takt/intellij/
  TaktLanguage.kt, TaktFileType.kt, TaktIcons.kt   — язык и тип файла (0022-01)
  psi/          TaktTokenType, TaktElementType, TaktTokenTypes  — токены
  lexer/        TaktLexer                          — лексер (0022-02)
  highlight/    TaktSyntaxHighlighter(+Factory),
                TaktHighlighterColors, TaktColorSettingsPage    — подсветка/цвета
  editor/       TaktCommenter, TaktBraceMatcher     — эргономика (0022-03)
  psi/          TaktFile, TaktTokenSets             — почти плоский PSI (0023)
                TaktElementTypes                   — композиты IMPORT_PATH/NAME_DECL/NAME_REF (0067)
                TaktImportPath(+Manipulator)       — ссылка на файл import, R5 (0067)
                TaktNameElements, TaktNameReference — PsiNamedElement/PsiReference, R3 (0067)
  parser/       TaktParserDefinition, TaktParser    — разбор + хирургическое оборачивание (0023/0067)
  navigation/   TaktSymbolScanner, TaktImports,
                TaktGotoDeclarationHandler         — навигация/import (0023)
  refactoring/  TaktNamesValidator                 — валидатор имён для rename (0067)
src/main/resources/META-INF/plugin.xml           — точки расширения
src/main/resources/icons/takt.svg                 — иконка типа файла
```

## Источник истины лексики

Набор ключевых слов и операторов зеркалит Rust-лексер компилятора
`takt-lang/src/parser/lexer.rs`. Соответствие проверяется регресс-тестом
`TaktKeywordSyncTest` (читает таблицу `KEYWORDS` из лексера и сверяет с плагином).
При изменении лексики языка обновите `TaktTokenTypes.KEYWORDS`/`TaktLexer`.
