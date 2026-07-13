# Задача 0023-01: Плагин IntelliJ IDEA — навигация к декларации и include

> Фича: [../features/0023-intellij-navigation-include.md](../features/0023-intellij-navigation-include.md) · ADR: [../adr/0023-intellij-navigation-include.md](../adr/0023-intellij-navigation-include.md) · анализ: [../analyze/0023-intellij-navigation-include.md](../analyze/0023-intellij-navigation-include.md)

## Что было

Плагин 0022 — чисто лексический: `LamLexer` + `SyntaxHighlighter` + эргономика.
PSI-дерева и навигации не было; `Ctrl+Click`/`Ctrl+B` по имени и по пути `import`
ничего не делали.

## Что сделано

Реализован **Option A** ADR (лёгкий путь) поверх существующего лексера, одной
задачей `0023-01`. Затронут только подпроект `extensions/intellij-lam` (Kotlin);
крейты `grammar`/`simulation`, синтаксис/семантика и версия языка — **не тронуты**
(правило 11: для них «н/п», аддитивная фича редакторной оснастки; правило 22
неприменим). Версия плагина `0.1.1 → 0.2.0`.

### Новые компоненты

- **Плоский PSI-разбор** (нужен, чтобы под кареткой были реальные `PsiElement`):
  - `parser/LamParserDefinition.kt` — `ParserDefinition` (лексер `LamLexer`,
    комментарии/строки/пробелы через `psi/LamTokenSets.kt`, корневой `FILE`).
  - `parser/LamParser.kt` — принимает все токены листьями под корнем (структура
    не строится; разбор всегда успешен — подсветку 0022 не ломает).
  - `psi/LamFile.kt` — `PsiFileBase` языка Lam.
- **Индекс деклараций** — `navigation/LamSymbolScanner.kt`: поверх `LamLexer`
  собирает `имя → диапазон` для `model`/`state`/`start`/`type`/`enum`/`cond`/
  `var`/`const`/`fn` (форма `kw <Id>`) и локальных имён из `import` (алиасы после
  `as`, «голые» имена в `{ … }`). Источник истины — `grammar.lalrpop`.
- **Переход к декларации** — `navigation/LamGotoDeclarationHandler.kt`
  (`GotoDeclarationHandler`): по идентификатору под кареткой отдаёт листовой
  элемент одноимённого объявления; на самой декларации/не-идентификаторе молчит.
- **Навигация по `import`** — `navigation/LamImports.kt`: распознаёт строку-путь
  (ближайший слева значимый токен — `import`/`from`), резолвит файл относительно
  каталога текущего файла и корней контента проекта. Тот же
  `LamGotoDeclarationHandler` возвращает найденный файл как цель.

### Отклонение от формулировки ADR (зафиксировано)

ADR (Option A) допускал `PsiReferenceContributor` для путей `import`. На практике
ссылки контрибьютора **не привязываются** к листовым токенам (`LeafPsiElement` не
является `ContributedReferenceHost`). Поэтому навигация по `import` реализована
через `GotoDeclarationHandler` (срабатывает на любом листе) — пользовательский
`Ctrl/⌘+Click`/`B` работает идентично. Полноценные `PsiReference` (для
find-usages/rename пути) остаются за будущей PSI-фичей (бэклог).

## Проверки

- Юнит-/интеграционные тесты (`BasePlatformTestCase`): `LamSymbolScannerTest` (9),
  `LamGotoDeclarationTest` (6), `LamImportReferenceTest` (5) — **20 новых**.
- Команда: `./gradlew --offline clean buildPlugin test` → **BUILD SUCCESSFUL**,
  **47/47 тестов зелёные** (27 из 0022 + 20 новых). Собран `intellij-lam-0.2.0.zip`.
- Соответствие анализу: R1–R5 и критерии A1–A6 покрыты (см. тест-план/отчёт).
