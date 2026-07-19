# Задача 0096-03: цели `c`/`rust`/`st` — embedded-путь `float → q(m, n)` при `--float-embedded`

> Фича: [../features/0096-fixed-point-native-float.md](../features/0096-fixed-point-native-float.md) · ADR: [../adr/0096-fixed-point-native-float.md](../adr/0096-fixed-point-native-float.md) · тест-план: [../tests/0096-fixed-point-native-float.md](../tests/0096-fixed-point-native-float.md)

> **Задача 0096-03 реализована** (2026-07-19). Ниже — что сделано.

## Что было

Задача 0096-02 применила трансформацию `float → q(m, n)`
(`semantic::lower_float`) к цели `sv`. Цели `c`/`rust`/`st` её не вызывали — на
`float` давали нативный `double`/`f64`/`LREAL` **всегда**.

## Что сделано

### Точки применения (embedded-гейт)

Общий помощник `lib.rs::apply_float_lowering(model, options, embedded_gate)`:
трансформация применяется при `options.float_as_q == Some((m, n))` **и**
(`embedded_gate == false` **или** `options.float_embedded == true`). Вызовы:

| Цель | Функция | `embedded_gate` |
|---|---|---|
| `sv` | `compile_to_sv` | `false` (флаг применяется всегда, 0096-02) |
| `c` | `compile_to_c` | `true` |
| `c-hal` | `compile_to_c_hal` | `true` (основная embedded-цель) |
| `st` | `compile_to_st` | `true` |
| `st-at` | `compile_to_st_at` | `true` |
| `rust` | `compile_to_rust` | `true` |

Итог: `c`/`rust`/`st` без `--float-embedded` — прежний native (`double`/`f64`/
`LREAL`); с `--float-embedded` + `--float-as-q=m.n` — целочисленный `q(m, n)`
(reuse 0061 кодогена `c_expr/fixed`, `rust_fixed`, `st_fixed`).

**Зонд** (`--float-as-q=8.8 --float-embedded`): `c` → `int16_t acc`, `rust` →
`acc: i16` + `(x as i128 * two as i128) >> 8`, `st` → `acc : INT` +
`LAM_Q_FLOORDIV(...)`. Без флага — `double`/`f64`/`LREAL`.

### Сверки (тест-план)

Трансформация даёт **тот же** AST, что и явный `q(m, n)`, поэтому per-target
Q-кодоген уже проверен 0061. Проверяем, что float-путь его действительно даёт:

| Цель | Тест | Способ |
|---|---|---|
| `c` | `conformance_c_tests::float_embedded_q_matches_generated_c` | **потактовая** сверка симулятора (Q-режим) и порождённого C (`cc`), трасса = явная q-версия (`-768,-384,-2,510`) |
| `rust` | `conformance_rust_tests::float_embedded_matches_explicit_q_rust` | **byte-equality**: вывод float+embedded == вывод q-двойника (одинаковый basename) |
| `st` | `conformance_st_tests::float_embedded_matches_explicit_q_st` | **byte-equality** (то же) |

⚠️ **Почему rust/st — byte-equality, а не runtime.** У цели `rust` поля модели
приватны, а q-**выходной порт** не поддержан (`RS-016`: порт — только бит/число),
поэтому Q-модель без портов рантайм-наблюдать нечем. Byte-equality с проверенным
q-двойником (`conformance_float_q_twin.lam` — та же модель на явном `q(8, 8)`)
доказывает, что float→q даёт **ровно** проверенный 0061 q-кодоген. У `c` поля
публичны → полная потактовая сверка возможна (сделана).

### Двухрежимный эталон и native-гейты (`conformance_float_modes_tests.rs`)

- `float_native_and_q_modes_differ` (T8/T9/R7): без трансформации `acc` —
  `Value::Real` (native f64); с ней — `Value::Fixed { repr: -768 }` (q). **Разные**
  численные семантики → сверка ведётся ВНУТРИ режима. Сторож направления: мутация
  «эталон native, цель Q» дала бы `Real` там, где Q-цель ждёт repr.
- `float_as_q_without_embedded_is_native_c` + `..._rust` + `..._st` (T5/A3): без
  `--float-embedded` вывод остаётся native (`double`/`f64`/`LREAL`) — Q-путь только
  со вторым флагом, молчаливого Q нет.

## Заметка о «двухрежимном симуляторе» (ADR A-1)

`eval::fixed` **не трогался**. Q-режим эталона достигается **той же**
трансформацией над моделью симулятора перед `build_unit` (Q = проход применён,
native = не применён). То есть «двухрежимность» — свойство подаваемой модели, а не
двух веток вычислителя: `float`(Rational) → `Value::Real`, `q`(Fixed) →
`Value::Fixed`. Риск рассинхрона (A-1) снят конструктивно — режим один и тот же
проход на обеих сторонах.

## Проверки

| # | Проверка | Тест | Статус |
|---|---|---|---|
| T1 | Корпус без флагов неизменен | `git diff examples/generated` пуст | ✅ |
| T5 | `c`/`rust`/`st` native по умолчанию | `float_as_q_without_embedded_is_native_{c,rust,st}` | ✅ |
| T6 | Embedded Q побитово | `float_embedded_q_matches_generated_c` (runtime) + `float_embedded_matches_explicit_q_{rust,st}` (byte) | ✅ |
| T8 | Двухрежимный эталон | `float_native_and_q_modes_differ` | ✅ |
| T9 | Сторож направления | тот же (Real ≠ Fixed) + native-гейты | ✅ |
| T10 | Явный `q(m,n)` не задет | сверки 0061 зелены; гейт исходного типа (0096-02) | ✅ |

## Осталось (0096-04)

Корпусной пример-регулятор на `float` (native c/rust/st + синтезируем под `sv` с
yosys-гейтом), README, `CLAUDE.md` (ослабление 0042, A-7), отчёт — закрытие фичи.
