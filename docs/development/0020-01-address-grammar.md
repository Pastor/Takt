# Задача 0020-01: грамматика и AST оператора `address`

> Фича: [../features/0020-port-address-decl.md](../features/0020-port-address-decl.md) · ADR: [../adr/0020-port-address-decl.md](../adr/0020-port-address-decl.md) · анализ: [../analyze/0020-port-address-decl.md](../analyze/0020-port-address-decl.md)

> **Статус:** ЗАПЛАНИРОВАНО (реализация не начата — фича доведена до стадии
> «Разработка» по запросу; код пишется отдельно с TDD).

## Что было

Адрес порта — только инлайн: `in NAME: T = <addr>;` → `VariableDefine::Port { initializer }`.

## План реализации (TDD)

1. **Лексер** (`parser/lexer.rs`): добавить `Token::Address` и `"address"` в
   `KEYWORDS`. Предварительно — grep по `examples/`/фикстурам, что `address` не
   используется как идентификатор (риск из анализа).
2. **Грамматика** (`grammar.lalrpop`): правило
   `AddressDecl: ... = "address" name "=" Expression ";"` на уровне
   `ModelElement`; собрать без LR(1)-конфликтов.
3. **AST** (`parser/ast.rs`): новый вариант (напр. `ModelElement::Address` или
   `VariableDefine::Address { loc, name, expr }`).
4. **Фикстуры парсинга**: `valid/port_address_separate.lam`.

## Проверки (ожидаемые)

- Парсинг `address NAME = 0x200000;` даёт ожидаемый AST-узел.
- Регресс инлайн-формы = 0 (R2), нет новых LR(1)-конфликтов (R7).
- `cargo test --features lsp -- --test-threads=1` зелёный.

## Дальше

Семантика и C-генерация — задача **0020-02** (разрешение адреса, диагностики
конфликта/висячей привязки, идентичный C-код).
