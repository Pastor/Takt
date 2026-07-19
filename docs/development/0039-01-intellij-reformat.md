# Задача 0039-01: Внешний форматтер — `AsyncDocumentFormattingService` + `lamc fmt --stdin`

> ⚠️ **СНЯТА как неактуальная (2026-07-19).** Развилка фичи 0039 разрешена в
> пользу **Option B (LSP4IJ)** после принятия LSP4IJ в фиче 0038 — форматирование
> приходит от `lam-lsp` `textDocument/formatting`, внешний форматтер `lamc fmt
> --stdin` **не реализуется**. Задача сохранена как история решения (Option C).
> См. [ADR, «Обновление решения (2026-07-19)»](../adr/0039-intellij-reformat.md) и
> [карточку фичи, «Итог»](../features/0039-intellij-reformat.md).

> Фича: [../features/0039-intellij-reformat.md](../features/0039-intellij-reformat.md) · ADR: [../adr/0039-intellij-reformat.md](../adr/0039-intellij-reformat.md) · анализ: [../analyze/0039-intellij-reformat.md](../analyze/0039-intellij-reformat.md) · тест-план: [../tests/0039-intellij-reformat.md](../tests/0039-intellij-reformat.md) · следующие: [0039-02](0039-02-lamc-settings.md), [0039-03](0039-03-golden-tests.md)

> **Статус: Планируется (разработка не начата).** Раздел «Что было» — реальное
> состояние на 2026-07-15, проверенное фактами. Разделы «Что сделано» и
> «Проверки» — **план**, а не отчёт.
>
> **Оговорка (см. карточку фичи):** брать в работу после решения заказчика по
> LSP4IJ в фиче [0038](../features/0038-intellij-semantic-tokens.md). Если LSP4IJ
> будет принят, эта задача **не выполняется** — форматирование придёт бесплатно,
> и фича сведётся к приёмочным тестам.

## Что было

Реальное состояние, проверенное перед планированием (не пересказ источников —
каждый пункт подтверждён).

### Ядро форматирования — готово (фича 0024, закрыта)

| Компонент | Где | Состояние |
|---|---|---|
| `format_source` — каноническая печать от АСД | `grammar/src/format/` | готово |
| `lamc fmt --stdin` — stdin → stdout | `grammar/src/bin/lamc.rs:322–345` (разбор аргументов — `:274–295`) | **готово; проверено реальной пробой** |
| `lamc fmt --check` — код `0`/`1` | `grammar/src/bin/lamc.rs` | **готово; проверено** |
| LSP `textDocument/formatting` | `grammar/src/bin/lam_lsp.rs:50` (capability), `:144–162` (обработчик); ядро — `grammar/src/lsp.rs:146` | готово (задача [0024-04](0024-04-lsp-formatting.md)) |

Проба `lamc fmt --stdin` на `examples/extend_complex.lam`: вывод из stdin
**байт-в-байт** совпал с файловым режимом **и** с каноническим оригиналом;
`--check --stdin` → код `0` на каноничном входе, `1` на изменённом. То есть
контракт CLI, нужный этой задаче, **уже достаточен** — правок в Rust не
требуется.

### Плагин — подсветка и навигация есть, форматирования нет

| Компонент | Где | Состояние |
|---|---|---|
| `LamFileType`, `LamLanguage` | `src/main/kotlin/org/lam/intellij/` | готово (0022) |
| `LamLexer`, `LamSyntaxHighlighter` | `lexer/`, `highlight/` | готово (0022) |
| Навигация, `import` | `navigation/` | готово (0023) |
| **`LamParser` — плоский** | `parser/LamParser.kt:15–24` | **подтверждено**: цикл `advanceLexer()` под одним `rootMarker`; структура **не строится** — осознанное решение [ADR 0023](../adr/0023-intellij-navigation-include.md), Option A |
| Форматирование | — | **отсутствует**: в `plugin.xml` нет ни `lang.formatter`, ни `formattingService` |

Платформа: `IC 2024.1.7`, JDK 17, `pluginSinceBuild = 241`, `pluginUntilBuild`
пуст (открытый диапазон), `pluginVersion = 0.4.0`. Тестов — **53**, зелёные.

### Ключевой факт: посылка «нужен PSI» — неверна

Задача [0024-04](0024-04-lsp-formatting.md) и витрина `FEATURES.md` исходили из
того, что реформат требует структуры PSI (через `FormattingModelBuilder`),
которой у плоского `LamParser` нет. **Это опровергнуто** проверкой самой
платформы сборки:

