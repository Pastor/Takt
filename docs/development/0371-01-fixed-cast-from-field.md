# Задача 0371-01: Приведение q из поля структуры масштабируется

> Фича: [../features/0371-fixed-cast-from-field.md](../features/0371-fixed-cast-from-field.md) · ADR: [../adr/0371-fixed-cast-from-field.md](../adr/0371-fixed-cast-from-field.md) · анализ: [../analyze/0371-fixed-cast-from-field.md](../analyze/0371-fixed-cast-from-field.md)

## Что было

Масштабирование при `q → целое` применяется, только когда цель знает тип
операнда. Тип **поля структуры** не выводил никто: `extract_type` отвечал
`Unsupported`, у `rust` поле давало `None`, у `sv` ветви не было, а `st` знал
поле лишь в `inner_expr_type_in`, который в q-пути не звался.

## Что сделано

**`takt-lang/src/semantic/type_inference.rs`** — `extract_type` выводит тип
поля структуры (`BitAccess` с `Member::Identifier`), разряд не трогает.
Этим чинится цель `c`: она зовёт семантику.

**`takt-lang/src/generator/rust/rust_fixed.rs`** — `fixed_format_in(expr,
model)`: собственный вывод, затем разбор с оглядкой на объявления (поле,
арифметика, скобки); `cast` и четыре вызова в `rust_expr` переведены на него.

**`takt-lang/src/generator/sv/sv_fixed.rs`** — то же с `Scope.structs`
(снимок карты), включая поле **элемента массива** (тип базы даёт
`sv_array::array_type_expr`, фича 0365).

**`takt-lang/src/generator/st/st_fixed.rs`** — `fixed_format` принимает модель
и спрашивает `inner_expr_type_in` (0349); `storage_of` тоже.

**Сверка**: `fixed_cast_from_field_matches_generated_c` — три формы (поле,
выражение над полями, поле элемента массива) с **разными** значениями.

⚠️ **Одной правкой в семантике класс не закрывается:** `rust`, `sv` и `st`
семантический вывод не зовут — у каждого свой. Проверено прогоном после
первой правки.

## Проверки

```sh
cargo test --test conformance fixed_cast_from_field
cargo test --test conformance   # 170
cargo test --test targets       # 391
cargo test -p takt-lang --lib   # 1108
for f in examples/*.takt; do scripts/probe.sh -n 1 "$f"; done   # корпус чист
./scripts/precheck.sh
```

Мутация «семантика без типа поля» — сверка красная («расхождение one»).
