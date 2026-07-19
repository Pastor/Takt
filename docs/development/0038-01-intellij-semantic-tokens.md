# Задача 0038-01: Интеграция LSP4IJ — запуск lam-lsp из плагина

> Фича: [../features/0038-intellij-semantic-tokens.md](../features/0038-intellij-semantic-tokens.md) · ADR: [../adr/0038-intellij-semantic-tokens.md](../adr/0038-intellij-semantic-tokens.md) · анализ: [../analyze/0038-intellij-semantic-tokens.md](../analyze/0038-intellij-semantic-tokens.md) · тест-план: [../tests/0038-intellij-semantic-tokens.md](../tests/0038-intellij-semantic-tokens.md)

## Что было

**Реальное состояние на 2026-07-15** (проверено чтением кода, а не по документам).

### Плагин `extensions/intellij-lam` — версия `0.4.0`

Собирается под IntelliJ IDEA **Community**: `platformType = IC`,
`platformVersion = 2024.1.7` (последняя линейка на Java 17), `jvmToolchain(17)`,
IntelliJ Platform Gradle Plugin `2.1.0`, Kotlin `2.0.21`. Диапазон:
`pluginSinceBuild = 241`, `pluginUntilBuild` **пуст** (открытый верхний диапазон,
фикс [0023-01](../fixes/0023-01-verifyplugin-descriptor.md)); `verifyPlugin`
настроен на IC 2024.3 / 2025.1 / 2025.2.

Что умеет (21 Kotlin-файл в `src/main`, 10 тестовых классов):

| Возможность | Реализация | Фича |
|---|---|---|
| Тип файла `.lam`, язык, иконка | `LamFileType`, `LamLanguage`, `LamIcons` | 0022-01 |
| **Лексическая** подсветка | `LamLexer` (рукописный `LexerBase`, зеркалит `parser/lexer.rs`), `LamSyntaxHighlighter`, `LamHighlighterColors` | 0022-02 |
| Настройка цветов, commenter, brace matcher | `LamColorSettingsPage`, `LamCommenter`, `LamBraceMatcher` | 0022-03 |
| Плоский PSI (токены листьями под корень) | `LamParser`, `LamParserDefinition`, `LamFile` | 0023 |
| Go to Declaration, навигация по `import` | `LamGotoDeclarationHandler`, `LamSymbolScanner`, `LamImports` | 0023 |

**Чего нет:** ни одного упоминания LSP в плагине. `plugin.xml` объявляет только
`<depends>com.intellij.modules.platform</depends>`; точек расширения LSP нет,
LSP4IJ в `build.gradle.kts` не подключён, внешние процессы не запускаются.
Подсветка целиком автономна и целиком лексическая.

**Потолок лексического слоя (проблема из ADR).** `LamLexer` видит только форму
токена: любое имя — `IDENTIFIER`. Функция `process`, тип `Celsius`, вариант
`enum` `RED`, состояние `Idle` и переменная `counter` окрашены **одинаково**.
Различить их без семантической модели невозможно в принципе.

### Сервер `lam-lsp` — semantic tokens **уже реализованы**

Ключевой факт, перепроверенный при анализе (кандидат в `FEATURES.md` оставлял его
открытым; вердикт — **умеет**):

- `grammar/src/lsp.rs:17` — `pub const SEMANTIC_TOKEN_TYPES` — легенда из 10
  типов: `KEYWORD`, `VARIABLE`, `FUNCTION`, `TYPE`, `ENUM_MEMBER`, `STRING`,
  `NUMBER`, `COMMENT`, `OPERATOR`, `CLASS` (состояния и модели).
- `grammar/src/lsp.rs:1412` — `pub fn semantic_tokens(source: &str) -> SemanticTokens`:
  токенизация лексером, **обогащение идентификаторов семантической моделью**
  (`search_func` → `FUNCTION`; `types`/`enums` → `TYPE`; `search_enum_variant` →
  `ENUM_MEMBER`; `search_state`/`models` → `CLASS`; иначе `VARIABLE`;
  `BUT_BUILTIN_TYPES` имеют приоритет → `TYPE`), добавление комментариев,
  сортировка, дельта-кодирование, длина в кодовых единицах **UTF-16**.
- `grammar/src/bin/lam_lsp.rs:51` — `semantic_tokens_provider` в
  `ServerCapabilities`: `full: Some(Bool(true))`, `range: None`,
  `token_modifiers: vec![]`.
- `grammar/src/bin/lam_lsp.rs:200` — обработчик
  `textDocument/semanticTokens/full` → `grammar::lsp::semantic_tokens(text)`.
