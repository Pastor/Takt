# Задача 0153-02: `references` и `rename` по рабочей области

> Фича: [../features/0153-lsp-workspace-index.md](../features/0153-lsp-workspace-index.md) · ADR: [../adr/0153-lsp-workspace-index.md](../adr/0153-lsp-workspace-index.md) · анализ: [../analyze/0153-lsp-workspace-index.md](../analyze/0153-lsp-workspace-index.md)

## Что было

`references_at(source, …)` возвращал диапазоны **одного** текста, а на
импортированном имени — `None` (замер анализа: `meas` в `pid_heater.takt`
встречается пятнадцать раз, ответ был пуст). `rename_at` отказывал причинами
`ForeignDeclaration` и `ModelName`.

## Что сделано

- **`references_in_workspace`** отдаёт `Vec<FileReference>` — путь файла плюс
  диапазон в **его** координатах (перевод смещения по чужому тексту дал бы
  верное на вид, но неверное место).
- **`prepare_rename_in_workspace` / `rename_in_workspace`** — правки по всем
  файлам области, сгруппированные по путям.
- **Три новые причины отказа**: `UnparsableConsumer` (файл области, где имя
  встречается, не разбирается), `AmbiguousImport` (имя объявлено несколькими
  подключёнными файлами), `NameTaken` (новое имя занято в затрагиваемом файле).
- **Отказ `ForeignDeclaration` сохранён** для случая, когда объявления не
  нашлось нигде в области: так выглядит имя, введённое `import "файл";` у
  импортёра, и правка одного вхождения оторвала бы его от имени файла.
- Однофайловые `references_at`/`rename_at` остались публичными — их зовут
  потребители без области; политика отказа при этом **одна** (`check_resolution`
  и `validate_new_name`), а не по копии на вход.

## Проверки

Тринадцать тестов набора `lsp_workspace_tests` (перечислены в тест-плане).
Существующие `lsp_references_tests` и `lsp_rename_tests` не правились —
однофайловые ответы прежние.
