# Анализ фичи 0067: Rename и `PsiReference` для `import` в плагине IntelliJ

> Фича: [../features/0067-intellij-rename-psi-import.md](../features/0067-intellij-rename-psi-import.md) · ADR: [../adr/0067-intellij-rename-psi-import.md](../adr/0067-intellij-rename-psi-import.md) · тест-план: [../tests/0067-intellij-rename-psi-import.md](../tests/README.md)

## Цель и контекст

Дать плагину `extensions/intellij-lam` две возможности, которые LSP-путь (0038)
не покрывает: **R5** — настоящую `PsiReference` для путей `import` (Ctrl+Click +
**rename-on-move**) и **R3** — нативный rename имён Lam. Направление и механизм
заданы [ADR 0067](../adr/0067-intellij-rename-psi-import.md) (Option B —
хирургическое оборачивание одиночных токенов в композитные PSI-узлы; полное
структурное дерево отвергнуто). Проработка отменённой 0040 (анализ/тест-план/
dev-подзадачи) переиспользуется.

## Зависимости фичи (правило 17/19)

- **Зависит от:** **0038** (LSP4IJ вживую) — **закрыта**. Зависимость реальная:
  без 0038 нельзя было измерить остаток (R2/R4/R6 закрыты именно ей). ADR 0067
  исполнил контрольную точку ADR 0040. Иных зависимостей нет
  (`grammar`/`simulation` не затрагиваются — фича редакторная).
- **Влияние на порядок разработки:** завершение 0067 не разблокирует другие фичи
  (0089 — «остаточные проверки плагина» — независима). Порядок хвоста не меняется.

## Требования и проверяемые условия

- **R5.1.** Путь во всех четырёх формах `import` (`import "P";`, `import "P" as
  X;`, `import * as X from "P";`, `import { A as C } from "P";`) несёт
  `PsiReference`, `resolve()` → `PsiFile` целевого файла.
- **R5.2.** Переименование/перемещение целевого файла средствами IDEA обновляет
  строку-путь в `.lam` (`bindToElement`/`handleElementRename` у `FileReference`).
- **R5.3.** Битый путь → `resolve() == null`, без исключений. Строка вне `import`
  (в `formula`) `PsiReference` **не** несёт (регресс поведения 0023).
- **R3.1.** `renameElementAtCaret` переименовывает декларацию и её использования
  **в том же файле** для всех видов имён: `model`/`state`/`start`/`type`/`enum`/
  вариант `enum`/`cond`/`var`/`const`/`fn`/порт (`in`/`out`/`inout`)/алиас
  `import`.
- **R3.2.** Идентичные подстроки в комментариях и строковых литералах **не**
  затрагиваются (следствие механизма: в них нет `PsiReference`).
- **R3.3.** Undo возвращает текст байт-в-байт; конфликт имён (rename в занятое
  имя) — штатное предупреждение IDEA; недопустимое имя (ключевое слово Lam) —
  отвергается `NamesValidator` (список слов из `LamLexer`, сторож
  `LamKeywordSyncTest`).
- **Анти-R (антидивергенция).** `LamPsiCorpusTest` по всему корпусу `.lam`:
  round-trip PSI байт-в-байт + вердикт (наличие `PsiErrorElement`) совпадает с
  оракулом `lamc`.
- **Регресс.** 47 тестов 0022/0023 зелёные **без правки ожиданий**.

## Проектные решения по реализации (по подзадачам ADR)

### 0067-01 — R5 (PsiReference для `import`)

- **Парсер** (`LamParser`): почти плоский обход дополняется одним правилом —
  токен `STRING`, у которого **предыдущий значимый** токен — ключевое слово
  `import`/`from`, оборачивается в композит `LamElementTypes.IMPORT_PATH`
  (`builder.mark()` … `advanceLexer()` … `m.done(IMPORT_PATH)`). PsiBuilder
  пропускает пробелы/комментарии, поэтому «предыдущий значимый» = предыдущий
  токен цикла. Это тот же контекст, что уже кодирует `LamImports`/`LamSymbolScanner`.
- **PSI-узел** `LamImportPath : ASTWrapperPsiElement` переопределяет
  `getReferences()` → `FileReferenceSet(content, this, startInElement = 1, null,
  caseSensitive = true).allReferences` (start = 1 — после открывающей кавычки).
  `computeDefaultContexts()` переопределяется на каталог содержащего файла
  (паритет с ядром 0055 «рядом с импортирующим»). Логика извлечения содержимого
  пути переиспользует форму из пробы; резолв — штатный `FileReferenceSet`.
- **`LamImports.isImportPathElement`** переносится с эвристики `prevSibling` на
  «мой родитель — `IMPORT_PATH`» (или лист внутри него) — иначе оборачивание
  сломало бы `prevSibling`-цепочку. `LamGotoDeclarationHandler` продолжает
  работать: `sourceElement` — лист `STRING`, его тип не изменился.
