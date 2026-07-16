# Задача 0049-03: Движок верификации verify_model

> Фича: [../features/0049-model-checking-ltl.md](../features/0049-model-checking-ltl.md) · ADR: [../adr/0049-model-checking-ltl.md](../adr/0049-model-checking-ltl.md) · анализ: [../analyze/0049-model-checking-ltl.md](../analyze/0049-model-checking-ltl.md)

## Что было

Звенья по отдельности: `build_kripke` (0049-01), `product` + `emptiness`
(0049-02), `build_buchi` (`buchi.rs:327`, затравка). `build_buchi` **не
вызывается** из продуктового пути. Отрицание — `to_nnf` (`buchi.rs:6`)
разворачивает `Not` до атомов.

## Что сделано

> **Планируется (разработка не начата).** План по ADR 0049.

1. Публичная функция (по образцу `grammar::ltl_warnings`):
   ```rust
   pub enum Verdict { Holds, Violated(Counterexample), Unsupported(Vec<String>) }
   pub fn verify_model(model: &ModelNode, phi: &Ltl) -> Verdict;
   ```
2. **Связывание атомов (R2/R7).** Перед проверкой собрать атомы формулы
   (`collect_atoms`, `ltl_check.rs:172`); атом, не являющийся именем состояния
   модели, → `Verdict::Unsupported([имена])` (честная граница: не молча false).
3. **Конвейер (R3–R5):** `A = build_buchi(&Ltl::Not(Rc::new(phi.clone())))` →
   `product(&kripke, &A)` → `emptiness` → `Holds` при пустоте, `Violated(lasso)`
   при непустоте.
4. Реэкспорт `grammar::verify_model` в `lib.rs` (публичный вход, R11).
5. `verification/mod.rs` дополняется `pub mod {kripke, product, check};`.

## Проверки

- Юнит-тесты на построенных вручную моделях: `G(a -> F b)` держится/нарушено;
  `F Done` по достижимости; `Unsupported` для атома-переменной.
- Детерминизм (гейт 0048): один вердикт на 10 прогонов.
- Соответствие анализу: R2, R3, R7, R11; критерии A2, A3, A4, A7, A8.
