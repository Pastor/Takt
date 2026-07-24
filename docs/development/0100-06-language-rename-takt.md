# Задача 0100-06: Документация, `Takt.ebnf`, подъём версии 0.5.0, грамматика A1

> Фича: [../features/0100-language-rename-takt.md](../features/0100-language-rename-takt.md) · ADR: [../adr/0100-language-rename-takt.md](../adr/0100-language-rename-takt.md) · анализ: [../analyze/0100-language-rename-takt.md](../analyze/0100-language-rename-takt.md)

## Что сделано (финальный слой)

- **Версия языка `0.4.0` → `0.5.0`** ([ADR 0100](../adr/0100-language-rename-takt.md),
  правило 22): константа `takt_lang::LANGUAGE_VERSION` (`version.rs`) + якорь
  `**Версия языка: 0.5.0**` README; гейт `check-language-version.sh` зелёный.
  Коммит помечен тегом **`v0.5.0`**. ⚠️ Тег `v0.4.0` в истории отсутствовал (был
  только `v0.3.0`) — пре-существующий пробел, к фиче не относится (кандидат).
- **`Lam.ebnf` → `Takt.ebnf`** (`git mv` + содержимое) + ссылка в README.
- **Проза живых документов** (`\bLam\b`→`Takt`, `.lam`→`.takt`, `lamc`→`taktc`,
  `lam-lsp`→`takt-lsp`, `Language of Automata Models`→`Typed, Automata, Known
  Timing`, крейт `` `grammar` ``→`` `takt-lang` ``, `grammar::`→`takt_lang::`,
  `grammar/`→`takt-lang/`): `README.md`, `README.typ`, `CLAUDE.md`, `docs/RULE.md`,
  `docs/CODE.md`, `FEATURES.md`, `docs/diagnostics/README.md`, `.zed/settings.json`,
  `.github/workflows/ci.yml`, скрипты `precheck.sh`/`check-language-version.sh`.
- **Крейт-имена в комментариях кода/плагина/тестов** `` `grammar` ``/
  `` `simulation` `` → `` `takt-lang` ``/`` `takt-sim` `` (не попали в 0100-01/03/05
  — это не `Lam`, не путь, не `::`).
- **Примеры `examples/*.takt`** и рукописная обвязка `examples/generated/` (tb/
  `Cargo.toml`/`lib.rs`): `Lam`→`Takt`, `lamc`→`taktc`. Снапшоты пересобраны — дифф
  только эти токены (кодоген по существу неизменен).

## Грамматика A1 (нет остаточного `Lam`) — с документированными исключениями

Проверка `git grep -nEw 'Lam|lamc|lam-lsp'` (и `` `grammar` ``/`.lam`) по коду,
скриптам, примерам и **активной** документации — **чисто**. Исключения (замысел,
не долг):

- **Исторические артефакты** — `docs/{features,adr,analyze,reports,development,
  fixes,tests}/**`, `CHANGES.md`, `.claude/report_ru.html`: датированные записи,
  описывающие прошлое под тогдашними именами (как карточки закрытых фич). Имя
  файла `0081-lamc-print-warnings.md` — часть исторического slug, не переименован
  (ссылки на него сохраняют путь).
- **`takt-lang/src/lib.rs:117`** `grammar::SourceUnitParser` — **не** крейт, а
  локальный LALRPOP-модуль `mod grammar` (инвариант 0100-01).
- **Тест-фикстуры `tests/data/**/*.{takt,st}`** — комментарии со старыми именами
  (`lamc`, `языка Lam`) оставлены: часть LSP/goto-фикстур **позиционно-связана** с
  ассертами тестов (смена длины `Lam`→`Takt` сдвинула бы байтовые офсеты и сломала
  проверки). Внутренние, не user-facing. Кандидат на аккуратный проход.

## Проверки

- `check-links` (после отката ссылок на исторический `0081-lamc-*`),
  `check-language-version` (0.5.0) — зелёные.
- `./scripts/precheck.sh` — зелёный (version.rs/скрипты/примеры затронуты →
  Rust-гейты прогнаны; снапшоты пересобраны).
- Тег `v0.5.0` на коммите подъёма версии.
