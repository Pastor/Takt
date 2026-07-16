# Задача 0057-04: Потактовая сверка и гейт для `+` в SV

> Фича: [../features/0057-sv-sequential-composition.md](../features/0057-sv-sequential-composition.md) · ADR: [../adr/0057-sv-sequential-composition.md](../adr/0057-sv-sequential-composition.md) · анализ: [../analyze/0057-sv-sequential-composition.md](../analyze/0057-sv-sequential-composition.md) · тест-план: [../tests/0057-sv-sequential-composition.md](../tests/0057-sv-sequential-composition.md)
>
> Зависит от [0057-02](0057-02-emit-concatenation.md); заводится **вместе** с
> кодогеном, а не после (урок 0045/0050: гейт доказывает валидность, не верность).

## Что было

`simulation/tests/conformance_sv_tests.rs` сверяет трассу симулятора с трассой
порождённого SV потактово (образец `per_tick_trace_matches_generated_sv`), но
покрывает только модели без `+`. `scripts/precheck.sh` держит `extend_complex`
вне `SV_TRANSLATABLE` (он не транслировался). После доделки 0045-01 в каталоге
`examples/generated/sv/` — зеркальный сторож (ровно `SV_TRANSLATABLE`).

## Что сделано

- **Сверка `A + B`** (глубина 1): минимальная фикстура, потактовое сравнение
  трасс симулятора и SV; проверить тайминг активации (шаг 2 меняет сигналы на
  такте после завершения шага 1) и переход родителя.
- **Сверка `extend_complex`** (`A → B → (C|D) → E`, 6 тактов): трассы совпадают;
  переход к `E` — только когда C и D завершены; переход корня в `Next` — только
  по завершении `E`. Наблюдение — иерархической ссылкой `dut.<сигнал>`.
- **Гейт:**
  - добавить `extend_complex` в `SV_TRANSLATABLE` (`precheck.sh`) — теперь
    транслируется; зеркальный сторож (0045-01) сам потребует наличия
    `extend_complex.sv`;
  - `verilator --lint-only -Wall` и `yosys synth -top` по нему — зелёные;
  - гейт воспроизводимости (0048) — два прогона `-t sv` равны.
- **Аддитивность:** `stacker.sv`/`elevator_mini.sv` байт-в-байт неизменны
  (`git diff` пуст).
- **Мягкая деградация:** без verilator/yosys сверка-пиннинг трассы симулятора всё
  равно исполняется (образец `cc_available`).

**Статус по обратной функциональности (правило 11):** расширяются
`conformance_sv_tests.rs` и `precheck.sh`; регенерируется
`examples/generated/sv/extend_complex.sv`. Существующие сверки и примеры — не
трогаются.

## Проверки

- `cargo test -- --test-threads=1` (сверки `conformance_sv_tests`) зелёный.
- `./scripts/precheck.sh` зелёный целиком (гейт sv + воспроизводимость).
- `git diff examples/generated/sv/{stacker,elevator_mini}.sv` пуст (A7).
- Связь с A2/A3/A6/A7 тест-плана (T5–T8, T13–T15).
