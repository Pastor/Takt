# Задача 0078-01: Семантика `[bit;N]` — упакованный бит-вектор

> Фича: [../features/0078-bit-array-semantics.md](../features/0078-bit-array-semantics.md) · ADR: [../adr/0078-bit-array-semantics.md](../adr/0078-bit-array-semantics.md) · анализ: [../analyze/0078-bit-array-semantics.md](../analyze/0078-bit-array-semantics.md)

## Что было

`[bit;8]` трактовался пятью потребителями по-разному: C — скаляр `uint8_t`; rust —
`[bool;8]`; st — `ARRAY OF BOOL`; sv — распакованный массив; симулятор —
`Value::Array` из 8 значений. `[bit;N]` при N ∉ {8,16,32,64} → `CC-014` (невыразим).
Бит-доступ `x.k` в симуляторе работал только на скаляре. Референс `MATRIX: u8 :=
{…}` сталкивал три трактовки в одной строке.

## Что сделано

Реализация по [ADR 0078](../adr/0078-bit-array-semantics.md) (Option A) — `[bit;N]`
= упакованный N-битный вектор, правило упаковки в одном слое:

- **`grammar/src/semantic/bit_vector.rs`** (новый слой): `is_bit_vector(ty)` →
  `Some(N)` для `Array(N, Bit|Bool)`, N ≥ 1; `layout(n)` → `Scalar{round_up(n)}`
  (n ≤ 64) / `Words{⌈n/64⌉}` (n > 64); `round_up` (8/16/32/64); `bit_slot(k)` =
  `(k/64, k%64)`.
- **Цель C** (`c/mod.rs`): `bit_vector_type` → `uint{round_up}_t` (скаляр);
  `map_typed_variable` → `uint64_t name[⌈N/64⌉]` (слова). Вариант ошибки
  `CTypeError::BitVectorWidth` и код **`CC-014` удалены** (все N выразимы).
- **Rust** (`rust_type.rs`): `u{round_up}` / `[u64; count]`. **Порт** (`rust_port.rs`):
  скалярный бит-вектор нормализуется к `Integer` → порт-число (`RS-016` снят);
  слова остаются `Array` → `RS-016`.
- **ST** (`st_type.rs`): `USINT/UINT/UDINT/ULINT` / `ARRAY [0..count-1] OF ULINT`.
- **SV** (`sv_type.rs`): нативный `logic [N-1:0]` при любом N (слова не нужны).
- **Симулятор** (`eval/mod.rs`): `coerce_bit_vector` → `Value::Number` (N ≤ 64,
  как `coerce_integer`) / `Value::Array` слов (N > 64). Бит-чтение по словам —
  `eval/access.rs::read_bit_words`.
- **Реестр кодов (0077):** `CC-014` помечен `RETIRED` — гейт `check-diagnostic-codes`
  поймал рассинхрон (демонстрация работы 0077).
- **Референс** (`lib.rs::SRC`): `MATRIX: u8 := {…}` → `:= 0xA5` (скаляр-init).

**Статус по функциональности (правило 11):**

| Функциональность | Статус |
|---|---|
| Слой `bit_vector` | ✅ создан + юнит-тесты |
| C / Rust / ST / SV | ✅ упаковка (скаляр + слова/нативный SV) |
| Симулятор | ✅ coerce + бит-чтение по словам |
| Rust-порт | ✅ бит-вектор-скаляр = порт-число |
| Реестр кодов | ✅ CC-014 RETIRED |

## Проверки

```sh
cargo test -p grammar -p simulation -- --test-threads=1   # юнит + сверка бит-вектора
./scripts/precheck.sh                                      # итог (включая ST iec2c, детерминизм, коды)
```

- **A1** — `bit_vector.rs` тесты; **A2/A3** — `c/mod.rs` + пробы lamc rust/st/sv;
- **A4** — `conformance_c_bitvec_tests` (значение + биты сверены с C);
- **A5** — `rust_port` тесты; **A6/A7** — `precheck.sh` (extend_complex iec2c,
  детерминизм); **A8** — гейт кодов зелёный после RETIRED.
