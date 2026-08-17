# Задача 0225-01: Модуль semantic/statement.rs — 999 строк при пределе 1000

> Фича: [../features/0225-statement-module-size.md](../features/0225-statement-module-size.md) · ADR: [../adr/0225-statement-module-size.md](../adr/0225-statement-module-size.md) · анализ: [../analyze/0225-statement-module-size.md](../analyze/0225-statement-module-size.md)

## Что было

`takt-lang/src/semantic/statement.rs` — **976** строк при пределе 1000 (замер
кандидата «999» относился к состоянию на закрытии 0155; с тех пор файл правили
трижды и он даже сократился). Из 976 строк **599** — блок `#[cfg(test)] mod
tests`, то есть 61 % файла.

## Что сделано

Файл превращён в директорию-подмодуль по образцу `semantic/expression/`
(приём ADR 0088, применённый фичей 0129):

| Файл | Строк | Содержимое |
|---|---|---|
| `takt-lang/src/semantic/statement/mod.rs` | **381** | док-шапка, `resolve_statement`, `resolve_ast_statement`, `register_local_var`/`unregister_local_vars`; в конце — `#[cfg(test)] mod tests;` |
| `takt-lang/src/semantic/statement/tests.rs` | **602** | перенесённый блок тестов; шапка объясняет причину выноса, дальше `use super::*` |

Логика **не переписывалась**: ни одна строка `resolve_ast_statement` не
тронута, ветви исчерпывающего `match` остались в одном файле (Option B ADR
отвергнут именно за их разнос).

Путь модуля в дереве не изменился (`crate::semantic::statement`), поэтому
`named_block.rs` и `function.rs`, зовущие `resolve_statement`, правок не
потребовали.

Живой контекст `CLAUDE.md` поправлен: запись об инварианте 0155 указывала на
`semantic/statement.rs`, теперь — на `semantic/statement/mod.rs`. Исторические
ADR (0035, 0044, 0155) не трогались — это артефакты стадий, то есть история.

Обратная функциональность (правило 11): публичный API крейта не менялся, язык —
тоже; версия языка не поднимается.

## Проверки

```sh
cargo test --lib -- --list | grep semantic::statement   # список тестов до и после — совпал
./scripts/check-module-size.sh                          # код 0, реестр долга не пополнен
./scripts/precheck.sh                                   # код 0
git status --porcelain examples/generated/              # пусто — вывод целей не изменился
```

Результаты — в отчёте [../reports/0225-statement-module-size.md](../reports/0225-statement-module-size.md).
