# Задача 0304-01: Вывод типа локального объявления и объявления в `sv`

> Фича: [../features/0304-local-declaration-type-inference.md](../features/0304-local-declaration-type-inference.md) · ADR: [../adr/0304-local-declaration-type-inference.md](../adr/0304-local-declaration-type-inference.md) · анализ: [../analyze/0304-local-declaration-type-inference.md](../analyze/0304-local-declaration-type-inference.md)

## Что сделано

**1. `semantic/statement/mod.rs`** — при понижении `ast::Statement::Variable`
тип `TypeNode::Inference` заменяется выведенным из инициализатора
(`type_inference::extract_type`).

⚠️ Выводить надо **здесь**, а не общим проходом: локальное объявление живёт в
теле блока, а не в таблице модели, и к моменту `type_inference` тела ещё не
построены.

**2. `generator/sv/sv_blocks.rs` + `sv_stmt.rs`** — тела состояний и модели
получают объявления локальных переменных: новый `emit_hoisted_locals_auto`
печатает `automatic logic …` перед операторами.

⚠️ Это **отдельный дефект**, не зависевший от первого: `hoist_locals` звался
только для тел функций, и цель печатала `g = (F + 1);` без объявления — при
**нулевом** коде возврата. Воспроизводится и на явном типе.

⚠️ `automatic` обязателен: тело печатается внутри ветви `unique case` в
`always_comb`, где обычное объявление было бы статической переменной. Форма
проверена **обоими** инструментами (`verilator --lint-only -Wall` и
`yosys -p synth`) — урок 0235.

**3. Сверка** `takt-sim/tests/conformance/conformance_local_decl_tests.rs`:
потактовые трассы эталона и цели `c` на **накапливающем** теле (`sum := sum + g`)
плюс проверка валидности вывода `sv` линтом.

## Проверка

```sh
sh scripts/probe.sh -n 2 <модель с локальным объявлением>
cargo test --test conformance conformance_local_decl_tests::
```