- Бинарник собирается **только** с флагом: `grammar/Cargo.toml` —
  `[[bin]] name = "lam-lsp"`, `required-features = ["lsp"]`.

Сервер также уже отдаёт `document_formatting_provider` (`lam_lsp.rs:50`) — это
делает LSP4IJ-путь выгодным и для фичи [0039](../features/0039-intellij-reformat.md).

**Вывод:** серверную часть писать не нужно. Отсутствует **потребитель** — это и
есть предмет фичи.

## Что сделано

> **Этап 1 — миграция платформы (2026-07-19).** Клиентский Kotlin-слой
> (`LamLspServerSupportProvider`, резолвинг пути, настройки, деградация) — далее.

⚠️ **Пересмотр драйвера 1 ADR (см. врезку в [ADR 0038](../adr/0038-intellij-semantic-tokens.md)).**
Реализация вскрыла: **LSP4IJ несовместим с build 241** — все актуальные версии
(0.19+, 0.20.1) требуют `sinceBuild = 242` (2024.2 → JDK 21). Плагин был на
2024.1.7/JDK 17. Решение заказчика — **поднять платформу**, не пиннить старый
LSP4IJ. Сделано:

- `settings.gradle.kts` — foojay-резолвер (`org.gradle.toolchains.foojay-resolver-convention`):
  toolchain **авто-провизионирует JDK 21** (проверено — скачивается и компилирует).
- `gradle.properties` — `platformVersion 2024.1.7 → 2024.2.5`, `pluginSinceBuild
  241 → 242`, `pluginVersion 0.4.0 → 0.5.0`. `platformType = IC` (Community цел).
- `build.gradle.kts` — `jvmToolchain(17 → 21)`; зависимость
  `plugin("com.redhat.devtools.lsp4ij:0.20.1")` (опциональность — в plugin.xml);
  `testImplementation("org.opentest4j:opentest4j:1.3.0")` — тест-фреймворк 2024.2
  требует opentest4j на classpath (при 2024.1 подтягивался транзитивно).
- **Фикс дрейфа (побочно):** `LamTokenTypes.KEYWORDS` не содержал `invariant`
  (ключевое слово фичи 0044) — `LamKeywordSyncTest` это ловит, но был **закэширован
  зелёным** на старой платформе; свежий прогон под 2024.2 вскрыл. Добавлено.
- **53 существующих теста плагина зелёные** под 2024.2.5/JDK 21 (`./gradlew test`).

### Клиентский слой (этап 2, 2026-07-19)

Реализован в пакете `org.lam.intellij.lsp` (LSP4IJ 0.20.1):

- **`LamLspBinary`** — резолвинг пути (настройка → `PATH` → `null`), чистая логика,
  тест `LamLspBinaryTest` (6 тестов: приоритеты, деградация без исключений).
- **`LamLspSettings`** — `PersistentStateComponent` уровня приложения (путь к
  серверу); зарегистрирован `applicationService` в `plugin.xml` (класс не зависит
  от LSP4IJ, доступен всегда).
- **`LamLspServerFactory`** (`LanguageServerFactory`) + `LamLspConnectionProvider`
  (`OSProcessStreamConnectionProvider`, stdio): поднимает `lam-lsp`. Тихая
  деградация (R3): нет бинарника → `commandLine` не задан → `start()` бросает
  `CannotStartProcessException`, LSP4IJ показывает сервер остановленным **без**
  модального диалога.
- **`plugin.xml`**: `<depends optional="true" config-file="lam-lsp4ij.xml">com.redhat.devtools.lsp4ij</depends>`
  (R1 — без LSP4IJ плагин грузится). **`lam-lsp4ij.xml`**: `<server>` +
  `<languageMapping>` + `<semanticTokensColorsProvider>` (⚠️ атрибут класса —
  `class`, не `className`; вскрыто `buildSearchableOptions`).
- **Проверки:** `./gradlew clean buildPlugin test` — BUILD SUCCESSFUL, все тесты
  зелёные (53 регресс 0022/0023 + 6 резолвер + 3 цвета 0038-02); плагин
  `0.5.0.zip` собран, LSP4IJ-провайдеры инстанцируются без ошибок.

⚠️ **Отложено (GUI-уточнения R3, слабо проверяемы без `runIde`):** страница
настроек (`Configurable`) для правки пути в UI и одноразовое уведомление о
ненайденном бинарнике. Ядро работает без них: `PATH`-автопоиск даёт сервер
«из коробки», путь хранится в `LamLspSettings`. **A11** (цвета в редакторе) —
только визуально в среде с GUI.

