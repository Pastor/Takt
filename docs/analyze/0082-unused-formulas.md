# Анализ фичи 0082: `unused.rs` не обходит `formulas`

> Фича: [../features/0082-unused-formulas.md](../features/0082-unused-formulas.md) · ADR: [../adr/0082-unused-formulas.md](../adr/0082-unused-formulas.md)

## Зависимости

- **Нет.** Опора — `semantic/unused.rs` (Ce13) и типы `Formula`/`Ltl`.
- **Обострено [0081](../features/0081-lamc-print-warnings.md):** Ce13 теперь
  печатается пользователю, ложное срабатывание стало видимым.
- **Приоритет / Tier:** **Tier 2** — ложное предупреждение (не порча кода, но
  вводит в заблуждение).

## Точки интеграции (замер по коду)

| Что | Где | Как |
|---|---|---|
| Обход использований | `unused.rs::collect_from_model_tree` | vars/funcs/conditions/blocks/states/nested — **без** formulas |
| Формулы модели | `ModelNode::formulas: Vec<Formula>` | не обходятся |
| Формулы состояния | `StateNode::{Simple,Implement}::formulas` | не обходятся (в `..`) |
| `Formula::Guard(ConditionNode, _)` | — | переиспользует `collect_from_condition` |
| `Formula::LTL(Ltl)` | `verification::ltl::Ltl` | нужен обход атомов |

## Ключевые решения анализа

1. **Guard → `collect_from_condition`** (уже есть); **LTL → новый
   `collect_from_ltl`** (обход атомов, `Atom(name) → used`).
2. **Формулы состояния** требуют добавить `formulas` в деструктуризацию обоих
   вариантов `StateNode`.
3. **Атом LTL — имя переменной или состояния** (0049/0068); имя состояния в `used`
   безвредно.

## Риски

- **Заглушить настоящую находку** — снят негативным сторожем
  (`test_truly_unused_var_still_warns_after_formula_traversal`).
