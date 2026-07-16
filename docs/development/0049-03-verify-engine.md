# Задача 0049-03: Движок верификации verify_model

> Фича: [../features/0049-model-checking-ltl.md](../features/0049-model-checking-ltl.md) · ADR: [../adr/0049-model-checking-ltl.md](../adr/0049-model-checking-ltl.md) · анализ: [../analyze/0049-model-checking-ltl.md](../analyze/0049-model-checking-ltl.md)

## Что было

Звенья по отдельности: `build_kripke` (0049-01), `product` + `emptiness`
(0049-02), `build_buchi` (`buchi.rs:327`, затравка). `build_buchi` **не
вызывается** из продуктового пути. Отрицание — `to_nnf` (`buchi.rs:6`)
разворачивает `Not` до атомов.

## Что сделано

> **Готово.** Реализовано по ADR 0049.

1. Модуль `verification/verify.rs`:
   ```rust
   pub enum Verdict { Holds, Violated(Counterexample), Unsupported(Vec<String>), NoStartState }
   pub struct Counterexample { pub prefix: Vec<String>, pub cycle: Vec<String> }
   pub fn verify_model(model: &ModelNode, phi: &Ltl) -> Verdict;
   pub fn verify_model_traced(model: &ModelNode, phi: &Ltl) -> (Verdict, String);
   ```
   **Отличия от плана:** (а) добавлен вариант `NoStartState` — модель без
   стартового состояния (`build_kripke → None`, задача 0049-01); (б) движок
   вынесен в отдельный модуль `verify.rs`, а не в `mod.rs`/`check.rs`: `check.rs`
   отвечает за пустоту, `verify.rs` — за вердикт; (в) добавлен
   `verify_model_traced` — дамп конвейера для отладки (`--trace`).
2. **Связывание атомов (R2/R7).** `collect_atoms` (`ltl_check.rs`, стал
   `pub(crate)` — общий источник истины с SE-056) + `Kripke::unknown_atoms`;
   атом, не являющийся именем состояния, → `Verdict::Unsupported([имена])`.
3. **Конвейер (R3–R5):** `A = build_buchi(&Ltl::Not(phi))` → `product` →
   `emptiness` → `Holds` / `Violated(lasso)`.
4. Публичный вход (R11) в `lib.rs`: `verify_model` (одна формула),
   `verify_all` / `verify_all_traced` (все формулы модели **и вложенных**;
   формула вложенной модели проверяется против **её** графа), `PropertyResult`,
   `model_ltl_formulas`, `parse_ltl_property`.
5. `verification/mod.rs` дополнен `pub mod {kripke, product, check, verify};`.
6. **Читаемость контрпримера.** Спроецированное лассо приводится к
   минимальной записи **того же** ω-слова: цикл сжимается до минимального
   периода (`[F, F]` → `[F]`, т.к. `p^k` и `p` задают одно `p^ω`), хвост
   префикса, дублирующий цикл-самопетлю, отбрасывается (`x · x^ω = x^ω`).
   Для периода длиннее 1 правка **не** применяется — `A W · (W K)^ω ≠ A · (W K)^ω`
   (тест-сторож `non_periodic_cycle_is_left_intact`). Меняется запись, не слово:
   иначе это была бы ложь о прогоне (правило 5).

**Общий обход формул.** `ltl_check.rs` переработан: обход мест объявления LTL
(модель / состояния / именованные блоки / тела функций) вынесен в
`model_ltl_formulas` и переиспользован диагностиками (SE-055/SE-056) и
верификацией. Паритет уровней (наследие 0035-01) держится одним кодом: новое
место объявления формулы нельзя добавить в один потребитель и забыть в другом.

## Проверки

Юнит-тесты `verification/verify.rs::tests` (17, зелёные) — **на вердикт**, а не
на форму автомата (капкан 0025):

- известные теоремы (A7): `F Done` держится при неизбежности; нарушено при
  обходимости и при недостижимости; `G A` держится/нарушено;
- управляющие свойства (A2): `G (Fault -> F Idle)` держится при гарантированном
  возврате и нарушено при залипании (контрпример — вечный `Fault`);
- тупик со stutter: `G F Done` на конечном состоянии держится (A2);
- честная граница (A4): атом-переменная и опечатка → `Unsupported`; в смешанной
  формуле перечисляются только неподдержанные атомы; формула без атомов
  (`G true`) проверяется;
- запись контрпримера: сжатие периода, неприкосновенность непериодичного цикла;
- детерминизм: 10 прогонов — один вердикт (A6);
- `verify_model_traced` печатает все три звена конвейера.

Соответствие анализу: R2, R3, R7, R11; критерии A2, A3, A4, A6, A7, A8;
тест-план T12–T22, T24, T34–T35.
