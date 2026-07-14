# Задача 0020-02: семантика оператора `address` — привязка и диагностики

> Фича: [../features/0020-port-address-decl.md](../features/0020-port-address-decl.md) · ADR: [../adr/0020-port-address-decl.md](../adr/0020-port-address-decl.md) · анализ: [../analyze/0020-port-address-decl.md](../analyze/0020-port-address-decl.md)

> **Статус:** ВЫПОЛНЕНО (захват привязок + диагностики конфликта/висячей
> привязки; тесты зелёные). Внешняя `.ld`-карта и приоритет-оверлей — 0020-03;
> полнота по достижимости — 0020-04; потребление в C — 0020-05.

## Что было

После 0020-01 оператор `address` парсился в `ModelElement::Address`, но
`construct_model` его **игнорировал** — привязка нигде не сохранялась, порт
оставался без адреса, диагностик не было.

## Что сделано (факт)

- **`semantic/mod.rs`:** новый узел `AddressBindingNode { port, loc, value }` и
  поле `ModelNode::address_defs: Vec<AddressBindingNode>`. Поле добавлено в
  full-literal конструкторы (`ModelNode::new`, клон с переименованием);
  `..Default::default()`-сайты и `#[derive(Default)]` подхватывают пустой `Vec`
  автоматически. В `PartialEq for ModelNode` **не** включено (не часть
  идентичности модели, как `loc`/`type_locs`).
- **`semantic/tree.rs` (`construct_model_stage0`):** новый arm
  `ModelElement::Address(def)` — извлекает имя порта (`extract_name`) и сохраняет
  привязку в `address_defs` (значение — сырое `ExpressionNode::Unresolved`).
  Захват отделён от проверки: порт может объявляться **после** оператора.
- **`semantic/validate.rs`:** `check_port_addresses(model)` в конвейере
  `validate_model` (после `check_array_sizes`). Диагностики:
  - **SE-048** (R5) — `address` ссылается на имя, которого нет среди портов
    модели (висячая привязка).
  - **SE-049** (R4) — адрес задан одновременно inline (`:= <addr>`) и оператором
    `address`, **либо** несколькими операторами `address` для одного порта.

  «Наличие inline-адреса» = `VariableNode::Port.expr != ExpressionNode::None`.

## Тесты

- `semantic_tests.rs`:
  - `address_operator_is_captured_in_address_defs` — привязка попадает в
    `address_defs` с именем порта.
  - `example_port_address_separate_is_valid` — фикстура
    `valid/port_address_separate.lam` (адрес отдельным оператором, 2 порта) —
    валидна.
  - `port_address_conflict_inline_and_operator_is_error` →
    `invalid/port_address_conflict.lam` → SE-049.
  - `port_address_duplicate_operator_is_error` (из строки) → SE-049.
  - `port_address_dangling_reference_is_error` →
    `invalid/port_address_dangling.lam` → SE-048.

## Решения и границы

- **Приоритет inline < `address`** (из ADR) внутри одной модели **не** реализуется
  как «тихое переопределение»: по решению заказчика (R4) наличие обоих источников
  для одного порта — **ошибка**. Приоритет-оверлей (внешняя карта поверх модели с
  warning) появляется в 0020-03, где источники в разных областях.
- **Консолидированный `AddressMap`** (PortId → разрешённый адрес) **не** строится:
  без внешней карты (0020-03) и потребителя (0020-05) он спекулятивен. Сохранён
  сырой субстрат `address_defs`; понижение выражения-адреса в конкретное значение
  — задача 0020-05.

## Заметки для 0020-03/05

- Голый `0x…` → `Expression::Number`, `0xADDR:bit` → `Expression::Address` —
  понижение адреса должно принимать оба (см. `value: ExpressionNode::Unresolved`).
- Внешняя карта наполняет тот же слой; правило приоритета — оверлей поверх
  inline/`address` c warning (0020-03).

## Проверки

- `cargo build --bin lamc` — успешно.
- `cargo test --features lsp -- --test-threads=1` — все зелёные (605 lib + наборы);
  `cargo fmt --check` чист; новых clippy-предупреждений нет (базлайн 106).