```
ideaIC-2024.1.7/lib/app-client.jar :: META-INF/CodeStyle.xml
  <extensionPoint name="formattingService"
                  interface="com.intellij.formatting.service.FormattingService"
                  dynamic="true"/>

ideaIC-2024.1.7/lib/app-client.jar
  com/intellij/formatting/service/AsyncDocumentFormattingService.class
  com/intellij/formatting/service/FormattingService.class
```

Тот же класс присутствует и в `ideaIC-2025.1` — открытый `until-build` не под
угрозой. Это штатный механизм платформы для **внешних** форматтеров (по нему
работают Prettier, rustfmt, shfmt); PSI он не требует. Поэтому задача
**реализуема на текущем плагине без единой правки `LamParser`**.

## Что сделано

> **Планируется (разработка не начата).** Ниже — план задачи.

### План реализации

1. **`LamExternalFormatter : AsyncDocumentFormattingService`**
   (`src/main/kotlin/org/lam/intellij/format/LamExternalFormatter.kt`):
   - `canFormat(file)` — только `LamFileType`;
   - `getFeatures()` — **без** `FORMAT_FRAGMENTS`: ядро 0024 форматирует документ
     **целиком**, печать выделенного фрагмента им не поддержана — заявлять такую
     возможность платформе было бы ложью;
   - `createFormattingTask(request)` — запуск `lamc fmt --stdin`: исходник в
     stdin, результат из stdout, обмен в **UTF-8** (R8), таймаут (R9);
   - ненулевой код возврата → `request.onError(...)` со stderr `lamc`; документ
     **не трогается** (R4);
   - пустой stdout при коде `0` → тоже отказ (защита от «отформатировать во
     что-то»; R4);
   - путь к бинарнику — из резолвера задачи [0039-02](0039-02-lamc-settings.md)
     (до её готовности — временно `lamc` из `PATH`).
2. **Регистрация в `plugin.xml`** (в `defaultExtensionNs="com.intellij"`):
   ```xml
   <formattingService implementation="org.lam.intellij.format.LamExternalFormatter"/>
   ```
   `lang.formatter` / `FormattingModelBuilder` **не добавляются** — инвариант R3.
3. **`LamParser` не трогается** — плоский разбор остаётся решением ADR 0023
   (R3, T16).

### Статус по функциональности (правило 11)

| Функциональность | Работа | Обоснование |
|---|---|---|
| Язык `.lam` | **н/п** | синтаксис/семантика не меняются; версия языка не растёт (правило 22) |
| Крейт `grammar` | **н/п** | контракт `lamc fmt --stdin` уже достаточен (проверено пробой) — правок нет |
| Крейт `simulation` | **н/п** | не затрагивается |
| Плагин 0022 (подсветка) | **аддитивно** | новый EP; существующие компоненты не трогаются |
| Плагин 0023 (навигация, `LamParser`) | **не трогается** | инвариант плоского разбора **подтверждается**, а не отменяется |
| Настройки пути к `lamc` | вынесено | задача [0039-02](0039-02-lamc-settings.md) |
| Тесты «байт-в-байт» | вынесено | задача [0039-03](0039-03-golden-tests.md) |

## Проверки

> **Планируется (разработка не начата).** Ниже — план проверок; фактические
> результаты появятся в отчёте о тестировании.

- `./gradlew test` — новые тесты задачи: **T5** (вызов идёт **через процесс**:
  шпион фиксирует аргументы `fmt --stdin` и переданный stdin), **T6–T9** (отказ
  вместо порчи: неверный синтаксис, `assembly`, пустой stdout, таймаут).
  Существующие **53 теста** обязаны остаться зелёными (T23).
- `./gradlew buildPlugin` — BUILD SUCCESSFUL (T22).
- Статически: `grep` подтверждает отсутствие `FormattingModelBuilder` и
  `lang.formatter` (T15, T17); `git diff` — `LamParser.kt` не изменён (T16).
- `cargo test --all-features -- --test-threads=1` — ожидается **отсутствие
  дельты**: задача не трогает крейты Rust (T18, A10).
- Среда проверки (подтверждено фактом): сборка и тесты плагина идут **headless**
  — `./gradlew cleanTest test --offline --no-build-cache` → BUILD SUCCESSFUL,
  53 теста, ~8 с. GUI нужен **только** `runIde` (пробы V1/V2 тест-плана).

Ожидаемые результаты соответствуют требованиям **R1, R3, R4, R8, R9** и критериям
**A3, A4** анализа.
