# Разработка 0348-01: массив в параметре функции

> Фича: [../features/0348-st-array-parameter.md](../features/0348-st-array-parameter.md) · ADR: [../adr/0348-st-array-parameter.md](../adr/0348-st-array-parameter.md)

## Состав правки

| Файл | Что сделано |
|---|---|
| `takt-lang/src/generator/st/st_type.rs` | `array_form_name` — имя типа по **форме** массива |
| `takt-lang/src/generator/st/st_decl.rs` | сбор форм из параметров функций, объявление в `TYPE`, поле `Extras::array_forms`, печать переменной именованной формой |
| `takt-lang/src/generator/st/st_func.rs` | параметры-массивы → `VAR_IN_OUT`; `reorder_by_sections` для аргументов вызова |
| `takt-lang/src/generator/st/mod.rs` | список форм считается один раз и передаётся в каждый блок |
| `takt-sim/tests/conformance/conformance_st_tests/array_param.rs` | сверка значений **полным циклом** `iec2c` → `cc` |

## Проверено

- `iec2c` принимает вывод; порождённый им C собирается и даёт `7 9 3` — как
  эталон.
- Вывод корпуса не изменился; `cargo test` зелёный.
