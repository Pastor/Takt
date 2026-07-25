# Задача 0125-01: Плагин IntelliJ Takt: LSP-проверка, автодополнение, документация и настройки инструментов

> Фича: [../features/0125-intellij-takt-lsp-tooling.md](../features/0125-intellij-takt-lsp-tooling.md) · ADR: [../adr/0125-intellij-takt-lsp-tooling.md](../adr/0125-intellij-takt-lsp-tooling.md) · анализ: [../analyze/0125-intellij-takt-lsp-tooling.md](../analyze/0125-intellij-takt-lsp-tooling.md)

## Что было

Плагин `extensions/intellij-takt` поднимал LSP-сервер `takt-lsp` через LSP4IJ
(фича 0038), но:

- **Настройки** (`TaktLspSettings`/`TaktLspConfigurable`) хранили **только** путь
  к `takt-lsp` — путей к `taktc`/`takt-sim`, каталогов `-I`, доп. параметров и
  выходной директории не было.
- **`initializationOptions` серверу не передавались**: `TaktLspConnectionProvider`
  поднимал процесс, но клиент не отдавал `searchPaths`, хотя сервер их читает
  (0072). Импорт из общих библиотек в редакторе не разрешался — только каталог
  документа.

Диагностики/автодополнение/hover на стороне сервера уже реализованы и анонсированы
(`takt-lang/src/lsp/`), LSP4IJ включает их по capabilities — доработок сервера не
требовалось (ADR 0125, Option A).

## Что сделано

Реализована **Option A** ADR 0125 — правки только в `extensions/intellij-takt`,
LSP-сервер не тронут:

- **Настройки** (`lsp/TaktLspSettings.kt`): в `State` добавлены поля
  `compilerPath` (`taktc`), `simulatorPath` (`takt-sim`),
  `includeDirs: MutableList<String>` (каталоги `-I`), `compilerArgs` (доп. флаги),
  `outputDir` (выходная директория) + аксессоры. Умолчания пусты (аддитивность).
- **Сборка init-options** (`lsp/TaktInitOptions.kt`, **новый**): чистая функция
  `build(includeDirs)` → `{ "searchPaths": [<непустые dirs>] }` либо `null`
  (пустой список ≡ прежнее поведение). Плюс `parseDirs`/`joinDirs` — построчный
  текст поля ↔ список. Тестируемо без GUI (драйвер 5 ADR).
- **Прокидка в сервер** (`lsp/TaktLspServerFactory.kt`):
  `TaktLspConnectionProvider.getInitializationOptions(rootUri)` возвращает
  результат `TaktInitOptions.build(...)` из настроек. Сервер сам разрешает
  относительные пути от корня рабочей области (0072).
- **UI настроек** (`lsp/TaktLspConfigurable.kt`): панель на `GridBagLayout` (без
  UI-DSL — минимальная зависимость от версии платформы) с полями: путь к
  `takt-lsp`/`taktc`/`takt-sim`/выходной директории (`TextFieldWithBrowseButton`),
  каталоги `-I` (многострочная `JTextArea`, по пути на строку), доп. параметры
  (`JTextField`). `isModified`/`apply`/`reset` покрывают все поля.

**Статус по функциональности (правило 11):**

- **Валидация файла / автодополнение / hover** — «н/п по коду»: обеспечены
  сервером (0038 + `takt-lang/src/lsp/`), LSP4IJ прокидывает по capabilities;
  дескриптор `takt-lsp4ij.xml` (server + languageMapping) уже достаточен —
  дополнительных точек расширения не потребовалось.
- **Пути `taktc`/`takt-sim`/`outputDir`/`compilerArgs`** — **задел** под
  фичу-преемника (действия компиляции/симуляции): хранятся и редактируются, но
  **не исполняются** (Option C ADR отвергнут в объёме).
- Существующий `serverPath` и его поведение (пусто → `PATH`) — не тронуты.

## Проверки

Плагин собирается и тестируется **только локально** (в `precheck.sh`/CI не входит).
Сборка требует JDK 21 (Kotlin 2.0.21 не парсит версию текущего JDK 25 —
особенность окружения, не кода):

```sh
cd extensions/intellij-takt
JAVA_HOME=<jdk-21> ./gradlew test
```

- `compileKotlin`/`compileTestKotlin` — `BUILD SUCCESSFUL`.
- `test` — `BUILD SUCCESSFUL`; новые классы: `TaktInitOptionsTest` (7/7),
  `TaktLspSettingsTest` (2/2), прочие тесты плагина зелены (регрессий нет).
- Проверки R1/R2/R4/R5 (тест-план T1–T5) — автоматизированы JUnit. R3/A3/A4
  (поведение в живом редакторе) — **ручная** приёмка запуском sandbox-IDE
  (`runIde`), объективного гейта нет (граница автоматизации, анализ).
