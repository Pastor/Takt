# Тест-план фичи 0072: LSP не читает initializationOptions (пути поиска импортов)

> Фича: [../features/0072-lsp-initialization-options.md](../features/0072-lsp-initialization-options.md) · ADR: [../adr/0072-lsp-initialization-options.md](../adr/0072-lsp-initialization-options.md) · анализ: [../analyze/0072-lsp-initialization-options.md](../analyze/0072-lsp-initialization-options.md)

## Область и цель

Проверяем, что `lam-lsp` читает `initializationOptions.searchPaths` и разрешает
импорты как `lamc -I` (R1–R7, A1–A7 анализа). Разбор настроек — юнит-тесты
библиотечной функции; сквозное разрешение импортов — интеграционные тесты
потребителей ядра, которые зовёт бинарник. Обвязка бинарника (`lam_lsp.rs`)
проверяется инспекцией (юнит-тестами бинарник не покрыть; риск снят в анализе).

## Проверки (условие → ожидаемый результат)

| # | Проверка | Предусловие | Ожидаемый результат | Ссылка на R/A |
|---|---|---|---|---|
| T1 | чтение массива `searchPaths` | `{"searchPaths":["/abs/lib","/abs/shared"]}` | вернулись оба пути в порядке | R1 / A1 |
| T2 | относительный от корня, абсолютный как есть | `["lib","/abs/shared"]`, root=`/work/project` | `/work/project/lib`, `/abs/shared` | R2 / A2 |
| T3 | относительный без корня — как есть | `["lib"]`, root=`None` | `["lib"]` | R2 / A2 |
| T4 | порядок сохраняется | `["/z","/a","/m"]` | `["/z","/a","/m"]` | R1 / A1 |
| T5 | нет `initializationOptions` | `None` | `[]` | R3 / A3 |
| T6 | нет ключа `searchPaths` | `{"other":1}` | `[]` | R3 / A3 |
| T7 | `searchPaths` не массив | `{"searchPaths":"lib"}` | `[]` | R3 / A3 |
| T8 | элемент не строка | `["/lib",42,null,"/shared"]` | `["/lib","/shared"]` (пропуск) | R3 / A3 |
| T9 | импорт вне каталога документа без путей | `main.lam` в `proj/`, библиотека в `lib/`, пути `&[]` | диагностика «не найден» | R4/R5 / A4 |
| T10 | тот же импорт с `searchPaths=[lib]` | те же файлы, пути `[lib]` | диагностик нет | R4 / A4 |
| T11 | goto в файл из `searchPaths` | курсор на `Shared`, пути `[lib]` | открыт `lsp72/lib/shared.lam` | R4 / A5 |
| T12 | goto без путей — некуда | курсор на `Shared`, пути `&[]` | `None` (сторож паритета) | R5 / A5/A6 |
| T13 | пустой список ≡ прежнее поведение | тесты 0055/0056 | зелены без правок | R5 / A6 |
| T14 | контракт в README | — | `searchPaths` описан, ⚠-оговорка снята | R7 / A7 |

## Разбивка проверок по функциональности

- **Разбор настроек** (`grammar::lsp::init_options`): T1–T8 — ✅ (8 юнит-тестов,
  lib).
- **Диагностика с путями** (`collect_diagnostics_at`): T9–T10 — ✅
  (`lsp_init_options_tests::lsp72_init_options`).
- **Переход к декларации с путями** (`goto_declaration_at`): T11–T12 — ✅
  (`lsp_init_options_tests::lsp72_init_options`).
- **Регрессия 0055/0056** (пустой список): T13 — ✅ (существующие тесты зелены).
- **Контракт README**: T14 — ✅ (инспекция + `precheck.sh` проверка ссылок).

<!-- Легенда: ✅ пройдено · ❌ провалено · ⬜ не проверялось · — не применимо -->

## Тестовые данные и окружение

- Юнит-фикстуры — литералы `serde_json::json!` в `src/lsp/init_options.rs`.
- Интеграционные фикстуры — `grammar/tests/data/lsp72/`: `proj/main.lam`
  (документ, импортирует `shared.lam`), `lib/shared.lam` (библиотека в **соседнем**
  каталоге — неявный путь 0055 её не находит, только `searchPaths`).
- Окружение: `cargo test --features lsp -- --test-threads=1`; полный гейт —
  `./scripts/precheck.sh`. Ручная проверка в редакторе не автоматизируется
  (Actions заблокирован по биллингу, прецедент 0090).
