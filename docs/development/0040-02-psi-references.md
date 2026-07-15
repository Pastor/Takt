# Задача 0040-02: Ссылки и резолв имён на PSI; `PsiReference` для путей `import`

> Фича: [../features/0040-intellij-psi-parser.md](../features/0040-intellij-psi-parser.md) · ADR: [../adr/0040-intellij-psi-parser.md](../adr/0040-intellij-psi-parser.md) · анализ: [../analyze/0040-intellij-psi-parser.md](../analyze/0040-intellij-psi-parser.md)

> **Планируется (разработка не начата).** Фича **ЗАБЛОКИРОВАНА** зависимостью
> [0038](../features/0038-intellij-semantic-tokens.md); задача выполняется после
> [0040-01](0040-01-intellij-psi-parser.md).

## Что было

- **Резолв имён (0023):** `LamSymbolScanner.scan(file.text)` заново токенизирует
  текст и отдаёт список `LamDeclaration(name, range, kind)`;
  `LamGotoDeclarationHandler` фильтрует по имени под кареткой и через
  `file.findElementAt(range.startOffset)` отдаёт **листовой токен** как цель.
  Настоящих `PsiReference` нет — платформа о связи «использование → декларация»
  не знает.
- **Пути `import` (0023):** `LamImports.pathOf(element)` достаёт путь из
  строкового токена, `LamImports.resolve(element, path)` ищет файл относительно
  каталога и корней контента. Навигация работает, но через тот же
  `GotoDeclarationHandler`. **`PsiReferenceContributor` в `plugin.xml`
  отсутствует** — итог 0023 фиксирует причину: «ссылки не привязываются к
  листовым токенам».
- **Следствие (ADR 0023, Cons):** без `PsiReference` платформа не может ни
  переименовать, ни найти использования, ни обновить путь при перемещении файла.

## Что сделано

**Планируется (разработка не начата).** План задачи:

1. **`LamNamedElement`** — `NAME_ID` из [0040-01](0040-01-intellij-psi-parser.md)
   реализует `PsiNamedElement` (`getName`/`setName`/`getNameIdentifier`).
   `setName` — основа rename ([0040-04](0040-04-rename.md)).
2. **`LamNameReference`** — `PsiReferenceBase<LamNameElement>` на идентификаторе-
   использовании: `resolve()` ищет декларацию **в текущем файле** по PSI (не
   перетокенизацией текста). Область — файл; кросс-файловый резолв **не
   реализуется** (R6 принимается от 0038, см. [0040-05](0040-05-lsp-arbitration.md)).
3. **`LamImportPathReference` (R5, ядро остатка фичи)** — `FileReference`/
   `PsiReferenceBase` на строковом литерале-пути, регистрируется через
   `psi.referenceContributor` в `plugin.xml`. Даёт то, чего LSP-путь дать не
   может: `bindToElement` ⇒ **переименование/перемещение файла средствами IDEA
   обновляет путь** в `.lam`. Все 4 формы: `import "P";`, `import "P" as X;`,
   `import * as X from "P";`, `import { A as C } from "P";`. Логика поиска файла
   **переиспользуется** из `LamImports` (не переписывается).
4. **Совместимость с 0023 (R8) — жёсткое требование.** `LamGotoDeclarationHandler`
   либо остаётся (поверх PSI-резолва), либо снимается в пользу штатного
   `PsiReference.resolve()` — **но наблюдаемое поведение не меняется**:
   `LamGotoDeclarationTest`, `LamSymbolScannerTest`, `LamImportReferenceTest`
   зелёные **без правок ожиданий**. Строка вне `import` ссылки не несёт (T11 —
   регресс поведения 0023).
5. **Токенная эвристика уходит из горячего пути:** резолв по PSI вместо
   `scan(file.text)` при каждом запросе.

**Статус по функциональности (правило 11):**

- Плагин `intellij-lam` — ссылки/резолв/контрибьютор (основная работа).
- Навигация 0023 — обязана работать без изменений (R8; T21).
- `grammar`/`simulation`, язык — **н/п** (не затрагиваются).
- `lam-lsp` — **н/п** (LSP-эквивалент `documentLink` отнесён к 0038).

## Проверки

**Планируется.** Соответствие: R5, R8 анализа; критерии A5–A7, A13; проверки
T8–T11, T21 тест-плана.

```sh
cd extensions/intellij-lam
./gradlew --offline clean buildPlugin test
```

- **T8/A5** — `reference.resolve()` == ожидаемый `PsiFile` во всех 4 формах `import`.
- **T9/A6** — `myFixture.renameElement(psiFile, "t.lam")` → в тексте `import "t.lam";`.
- **T10/A7** — битый путь: `resolve()` == `null`, без исключений.
- **T11** — строка в `formula "…"` ссылки не несёт (регресс 0023).
- **T21/A13** — 47 существующих тестов зелёные без правок ожиданий.
- Новые тесты — по правилу `CLAUDE.md`: сперва зонд для захвата реального вывода,
  затем assertions против захваченных значений.
