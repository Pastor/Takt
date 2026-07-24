# Задача 0100-02: Переименование бинарников (lamc→taktc, lam-lsp→takt-lsp, simulation→takt-sim)

> Фича: [../features/0100-language-rename-takt.md](../features/0100-language-rename-takt.md) · ADR: [../adr/0100-language-rename-takt.md](../adr/0100-language-rename-takt.md) · анализ: [../analyze/0100-language-rename-takt.md](../analyze/0100-language-rename-takt.md)

## Что было

Бинарники `lamc` (`src/bin/lamc.rs`), `lam-lsp` (`src/bin/lam_lsp.rs`) в
`takt-lang`, `simulation` (`src/bin/simulation.rs`) в `takt-sim`. Имена
фигурировали в `[[bin]]`-блоках, `env!("CARGO_BIN_EXE_…")` тестов, usage-строках
CLI, LSP-идентичности (`ServerInfo.name`, `source` диагностик), лог-префиксах,
скриптах и реестре размера.

## Что сделано (слой 2 ренейма)

- **Файлы бинарников** (`git mv`): `lamc.rs`→`taktc.rs`, `lam_lsp.rs`→`takt_lsp.rs`,
  `simulation.rs`→`takt_sim.rs`.
- **Манифесты `[[bin]]`:** `taktc`/`takt-lsp` (в `takt-lang`), `takt-sim` (в
  `takt-sim`); пути файлов обновлены.
- ⚠️ **`env!("CARGO_BIN_EXE_…")`** — Cargo формирует имя переменной из **имени
  бинарника ДОСЛОВНО** (дефис **не** заменяется на подчёркивание, вопреки первой
  догадке): `CARGO_BIN_EXE_lamc`→`CARGO_BIN_EXE_taktc` (3 теста `takt-lang/tests`),
  `CARGO_BIN_EXE_simulation`→**`CARGO_BIN_EXE_takt-sim`** (с дефисом!) в
  `takt-sim/tests/diagnostics_tests.rs`. Первый прогон с `_takt_sim` дал
  `environment variable not defined at compile time` — Cargo определяет её как
  `CARGO_BIN_EXE_takt-sim`. Без верного имени тест не компилируется (E-101).
- ⚠️ **Функциональная ссылка на файл:** `tests/lsp_goto_tests.rs:220` читает
  `src/bin/lam_lsp.rs` в рантайме → `src/bin/takt_lsp.rs` (иначе тест падает
  «файл не найден»).
- **LSP-идентичность:** `ServerInfo.name` и `source` диагностик `"lam-lsp"`→
  `"takt-lsp"` (`bin/takt_lsp.rs`, `lsp/diagnostics.rs`, тест `lsp/mod.rs`),
  лог-префиксы `[lam-lsp]`→`[takt-lsp]`.
- **Usage-строки CLI** и упоминания инструментов в коде/комментариях/тестах:
  сплошь `lamc`→`taktc`, `lam-lsp`→`takt-lsp`, `lam_lsp`→`takt_lsp`; перечисление
  бинарников `(taktc, simulation)`→`(taktc, takt-sim)`.
- **Скрипты:** `precheck.sh` `--bin taktc`/`--bin takt-lsp`; `run_simulations.sh`
  `BINARY=target/debug/takt-sim`, `--bin takt-sim`.
- **Реестр размера:** запись `bin/lamc.rs`→`bin/taktc.rs` (1994).

**Решение по симулятору:** бинарник `simulation`→`takt-sim` (завершение «всех
тулов» из запроса; крейт уже `takt-sim`). ADR явно называл `taktc`/`takt-lsp` —
симулятор переименован по духу решения (не «Lam», но тул проекта).

**Вне объёма (следующие слои):** расширение `.lam`→`.takt` и имя языка `Lam`
в коде (0100-03); README/CLAUDE/docs-проза, плагин IntelliJ (`LamLspBinary` ищет
старый бинарник — 0100-05); подъём версии (0100-06).

## Проверки

- `cargo build --bin taktc` / `--features lsp --bin takt-lsp` / `--bin takt-sim`
  — успешно.
- `cargo test --test lsp_goto_tests` (11) и `--test cli_warnings_tests` (4) —
  passed (`CARGO_BIN_EXE_taktc` и чтение файла работают).
- `./scripts/precheck.sh` — зелёный (размер на переехавшем пути `taktc.rs`).
