# Задача 0057-01: Регистр шага и его enum в `Fsm`/минимапе

> Фича: [../features/0057-sv-sequential-composition.md](../features/0057-sv-sequential-composition.md) · ADR: [../adr/0057-sv-sequential-composition.md](../adr/0057-sv-sequential-composition.md) · анализ: [../analyze/0057-sv-sequential-composition.md](../analyze/0057-sv-sequential-composition.md)
>
> Порядок: **первая** — данные до кодогена. Далее [0057-02](0057-02-emit-concatenation.md).

## Что было

`Fsm::build` (`grammar/src/generator/sv/sv_fsm.rs:292-399`) заводит регистры для
корня и каждой под-модели (`state_reg_name`), переменных (`var_signal_name`) и
выходных портов, но **не знает про цепочки `+`**: `StateExtend::Concatenation`
доходит только до `emit_extend`, где отвергается.

## Что сделано

- **Обход композиции при сборке `Fsm`.** Пройти дерево `StateExtend` каждого
  состояния и для каждой встреченной `Concatenation` (включая вложенные)
  аллоцировать **регистр шага** — новый `Reg`:
  - имя: `<уникальное-имя-несущего-состояния>_step` (ключ уникален на площадку
    композиции; при вложенности — по несущему состоянию/индексу);
  - тип-префикс: `typedef enum logic [w:0] { <STATE>_STEP_0 … <STATE>_STEP_{N-1} }`,
    ширина `w` — из числа шагов (та же `enum_width`, что у состояний);
  - `reset = <STATE>_STEP_0`, `declare_reg = true`, пара `_next` как у прочих.
- **Enum шага** эмитировать рядом с `emit_state_enums` (тем же порядком —
  детерминизм 0048).
- **Сброс** — регистр шага попадает в ветку `!rst_n` `emit_ff` автоматически, раз
  он в `fsm.regs` с `reset`.
- **Не течёт наружу.** Убедиться зондом: регистр шага не входит в `SvPorts.outputs`
  и не участвует в `assign is_done` (тот смотрит только на регистр состояния
  корня).

**Статус по обратной функциональности (правило 11):** затрагивается только
сборка `Fsm` цели `sv`; ветка `Parallel`, регистры уровней, переменные и порты —
не трогаются (аддитивно). Прочие цели и симулятор — н/п.

## Проверки

- Юнит-зонд генератора: на модели `start S = A + B` в выводе присутствуют
  `typedef enum … <state>_step_e`, объявления `<state>_step`/`<state>_step_next`,
  строка сброса `<state>_step <= …STEP_0`.
- Регистр шага **отсутствует** в объявлениях `output` и в `assign is_done`.
- `cargo test --features …` (юнит генератора SV) зелёный; связь с A4 тест-плана.
