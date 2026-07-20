# Задача 0098-01: диапазон бита адреса (`SE-060`) + безопасный дефолтный HAL

> Фича: [../features/0098-port-bit-range-safe-hal.md](../features/0098-port-bit-range-safe-hal.md) · ADR: [../adr/0098-port-bit-range-safe-hal.md](../adr/0098-port-bit-range-safe-hal.md) · закрывает [фикс 0020-01](../fixes/0020-01-port-bit-out-of-range.md)

## Что было

Бит адреса порта не проверялся; дефолтный HAL цели `c-hal` читал **один байт** и
сдвигал на `b.bit`: бит 8…31 → молча ноль, бит ≥ 32 → UB (`int >> 33`). `c-hal`
не компилировалась ни одним гейтом.

## Что сделано

1. **Диагностика `SE-060`** (`address_map/resolve.rs::resolve_model`): при
   финализированном бите вне `[0, 63]` — ошибка с локацией порта. В
   адрес-потребляющем слое (`c-hal`/`st-at`); цель `c` не задета (правило 4 ADR).
2. **Безопасный HAL** (`generator/c/c_hal.rs`, вынесен из `c_header.rs`): ширина
   доступа бит-порта — минимальное слово, содержащее бит (`word_bytes_for_bit`:
   1/2/4/8 байт), а не тип `bool`. `read_bit`/`write_bit` выбирают тип по
   `b.width` (`switch`), маска `(uint64_t)1u << s`. UB исключён конструктивно.
3. **Гейт компиляции `c-hal`** (`scripts/precheck.sh`): каждый пример
   генерируется в c-hal и компилируется `cc -c`. Примеры без адресов (SE-052) —
   пропуск.
4. **Вынос HAL в `c_hal.rs`**: `c_header.rs` упёрся в лимит размера модуля;
   удалён из `module-size-baseline.txt` (955 строк < 1000).

## Проверки

- **A1:** `SE-060` на бите 64, валидность бита 63 — `hal_bit_range_tests::bit_in_range_ok_out_of_range_is_se060`.
- **A2 (значенческий):** бит 33 читается `uint64_t`, результат «1001» — `hal_bit_range_tests::hal_reads_wide_bit_without_ub` (mmap-эмуляция MMIO; скип при недоступности).
- **Границы слова:** `c_hal::tests::bit_word_width_by_bit_index`.
- **A3:** гейт компиляции c-hal в `precheck.sh` (7 примеров компилируются, `elevator_mini` — пропуск SE-052).
- **A4:** `extend_complex.lam` не правлен (бит 33 обоснован); вывод цели `c` байт-в-байт прежний.
