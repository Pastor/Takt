# Задача 0174-01: Эмиссия impl Default и сторож на гейте

> Фича: [../features/0174-rust-new-without-default.md](../features/0174-rust-new-without-default.md) · ADR: [../adr/0174-rust-new-without-default.md](../adr/0174-rust-new-without-default.md) · анализ: [../analyze/0174-rust-new-without-default.md](../analyze/0174-rust-new-without-default.md)

## Что было

Корень без транзитивной нужды в HAL получал `pub fn new() -> Self` без
аргументов, а `impl Default` не печатался **никогда**. Под `clippy -D warnings`
(политика гейта цели `rust`) такой вывод — красный.

## Что сделано

### Генератор

`takt-lang/src/generator/rust/rust_model.rs` — новая функция
`emit_default_impl`, вызывается сразу после закрытия блока `impl`:

```rust
if !is_root || uses_hal { return; }
```

Условие буквально совпадает с условием срабатывания линта: конструктор
**публичен** (публичен только корневой) и аргументов не имеет (аргумент
появляется ровно при `uses_hal`). ⚠️ При `uses_hal` печатать не просто не
нужно, а **нельзя**: значение `H` взять неоткуда — вывод не скомпилируется.

Выбран `impl Default`, а не `#[allow(clippy::new_without_default)]`: политика R9
фичи 0050 — «не эмитить то, на что линт ругается»; атрибут заглушил бы и будущие
срабатывания.

### Сторож

Новый `takt-lang/tests/targets/rust_default_impl_tests.rs`, два слоя:

| Тест | Что доказывает |
|---|---|
| `model_without_ports_emits_default_impl` | `impl Default` печатается при конструкторе без аргументов |
| `model_with_port_does_not_emit_default_impl` | **контрпример**: при `new(hal: H)` не печатается |
| `generated_module_passes_clippy_gate` | тот же `clippy -D warnings`, что в `precheck.sh`, принимает вывод |

Второй слой нужен потому, что первый доказывает лишь печать задуманной строки, а
не согласие **настоящего** линта. Мягкая деградация: нет `clippy-driver` →
пропуск с сообщением (образец — `conformance_rust_tests`).

Обратная функциональность (правило 11): изменение **аддитивно** — печатается
новый элемент, существующие не меняются. Симулятор и прочие цели не затронуты —
**н/п**.

## Проверки

| Что | Результат |
|---|---|
| `cargo test --all-features --test rust_default_impl_tests` | 3 из 3 |
| `cargo test --all-features -- --test-threads=1` | зелёный |
| `cargo clippy --all-targets --all-features -- -D warnings` | чисто |
| перегенерация корпуса целью `rust` | `git diff examples/generated/rust` **пуст** |
| `./scripts/precheck.sh` | зелёный |

**Взведённость доказана двумя мутациями генератора** — каждая валит свой класс
тестов и только его:

| Мутация | Падают | Остаются зелёными |
|---|---|---|
| эмиссия отключена (`return` сразу) | `model_without_ports_emits_default_impl`, `generated_module_passes_clippy_gate` | контрпример |
| эмиссия безусловна (снято `uses_hal`) | контрпример | остальные два |

Пустой дифф корпуса — не «нечего было чинить», а подтверждение диагноза: **ни
один** пример не обходится разом без портов и без `extern fn`, поэтому гейт этот
класс не видел и без отдельного сторожа не увидел бы впредь.
