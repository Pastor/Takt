# Задача 0020-01: грамматика и AST оператора `address`

> Фича: [../features/0020-port-address-decl.md](../features/0020-port-address-decl.md) · ADR: [../adr/0020-port-address-decl.md](../adr/0020-port-address-decl.md) · анализ: [../analyze/0020-port-address-decl.md](../analyze/0020-port-address-decl.md)

> **Статус:** ВЫПОЛНЕНО (лексер + грамматика + AST реализованы, LR(1) без
> конфликтов, тесты зелёные). Семантика (`AddressMap`, привязка, диагностики) —
> задача 0020-02.

## Что было

Адрес порта задавался только инлайн-инициализатором в объявлении
(`in BTN: u8 := 0x200000;`). Отдельного оператора задания адреса не было. В
лексере уже существовал терминал адресного литерала `address =>
Token::AddressLiteral(...)` (для формы `0xADDR:bit`) — это **другой** терминал,
не путать с ключевым словом.

## План реализации (по шагам)

1. **Лексер (`parser/lexer.rs`).** Новый токен-ключевое слово `Token::Address`;
   запись `"address" => Token::Address` в `KEYWORDS`; `Display`. Жёсткое ключевое
   слово (решение ADR).
2. **Грамматика (`grammar.lalrpop`).** Терминал `"address" => Token::Address` в
   extern-мапе (сосуществует с bare-терминалом `address` = адресный литерал);
   правило `AddressDefine: "address" IdentifierOrError "=" Expression ";"`
   (связка `=`, как в определениях имён — правило 0021); вариант
   `ModelElement::Address`.
3. **AST (`parser/ast.rs`).** Структура `AddressDefine { loc, name:
   Option<Identifier>, value: Expression }` и вариант `ModelElement::Address(Box<…>)`.
4. **Исчерпываемость match.** `semantic/docs.rs::element_start` и
   `lsp.rs::symbols_from_model` — новые arm (`docs.rs` — `loc.start()`; LSP —
   no-op). `semantic/tree.rs::construct_model` — цепочка `if let …`, новый вариант
   игнорируется естественно (семантика в 0020-02). `lib.rs`-проходы — `_ => {}`.
5. **Тесты (сперва зонд, затем ассерты — CLAUDE.md).**

## Что сделано (факт)

- **`parser/lexer.rs`:** вариант `Token::Address`, `Display => "address"`,
  `"address" => Token::Address` в `KEYWORDS`.
- **`grammar.lalrpop`:** `"address" => Token::Address` в extern-мапе;
  `AddressDefine` = `"address" IdentifierOrError "=" Expression ";"`;
  `AddressDefine => ModelElement::Address(Box::new(<>))` в `ModelElement`.
- **`parser/ast.rs`:** `AddressDefine` (derive `Debug, PartialEq, Eq, Clone` +
  условный serde) и `ModelElement::Address(Box<AddressDefine>)`.
- **`semantic/docs.rs`:** arm `Address(a) => Some(a.loc.start())`.
- **`lsp.rs`:** `Address(_)` в no-op-группе `symbols_from_model`.
- **Тесты:**
  - `lexer_tests.rs`: `address_keyword_produces_address_token`; `address`/`inout`
    добавлены в `is_keyword`-список; `address` — в фикстуру `keywords.lam`.
  - `parser_tests.rs`: `address_operator_parses_to_model_element_address`
    (имя + `Expression::Number(0x200000)`), `…_accepts_bit_addressed_literal`
    (`Expression::Address(_,0x200004,3)`), `…_coexists_with_port_declaration`.
  - `semantic_tests.rs`: `address_operator_is_accepted_and_ignored_by_semantics`
    (строится, адрес пока игнорируется).
  - Фикстура `tests/data/parser/valid/port_address.lam` (авто-подхват).

## Заметки для 0020-02

- **Голый `0x…` → `Expression::Number`**, `0xADDR:bit` → `Expression::Address`;
  разрешение адреса должно принимать оба (число и адресный литерал).
- В `construct_model` оператор сейчас **игнорируется** — там появится сбор в
  `AddressMap` и приоритет источников (inline < `address` < внешняя карта).
- Диагностики (конфликт в одной области, висячая привязка, полнота по
  достижимости) ещё не заведены — новые SE-коды в 0020-02.

## Проверки

- `cargo build --bin lamc` / `--features lsp --bin lam-lsp` — успешно, без
  LR(1)-конфликтов (R9/A8).
- `cargo test --features lsp -- --test-threads=1` — все зелёные (605 lib + наборы
  lexer/parser/semantic и пр.); новых clippy-предупреждений нет (базлайн = 106).
