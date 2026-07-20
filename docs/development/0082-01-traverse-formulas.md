# Задача 0082-01: обход формул в `unused.rs`

> Фича: [../features/0082-unused-formulas.md](../features/0082-unused-formulas.md) · ADR: [../adr/0082-unused-formulas.md](../adr/0082-unused-formulas.md)

## Что было

`collect_from_model_tree` (`semantic/unused.rs`) обходил vars/functions/conditions/
named_blocks/states/nested, но **не** `ModelNode::formulas` и `StateNode::formulas`.
Переменная только в LTL/Guard-формуле → ложный `SE-036`.

## Что сделано

1. **`collect_from_formula(formula, used)`**: `Guard(cond, _)` →
   `collect_from_condition`; `LTL(ltl)` → `collect_from_ltl`; `Formulas` →
   рекурсия; `None` → ничего.
2. **`collect_from_ltl(ltl, used)`**: `Atom(name) → used.insert`; рекурсия по всем
   темпоральным/логическим операторам (match исчерпывающий).
3. **Вызов** для `borrowed.formulas` в `collect_from_model_tree` и для `formulas`
   в `collect_from_state` (добавлено в деструктуризацию обоих вариантов `StateNode`).

## Проверки

- **T1/A1:** `var_used_only_in_ltl_formula_no_unused_warning`.
- **T2/A2:** `var_used_only_in_invariant_no_unused_warning`.
- **T3/A3:** `truly_unused_var_still_warns_after_formula_traversal` (негативный сторож).
- **T4/A4:** `precheck.sh` зелёный.
