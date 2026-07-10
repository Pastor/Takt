# Фича 0010: Верификация свойств (LTL и автоматы Бюхи)

- **Номер:** 0010
- **Статус:** ГОТОВО
- **Зависит от:** 0001
- **Крейт:** `grammar`

## Краткое описание

Проверка формальных свойств моделей: разбор LTL-формул и построение автоматов
Бюхи (алгоритм GPVW).

## Итог (что сделано)

- `verification/ltl.rs` (`Ltl`), `verification/buchi.rs` (`BuchiAutomaton`, `NodeMap`),
  `verification/mod.rs`.
- Встроенные формулы в AST: `InlineFormulaDefine`, `LtlExpr`, `FormulaStatement`,
  `FormulaExpression` (`parser/ast.rs`); `semantic/formula.rs` (`Formula`).

> Ретроспективная карточка (правило 17). Детали — в истории git и `CHANGES.md`.
