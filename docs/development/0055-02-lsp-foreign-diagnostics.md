# Задача 0055-02: Импорты и чужие диагностики в LSP

> Фича: [../features/0055-lsp-multifile.md](../features/0055-lsp-multifile.md) · ADR: [../adr/0055-lsp-multifile.md](../adr/0055-lsp-multifile.md) · анализ: [../analyze/0055-lsp-multifile.md](../analyze/0055-lsp-multifile.md)

## Что было

- `collect_diagnostics(source)` (`lsp.rs:157`) звала `construct_model(&ast, None,
  &[])` — с пустыми путями поиска: `import` в редакторе **всегда** был ошибкой.
- `grammar_diagnostic_to_lsp` (`lsp.rs:224`) отбрасывала `file_no`
  (`Location::Source(_, start, end)`) и мапила смещение в текст **текущего**
  документа — подсветка ложилась не туда.

## Что сделано

> **Готово.**

1. **`collect_diagnostics_at(path, source, search_paths)`** — знает путь
   документа, поэтому каталог документа работает как неявный путь поиска (задача
   [0055-01](0055-01-implicit-import-path.md)). `collect_diagnostics(source)`
   осталась обёрткой (`path = ""` → неявного пути нет) — прежние вызовы и тесты
   не тронуты.

2. **`diagnostic_to_lsp`** различает свой файл и чужой по `file_no`:
   - `file_no == 0` → как прежде, на своём месте;
   - иначе → якорь на строке `import` (заметка с `file_no == 0`), текст —
     `в файле lib.lam:2:18: …` (позицию строит общий слой
     `grammar::diagnostics::position_prefix`, фича 0054).

3. **`lam_lsp.rs`**: `uri_to_path` + `percent_decode` — путь документа из URI
   (`file://…`). Обрабатывается только схема `file:`: у прочих (`untitled:`)
   пути нет, и угадывать каталог честнее не пытаться.

## Ловушка стадии

Тесты LSP живут под `#[cfg(feature = "lsp")]`, и **обычная `cargo build` их не
видит**: первая редакция добавила тесты без гейта — сборка проходила, а
`precheck.sh` (`--all-features --all-targets`) падал с `cannot find lsp in
grammar`. Та же ловушка, что в фиче 0053 с литералами `Diagnostic` под `lsp`.
