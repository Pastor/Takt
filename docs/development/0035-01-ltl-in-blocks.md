# Задача 0035-01: Разбор LTL-формул в блоках кода (устранение тихой потери)

> Фича: [../features/0035-ltl-in-blocks.md](../features/0035-ltl-in-blocks.md) · ADR: [../adr/0035-ltl-in-blocks.md](../adr/0035-ltl-in-blocks.md) · анализ: [../analyze/0035-ltl-in-blocks.md](../analyze/0035-ltl-in-blocks.md)

> **Планируется (разработка не начата).** Разделы «Что сделано» и «Проверки» —
> план, а не отчёт. Покрывает **R1, R2**; критерии **A1, A2, A3**.

## Что было

**Реальное состояние кода на 2026-07-15 (ветка `v2`, коммит `6984471`).**

Семантика встроенной формулы расходится по позициям конструкции.

**Модель** (`grammar/src/semantic/tree.rs:574-580`) — разбирает:

```rust
ast::InlineFormulaDefine::Ltl { formulas, .. } => {
    for f in formulas {
        model_node.borrow_mut().formulas
            .push(Formula::LTL(formula::ltl_ast_to_semantic(f)));
    }
}
```

**Состояние** (`grammar/src/semantic/tree.rs:1189-1192`) — разбирает тем же
способом (`state_formulas.push(Formula::LTL(formula::ltl_ast_to_semantic(f)))`).

**Блок кода** (`grammar/src/semantic/statement.rs:256-270`) — **теряет**:

```rust
ast::Statement::InlineFormula(inline) => {
    match &**inline {
        ast::InlineFormulaDefine::Guard { conditions, .. } => {
            let resolved: Vec<Formula> = conditions
                .iter()
                .filter_map(|c| resolve_condition(c, model.clone()).ok())
                .map(|cn| crate::semantic::formula::condition_to_formula(&cn))
                .collect();
            Ok(StatementNode::InlineFormula(resolved))   // ← Guard работает
        }
        ast::InlineFormulaDefine::Ltl { .. } => {
            // TODO: Реализовать поддержку LTL в блоках кода
            Ok(StatementNode::InlineFormula(Vec::new()))  // ← формула исчезает
        }
    }
}
```

Формула отбрасывается молча: без диагностики, с кодом возврата `0`.

**Что уже готово и переиспользуется (делает задачу дешёвой):**

- `grammar/src/semantic/formula.rs:50-80` — `ltl_ast_to_semantic(&LtlExpr) -> Ltl`
  **тотальна** по всем вариантам `LtlExpr` (`True`, `False`, `Atom`, `Not`,
  `Next`, `Finally`, `Globally`, `And`, `Or`, `Until`, `Release`, `Implies`,
  `Parenthesis`). Дописывать преобразование не нужно.
- Приёмник `StatementNode::InlineFormula(Vec<Formula>)`
  (`grammar/src/semantic/mod.rs:783`) — тип уже подходящий, менять не требуется.
- Путь потребления в блоке **исправен**: `generator/c/c_expr.rs:1693-1699`
  обходит вектор и зовёт `generate_formula_check`.
- `resolve_formulas` (`tree.rs:786-787`, `:897`) пропускает `Formula::LTL(ltl)`
  тождественно — повторное разрешение не потребуется.

**Проба, зафиксировавшая дефект** (модель
`start S { always { a := a + 1; : a = 0; : [LTL] G (a -> F a); } ref Done: a = 3; }`,
`lamc compile -t c`, код возврата `0`, предупреждений нет):

```c
case P8_S: {
    model->a = model->a + 1;   // обычный оператор  — есть
    assert(model->a == 0);     // : [Guard] в блоке — есть
    /* : [LTL] G (a -> F a);   — в выводе ОТСУТСТВУЕТ, молча */
```

Соседний `Guard` из того же блока доезжает до C — значит блочный путь рабочий, и
ломается всё ровно на `Vec::new()`.

**Почему дефект дожил до 0035 (слепое пятно тестов).**
`grammar/tests/ltl_tests.rs::test_parse_ltl_formula` берёт именно сломанный
случай (`always { :[LTL] X a; … }`), но останавливается на `parse` и утверждает
только форму **АСД** — `construct_model` не вызывает.
`semantic_tests.rs::test_inline_formula_model_resolved` / `…_state_resolved`
доходят до семантики, но проверяют **`Guard`** и уровни модели/состояния. Пара
«блок × LTL после семантики» не покрыта ничем.

## Что сделано

> **Планируется (разработка не начата).**

**Суть.** Ветка `Ltl` в `semantic/statement.rs` приводится к паритету с
`tree.rs`: `Vec::new()` и `TODO` заменяются на вызов уже существующей
`formula::ltl_ast_to_semantic` — той же функции, что используют уровни модели и
состояния. Новой логики преобразования **не вводится** (это гарантия R2:
паритет обеспечивается общим кодом, а не параллельной реализацией).

Планируемая правка (`grammar/src/semantic/statement.rs:266-269`):

