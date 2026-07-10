# Разработка 0018-02: опции генератора (`GenerateOptions`); Builder — не требуется

- **Фича:** [0018](../features/0018-code-guidelines.md)
- **Подзадача:** 0018-02
- **Статус:** ВЫПОЛНЕНО
- **Анализ:** [docs/analyze/0018-code-guidelines.md](../analyze/0018-code-guidelines.md) (P05, P07)

## P05 — устранение «bool trap» в API генератора

**Проблема.** Хвостовой булев флаг `guard_enable: bool` на границе публичного API
нечитаем на месте вызова: `compile_to_c(file, src, out, &[], true)`, а тесты
буквально передавали `true`/`false` без пояснения.

**Решение.** Введён тип опций `generator::GenerateOptions` (реэкспорт
`grammar::GenerateOptions`):

```rust
#[derive(Debug, Clone)]
#[non_exhaustive]
pub struct GenerateOptions { pub guard_enable: bool }
impl GenerateOptions { pub fn new(guard_enable: bool) -> Self { … } }
impl Default for GenerateOptions { fn default() -> Self { Self { guard_enable: true } } }
```

Демонстрирует сразу три практики CODE.md: **options-struct вместо bool-trap**,
`#[non_exhaustive]` (расширяемость, правило 11) и `Default` + `new()`.

**Проведено через:**
- трейт `generator::Generator::generate(&self, model, output_path, &GenerateOptions)`
  и диспетчер `generator::generate(...)` (`generator/mod.rs`);
- реализации C (`generator/c/mod.rs`, `options.guard_enable` → `CMap::new`) и
  PlantUML (`generator/plantuml/mod.rs`, `_options` — guard игнорируется);
- публичную функцию `compile_to_c(..., options: &GenerateOptions)` (`lib.rs`),
  `compile_to_plantuml` внутри передаёт `&GenerateOptions::default()`;
- вызов в `bin/lamc.rs`: `&grammar::GenerateOptions::new(options.guard_enable)`;
- все тестовые вызовы (`grammar/tests/codegen_tests.rs`, unit- и doc-тесты `lib.rs`)
  → `&GenerateOptions::default()`.

**Совместимость (правило 11).** Публичная сигнатура `compile_to_c` изменена
(последний аргумент `bool` → `&GenerateOptions`) — осознанный слом ради читаемости
и расширяемости; проект на активной ветке `v2` до стабилизации API.

## P07 — Builder для `GraphicsConfig`: НЕ ТРЕБУЕТСЯ

Проверка YAGNI-предусловия из анализа: `GraphicsConfig` **нигде не конструируется
по частям**. Единственные способы получения — `GraphicsConfig::from_file(path)`
(десериализация JSON, `--graphics-config`) и `GraphicsConfig::default()`. Ручного
пошагового `GraphicsConfig { … }` в коде нет (проверено grep'ом по `simulation/src`).

Builder добавил бы слой абстракции **без единого места использования** — прямое
нарушение CODE.md («Не вводи преждевременные обобщения и слои абстракции», YAGNI).
**Решение:** задача закрыта как «не требуется». Ергономика уже обеспечена связкой
serde + `Default`.

## Проверка

- `cargo build --all-targets --all-features` — успешно.
- `cargo test --features lsp -- --test-threads=1` — все наборы зелёные, включая
  doc-тесты (34 passed / 7 ignored / 0 failed).
- Поведение генерации не изменилось (guard по умолчанию включён, как и раньше).

## Осталось в фиче 0018

P08/P10 (аудит `.clone()`/`mem::take`), P09, P11 (`with_capacity`), P12 (doctests),
P13 (`new()`/`Default`), P04b (`#[non_exhaustive]` на узлах AST/`TypeNode`).
