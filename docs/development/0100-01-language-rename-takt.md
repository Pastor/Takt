# Задача 0100-01: Крейты и пути импорта (grammar→takt-lang, simulation→takt-sim)

> Фича: [../features/0100-language-rename-takt.md](../features/0100-language-rename-takt.md) · ADR: [../adr/0100-language-rename-takt.md](../adr/0100-language-rename-takt.md) · анализ: [../analyze/0100-language-rename-takt.md](../analyze/0100-language-rename-takt.md)

## Что было

Два крейта в каталогах `grammar/` (пакет `grammar`, бинарники `lamc`/`lam-lsp`) и
`simulation/` (пакет `simulation`, бинарник `simulation`); пути импорта
`grammar::…` (147 вхождений, включая доктесты/бины) и `simulation::…` (в бинарнике
и интеграционных тестах). Скрипты-гейты и реестр размера ссылались на каталоги
`grammar/`/`simulation/` жёстко.

## Что сделано (первый слой ренейма, [ADR 0100](../adr/0100-language-rename-takt.md) Option A)

- **Каталоги:** `git mv grammar takt-lang`, `git mv simulation takt-sim` (476
  переименований).
- **Манифесты:** корневой `Cargo.toml` `members = ["takt-lang", "takt-sim"]`;
  `[package] name` → `takt-lang`/`takt-sim`; зависимость `takt-sim` →
  `takt-lang = { path = "../takt-lang" }`.
- **Пути импорта:** массово `grammar::`→`takt_lang::`, `simulation::`→`takt_sim::`
  во всех `.rs` крейтов (Rust: имя пакета `takt-lang` → `use`-имя `takt_lang`).
- ⚠️ **Ловушка (защищена):** `lib.rs` объявляет **`mod grammar`** —
  LALRPOP-генерируемый модуль парсера (`include!(…/grammar.rs)`), и
  `grammar::SourceUnitParser` (`lib.rs:117`) ссылается на **него**, а не на крейт.
  Единственная такая ссылка **возвращена** после массовой замены (модуль остаётся
  `grammar`, файл `grammar.lalrpop`/`grammar.rs` — тоже: это грамматика **языка**,
  не имя крейта).
- **Скрипты-гейты:** `check-module-size.sh`/`check-diagnostic-codes.sh`/
  `check-exhaustive-nodes.sh`/`check-language-version.sh` — пути `grammar/`→
  `takt-lang/`, `simulation/`→`takt-sim/`; комментарий `grammar::version`→
  `takt_lang::version`. `.gitignore` — пути артефактов `/grammar/`→`/takt-lang/`.
- **Реестр размера** `module-size-baseline.txt` — 6 путей переехали на
  `takt-lang/…` (файл `bin/lamc.rs` пока с прежним именем — ренейм бинарников —
  слой 0100-02). ⚠️ **`bin/lamc.rs` 1993→1994** — не рост долга, а **фактический
  размер перемещённого файла**: удлинение `grammar::`→`takt_lang::` в строке 726
  (`construct_model`) сделало её 101 символ, и `cargo fmt` штатно перенёс `{` на
  свою строку. Короткого реэкспорта `construct_model` нет, любой импорт добавил бы
  строку — держать ровно 1993 нечем. Запись переехала на новый путь
  (`grammar/`→`takt-lang/`), поэтому это **пере-регистрация по факту**, а не правка
  числа на стабильном пути; файл пришпилен и будет переименован в `taktc.rs`
  (0100-02) с очередной перезаписью записи.
- **Markdown-ссылки** (правило 14): 9 битых ссылок в `docs/features/{0041,0045,
  0057,0059}` вида «ссылка на `../../grammar/src/…`» → `takt-lang/` — **только цели ссылок**
  (проза/бэктики с именами каталогов — слой документации 0100-06).

**Вне объёма слоя (следующие подзадачи):** имена бинарников `lamc`/`lam-lsp`/
`simulation` (0100-02); расширение `.lam`→`.takt` и тексты в коде (0100-03);
`description="Lam"`/`keywords` в манифестах, README/CLAUDE/CHANGES-проза, плагин
IntelliJ (0100-05/06); подъём версии (0100-06).

## Проверки

- `cargo build` / `cargo build --features lsp --bin lam-lsp` / `cargo check
  --all-features --all-targets` — успешно.
- `./scripts/precheck.sh` — **зелёный** (fmt/clippy/тесты/доктесты/примеры/
  детерминизм/`check-module-size` на переехавших путях/`check-language-version`/
  `check-links`). Поведение неизменно — ренейм не трогает логику.
