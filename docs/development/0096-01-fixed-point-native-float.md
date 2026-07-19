# Задача 0096-01: Q-арифметика через нативный float и флаг генерации (embedded ↔ float)

> Фича: [../features/0096-fixed-point-native-float.md](../features/0096-fixed-point-native-float.md) · ADR: [../adr/0096-fixed-point-native-float.md](../adr/0096-fixed-point-native-float.md) · анализ: [../analyze/0096-fixed-point-native-float.md](../analyze/0096-fixed-point-native-float.md)

> **Стадия: проработка** — код не начат (фича ЗАБЛОКИРОВАНА до закрытия 0061).
> Ниже — декомпозиция и объём первой задачи.

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

### Что нужно сделать

- `lamc.rs`: разобрать `--float-as-q=<m>.<n>` (→ `Option<(u8, u8)>`) и
  `--float-embedded` (→ `bool`); валидация `m ≥ 1`, `n ≥ 1`, `m + n ≤ 64` (тот же
  предикат, что `construct_fixed` 0061-01) — иначе ошибка CLI (T3).
- `GenerateOptions`: поля `float_as_q: Option<(u8, u8)>`, `float_embedded: bool`.
- Пока **не** менять кодоген: задача инфраструктурная, вывод байт-в-байт прежний
  (T1). Поведение включают 0096-02/03.

### Проверки

- `cargo build --bin lamc`; юнит на разбор флага (валид/контрпримеры T2/T3).
- `git diff examples/generated` пуст (T1: флаги без значений ничего не меняют).
