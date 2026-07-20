# Отчёт о тестировании фичи 0082: `unused.rs` обходит формулы

> Фича: [../features/0082-unused-formulas.md](../features/0082-unused-formulas.md) · тест-план: [../tests/0082-unused-formulas.md](../tests/0082-unused-formulas.md) · ADR: [../adr/0082-unused-formulas.md](../adr/0082-unused-formulas.md)

- **Дата:** 2026-07-20
- **Окружение:** macOS (darwin 25.5.0), cargo nightly.
- **Вердикт:** **готово.** Три теста `unused_formula_tests` зелёные, `precheck.sh`
  зелёный. Язык и версия не изменились.

## Сверка с критериями приёмки (ADR 0082)

| Критерий | Проверка | Результат |
|---|---|---|
| **A1** var в LTL-формуле → нет `SE-036` | `var_used_only_in_ltl_formula_no_unused_warning` | ✅ |
| **A2** var в `invariant` → нет `SE-036` | `var_used_only_in_invariant_no_unused_warning` | ✅ |
| **A3** мёртвая var → `SE-036` | `truly_unused_var_still_warns_after_formula_traversal` | ✅ (не заглушено) |
| **A4** прочее не задето | `precheck.sh` | ✅ зелёный |

## Наблюдения

- **Дефект обострён 0081:** до подключения предупреждений к CLI ложный Ce13 был
  латентным; после — виден пользователю. 0082 закрывает его вовремя.
- **Синтаксис LTL-формулы:** `: [LTL] G flag;` (маркер `[LTL]`, оператор внутри);
  форма `: [G] …` парсером **не** принимается — уточнено пробой.
- **Guard переиспользует `collect_from_condition`** — новый код только у LTL
  (`collect_from_ltl`).

## Отклонения

- **Имя состояния в `used`** через LTL-атом — безвредная сверх-аппроксимация
  (проверяются только имена переменных; A-1 ADR).
