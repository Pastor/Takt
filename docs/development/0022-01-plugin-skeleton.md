# Задача 0022-01: Каркас плагина IntelliJ (Gradle, FileType, регистрация `.lam`)

> Фича: [../features/0022-intellij-syntax-highlight.md](../features/0022-intellij-syntax-highlight.md) · ADR: [../adr/0022-intellij-syntax-highlight.md](../adr/0022-intellij-syntax-highlight.md) · анализ: [../analyze/0022-intellij-syntax-highlight.md](../analyze/0022-intellij-syntax-highlight.md)

**Статус:** ЗАПЛАНИРОВАНО (разработка не начата; проработка до стадии «Разработка»).

## Что было

Оснастки IntelliJ для Lam нет: `.lam` открывается как plain text. Подпроекта
`extensions/intellij-lam/` не существует. Есть параллель — `extensions/zed-lam`
(структура расширения, `config.toml` с `path_suffixes=["lam"]`, `//`/`///`
комментарии) — используется как образец конфигурации языка.

## Что план (объём задачи)

Каркас плагина на **Gradle IntelliJ Platform Plugin** (Kotlin), покрывающий R1
(распознавание типа файла):

- Подпроект `extensions/intellij-lam/` с `build.gradle.kts` (плагин
  `org.jetbrains.intellij.platform`), `settings.gradle.kts`, `gradle.properties`
  (`sinceBuild`/`untilBuild`, версия платформы), обёртка `gradlew`.
- `src/main/resources/META-INF/plugin.xml`: `id`, `name`, `vendor`,
  `description`, регистрация расширений (`fileType`), заготовки точек расширения
  для 0022-02/03.
- `LamLanguage` (`com.intellij.lang.Language`), `LamFileType`
  (`LanguageFileType`, `getDefaultExtension() = "lam"`), иконка `lam.svg`.
- `LamIcons` (загрузка иконки), базовый `LamTokenTypes`/`LamElementType`-каркас
  под лексер (заполняется в 0022-02).

Функциональность по стекам (правило 11): язык/компилятор — **н/п** (аддитивно, не
трогаем `grammar`/`simulation`).

## Проверки (план)

- `gradle buildPlugin` собирает пустой плагин без ошибок.
- Тест: `LamFileType.getDefaultExtension() == "lam"`, язык зарегистрирован.
- `runIde`: открыть `examples/**/*.lam` — файл распознан как Lam (иконка/тип),
  подсветки пока нет (появится в 0022-02).
- Соответствие R1 и критерию приёмки A1 (анализ).