```rust
ast::InlineFormulaDefine::Ltl { formulas, .. } => {
    let resolved: Vec<Formula> = formulas
        .iter()
        .map(|f| Formula::LTL(crate::semantic::formula::ltl_ast_to_semantic(f)))
        .collect();
    Ok(StatementNode::InlineFormula(resolved))
}
```

Замечания к реализации:
- Порядок формул в списке `: [LTL] φ1, φ2;` **сохраняется** (T3).
- `filter_map(...ok())`, как в ветке `Guard`, здесь **неуместен**:
  `ltl_ast_to_semantic` инфаллибельна (возвращает `Ltl`, не `Result`), поэтому
  тихого пропуска в новом коде не появляется — это соответствует цели фичи.
- Импорт `formula` при необходимости поднимается в `use` в шапке модуля
  (`docs/CODE.md`).
- `TODO`-комментарий удаляется полностью (A3).

**Статус по функциональности (правило 11):**

| Функциональность | Работа | Обоснование |
|---|---|---|
| Семантика (`grammar`) | **да** | Ветка `Ltl` в `statement.rs` |
| Грамматика `.lam` | **н/п** | Синтаксис не меняется; `grammar.lalrpop` не трогается (R7, A8) |
| Версия языка | **н/п** | Роста нет — 0.2.0 (правило 22; прецедент ADR 0025) |
| Генератор C | **н/п** в этой задаче | Ветка `Formula::LTL(_)` остаётся немой; диагностика — задача 0035-02 |
| Форматтер | **н/п** в этой задаче | Печать `Statement::InlineFormula` — задача 0035-03 |
| Симулятор (`simulation`) | **н/п** | Игнорирует `InlineFormula` до и после; потребление — предмет фичи 0044 |
| LSP | **н/п** | `lsp.rs:1400` игнорирует `InlineFormula`; поведение прежнее |
| Верификатор | **н/п** | Потребление вне границ 0035 (расширение 0010) |

**Тесты (пишутся в этой же задаче), `grammar/tests/semantic_tests.rs`:**

1. `test_inline_formula_ltl_in_block_resolved` — **сторож против тихой потери**
   (T1/A1). `construct_model` на
   `var a: bit := 0; var b: bit := 0; start S { always { : [LTL] G a, F b, a U b; } }`,
   достаёт узел блока, требует `formulas.len() == 3` и все три —
   `Formula::LTL`. **При возврате `Vec::new()` падает (`0 != 3`)** — именно этого
   теста сегодня нет.
2. `test_inline_formula_ltl_levels_parity` — паритет (T2/A2): `Ltl` из блока,
   модели и состояния для `G (a -> F b)` попарно равны через `PartialEq`.
3. `test_inline_formula_ltl_operators` — полнота узлов и приоритетов (T4).
4. `test_inline_formula_ltl_order` — порядок формул в списке (T3).
5. `test_inline_formula_guard_in_block_unchanged` — регрессия `Guard` (T5).
6. `test_inline_formula_mixed_block` — смешанный блок (T6).

Методика (`CLAUDE.md`): **сперва зонд** для захвата реальной структуры `Ltl`,
затем assertions против захваченных значений — строки и деревья не угадывать.
Утверждаются **значения** (состав `Vec<Formula>`, форма `Ltl`), а не факт
наличия узла АСД: тест «на факт» и есть причина, по которой дефект дожил
(урок 0025).

## Проверки

> **Планируется (разработка не начата).** Ожидаемые результаты — из тест-плана.

```sh
cargo build --bin lamc
cargo test test_inline_formula -- --test-threads=1     # T1–T6 зелёные
cargo test -- --test-threads=1                          # без регрессий
cargo test --features lsp -- --test-threads=1
./scripts/precheck.sh                                   # правило 5
```

Ожидаемые результаты и соответствие анализу:

| Проверка | Ожидание | R / A / T |
|---|---|---|
| `test_inline_formula_ltl_in_block_resolved` | Зелёный; при откате правки — падает `0 != 3` | R1 / A1 / T1 |
| `test_inline_formula_ltl_levels_parity` | Три `Ltl` попарно равны | R2 / A2 / T2 |
| `grep -n "TODO: Реализовать поддержку LTL" grammar/src/semantic/statement.rs` | Пусто | R1 / A3 / T8 |
| `git diff --stat grammar/src/grammar.lalrpop` | Пусто (синтаксис не тронут) | R7 / A8 / T23 |
| Сборка `examples/` в `c`/`c-hal` | `.c`/`.h` байт-в-байт как до правки | R6 / A7 / T22 |
| `ltl_tests.rs`, `test_inline_formula_{model,state}_resolved` | Остаются зелёными | R9 / A10 / T25 |

**Контрольная проба вручную** (повтор пробы из «Что было» — та же модель,
`lamc compile -t c`): после 0035-01 вывод `.c` **остаётся прежним** (LTL всё ещё
не эмитится — этим занимается генератор), но формула теперь **доходит до
семантики**. Тишину в генераторе снимает задача 0035-02 — до её выполнения фича
**не закрывается** (риск Р1 анализа: иначе тихая потеря просто переезжает
на уровень ниже).
