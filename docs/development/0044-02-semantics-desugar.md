# Задача 0044-02: Семантика `invariant` — десахаризация и диагностики

> Фича: [../features/0044-sim-assert-invariant.md](../features/0044-sim-assert-invariant.md) · ADR: [../adr/0044-sim-assert-invariant.md](../adr/0044-sim-assert-invariant.md) · анализ: [../analyze/0044-sim-assert-invariant.md](../analyze/0044-sim-assert-invariant.md)

## Что было

*Реальное состояние кода на 2026-07-15 (ветка `v2`).*

- **Именованные условия (проход 3)** — образец для десахаризации.
  `ModelElement::Condition` захватывается в `semantic/tree.rs:379`, имя
  кладётся в локальный `HashMap` (`tree.rs:104`, `tree.rs:398`) → присваивается в
  `ModelNode::conditions` (`tree.rs:588`). Разрешение — `tree.rs:751–754`
  (`extract_conditions` вызывается **дважды** — так разрешаются условия,
  ссылающиеся на условия).
- **Формулы модели** — `ModelNode::formulas: Vec<Formula>` (`semantic/mod.rs:117`);
  захват `ModelElement::InlineFormula` — `tree.rs:564–582`:
  `Guard{conditions}` → `Formula::Guard(ConditionNode::Unresolved(cond))`,
  `Ltl{formulas}` → `Formula::LTL(formula::ltl_ast_to_semantic(f))`.
- **Формулы состояния** — `StateNode::Simple{ formulas }` (`mod.rs:1040`);
  захват `StateElement::InlineFormula` — `tree.rs:1181–1195` (та же развилка).
- **Формулы оператора** — `semantic/statement.rs:256–271`:
  ветка `Guard` (стр. 258–265) разрешает условия и строит
  `StatementNode::InlineFormula(Vec<Formula>)`; ветка `Ltl` (стр. 266–269) —
  `// TODO: Реализовать поддержку LTL в блоках кода` → `Vec::new()` (**тихая
  потеря**; это предмет фичи **0035**, а не 0044).
- **`Formula`** — `semantic/formula.rs:15–24`: `None | Formulas(Vec<Formula>) |
  LTL(Ltl) | Guard(ConditionNode)`. `ltl_ast_to_semantic` (`formula.rs:51`)
  переводит `LtlExpr::Atom(id)` → `Ltl::Atom(id.name.clone())` (`formula.rs:55`)
  — **атом строится из голого имени**, что и делает имя инварианта пригодным
  для LTL.
- **Диагностики:** последний занятый код — **SE-052** (фича 0020,
  `port_address_completeness`). SE-053 и далее — свободны.
- **Дефект по соседству:** `tree.rs:398` кладёт `cond` через `HashMap::insert`
  **без** проверки существования → одноимённые объявления молча перезаписывают
  друг друга. Для 0044 это значит: SE-053 нужно писать явно, «оно само» не
  сработает.

## Что сделано

> **Выполнено** (2026-07-16).

Десахаризация по ADR 0044: `invariant P = C;` в области `S` ≡ `cond P = C;` +
`: [Guard] P;` в той же области.

1. **Захват в `tree.rs`** рядом с `ModelElement::Condition` (стр. 379) и
   `StateElement::InlineFormula` (стр. 1181):
   - имя `P` → в тот же `HashMap` условий, что и `cond` (`tree.rs:104/398`);
   - `Formula::Guard(ConditionNode::Unresolved(<ссылка на P>))` → в
     `ModelNode::formulas` / `state_formulas`.
2. **Разрешение** — бесплатно: уже существующий двойной `extract_conditions`
   (`tree.rs:751–754`) разрешит и условие инварианта, и ссылку на него.
3. **SE-053** — коллизия имени: перед вставкой проверять `contains_key` в
   условиях/переменных/портах области; при совпадении — `Diagnostic::error` с
   позицией и кодом (`.with_code("SE-053")`), по образцу SE-012 (`tree.rs:1172–1179`).
4. **SE-054** — инвариант ссылается на неизвестное имя: наследуется от
   разрешения условий; убедиться, что диагностика **не теряется** и несёт
   позицию инварианта, а не внутреннего `cond`.
5. **АСД не переписывается** (R10) — десахаризация живёт **только** в
   семантическом дереве. Иначе форматтер напечатает `cond`+`: [Guard]` вместо
   `invariant` (нарушение ADR 0024).
6. **LSP** (`lsp.rs:1400`): добавить `ModelElement::Invariant(_)` в `match` —
   иначе сборка с `--features lsp` не пройдёт (перечисление не исчерпано).
   Минимально — пустая ветка; индексация имени — приятный бонус, не требование.

**Статус по функциональности (правило 11):**

- `grammar` — основная работа; `verification/` — **не трогается ни строкой**.
- `simulation` — н/п (потребление — задача 0044-03).
- Ветка `Ltl` в `statement.rs:266–269` — **не трогается** (зона фичи 0035).

## Проверки

> **Планируется (разработка не начата).**

- **T8 (A4)** — `ModelNode::conditions` содержит `P`; `ModelNode::formulas`
  содержит `Formula::Guard`.
- **T9 (A5)** — `invariant P = t <= 100; : [LTL] G(P);` →
  `Formula::LTL(Ltl::Globally(Ltl::Atom("P")))`. **Ключевой тест фичи:** до 0044
  реляционное условие внутри LTL невыразимо (`LtlPrimary`,
  `grammar.lalrpop:330–334`).
- **T10 (A6)** — `ref Next: P;` разрешается, переход срабатывает.
- **T11 (A7)** — контрпример `invariant_name_clash.lam` → **SE-053**, а **не**
  тихая перезапись.
- **T12 (A8)** — контрпример `invariant_unknown_var.lam` → **SE-054** с позицией.
- **T6 (A3)** — `invariant P = x = 1;`: `=` есть равенство → `x` не изменился
  (проверяется значением, задача 0044-03).
- Сборка с LSP: `cargo test --features lsp -- --test-threads=1`.

```sh
cargo test -- --test-threads=1
cargo test --features lsp -- --test-threads=1
```

Соответствие анализу: **R5–R10** → критерии **A4, A5, A6, A7, A8**.
