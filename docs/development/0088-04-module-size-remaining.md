# Задача 0088-04: Парсер — вынос `Token` в `parser/token.rs`

> Фича: [../features/0088-module-size-remaining.md](../features/0088-module-size-remaining.md) · ADR: [../adr/0088-module-size-remaining.md](../adr/0088-module-size-remaining.md) · анализ: [../analyze/0088-module-size-remaining.md](../analyze/0088-module-size-remaining.md)

## Что было

`grammar/src/parser/lexer.rs` — **1154 строки** (нарушитель). Содержал и лексер
(`Lexer`, `LexicalError`, сканеры), и **перечисление токенов** `Token` (все
терминалы грамматики) вместе с его `impl fmt::Display` (~300 строк).

## Что сделано

Перечисление токенов вынесено в новый модуль `grammar/src/parser/token.rs`
(311 строк):

- `pub enum Token<'input>` (все терминалы) + `impl fmt::Display for Token`.
  `Token` самодостаточен — варианты хранят только `&'input str`/`bool`/`i64`,
  внешних типов нет; `impl Display` зависит лишь от `std::fmt`.
- **Путь `parser::lexer::Token` сохранён реэкспортом** `pub use
  crate::parser::token::Token;` в `lexer.rs` — от него зависит lalrpop
  (`use super::parser::lexer::{Token, LexicalError};` + `extern { enum Token }`),
  а также doc-тесты и потребители (правило 11). `use std::fmt` в `lexer.rs`
  удалён (осел вместе с `Display`).
- `parser/mod.rs` — добавлено `pub mod token;`.

**Чистое перемещение:** `Token` и `Display` скопированы дословно, разбор/вывод
неизменны. `lexer.rs`: **1154 → 851** (уложился) — запись удалена из реестра
(**15 → 14**). `token.rs` (311) — не нарушитель.

Стеки: только `grammar`. `simulation` — н/п.

## Проверки

- `cargo build --bin lamc` — без предупреждений.
- `./scripts/precheck.sh` — зелёный (сборка lalrpop-грамматики против
  реэкспортированного `Token`, `lexer_tests`/`parser_tests`, детерминизм-гейт,
  `check-module-size.sh` −1 запись).
