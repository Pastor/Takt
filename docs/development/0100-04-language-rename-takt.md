# Задача 0100-04: Расширение `.lam` → `.takt` (атомарно: файлы, тесты, генератор, скрипты, снапшоты)

> Фича: [../features/0100-language-rename-takt.md](../features/0100-language-rename-takt.md) · ADR: [../adr/0100-language-rename-takt.md](../adr/0100-language-rename-takt.md) · анализ: [../analyze/0100-language-rename-takt.md](../analyze/0100-language-rename-takt.md)

## Что было

Расширение исходников — `.lam`: **244** файла (8 примеров + ~235 фикстур
`tests/data/**`), логика распознавания в `taktc.rs:476` (обход каталога у `fmt`)
и в 9 местах тестов (`ext == "lam"`), 375 хардкод-ссылок `".lam"` в 37 тест-`.rs`,
импорты `import "x.lam"` внутри 20 примеров/фикстур, генератор эмитил `.lam` в
комментариях снапшотов, скрипты globили `examples/*.lam`, `.plc.map` ссылались на
`.lam`.

## Что сделано (атомарный флип, [ADR 0100](../adr/0100-language-rename-takt.md) — жёсткий разрыв)

- **Файлы:** `git mv` всех **244** `.lam`→`.takt` (циклом по `git ls-files`).
- **Контент исходников `.takt`:** `import "x.lam"`→`import "x.takt"` и любые `.lam`
  внутри (20 файлов с импортами).
- **`.rs` обоих крейтов:** `.lam`→`.takt` (375 хардкод-путей фикстур + генератор-
  строки, эмитящие `.lam`) и extension-проверки `"lam"`→`"takt"` (9 мест:
  `taktc.rs:476`, `lexer`/`parser`/`format`/`verify`/`address_export`/
  `lsp_formatting_conformance`/`c_stub` тесты).
- **Скрипты:** `precheck.sh` (globы `examples/*.takt`, `basename … .takt`,
  `sv-mmio`, сообщения, комментарии-имена примеров), `run_simulations.sh` (поиск
  модели по `${candidate}.takt`).
- **Карты:** `examples/*.plc.map` — `.lam`→`.takt`.
- **Снапшоты `examples/generated/`:** пересобраны компилятором. Дифф **минимален —
  8 файлов, только `.lam`→`.takt`** в эмитируемых комментариях (`sv/*.sv`,
  `sv-mmio`, `rust/elevator.rs`): «служебный порт цели sv: в .takt его нет»,
  «в исходнике .takt». Кодоген по существу неизменен (проверено: строки диффа без
  `.takt`/`.lam` отсутствуют).

## Ключевые проверки согласованности

- Логика расширения — **одна** точка в компиляторе (`taktc.rs:476`, `fmt`
  каталога); `compile`/`verify` берут явные пути. Тесты-обходы каталогов
  (`ext == "takt"`) синхронны с переименованными фикстурами.
- `$LAMC="./target/debug/taktc"` (0100-02) — уже верен.

**Вне объёма:** плагин IntelliJ (`LamFileType` распознаёт `.lam`, корпус-тест) —
0100-05; README/CLAUDE/`docs/*.md`, `Takt.ebnf`, README.typ, подъём версии —
0100-06.

## Проверки

- `cargo build --all-features`; целевые `lexer`/`parser`/`format`/`verify` тесты
  (28+ passed) — обходы каталогов и пути фикстур согласованы.
- Пересборка снапшотов: дифф только `.lam`→`.takt` (8 файлов).
- `./scripts/precheck.sh` — зелёный (fmt примеров на `.takt`, детерминизм,
  прогон всех тестов, `run_simulations` вне precheck).
