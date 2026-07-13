# Фича 0022: Плагин IntelliJ IDEA — подсветка синтаксиса Lam

- **Номер:** 0022
- **Статус:** ГОТОВО
- **Зависит от:** нет (см. анализ: связь с [0011](0011-lsp-server.md) `lam-lsp` —
  необязательная, лишь для будущей семантической подсветки; лексическая подсветка
  автономна)
- **Крейт/подпроект:** новый подпроект `extensions/intellij-lam/`
  (Kotlin + Gradle IntelliJ Platform Plugin), рядом с `extensions/zed-lam/`

## Ссылки на артефакты (жизненный цикл, правило 17)

| Стадия | Артефакт |
|---|---|
| Архитектура (ADR) | [`docs/adr/0022-intellij-syntax-highlight.md`](../adr/0022-intellij-syntax-highlight.md) |
| Анализ | [`docs/analyze/0022-intellij-syntax-highlight.md`](../analyze/0022-intellij-syntax-highlight.md) |
| Разработка 0022-01 | [`docs/development/0022-01-plugin-skeleton.md`](../development/0022-01-plugin-skeleton.md) (**ВЫПОЛНЕНО** — каркас Gradle / `plugin.xml` / FileType; `buildPlugin` + 5 тестов зелёные) |
| Разработка 0022-02 | [`docs/development/0022-02-lexer-highlighter.md`](../development/0022-02-lexer-highlighter.md) (**ВЫПОЛНЕНО** — лексер `LexerBase` + `SyntaxHighlighter`; 22 теста зелёные) |
| Разработка 0022-03 | [`docs/development/0022-03-color-settings-docs.md`](../development/0022-03-color-settings-docs.md) (**ВЫПОЛНЕНО** — `ColorSettingsPage`, commenter, brace matcher, README; 27 тестов зелёные) |
| Тест-план | [`docs/tests/0022-intellij-syntax-highlight.md`](../tests/0022-intellij-syntax-highlight.md) |
| Отчёт о тестировании | [`docs/reports/0022-intellij-syntax-highlight.md`](../reports/0022-intellij-syntax-highlight.md) (✅ ГОТОВО) |

> Взята в разработку по запросу заказчика (2026-07-13). Проработка до стадии
> «Разработка» **без реализации** плагина (по образцу фич 0019/0020/0021): готовы
> ADR (выбор архитектуры подсветки), анализ (зависимости, требования, источник
> истины токенов), декомпозиция на 3 подзадачи и тест-план.
>
> **Ключевое решение (ADR):** нативный плагин IntelliJ Platform на базе
> `SyntaxHighlighter` + JFlex-лексера (**Option A**). Вариант «только TextMate»
> отвергнут (это не полноценный плагин, нет страницы настройки цветов); вариант
> «только LSP semantic tokens» отложен (платформенный LSP API — Ultimate-only /
> сторонний LSP4IJ) и вынесен в задел как *семантическая* надстройка над
> лексической подсветкой.

## Краткое описание

Отдельный плагин для JetBrains IntelliJ IDEA (и совместимых IDE на IntelliJ
Platform), обеспечивающий **подсветку синтаксиса** файлов `.lam`: регистрация
типа файла, лексическая раскраска ключевых слов, операторов (в т.ч. `:=`/`=`/`<=`
после фичи [0021](0021-swap-assign-compare.md)), литералов, комментариев и
идентификаторов, плюс базовая редакторная эргономика (парные скобки,
комментирование `//`/`///`, страница настройки цветов). Плагин пополняет линейку
редакторной оснастки языка наряду с существующим Zed-расширением
(`extensions/zed-lam`) и LSP-сервером `lam-lsp` ([0011](0011-lsp-server.md)).

Цель — дать пользователям самой распространённой IDE в промышленной разработке
удобное чтение и редактирование спецификаций автоматов Lam (правило 12), не
завязываясь на установку LSP-сервера для базовой подсветки.

> Фича зарегистрирована по запросу заказчика (2026-07-13); проходит жизненный
> цикл по правилу 17.

## Итог (что сделано)

Реализован **Option A** (ADR): полноценный плагин IntelliJ Platform с **лексической**
подсветкой `.lam` — новый подпроект `extensions/intellij-lam/` (Kotlin + IntelliJ
Platform Gradle Plugin 2.1.0), рядом с `extensions/zed-lam`. Подсветка автономна
(офлайн, в Community), от `lam-lsp` не зависит. Фича **аддитивна**: `grammar`/
`simulation`, синтаксис/семантика и **версия языка** не тронуты (правило 22
неприменим).

- **0022-01 — каркас:** `LamLanguage`, `LamFileType` (`.lam`, иконка
  `icons/lam.svg`), `LamIcons`; Gradle-сборка (wrapper 8.10.2), `plugin.xml`.
  Целевая платформа IC **2024.1.7** (последняя на Java 17 — под доступный JDK 17).
- **0022-02 — лексер + подсветка:** `LamLexer` (рукописный `LexerBase`, зеркалит
  `grammar/src/parser/lexer.rs`; **осознанное отклонение от JFlex** ради
  самодостаточной сборки). Операторы 0021 (`:=`→`OP_ASSIGN`, `=`→`OP_EQ`, `<=`/`>=`;
  **`==`→`BAD_CHARACTER`**), числа/строки/комментарии/скобки. `LamSyntaxHighlighter`
  + `LamHighlighterColors`. Регресс-тест `LamKeywordSyncTest` **читает** таблицу
  `KEYWORDS` из Rust-лексера и сверяет с плагином.
- **0022-03 — цвета/эргономика/доки:** `LamColorSettingsPage`, `LamCommenter`
  (`//` и `/* */`), `LamBraceMatcher` (`{}` `()` `[]`; типы скобок разделены на
  `L*/R*`); README подпроекта + раздел в корневом README (правило 15).
- **Фикс 0022-01 (приёмка):** снята верхняя граница совместимости IDE — плагин
  не ставился в сборки новее 243 (RustRover 261); `until-build` открыт, версия
  `0.1.0 → 0.1.1`. См. [`docs/fixes/0022-01-untilbuild-open-range.md`](../fixes/0022-01-untilbuild-open-range.md).

**Проверка:** `./gradlew buildPlugin test` → BUILD SUCCESSFUL, **27/27 тестов
зелёные** (лексер 13, FileType 5, highlighter 3, ColorSettingsPage 3, эргономика 2,
регресс ключевых слов 1). Собран `intellij-lam-0.1.1.zip`. Детали и остаточные
пункты (бинарный `verifyPlugin`/`runIde` в headless не запускались; расширение
диапазона на 2024.2+ требует JDK 21; семантическая LSP-подсветка) — в
[отчёте](../reports/0022-intellij-syntax-highlight.md) и бэклоге `FEATURES.md`.