### План клиентского слоя (исходный)

Тонкий слой интеграции LSP4IJ в `extensions/intellij-lam`, включающий сервер
`lam-lsp` **опционально** и деградирующий молча при его отсутствии.

1. **Опциональная зависимость на LSP4IJ (R1).**
   `build.gradle.kts`: `intellijPlatform { plugins("com.redhat.devtools.lsp4ij:<версия>") }`
   (версия фиксируется явно — API LSP4IJ моложе платформенных, риск анализа).
   `plugin.xml`: `<depends optional="true" config-file="lam-lsp4ij.xml">com.redhat.devtools.lsp4ij</depends>`;
   все LSP-точки расширения выносятся в **отдельный** `lam-lsp4ij.xml`, чтобы
   отсутствие LSP4IJ не срывало загрузку плагина.

2. **Дескриптор сервера (R2).** `org.lam.intellij.lsp.LamLspServerSupportProvider` +
   `LamLspServerDescriptor`: сопоставление `LamFileType` → сервер; команда
   запуска — резолвнутый путь к `lam-lsp`; транспорт stdio (сервер поднимается
   через `Connection::stdio()`).

3. **Резолвинг пути к бинарнику (R2, R3).**
   `org.lam.intellij.lsp.LamLspBinary` — чистая, тестируемая **без GUI** логика:
   (1) явная настройка плагина; (2) автопоиск `lam-lsp` в `PATH`;
   (3) не найден / не исполняемый → `null`.

4. **UI настройки (R3).** Страница настроек плагина: поле «Путь к `lam-lsp`»,
   кнопка проверки, чекбокс «не напоминать». Хранилище — `PersistentStateComponent`.

5. **Тихая деградация (R3).** `null` от резолвера ⇒ дескриптор не создаётся,
   сервер не стартует, семантический слой не включается. Модальных диалогов нет;
   уведомление — **не чаще одного на проект**, уровень `INFORMATION`, с действием
   «указать путь» и возможностью отключить.

6. **Совместимость (R8).** Сверить `sinceBuild` LSP4IJ с открытым `until-build`
   плагина; при конфликте — поднять `pluginSinceBuild` в `gradle.properties` и
   отразить в README. Платформа остаётся IC / JDK 17 (Community — драйвер 1 ADR).

**Статус по функциональности (правило 11):**

| Функциональность | Статус |
|---|---|
| Плагин `extensions/intellij-lam` | новая функциональность (LSP-слой), версия `0.4.0 → 0.5.0` |
| Крейт `grammar` | **н/п** — сервер уже реализует semantic tokens; тесты — задача [0038-03](0038-03-server-tokens-tests.md) |
| Крейт `simulation` | **н/п** — не затрагивается |
| Язык Lam (синтаксис/семантика/версия) | **н/п** — не затрагивается (правило 22 неприменим) |
| Лексическая подсветка 0022 / навигация 0023 | не изменяются; регресс обязателен (T10, T11) |

## Проверки

> **Планируется (разработка не начата).** Команды и ожидаемые результаты — из
> тест-плана; фактические — по ходу реализации.

| Проверка | Команда | Ожидаемый результат | T |
|---|---|---|---|
| Резолвинг: бинарник не найден | `./gradlew test` | «нет сервера», исключения нет | T12 |
| Резолвинг: битый путь | `./gradlew test` | слой выключен, без throw | T13 |
| Регресс 0022 без LSP4IJ | `./gradlew test` | 27 тестов зелёные | T10 |
| Регресс 0023 без LSP4IJ | `./gradlew test` | 20 тестов зелёные | T11 |
| Сборка плагина | `./gradlew clean buildPlugin test` | BUILD SUCCESSFUL, все тесты зелёные | T14 |
| Совместимость IDE | `./gradlew verifyPlugin` | Compatible для IC 2024.3 / 2025.1 / 2025.2 | T15 |
| **Живой запуск сервера из IDE** | `./gradlew runIde` (**GUI**) | LSP4IJ поднимает `lam-lsp`, статус «running» в окне LSP4IJ | T16 |
| **Деградация без сервера / без LSP4IJ** | `./gradlew runIde` (**GUI**) | подсветка = `0.4.0`, модальных ошибок нет | T18, T19 |

**Ограничение (предсуществующее, не снимается этой задачей):** собрать и
прогнать плагин в CI-среде нельзя, `runIde` требует GUI (остаточные пункты
0022/0023). Логика резолвинга пути вынесена в чистый класс **специально**, чтобы
её можно было проверить автотестом без GUI; живой запуск сервера проверяется
только визуально.
