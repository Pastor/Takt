---
okf_version: 0.2
format: 2
type: Task
key: TAKT-633
list: ТАКТ
title: "[0023-02] навигация к константам enum и к портам в выражениях `port.N`"
status: done
priority: none
author: pastor
model: gpt-6
links:
  part:
  - TAKT-23
generated:
  by: agent/gpt-6
  for: human:pastor
  at: 2026-11-11T07:09:37Z
published: 2026-11-11T07:09:37Z
created: 2026-11-11T07:09:37Z
updated: 2026-11-11T07:09:37Z
anal:
  project: 22
  id: 1687
  version: 1
checksum: sha256:015b6d8d764e2d642399ca38829e9af0ba7161e01e41b091f955cf69078cd915
---

Источник: [docs/fixes/0023-02-enum-variants-and-port-members.md](https://github.com/Pastor/Takt/blob/3c154a13d979b18312a1250ebfbe3db2fa5247d3/docs/fixes/0023-02-enum-variants-and-port-members.md)

# Исправление: навигация к константам enum и к портам в выражениях `port.N`

> Фича: [../features/0023-intellij-navigation-include.md](../features/0023-intellij-navigation-include.md) · разработка: [../features/0023-intellij-navigation-include.md#разработка](../features/0023-intellij-navigation-include.md#разработка) · связан с исправлением [проверка совместимости с новыми IDE (verifyPlugin) + валидность дескриптора](0023-01-verifyplugin-descriptor.md)

## Проблема

По запросу заказчика (2026-07-13) в готовой фиче не работали два сценария
`Go to Declaration` (`Ctrl/⌘+Click`, `Ctrl/⌘+B`):

1. **Константы `enum` не резолвились.** `LamSymbolScanner` индексировал только
   *имя* перечисления (`enum Floor { … }` → `Floor`), но не его константы-варианты.
   Переход от использования варианта (например `var a: Action := Closing;` →
   `enum Action { Idle = 670, Closing }`) не находил цели.
2. **Порты не резолвились из выражений `port.N`.** Порты объявляются ключевыми
   словами `in`/`out`/`inout` (`in sensors_cab: u8 := …;`), которых не было в списке
   декларирующих ключевых слов сканера. Поэтому даже прямое использование порта не
   вело к объявлению; заказчик наблюдал это на доступе к биту `sensors_cab.0`
   (`BitAccess`, `examples/elevator.lam:139`) и связал с частью `.0`. Реальная
   причина — отсутствие порта в индексе деклараций (сам `sensors_cab` под кареткой —
   штатный `IDENTIFIER`, `.0` навигации не мешает).

## Решение

`extensions/intellij-lam/.../navigation/LamSymbolScanner.kt`:

- **Порты** `in`/`out`/`inout` добавлены в `SIMPLE_DECL_KEYWORDS` — форма
  `kw <Id>` совпадает с прочими декларациями (`in name: Type := …;`). Это безопасно:
  по грамматике `in`/`out`/`inout` вводятся только как порты (цикл `for` в Lam —
  Си-стиль `for a; b; c { }`, `in` там не используется).
- **Константы `enum`** — выделенная ветка `scanEnum` (по образцу `scanImport`):
  записывает имя перечисления и каждый вариант в `{ … }`. Вариант — идентификатор в
  начале элемента (после `{` или `,`); значение `= n` — это `NUMBER`, за вариант не
  принимается. Ключевое слово `enum` вынесено из `SIMPLE_DECL_KEYWORDS` в эту ветку.

`GotoDeclarationHandler` и резолвинг не менялись — им достаточно расширенного
индекса. Ограничение Option A (без областей видимости) сохраняется: одноимённые
символы дают несколько целей (штатное поведение платформы).

## Проверка

`./gradlew --offline clean buildPlugin test` → **BUILD SUCCESSFUL, 53/53 теста**
(было 47 + 6 новых: сканер 4 — варианты enum ×2, порты ×2; переход 2 — `port.N`
и использование варианта enum). Версия плагина `0.2.1 → 0.3.0` (новая
функциональность, аддитивно; `grammar`/`simulation`, синтаксис/семантика и версия
языка не тронуты — свод неприменим).
