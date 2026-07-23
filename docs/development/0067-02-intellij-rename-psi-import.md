# Задача 0067-02: R3 — нативный rename имён Lam

> Фича: [../features/0067-intellij-rename-psi-import.md](../features/0067-intellij-rename-psi-import.md) · ADR: [../adr/0067-intellij-rename-psi-import.md](../adr/0067-intellij-rename-psi-import.md) · анализ: [../analyze/0067-intellij-rename-psi-import.md](../analyze/0067-intellij-rename-psi-import.md)

## Что было

Rename имён Lam платформой не поддерживался: PSI плоский, декларации не были
`PsiNamedElement`, использования не несли `PsiReference`. Токенная эвристика
`LamSymbolScanner` умела лишь Go to Declaration (0023).

## Что сделано

**Option B ADR 0067** для R3 (хирургическое оборачивание идентификаторов):

- **`LamElementTypes.NAME_DECL` / `NAME_REF`** — композитные типы над одиночным
  листом `IDENTIFIER`.
- **`LamParser`** различает декларацию и использование **эвристикой
  `LamSymbolScanner`** (единый источник форм `kw <Id>`/`Import`/`enum`): множество
  стартовых смещений деклараций считается один раз по `builder.originalText`;
  идентификатор на этом смещении → `NAME_DECL`, иначе → `NAME_REF`. Грамматика
  выражений/типов **не** дублируется.
- **`LamNameElements.kt`:** база `LamIdentifierElement` (замена текста листа через
  `LeafElement.replaceWithText`); `LamNameDecl : PsiNameIdentifierOwner`
  (`getName`/`setName`/`getNameIdentifier`); `LamNameRef` — носитель `getReference()`.
- **`LamNameReference : PsiReferenceBase<LamNameRef>`** (soft) — `resolve()` ищет
  первую одноимённую декларацию в файле (тем же `LamSymbolScanner`), возвращает её
  `LamNameDecl`; `handleElementRename` меняет текст листа напрямую (без манипулятора).
  Мягкость: неразрешённое имя (кросс-файловое, имя состояния, встроенное) НЕ
  подсвечивается ошибкой.
- **`LamNamesValidator : NamesValidator`** — отвергает ключевые слова Lam
  (набор из `LamTokenTypes.KEYWORDS`, сторож `LamKeywordSyncTest`).
- **`LamParserDefinition.createElement`** и **`plugin.xml`** (`lang.namesValidator`).

Область — файл; кросс-файловый rename не делается (принят от 0038). Go to
Declaration (0023) сохранён: `findElementAt` отдаёт лист прежнего типа `IDENTIFIER`.
Стеки `grammar`/`simulation` — **н/п** (редакторная фича).

## Проверки

`cd extensions/intellij-lam && ./gradlew --offline test` — зелёный. Новый
`LamRenameTest` (11 тестов), проверено (R3):

- **R3.1** — `renameElementAtCaret` переименовывает декларацию и использования для
  видов: `model` (из декларации и из использования), `type`, `var`, вариант
  `enum`, порт, `fn`, алиас `import` (путь `import` при этом не задет).
- **R3.2** — одноимённые подстроки в комментарии `//` и строке `"…"` **не**
  изменяются.
- **R3.3** — `LamNamesValidator` отвергает `model`/`state`/`address`, принимает
  `Producer`/`_x1`, отвергает `1abc`/пустое.
- Undo/конфликт/превью — штатная механика рефакторинга IDEA, визуально (`runIde`).
