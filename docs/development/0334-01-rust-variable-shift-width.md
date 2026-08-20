# Разработка 0334-01: сдвиг на величину не меньше ширины типа

> Фича: [../features/0334-rust-variable-shift-width.md](../features/0334-rust-variable-shift-width.md) · ADR: [../adr/0334-rust-variable-shift-width.md](../adr/0334-rust-variable-shift-width.md)

## Состав правки

| Файл | Что сделано |
|---|---|
| `takt-lang/src/generator/rust/rust_shift.rs` | `saturating_right` → `guarded(Direction, …)`: оба направления, переменная величина; `shift_amount` печатает `as u32` по нужде; заголовок переписан (прежняя граница опровергнута замером) |
| `takt-lang/src/generator/rust/rust_expr.rs` | обе ветви сдвига идут через общий помощник `shift` |
| `takt-sim/tests/data/eval/conformance_var_shift.takt` | фикстура: три формы плюс контрольный вход |
| `takt-sim/tests/conformance/conformance_var_shift_tests.rs` | сверка значений с прогоном настоящего `rustc` + текстовая проверка приведения |
| `takt-lang/tests/targets/rust_shift_width_tests.rs` | проверка «переменная величина печатается как есть» **закрепляла дефект** — заменена на насыщение; добавлен сторож обеих форм сдвига влево |
| `takt-sim/tests/conformance/conformance_shift_tests.rs` | заголовок: снят абзац, попавший туда из соседней фичи (описывал `for` в цели `sv`), добавлена ссылка на соседний класс |

## Проверено

- Мутация «снять ветвь переменной величины» валит оба теста.
- `clippy-driver -D warnings` на трёх порождённых модулях — чисто.
- Вывод корпуса не изменился (сдвигов в `examples/` нет).
