# Задача 0203-02: validate не обходит формулы: неизвестное имя в Guard молчит

> Фича: [../features/0203-validate-formulas-traversal.md](../features/0203-validate-formulas-traversal.md) · ADR: [../adr/0203-validate-formulas-traversal.md](../adr/0203-validate-formulas-traversal.md) · анализ: [../analyze/0203-validate-formulas-traversal.md](../analyze/0203-validate-formulas-traversal.md)

## Что было

Ни одна проверка `validate_model_all` формулы не обходила: `validate_conditions`
идёт по `model.conditions`, обход рёбер — по `references`, обход тел — по
операторам, а формулы не принадлежат ни одному из этих множеств. Опечатка в
имени внутри охранной формулы принималась **молча**: средство безопасности
переставало сторожить, цель `c` печатала `assert( < 3);` (отказ приходил от
`cc`), цель `rust` отвечала `RS-011`, цели `st`/`sv` формулу не печатали вовсе,
симулятор падал `SIM-016` **в такте** — один вход, четыре разных ответа.

## Что сделано

**1. Проверка `validate/formulas.rs`** в массиве `validate_model_all`. Судья —
существующий `validate_cond`, тот же, что судит `cond` и рёбра `ref`: проверка
только **доставляет** ему условия (образец `validate/bodies.rs`, 0188). Поэтому
формулам достался **весь** набор проверок условия, а не одно правило «нет ли
`Unresolved`» (ADR отверг Option C): `SE-025` на неизвестное имя, `SE-033` на
состояние чужой модели, предел глубины — и все будущие. Сайты берутся из общего
сбора (задача 0203-01); `FormulaLeaf::Ltl` пропускается — у LTL своя проверка с
иным режимом строгости (`SE-056`, предупреждение; R3 анализа).

Накопление (правило 0151): **одна диагностика на формулу, все формулы
высказываются**; внутри формулы ранний выход сохранён.

**2. Находка задачи: судья не знал краткой формы паттерна.** Включение проверки
уронило `examples/extend_complex.takt:74` (`: E != End;`) — сработал риск Р3
анализа, но причина оказалась не в фикстуре. Форму «текущее состояние модели»
распознавали **трижды и по-разному**:

| Место | `S(Модель) = Состояние` | краткое `Модель = Состояние` |
|---|---|---|
| канонизация скобок (`is_state_of`, 0074) | да | да |
| цель `c` (`state_of_model`), цель `rust` (`model_of`) | да | да |
| судья `validate_cond` | да | **нет → `SE-025`** |

То есть `ref X: E != End;` отвергался на записи, которую генератор переводит
(`model->…state != EXTEND_COMPLEX_E_END`), а в формуле та же запись молчала —
потому что формулы не обходил никто. Единственный носитель краткой формы в
корпусе стоял именно в формуле, и расхождение поэтому дожило до 0203.

**Решение заказчика (2026-08-15): краткую форму узаконить.** Разбор формы сведён
в **одну** функцию на проект — `semantic/condition/state_of.rs::state_of_model`;
её зовут судья, печатники целей `c` и `rust` и канонизация скобок (`is_state_of`
стал её булевым видом). Корпус не правился, вывод целей байт-в-байт прежний.

Функциональности: **семантика** — да; **цели `c`/`rust`** — да (перевод на общий
предикат, вывод неизменен); **симулятор** — н/п (сравнение с состоянием модели
он не исполняет и сегодня, `SIM-013`); **LSP** — н/п: диагностики он берёт той
же точкой `collect_compile_diagnostics`, правки не требуется; **документ** —
задача 0203-03.

## Проверки

```sh
cargo test --test formula_validation_tests   # 14 тестов, зелено
cargo test --all-features
./scripts/precheck.sh
```

Сторож — `takt-lang/tests/formula_validation_tests.rs` (фикстуры
`tests/data/formula0203/`), критерии анализа:

| Критерий | Тест |
|---|---|
| A1 формула состояния | `unknown_name_in_state_formula_is_diagnosed` |
| A2 краткая форма `: c;` | `unknown_name_in_short_form_formula_is_diagnosed` |
| A3 формула уровня модели | `unknown_name_in_model_level_formula_is_diagnosed` |
| A4 блок модели/состояния, функция, вложенный оператор | четыре теста `…_block_…`, `…_function_body_…`, `…_nested_statement_…` |
| A5 `invariant` — ровно одно сообщение | `invariant_with_unknown_name_reports_exactly_once` |
| A6 LTL остаётся предупреждением | `ltl_formula_with_unknown_atom_is_not_an_error` |
| A7 накопление | `two_broken_formulas_yield_two_diagnostics` |
| A8 разрешимые имена принимаются | `resolvable_formulas_are_accepted_in_all_sites` |
| A9 обход остаётся один | `every_declaration_site_of_a_formula_is_checked` |
| R2/R4 попутные проверки судьи | `formula_gets_the_other_condition_checks_too` (`SE-033`) |
| находка задачи | `bare_model_state_pattern_is_accepted_everywhere` |

**Сторожа проверены мутацией** (иначе они декоративны):

- снят вызов `validate_formulas` → **10 из 14** тестов краснеют;
- из общего обхода изъято место «тело функции» → краснеют
  `unknown_name_in_function_body_formula_is_diagnosed` и — списком, с именем
  осиротевшего места — `every_declaration_site_of_a_formula_is_checked`.
