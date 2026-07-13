# Задача 0022-03: Настройка цветов, эргономика редактора, документация

> Фича: [../features/0022-intellij-syntax-highlight.md](../features/0022-intellij-syntax-highlight.md) · ADR: [../adr/0022-intellij-syntax-highlight.md](../adr/0022-intellij-syntax-highlight.md) · анализ: [../analyze/0022-intellij-syntax-highlight.md](../analyze/0022-intellij-syntax-highlight.md)

**Статус:** ЗАПЛАНИРОВАНО (разработка не начата).

## Что было

После 0022-02 работает лексическая подсветка, но нет страницы настройки цветов и
базовой эргономики (парные скобки, комментирование), а также документации по
сборке/установке плагина.

## Что план (объём задачи)

Дооснащение и документация (R4, R5, R6):

- **`LamColorSettingsPage`** (`ColorSettingsPage`): группы атрибутов (Keyword,
  Operator, Number, String, Line comment, Doc comment, Identifier, Braces,
  Parentheses, Brackets, Bad character), демонстрационный Lam-код (использовать
  фрагмент из README/`examples/`), маппинг дескрипторов на `TextAttributesKey`
  из 0022-02.
- **`LamCommenter`** (`Commenter`): строчный `//` (Ctrl+/), блочного нет — задать
  `getBlockCommentPrefix()=null`; учесть `///` как строчный.
- **`LamPairedBraceMatcher`** (`PairedBraceMatcher`): `{}`, `()`, `[]`.
- Регистрация всех точек расширения в `plugin.xml`.
- **Документация:** `extensions/intellij-lam/README.md` (сборка `gradle
  buildPlugin`, установка из диска, `runIde`, диапазон версий IDE); дополнить
  корневой `README.md` (правило 15) разделом об оснастке IDE (Zed + IntelliJ);
  `CHANGES.md`.
- `verifyPlugin` в приёмке (R6).

Функциональность по стекам (правило 11): язык/компилятор — **н/п**.

## Проверки (план)

- `runIde`: Settings → Editor → Color Scheme → **Lam** — демо-текст раскрашен,
  цвета переопределяются (A5).
- Ctrl+/ комментирует строкой `//`; парные скобки подсвечиваются (A6).
- `gradle buildPlugin verifyPlugin test` — зелёные (A7).
- `git diff` не трогает `grammar/`/`simulation/`, версия языка не изменена (A8).
- Соответствие R4/R5/R6/R7 и критериям приёмки A5–A8 (анализ).
