# Задача 0067-01: R5 — `PsiReference` для `import` (rename-on-move)

> Фича: [../features/0067-intellij-rename-psi-import.md](../features/0067-intellij-rename-psi-import.md) · ADR: [../adr/0067-intellij-rename-psi-import.md](../adr/0067-intellij-rename-psi-import.md) · анализ: [../analyze/0067-intellij-rename-psi-import.md](../analyze/0067-intellij-rename-psi-import.md)

## Что было

Навигация по `import` работала лишь через `GotoDeclarationHandler` (0023):
Ctrl+Click открывал файл, но настоящей `PsiReference` не было — значит, не было
и **rename-on-move** (перемещение/переименование файла не обновляло путь в
`.lam`). Проба ADR 0067 доказала: ссылка не привязывается к листовому токену
(`LeafPsiElement` не опрашивает `ReferenceProvidersRegistry`) — нужен композит.

## Что сделано

Реализован **Option B ADR 0067** для R5 (хирургическое оборачивание):

- **`LamElementTypes.IMPORT_PATH`** — новый композитный тип узла.
- **`LamParser`** (почти плоский): токен `STRING`, чей предыдущий значимый токен —
  ключевое слово `import`/`from`, оборачивается в `IMPORT_PATH`
  (`mark()`/`advanceLexer()`/`done`). Больше ничего структурного; текст не
  теряется по построению. `PsiBuilder` пропускает пробелы/комментарии, поэтому
  «предыдущий значимый» = предыдущая итерация цикла.
- **`LamImportPath : ASTWrapperPsiElement`** — `getReferences()` строит
  `FileReferenceSet` (start = 1, после кавычки); подкласс переопределяет
  `computeDefaultContexts()` на каталог импортирующего файла (паритет с ядром
  0055 «рядом с импортирующим»).
- **`LamImportPathManipulator : AbstractElementManipulator<LamImportPath>`** —
  нужен `FileReference.rename` (иначе «Cannot find manipulator»); меняет текст
  дочернего листа через `LeafElement.replaceWithText` (создание нового
  `LeafPsiElement` валит ассерт «old indentation must be defined»).
- **`LamParserDefinition.createElement`** — фабрика `LamImportPath` для
  `IMPORT_PATH` (перестала быть заглушкой).
- **`LamImports.isImportPathElement`** переведён с эвристики `prevSibling` на
  структурный признак «родитель — `IMPORT_PATH`» (оборачивание сломало бы
  `prevSibling`-цепочку); контракт сохранён, `GotoDeclarationHandler` цел.
- **`plugin.xml`** — регистрация `lang.elementManipulator`. Отдельный
  `psi.referenceContributor` не нужен (узел несёт ссылку сам).

Стеки: только плагин IntelliJ (Kotlin). `grammar`/`simulation` — **н/п**
(редакторная фича).

## Проверки

`cd extensions/intellij-lam && ./gradlew --offline test` — **зелёный, 69 тестов,
0 падений** (было 62 + 7 новых `LamImportPsiReferenceTest`). Проверено (R5):

- **R5.1** — `resolve()` → `PsiFile` во всех 4 формах `import` (`import "P";`,
  `import "P" as X;`, `import * as X from "P";`, `import { A as C } from "P";`).
- **R5.2** — `myFixture.renameElement(psiFile, "renamed.lam")` → текст содержит
  `import "renamed.lam";` (rename-on-move).
- **R5.3** — битый путь → `resolve() == null`; строка в `formula` ссылки не несёт.
- **Регресс** — 62 теста 0022/0023 зелены без правки ожиданий (форма дерева
  изменилась только для строки-пути `import`).