- **`plugin.xml`:** узел несёт ссылки сам (`getReferences()`), отдельный
  `psi.referenceContributor` не обязателен — проба показала, что контрибьютор на
  лист не срабатывает, а на композит достаточно прямого `getReferences()`.

### 0067-02 — R3 (нативный rename)

- **Парсер:** идентификатор-**декларация** (по форме `kw <Id>`/порт/вариант
  `enum`/алиас `import` — правила `LamSymbolScanner`) оборачивается в
  `NAME_DECL`; прочие идентификаторы — в `NAME_REF`.
- **PSI-узлы:** `LamNameDecl : ASTWrapperPsiElement, PsiNamedElement`
  (`getName`/`setName`/`getNameIdentifier`; `setName` заменяет дочерний
  `IDENTIFIER`); `LamNameRef : ASTWrapperPsiElement`, `getReference()` →
  `LamNameReference : PsiReferenceBase<LamNameRef>`, `resolve()` ищет одноимённый
  `LamNameDecl` в файле (переиспользуя `LamSymbolScanner` по PSI, с кэшом на файл).
- **Валидатор:** `LamNamesValidator : NamesValidator` (`isKeyword` — из
  `LamTokenTypes.KEYWORDS`, `isIdentifier` — по правилу лексемы Lam).
- **Область — файл** (кросс-файловость от 0038). `getUseScope` не расширяется.

### 0067-03 — антидивергенция + регресс + арбитраж

- **`LamPsiCorpusTest`** (`BasePlatformTestCase`): по каждому `.lam` из
  `examples/**` и `grammar/tests/data/**` — (а) конкатенация текстов листьев PSI
  == исходник; (б) нет `PsiErrorElement`, где оракул = `OK`. Оракул —
  `src/test/resources/psi-oracle.txt` (строки `путь = OK|PARSE_ERR`), порождается
  скриптом прогоном `lamc`; отсутствие/рассинхрон валит тест. При Option B тест
  почти тривиален (структура не разбирается), но стоит сторожем на будущее.
- **Арбитраж PSI↔LSP4IJ** (0040-05): rename/ссылки/find usages — за PSI даже при
  живой LSP-сессии; навигация к декларации и диагностика уступают LSP. Проверка —
  визуальная (`runIde`), в остаточных пунктах.

## Критерии приёмки и способ проверки

| # | Критерий | Способ проверки |
|---|---|---|
| A1 (R5.1) | ссылка резолвится в файл во всех 4 формах | `LamImportPsiReferenceTest`, `reference.resolve()` |
| A2 (R5.2) | rename файла обновляет путь | `myFixture.renameElement(psiFile, "T.lam")` → текст |
| A3 (R5.3) | битый путь = null, formula-строка без ссылки | тест resolve==null / getReferenceAtCaretPosition==null |
| A4 (R3.1) | rename имени во всех видах | `LamRenameTest.renameElementAtCaret` × виды |
| A5 (R3.2) | комментарии/строки не задеты | тест-эталон после rename |
| A6 (R3.3) | Undo байт-в-байт; конфликт; ключевое слово | `LamRenameTest` + `NamesValidator` |
| A7 (Анти-R) | round-trip + оракул | `LamPsiCorpusTest` зелёный |
| A8 (регресс) | 47 тестов 0022/0023 без правок | `./gradlew --offline test` |
| A9 | Plugin Verifier Compatible | `./gradlew verifyPlugin` |

## Особенности по обратной функциональности

Аддитивно (правило 11). Форма дерева меняется (композиты над отдельными
токенами), но наблюдаемое поведение 0022/0023 сохраняется: `findElementAt` даёт
лист прежнего типа; `LamImports`/`LamGotoDeclarationHandler`/`LamSymbolScanner`
переносятся без изменения контракта. Ожидания 47 тестов **не** правятся — правка
ожиданий = слом совместимости.

## Риски и зависимости

- **Регресс `prevSibling`-цепочек** при оборачивании — снижается переносом
  `isImportPathElement` на родителя и прогоном 47 тестов на каждом шаге.
- **Молчаливое расхождение второй грамматики** (0025) — снижается узостью
  дублирования (только `kw <Id>` и путь `import`) + `LamPsiCorpusTest`.
- **`FileReference` rename-on-move на композите одного листа** — проверяется
  тестом A2; если штатный `FileReferenceSet` не редактирует текст композита,
  fallback — `handleElementRename` вручную на `LamImportPath`.
- **Проверяемость визуального** (диалог rename, арбитраж) — остаётся в `runIde`,
  как A11 у 0022/0023; плагин не в CI.
