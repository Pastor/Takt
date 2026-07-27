# Фича 0131: LSP: `definition`, `references` и `rename`

- **Номер:** 0131
- **Статус:** АРХИТЕКТУРА
- **Зависит от:** нет
- **Tier:** 2
- **Связанные issue (анализ):** кандидат блока 2 `FEATURES.md` (аудит 2026-07-27, чтение `bin/takt_lsp.rs`)

## Ссылки на артефакты (жизненный цикл, правило 17)

| Стадия | Артефакт |
|---|---|
| Архитектура (ADR) | [`docs/adr/0131-lsp-definition-references-rename.md`](../adr/0131-lsp-definition-references-rename.md) |
| Анализ | не заведён (стадия 3) |
| Разработка | [`docs/development/`](../development/README.md) (задачи `0131-YY-*`) |
| Тест-план | [`docs/tests/README.md`](../tests/README.md) |
| Отчёт о тестировании | [`docs/reports/README.md`](../reports/README.md) |
| Исправления | [`docs/fixes/`](../fixes/README.md) (при необходимости `0131-YY-*`) |

## Краткое описание

Языковой сервер не отдаёт `textDocument/definition`, `references` и `rename`.
В редакторах, где «Go to Definition» (F12) идёт через `definitionProvider`
(например VS Code), переход **не работает**: реализован только `declaration`.

> Фича зарегистрирована из бэклога кандидатов (отобрана заказчиком 2026-07-27); далее проходит жизненный цикл по правилу 17.

## Что известно на входе

- В `ServerCapabilities` объявлены только `completion`, `hover`, `declaration`,
  `documentSymbol`, `semanticTokens`, `formatting`.
- Основание для остального уже готово: `SemanticIndex` адресует пару
  `(file_no, offset)` и знает вид узла (фича [0056](0056-lsp-goto-exact-file.md)).
- Плагин IntelliJ делает rename **сам**, собственным PSI (фича
  [0125](0125-intellij-takt-lsp-tooling.md)), — то есть одна и та же работа
  дублируется вне сервера.

## Направление решения (наметка, не решение)

По возрастанию цены:

1. `definition` как алиас `goto_declaration` — дёшево, чинит F12;
2. `references` поверх `SemanticIndex`;
3. `rename` — нужен обход всех файлов рабочей области и согласование с
   `searchPaths` (фича [0072](0072-lsp-initialization-options.md)).

⚠️ Тесты LSP — под `#[cfg(feature = "lsp")]`: обычная `cargo build` их не видит,
ловит только `precheck.sh`.

⚠️ Окончательный выбор — за стадиями **архитектуры** (ADR) и **анализа**: раздел
выше фиксирует то, что уже проверено, а не предрешает реализацию.

## Документирование (правило 24)

**Не требуется.** Возможности редактора; язык не меняется.
