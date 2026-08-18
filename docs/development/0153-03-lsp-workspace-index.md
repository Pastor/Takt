# Задача 0153-03: Бинарник, контекст и документ

> Фича: [../features/0153-lsp-workspace-index.md](../features/0153-lsp-workspace-index.md) · ADR: [../adr/0153-lsp-workspace-index.md](../adr/0153-lsp-workspace-index.md) · анализ: [../analyze/0153-lsp-workspace-index.md](../analyze/0153-lsp-workspace-index.md)

## Что было

`takt_lsp.rs` знал только `searchPaths` и словарь открытых документов; ответ
`rename` клал правки под **один** URI.

## Что сделано

- **Корни рабочей области** извлекаются из `InitializeParams`:
  `workspaceFolders`, затем `root_uri` (устаревший, но именно его шлют живые
  клиенты). Если клиент не дал ни того, ни другого — область сводится к
  каталогу открытого документа: иначе её не было бы вовсе.
- **`ServerState::overlay`** отдаёт слою тексты открытых документов, а
  `roots_for` — корни запроса.
- **`WorkspaceEdit` с несколькими URI**: `path_to_uri` строит URI по пути.
  Кодируются только пробел и `%` — полное процентное кодирование дало бы URI,
  не совпадающий с присланным клиентом, и редактор счёл бы это другим файлом.
- Документ (`book/src/17-tools`) описывает охват и границу; живой контекст и
  `CHANGES.md` — то же; версия крейта `takt-lang` `0.37.0` → `0.38.0`.

## Проверки

```sh
cargo build --features lsp --bin takt-lsp
cargo test --all-features
./scripts/precheck.sh
```
