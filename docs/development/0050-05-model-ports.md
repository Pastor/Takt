# Задача 0050-05: Модель и порты — `struct`, HAL-трейт, `in`/`out`/`inout`

> Фича: [../features/0050-rust-backend.md](../features/0050-rust-backend.md) · ADR: [../adr/0050-rust-backend.md](../adr/0050-rust-backend.md) · анализ: [../analyze/0050-rust-backend.md](../analyze/0050-rust-backend.md) · тест-план: [../tests/0050-rust-backend.md](../tests/0050-rust-backend.md)
>
> Требования: **R6**, **R12**. Критерии: **A4**, **A8**.

## Что было

*Проверено чтением порождённого кода 2026-07-16.*

Цель `c` (`examples/generated/c/elevator_mini.h`) описывает порты так:

```c
typedef enum { ELEVATOR_MINI_CABIN_CABIN_BUTTON_DC = 0, /* … 40 портов */ } ElevatorMini_In_BitPort;
typedef enum { ELEVATOR_MINI_CABIN_DOOR_OPEN = 0, /* … */ } ElevatorMini_Out_BitPort;

struct ElevatorMini {
    /* … переменные, состояние, под-модели … */
    void  *userdata;
    void  (*write_bit)(ElevatorMini_Out_BitPort port, bool val, void *userdata);
    bool  (*read_bit )(ElevatorMini_In_BitPort port, void *userdata);
};
```

Плата: `void *userdata` — стёртый тип, за корректность приведения отвечает
пользователь; забыть инициализировать указатель на функцию = вызов по нулевому
адресу.

Режим `c-hal` (фича 0020-05) вместо колбэков эмитит таблицу адресов и доступ
через `*(volatile T*)addr`. Цель `rust` карту адресов **не потребляет** (решение
ADR): это аналог режима `c`, а не `c-hal`; MMIO — кандидат `rust-hal`.

## Что сделано

> **Планируется (разработка не начата).** План по ADR 0050.

1. **Enum'ы портов** — по образцу C, но в CamelCase: `pub enum InBitPort { … }`,
   `pub enum OutBitPort { … }`. Порядок вариантов — из `BTreeMap` (детерминизм,
   гейт 0048).
2. **HAL-трейт** вместо пары указателей и `userdata`:

   ```rust
   pub trait Hal {
       fn read_bit(&mut self, port: InBitPort) -> bool;
       fn write_bit(&mut self, port: OutBitPort, value: bool);
   }
   ```

   Порты не-битовых типов — по методу на тип (`read_u8`/`write_u8`, …), состав
   определяется фактическим набором портов модели.
3. **`struct` модели** с параметром типа:

   ```rust
   pub struct ElevatorMini<H: Hal> {
       command: Command,       // переменные модели
       current_floor: u8,
       state: RootState,       // состояние — enum (0050-06)
       cabin: CabinState,      // под-модели — поля
       motor: MotorState,
       hal: H,                 // ← вместо void *userdata
   }
   ```

   `userdata` **исчезает как понятие**: состояние HAL живёт в самом `H`,
   типобезопасно, инициализация обязательна (конструктор требует `hal`) —
   «забыть колбэк» невозможно by construction.
4. **API модели** — паритет с C (`_init`/`_tick`/`_reset`/`_is_done`):
   `new(hal)`, `init()`, `tick()`, `reset()`, `is_done()`. `tick` — в 0050-06.
5. **`inout`** — решение и диагностика. Цель `sv` его запретила (`SV-006`:
   нужен сигнал `oe`, которого Lam не выражает); для Rust ограничения нет —
   `inout` ложится на пару методов трейта. Задача обязана **явно** решить:
   поддержать или запретить с `RS-006`. Тихо игнорировать — нельзя (в корпусе
   `inout` не используется, проверено `grep`, но фича 0032 показала, что
   `inout`-порты — реальный сценарий).

## Проверки

- **A4:** флагман `elevator_mini.lam` даёт модуль с `Hal`, `InBitPort` (40
  портов), `OutBitPort` (4) — компилируется гейтом.
- Тест: `struct` модели содержит поле `hal: H`, а `userdata`/указателей на
  функции в порождённом коде **нет** (`grep`) — сторож против кальки с C.
- Тест: порядок вариантов enum'ов портов детерминирован (два прогона → `diff`).
- Тест на решение по `inout` (поддержан → компилируется; запрещён → `RS-006`).
- Тест: модель **без** портов не эмитит ни трейта, ни параметра типа `H`.
  Проба (2026-07-16) под `-D warnings`:

  | Проба | Результат |
  |---|---|
  | `struct M<H: Hal> { hal: H, … }`, где `hal` не читается (модель без портов) | **ОТКАЗ**: `field 'hal' is never read` |
  | `struct M { … }` без параметра типа | принято |
  | `pub trait Hal` с методом `write_bit`, который никто не вызывает | принято (публичный трейт `dead_code` не задевает) |

  То есть эмиссию ограничивает не «неиспользуемый параметр типа», а `dead_code`
  на **поле**: HAL допустимо класть в `struct` только у модели, которая его
  действительно читает. Ещё один случай общего правила R9 (см.
  [0050-02](0050-02-gate.md)): гейт диктует форму эмиссии — калька с C не годится.
