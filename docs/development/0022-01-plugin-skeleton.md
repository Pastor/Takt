# Задача 0022-01: Каркас плагина IntelliJ (Gradle, FileType, регистрация `.lam`)

> Фича: [../features/0022-intellij-syntax-highlight.md](../features/0022-intellij-syntax-highlight.md) · ADR: [../adr/0022-intellij-syntax-highlight.md](../adr/0022-intellij-syntax-highlight.md) · анализ: [../analyze/0022-intellij-syntax-highlight.md](../analyze/0022-intellij-syntax-highlight.md)

**Статус:** ВЫПОЛНЕНО (2026-07-13; сборка и тесты зелёные — см. «Проверки»).

## Что было

Оснастки IntelliJ для Lam нет: `.lam` открывается как plain text. Подпроекта
`extensions/intellij-lam/` не существует. Есть параллель — `extensions/zed-lam`
(структура расширения, `config.toml` с `path_suffixes=["lam"]`, `//`/`///`
комментарии) — используется как образец конфигурации языка.

## Что сделано

Заведён подпроект `extensions/intellij-lam/` (Kotlin + **IntelliJ Platform Gradle
Plugin** 2.1.0), покрывающий R1 (распознавание типа файла):

- **Gradle-каркас:** `build.gradle.kts` (плагин `org.jetbrains.intellij.platform`,
  Kotlin JVM 2.0.21, JVM toolchain 17, `instrumentCode = false` — форм/NLS нет),
  `settings.gradle.kts`, `gradle.properties` (версии/диапазон вынесены), рабочая
  обёртка `gradlew` (Gradle 8.10.2), `.gitignore`.
- **Целевая платформа:** IC **2024.1.7**, `sinceBuild = 241`, `untilBuild = 243.*`.
  Выбор линейки 2024.1.x осознан: она последняя на **Java 17** (2024.2+ требует
  Java 21) — при доступном JDK 17 это даёт воспроизводимую сборку и прогон тестов
  платформенного фреймворка. Диапазон расширяется при появлении JDK 21 в CI.
- **`plugin.xml`:** `id=org.lam.intellij`, name/vendor/description, `depends`
  платформы; регистрация `com.intellij.fileType` для `.lam` (fieldName `INSTANCE`,
  language `Lam`, extensions `lam`); закомментированные точки расширения-заготовки
  под 0022-02 (`lang.syntaxHighlighterFactory`) и 0022-03 (`colorSettingsPage`,
  `lang.commenter`, `lang.braceMatcher`). Диапазон since/until патчится
  Gradle-плагином из `gradle.properties`.
- **Исходники (Kotlin, пакет `org.lam.intellij`):** `LamLanguage`
  (`Language("Lam")`), `LamFileType` (`LanguageFileType`,
  `getDefaultExtension()="lam"`, иконка), `LamIcons` (загрузка `/icons/lam.svg`),
  каркас `psi/LamTokenType` и `psi/LamElementType` под лексер/PSI (заполнится в
  0022-02). Иконка типа файла — `resources/icons/lam.svg` (16×16, «λ»).
- **Тест:** `LamFileTypeTest` (`BasePlatformTestCase`) — расширение, имя, привязка
  к языку, распознавание `sample.lam` как `LamFileType`, загрузка иконки.

Функциональность по стекам (правило 11): язык/компилятор — **н/п** (аддитивно, не
трогаем `grammar`/`simulation`; версия языка не меняется).

## Проверки

Прогон в этом окружении (JDK 17, Gradle 8.10.2 через обёртку; платформа скачана
Gradle-плагином):

- `./gradlew buildPlugin test --no-daemon` → **BUILD SUCCESSFUL** (3m 3s);
  собран артефакт `build/distributions/intellij-lam-0.1.0.zip`.
- `:test` → **5/5 зелёных** (`LamFileTypeTest`, `failures=0, errors=0`):
  `testDefaultExtensionIsLam`, `testFileTypeName`, `testFileTypeBoundToLamLanguage`,
  `testLamFileIsRecognizedAsLamType` (`.lam` → `LamFileType`, **не** PLAIN_TEXT),
  `testIconLoads`. → выполнены R1 и критерий приёмки **A1** (анализ).
- Первый прогон падал на `:instrumentCode` (нужен инструментарий) и предупреждал о
  Java 21 для платформы 2024.2 — исправлено: `instrumentCode = false` + переход на
  линейку 2024.1.x (Java 17).
- `runIde` (ручная проверка открытия `.lam` в песочнице IDE) в headless-окружении
  не выполнялся; распознавание типа файла подтверждено автотестом
  `testLamFileIsRecognizedAsLamType`.
