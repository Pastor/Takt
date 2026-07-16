# Задача 0057-02: `emit_extend` для `Concatenation` — инлайн активного шага

> Фича: [../features/0057-sv-sequential-composition.md](../features/0057-sv-sequential-composition.md) · ADR: [../adr/0057-sv-sequential-composition.md](../adr/0057-sv-sequential-composition.md) · анализ: [../analyze/0057-sv-sequential-composition.md](../analyze/0057-sv-sequential-composition.md)
>
> Зависит от [0057-01](0057-01-step-register.md) (регистр шага уже собран). Далее [0057-03](0057-03-nesting-diagnostics.md).

## Что было

`emit_extend` (`grammar/src/generator/sv/sv_fsm.rs`, ветка
`StateExtend::Concatenation`) возвращает `SV-002`. Ветка `StateExtend::Parallel`
(~817–849) — рабочий образец: инлайнит тела под-моделей через `emit_model_body`,
строит done-выражение (`<sub>_state_next == end_variant(sub)`) и переводит
родителя, когда done всех ветвей истинно.

## Что сделано

Заменить отказ на генерацию (по ADR 0057, Option A):

- **`case (<step>)`** внутри `case`-ветви несущего состояния; регистр `<step>` —
  из 0057-01.
- **Каждый `STEP_i`** инлайнит тело **только** шага `i`:
  - `StateExtend::Model(sub)` → `emit_model_body(sub)` (как в `Parallel`);
  - `StateExtend::Parallel(items)` → существующая логика `Parallel` (инлайн всех
    ветвей, done = конъюнкция);
  - `StateExtend::Concatenation(_)` (вложенная `+`) → рекурсия — свой регистр
    шага (координация с 0057-03).
- **Done-выражение шага** строить **на `_next`** (как в `Parallel` — иначе
  повторится дефект SV-`|`: чтение регистра даёт значение прошлого такта).
- **Продвижение:** в `STEP_i` при done: не последний → `<step>_next = STEP_{i+1}`;
  последний → **переход родителя** общим хвостом `emit_extend` (тот же код, что
  выполняет переход после `Parallel`).
- **Тайминг:** смена `<step>_next` в такте T → шаг `i+1` активен с T+1 (регистр
  защёлкнулся). Под-автомат шага `i+1` держится на старте (значение сброса) →
  его тело/`enter` исполнятся первым тактом активности через `emit_model_body`
  (контракт 0033), без явного `_init`. Совпадает с `break` в C
  `generate_concat_tick`.

**Статус по обратной функциональности (правило 11):** меняется ровно ветка
`Concatenation` в `emit_extend`; ветка `Parallel` и её хвост переиспользуются без
правок. Прочие цели/симулятор — н/п.

## Проверки

- Зонд вывода на `A + B` и `extend_complex`: присутствует `case (<step>)`, в
  каждом шаге инлайн тела, продвижение `<step>_next`, на последнем — переход
  родителя.
- verilator `--lint-only -Wall` и yosys `synth` принимают вывод (A3).
- Потактовая сверка — задача 0057-04 (заводится **вместе** с этим кодогеном, а
  не после: урок 0045/0050).
- `cargo test` (юнит генератора) зелёный.
