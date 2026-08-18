# Задача 0154-01: Снятие PSI-переименования и сторожа

> Фича: [../features/0154-intellij-server-rename.md](../features/0154-intellij-server-rename.md) · ADR: [../adr/0154-intellij-server-rename.md](../adr/0154-intellij-server-rename.md) · анализ: [../analyze/0154-intellij-server-rename.md](../analyze/0154-intellij-server-rename.md)

## Что было

`TaktNameDecl` реализовал `PsiNameIdentifierOwner`, `TaktNameReference` —
`handleElementRename`. Из-за этого IDEA считала доступным штатный
`PsiElementRenameHandler`, а `LSPRenameHandler` из LSP4IJ **уступал** (его
предикат требует отсутствия других доступных обработчиков). Серверный `rename`
— с областями видимости и рабочей областью (0131, 0153) — не включался никогда.

## Что сделано

- **`TaktNameDecl` больше не `PsiNameIdentifierOwner`**; `setName`,
  `getNameIdentifier`, `setIdentifierText` и `handleElementRename` сняты.
  Узлы имён и `PsiReference` **сохранены** — на них держится навигация без
  сервера (тихая деградация 0038).
- **`TaktRenameTest` переписан** (9 тестов PSI-переименования → 3 сторожа
  нового устройства + сохранённый тест валидатора имён).
- Комментарии, обещавшие нативный rename, приведены в соответствие:
  `plugin.xml`, `TaktElementTypes.kt`, `TaktParser.kt`, `README.md` плагина.

## Проверки

```sh
cd extensions/intellij-takt && ./gradlew --offline test   # 84 теста, зелено
```

**Зонд, изменивший замысел сторожа.** Первая редакция проверяла, что
`RenameHandlerRegistry.getRenameHandlers` пуст, — ровно предикат LSP4IJ. Тест
краснел. Зонд на `probe.txt` (файл, где PSI плагина нет вовсе) показал причину:
`PsiElementRenameHandler` доступен и там — он предлагает переименовать **файл**,
а не символ. В синтетическом `DataContext` теста этот обработчик присутствует
всегда, поэтому такой сторож проверял бы наличие файла, а не наше устройство.
Сторож перестроен на `PsiElementRenameHandler.canRename` **нашего узла**.

**Мутация** (проверка, что сторожа не декоративны): возврат
`PsiNameIdentifierOwner` в `TaktNameDecl` валит `testDeclarationIsNotRenamableNatively`
и `testDeclarationIsNotNamedElement` — 2 провала из 84.
