# Разработка 0331-01: именованное условие в теле

> Фича: [../features/0331-named-condition-in-body.md](../features/0331-named-condition-in-body.md) · ADR: [../adr/0331-named-condition-in-body.md](../adr/0331-named-condition-in-body.md)

## Состав правки

| Файл | Что сделано |
|---|---|
| `takt-sim/src/expression.rs` | `Condition` вычисляется адаптером условий |
| `takt-lang/src/generator/c/c_expr/expr.rs` | подстановка условия вместо имени макроса |
| `takt-lang/src/generator/c/c_expr/names.rs`, `mod.rs` | снята мёртвая `condition_macro_name` |
| `takt-lang/src/generator/st/st_expr.rs` | печать через `print_condition` |
| `takt-lang/src/generator/rust/rust_expr.rs` | то же (`rust_cond::print_condition`) |
| `takt-lang/src/generator/rust/rust_fixed.rs` | `expression_type`: `Condition` → `Bool` |
| `takt-lang/src/generator/rust/rust_cond.rs` | `print_as_bool` переехал сюда (приведение к условию — ответственность условий); в `rust_expr` остался ре-экспорт (правило 11) |
| `takt-lang/src/generator/sv/sv_expr.rs` | печать через `print_condition` |
| `takt-sim/tests/data/eval/conformance_named_cond.takt` | фикстура: условие меняется по тактам |
| `takt-sim/tests/conformance/conformance_named_cond_tests.rs` | сверка + проверка вывода цели `c` |

## Проверено

- `cargo test --test conformance conformance_named_cond` — 2/2.
- Проба: все девять потребителей согласны.
- `cargo test --all-features` — провалов нет.
