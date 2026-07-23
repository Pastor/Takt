# Задача 0088-05: Парсер — вынос `Expression` в `parser/ast_expr.rs`

> Фича: [../features/0088-module-size-remaining.md](../features/0088-module-size-remaining.md) · ADR: [../adr/0088-module-size-remaining.md](../adr/0088-module-size-remaining.md) · анализ: [../analyze/0088-module-size-remaining.md](../analyze/0088-module-size-remaining.md)

## Что было

`grammar/src/parser/ast.rs` — **1204 строки** (нарушитель). Содержал все узлы АСД;
крупнейший — узел **`Expression`** (`pub enum Expression` + `impl Expression`,
~300 строк).

## Что сделано

Узел выражения вынесен в новый модуль `grammar/src/parser/ast_expr.rs`
(315 строк):

- `pub enum Expression` (все виды выражений) + `impl Expression` (аксессоры
  `loc`/…). Импортирует из `parser::ast` типы, встречающиеся в вариантах:
  `Identifier`, `Member`, `NamedArgument`, `ParameterList`, `Statement`,
  `StringLiteral`, `Type` (взаимная ссылка `ast` ↔ `ast_expr` — Rust допускает).
- **Путь `parser::ast::Expression` сохранён реэкспортом** `pub use
  crate::parser::ast_expr::Expression;` в `ast.rs` — от него зависят
  lalrpop-грамматика, семантика и генераторы (правило 11).
- `parser/mod.rs` — добавлено `pub mod ast_expr;`.

**Чистое перемещение:** `Expression` и его `impl` скопированы дословно, разбор/
вывод неизменны. `ast.rs`: **1204 → 902** (уложился) — запись удалена из реестра
(**14 → 13**). `ast_expr.rs` (315) — не нарушитель.

Стеки: только `grammar`. `simulation` — н/п.

## Проверки

- `cargo build --bin lamc` — без предупреждений (каскад «`Expression` не найден»
  снят: `ast_expr` компилируется — не хватало импорта `ParameterList`).
- `./scripts/precheck.sh` — зелёный (lalrpop против реэкспорта, все тесты,
  детерминизм-гейт, `check-module-size.sh` −1 запись).
