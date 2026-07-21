# Задача 0072-01: LSP не читает initializationOptions (пути поиска импортов)

> Фича: [../features/0072-lsp-initialization-options.md](../features/0072-lsp-initialization-options.md) · ADR: [../adr/0072-lsp-initialization-options.md](../adr/0072-lsp-initialization-options.md) · анализ: [../analyze/0072-lsp-initialization-options.md](../analyze/0072-lsp-initialization-options.md)

## Что было

`bin/lam_lsp.rs` при инициализации брал `(init_id, _init_params)` и параметры
клиента **игнорировал**. Во все точки, разрешающие импорты, сервер передавал
пустой список путей поиска:

- `collect_diagnostics_at(&path, &text, &[])` — `didOpen`/`didChange`;
- `goto_declaration_at(&path, text, position, &[])` — переход к декларации.

Импорт из общей библиотеки вне каталога документа в редакторе не находился, хотя
`lamc -I lib` его собирает (ADR 0072, боль из 0055 A3).

## Что сделано

**Библиотека (`grammar`) — новый модуль `src/lsp/init_options.rs`:**
- `pub fn search_paths_from_options(options: Option<&Value>, root: Option<&str>)
  -> Vec<String>` — читает массив строк по ключу `searchPaths`; относительный
  путь разрешает от корня рабочей области `root`, абсолютный оставляет как есть;
  битые записи (не массив, элемент не строка) молча пропускает. Реэкспорт в
  `lsp/mod.rs` (`pub use init_options::search_paths_from_options`). Модуль
  подключён (`mod init_options;`) — `mod.rs` не рос (633 строки, лимит 1000).

**Бинарник (`grammar/src/bin/lam_lsp.rs`):**
- `initialize_start()` теперь берёт `(init_id, init_params)`; новая
  `search_paths_from_init(&Value)` десериализует `InitializeParams`, берёт
  `root_uri` (через `uri_to_path`) и `initialization_options`, зовёт
  библиотечную функцию. Нечитаемые параметры → пустой список (не падаем на старте).
- `ServerState` получил поле `search_paths: Vec<String>`; `ServerState::new`,
  `main_loop` принимают его от `main`.
- Оба потребителя (`goto_declaration_at`, обе ветки `collect_diagnostics_at`)
  получают `&state.search_paths` вместо `&[]`. Остальные хендлеры
  (completion/hover/formatting/symbols/semanticTokens) импортов не разрешают —
  не тронуты (R4).

**Затронутые стеки (правило 11):**
- `grammar` (LSP-сервер + библиотека) — **основная работа**.
- `simulation` — **н/п** (симулятор `search_paths` берёт из своего CLI, LSP не
  использует).
- Язык (синтаксис/семантика) — **н/п**: интеграция редактора, не язык. Версия
  языка `0.3.0` без изменений. Крейт `grammar` `0.7.0 → 0.8.0` (аддитивный
  публичный API).

## Проверки

```sh
cargo build --features lsp --bin lam-lsp                            # ок
cargo test --features lsp --lib init_options -- --test-threads=1    # 8/8
cargo test --features lsp --test lsp_init_options_tests -- --test-threads=1 # 4/4
./scripts/precheck.sh                                               # зелёный
```

Соответствие условиям анализа: R1–R7 покрыты (см. тест-план
[../tests/0072-lsp-initialization-options.md](../tests/0072-lsp-initialization-options.md)
и отчёт [../reports/0072-lsp-initialization-options.md](../reports/0072-lsp-initialization-options.md)).
