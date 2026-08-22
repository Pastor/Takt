# Задача 0389-01: Массив в параметре функции цели `rust` — по ссылке

> Фича: [../features/0389-rust-array-by-reference.md](../features/0389-rust-array-by-reference.md) · ADR: [../adr/0389-rust-array-by-reference.md](../adr/0389-rust-array-by-reference.md) · анализ: [../analyze/0389-rust-array-by-reference.md](../analyze/0389-rust-array-by-reference.md)

## Что было

Цель `rust` печатала `fn total(a: [u8; 4])` и вызов `total(self.data)` — копия
массива на каждый вызов, тогда как `c` передаёт указатель, `st` — `VAR_IN_OUT`,
`sv` — плоский вектор.

## Что сделано

**1. Признак в своём модуле.** `generator/rust/rust_byref.rs`:
`is_array_by_reference(ty)` — массив, кроме упакованного бит-вектора
(`[bit;N ≤ 64]` — скаляр по правилу 0078).

**2. Оба печатника спрашивают его:** сигнатуру печатает `rust_func`, аргумент —
`rust_expr`. Разъедься они, порождённый код не собрался бы (`E0308`) — поэтому
признак вынесен, а не продублирован.

⚠️ **Вынос был ещё и обязателен по размеру:** `rust_expr.rs` после правки
достиг 1017 строк при лимите 1000, и гейт отверг прогон.

Статус по функциональности (правило 11): правка только у цели `rust`; эталон,
прочие цели и семантика не трогались.

## Проверки

```sh
cargo test --all-features
scripts/probe.sh -n 2 <проба>.takt      # rustc + clippy
./scripts/precheck.sh
git diff --exit-code examples/generated/
```

- **Форма** (T2, T3): `fn total(a: &[u8; 4])`, вызов `total(&self.data)`.
- **Инструменты** (T1): `rustc` принял, `clippy -D warnings` принял.
- **Корпус** (T7): диффа нет — функций с массивом в параметре в `examples/` нет.
