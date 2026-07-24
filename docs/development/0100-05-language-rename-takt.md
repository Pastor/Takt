# Задача 0100-05: Редакторные расширения (IntelliJ + Zed) Lam → Takt

> Фича: [../features/0100-language-rename-takt.md](../features/0100-language-rename-takt.md) · ADR: [../adr/0100-language-rename-takt.md](../adr/0100-language-rename-takt.md) · анализ: [../analyze/0100-language-rename-takt.md](../analyze/0100-language-rename-takt.md)

## Корректировка объёма (правило 19)

Анализ называл слой 0100-05 «плагин IntelliJ». При проработке обнаружено **второе**
редакторное расширение — **Zed** (`extensions/zed-lam/`), пропущенное и в
исходной декомпозиции фичи. Оба — редакторный тул языка; слой расширен до
«**редакторные расширения**» (IntelliJ + Zed). Ни то, ни другое не входит в
`precheck.sh`/CI (IntelliJ собирается `./gradlew`, Zed — WASM-крейт вне
workspace).

## Что сделано — IntelliJ (`extensions/intellij-lam` → `intellij-takt`)

- **Каталог/пакет:** `intellij-lam`→`intellij-takt`, `org/lam/intellij`→
  `org/takt/intellij` (main + test).
- **Файлы:** 45 `Lam*.kt`→`Takt*.kt`; ресурсы `icons/lam.svg`→`takt.svg`,
  `META-INF/lam-lsp4ij.xml`→`takt-lsp4ij.xml`.
- **Контент:** `\bLam`→`Takt` (классы + слово; проверено — нет `Lam`+строчная,
  риска `Lambda` нет), `org.lam`→`org.takt`, `.lam`→`.takt`, `lam-lsp`→`takt-lsp`,
  `intellij-lam`→`intellij-takt`, `Language of Automata Models`→`Typed, Automata,
  Known Timing`. plugin.xml `<id>org.takt.intellij</id>`/`<name>Takt</name>`/тип
  файла `.takt`; `gradle.properties` `pluginGroup=org.takt`; `settings`
  `rootProject.name="intellij-takt"`.
- ⚠️ **Функциональные пропуски, пойманные проверкой:**
  - `TaktFileType.getDefaultExtension()` возвращал голое `"lam"` (без точки — не
    поймано `.lam`-паттерном) → `"takt"`.
  - Тесты плагина ссылались на каталог Rust-проекта **`grammar/`** (переименован в
    0100-01!): `TaktKeywordSyncTest` (`grammar/src/parser/lexer.rs`),
    `TaktPsiCorpusTest` (`grammar/tests/data`, детект корня по `grammar/`) →
    `takt-lang/`. Плюс `lamc`→`taktc`, storage настроек `lam.xml`→`takt.xml`.
- **Installer-скрипты** `extensions/install-rustrover-plugin.{sh,ps1}`:
  `intellij-lam`→`intellij-takt`, `Lam`→`Takt`.

## Что сделано — Zed (`extensions/zed-lam` → `zed-takt`)

- Каталог `zed-lam`→`zed-takt`, `languages/lam`→`languages/takt`.
- `extension.toml`: `id="takt"`, `name="Takt"`, `[language_servers.takt-lsp]`,
  `language="Takt"`, описание/`.takt`. `config.toml`: `path_suffixes=["takt"]`,
  `language_servers=["takt-lsp"]`. `Cargo.toml` `name="zed-takt"`. `src/lib.rs`:
  поиск бинарника `takt-lsp`, путь установки `takt-lang --bin takt-lsp`.
  `scripts/install.sh`: `EXT_ID="takt"`, `languages/takt`.

## Проверки

- **IntelliJ:** `./gradlew --offline test` — **BUILD SUCCESSFUL** (компиляция
  переименованных пакетов/классов, тесты; корпус-тест нашёл `.takt` в `examples/`
  и `takt-lang/tests/data/`).
- **Rust-precheck не затронут:** поверхность 0100-05 — только `extensions/` + одна
  markdown-ссылка `README.md` (`extensions/intellij-lam`→`intellij-takt`, правило
  14); ни одного файла `takt-lang`/`takt-sim`/`scripts`/`examples` не изменено →
  Rust-гейты идентичны зелёному 0100-04. `check-links` и `check-language-version`
  — зелёные.

**Вне объёма:** README/CLAUDE/`docs/*.md`-проза, `Takt.ebnf`, README.typ, подъём
версии — 0100-06. Историческая хроника `CHANGES.md` (упоминания `intellij-lam`/
`zed-lam` в прошедших записях) **не переписывается** — как карточки закрытых фич.
