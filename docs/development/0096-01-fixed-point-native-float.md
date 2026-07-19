# Задача 0096-01: Q-арифметика через нативный float и флаг генерации (embedded ↔ float)

> Фича: [../features/0096-fixed-point-native-float.md](../features/0096-fixed-point-native-float.md) · ADR: [../adr/0096-fixed-point-native-float.md](../adr/0096-fixed-point-native-float.md) · анализ: [../analyze/0096-fixed-point-native-float.md](../analyze/0096-fixed-point-native-float.md)

> **Задача 0096-01 реализована** (2026-07-19); 0061 закрыта. Ниже — декомпозиция
> фичи и что сделано в первой задаче.

## Декомпозиция фичи (намечено)

| Задача | Объём |
|---|---|
| **0096-01** | CLI-флаги `--float-as-q=m.n` и `--float-embedded`: разбор, валидация границ (правило 1 ADR 0061), прокидывание в `GenerateOptions`. Пока **без** влияния на кодоген (инфраструктура) |
| **0096-02** | Цель `sv`: `float` → `q(m, n)` глобальной точности (переиспользует 0061-04); снятие `SV-003` для `float`; `conformance_sv` в float-режиме + гейты verilator/yosys |
| **0096-03** | Симулятор двухрежимный (`float` как `f64` / `q(m, n)`) + `c`/`rust`/`st` embedded-путь `float`→q (переиспользует 0061-03); `conformance_{c,rust,st}` в Q-режиме; сторож направления (T9) |
| **0096-04** | Пример-регулятор на `float` (синтезируем под `sv`), README, `CLAUDE.md` (ослабление 0042), отчёт — закрытие фичи |

## Задача 0096-01: CLI-флаги (инфраструктура)

### Что было

`GenerateOptions` (`grammar/src/generator/mod.rs`) несёт `hal`, `address_map`,
`float_width` (0029). Флага точности fixed-point для `float` нет; `--float-width`
задаёт лишь ширину нативного float (32/64).

### Что сделано

- `GenerateOptions` (`generator/mod.rs`): поля `float_as_q: Option<(u8, u8)>`,
  `float_embedded: bool` (`new`/`Default` — `None`/`false`).
- `lamc.rs`: разбор `--float-as-q=<m>.<n>` (обе формы) и `--float-embedded`;
  `parse_float_as_q` валидирует границы правила 1 ADR 0061 (`m ≥ 1`, `n ≥ 1`,
  `m + n ≤ 64`) и формат — иначе **ошибка CLI**, а не молчаливое умолчание (T3).
  Прокидывание в `GenerateOptions` (`generate_options`), строки help.
- Кодоген **не тронут**: вывод байт-в-байт прежний (T1). Поведение включают
  0096-02/03.

⚠️ `lamc.rs` (монолитный CLI, уже в реестре размера) вырос на CLI-флаги →
baseline синхронизирован; **вынос парсера аргументов в подмодуль** — отдельный
кандидат (тесты флагов зовут `parse_compile_args`, поэтому точечно не выносятся).

### Проверки

- `cargo build --bin lamc` ✅; юнит-тесты `parse_float_as_q_valid`,
  `float_as_q_defaults_to_none`, `float_as_q_rejects_out_of_bounds_and_bad_format`,
  `parse_float_embedded_flag` (валид/умолчание/контрпримеры T2/T3) — зелёные.
- `git diff examples/generated` пуст (T1) ✅.
