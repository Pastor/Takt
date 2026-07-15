# Задача 0044-04: Генератор C — нулевой регресс и сверка с симулятором

> Фича: [../features/0044-sim-assert-invariant.md](../features/0044-sim-assert-invariant.md) · ADR: [../adr/0044-sim-assert-invariant.md](../adr/0044-sim-assert-invariant.md) · анализ: [../analyze/0044-sim-assert-invariant.md](../analyze/0044-sim-assert-invariant.md)

## Что было

*Реальное состояние кода на 2026-07-15 (ветка `v2`). Главная находка ADR 0044:
работа здесь — почти нулевая, потому что всё уже написано.*

- **Эмиссия ассерта уже есть:** `generator/c/c_expr.rs:1424–1448`
  (`generate_formula_check`): `Formula::Guard(cond)` →
  `printer.ident(&format!("assert({});", cond_expr))` (стр. 1440);
  `Formula::LTL(_)` → `// LTL-формулы в C-коде пока не проверяются` (стр. 1443–1445);
  `Formula::Formulas` — рекурсия; `Formula::None` — ничего.
- **`#include <assert.h>` эмитится безусловно** — `generator/c/c_source.rs:19`
  (рядом `#include <math.h>`, стр. 20).
- **Обходы формул уже написаны — все три:**
  - модель: `c_model.rs:547–553` — `if map.guard_enable() { … for formula in
    &raw_model.borrow().formulas { generate_formula_check(…) } }`, **до**
    `printer.ident("switch (model->state) {")` (стр. 555);
  - состояние: `c_model.rs:665–670` — `for formula in raw_state.formulas()`,
    **до** `generate_named_blocks(…, "always")` (стр. 672);
  - оператор: `c_expr.rs:1693–1698` — `StatementNode::InlineFormula(formulas)`.
- **Флаг уже есть:** `map.guard_enable()`; CLI `--guard-enable` /
  `--guard-disable` (`bin/lamc.rs:169`, `208–212`, `249`), по умолчанию
  **включён**; пробрасывается через `GenerateOptions::new(options.guard_enable)`
  (`bin/lamc.rs:559`, `594`; `generator/mod.rs:23–46`).
- **Сверка с C существует:** `simulation/tests/conformance_c_tests.rs`.

**Вывод:** после десахаризации (0044-02) `invariant` становится обычной
`Formula::Guard` → генератор C **не правится ни строкой**. Задача 0044-04 — не
«написать», а **доказать**: регресс = 0 и симулятор с C согласны.

**Ловушки (`CLAUDE.md`), учтённые в проверках:**

- порождённый C заводит служебные `INIT`-такты → потактовые трассы **смещены**;
  сверять можно **только установившиеся** значения;
- `get_c_type` дефектен на `Array`/`Bit`/`Rational` → фикстуры conformance —
  только `Integer`/`Enum`/`Bool`.

## Что сделано

> **Планируется (разработка не начата).**

1. **Ноль правок эмиссии** (R17) — подтвердить, что `generate_formula_check`,
   `c_model.rs:549`, `c_model.rs:667`, `c_expr.rs:1693` работают на инвариантах
   без изменений. Если правка потребуется — это сигнал, что десахаризация
   (0044-02) сделана неверно (АСД переписан вместо семантики).
2. **Доказательство регресса** (R18): побайтовая сверка вывода `-t c` и
   `-t c-hal` на всех `examples/` до и после фичи.
3. **Тест эквивалентности** (R19): `compile_to_c(invariant P = C;)` ==
   `compile_to_c(cond P = C; : [Guard] P;)` — фикстуры
   `invariant_model.lam` и `invariant_equiv_cond_guard.lam`.
4. **Тест флага** (R20): `--guard-disable` → в выводе нет `assert(`.
5. **Conformance** (R11, R12): фикстура T14, скомпилированная в C и прогнанная,
   даёт то же **установившееся** значение, что симулятор.

**Статус по функциональности (правило 11):**

- `grammar/src/generator/c/` — **правок не планируется**; задача доказательная.
- `simulation/tests/conformance_c_tests.rs` — новый сценарий.
- Цель `c` — байт-в-байт как прежде. Цель `c-hal` (фича 0020) — то же.
- **Вне зоны 0044** (вынесено в кандидаты, ADR): `#ifdef LAM_ASSERT`
  (дублирует `--guard-disable`); колбэк `Lam_assert_failed()` вместо `assert.h`
  (смена контракта цели `c` = регресс для существующих моделей, правило 11);
  проверка `Formula::LTL` в C (`c_expr.rs:1443`).

## Проверки

> **Планируется (разработка не начата).**

- **T23 (A13) — регресс = 0.** До фичи снять эталоны, после — сверить:

  ```sh
  # ДО фичи (на чистой ветке)
  for f in examples/*.lam; do
    cargo run --bin lamc -- compile -t c     "$f" -o "/tmp/before/$(basename $f).c"
    cargo run --bin lamc -- compile -t c-hal "$f" -o "/tmp/before/$(basename $f).hal.c"
  done
  # ПОСЛЕ — тот же цикл в /tmp/after, затем:
  diff -r /tmp/before /tmp/after    # обязан быть пуст
  ```

- **T24 (A14)** — эквивалентность `invariant` и пары `cond` + `: [Guard]`:
  строки вывода равны.
- **T25 (A15)** — `--guard-disable` → `! вывод.contains("assert(")`.
- **T26** — `--guard-enable` (по умолчанию) → `assert(` присутствует **до**
  `switch (model->state)`.
- **T27 (A11) — conformance.** Компиляция фикстуры T14 в C, сборка
  `cc -std=c99 -Wall` (как в 0020-05), прогон; сверка **установившегося**
  значения с `Unit::variable`. **Не** сверять номера тактов (`INIT`-такты).
- Полный прогон codegen-снапшотов и `./scripts/precheck.sh`.

```sh
cargo test -- --test-threads=1
cargo test --test conformance_c_tests -- --test-threads=1
./scripts/precheck.sh
```

Соответствие анализу: **R17–R20** (+ подтверждение R11, R12) → критерии
**A11, A13, A14, A15**.
