# Плагин IntelliJ IDEA — язык Lam

Плагин для JetBrains IntelliJ IDEA (и IDE на IntelliJ Platform) с поддержкой
языка **Lam** (Language of Automata Models): распознавание файлов `.lam` и
**подсветка синтаксиса**. Часть монорепозитория проекта Lam (фича
[`0022`](../../docs/features/0022-intellij-syntax-highlight.md)).

## Возможности

- Распознавание `*.lam` как языка Lam (свой тип файла и иконка).
- Лексическая подсветка: ключевые слова, операторы (пост-0021 `:=` присваивание,
  `=` сравнение, `<=` реляционный; `==` выведен — подсвечивается как ошибка),
  числа, строки, комментарии `//` / `///` / `/* … */`, скобки и пунктуация.
- Страница настройки цветов (**Settings → Editor → Color Scheme → Lam**).
- Комментирование строкой `//` (Ctrl+/) и блоком `/* … */`; подсветка парных
  скобок `{}` `()` `[]`.

Подсветка **лексическая** и работает офлайн (без LSP-сервера) в любой редакции,
включая Community. Семантическая подсветка через `lam-lsp` — задел на будущее.

## Требования

- JDK 17.
- Целевая платформа: IntelliJ IDEA Community **2024.1.x** (`sinceBuild 241` /
  `untilBuild 243.*`). Версии вынесены в [`gradle.properties`](gradle.properties).

## Сборка и запуск

```sh
cd extensions/intellij-lam

./gradlew buildPlugin      # собрать плагин → build/distributions/intellij-lam-<версия>.zip
./gradlew test             # юнит-тесты (лексер, highlighter, эргономика, регресс ключевых слов)
./gradlew runIde           # запустить песочницу IDE с установленным плагином
./gradlew verifyPlugin     # проверка совместимости (IntelliJ Plugin Verifier)
```

## Установка из файла

**Settings → Plugins → ⚙ → Install Plugin from Disk…** и выбрать собранный
`build/distributions/intellij-lam-<версия>.zip`.

## Структура

```
src/main/kotlin/org/lam/intellij/
  LamLanguage.kt, LamFileType.kt, LamIcons.kt   — язык и тип файла (0022-01)
  psi/          LamTokenType, LamElementType, LamTokenTypes  — токены
  lexer/        LamLexer                          — лексер (0022-02)
  highlight/    LamSyntaxHighlighter(+Factory),
                LamHighlighterColors, LamColorSettingsPage    — подсветка/цвета
  editor/       LamCommenter, LamBraceMatcher     — эргономика (0022-03)
src/main/resources/META-INF/plugin.xml           — точки расширения
src/main/resources/icons/lam.svg                 — иконка типа файла
```

## Источник истины лексики

Набор ключевых слов и операторов зеркалит Rust-лексер компилятора
`grammar/src/parser/lexer.rs`. Соответствие проверяется регресс-тестом
`LamKeywordSyncTest` (читает таблицу `KEYWORDS` из лексера и сверяет с плагином).
При изменении лексики языка обновите `LamTokenTypes.KEYWORDS`/`LamLexer`.
