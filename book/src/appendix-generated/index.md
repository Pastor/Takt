# Порождённый код примера

Здесь приведён **полный** код, порождённый из модели контроллера кабины лифта
(раздел [«Практический пример»](../18-showcase/index.md)) для каждой цели
генерации. Один и тот же исходник `lift.takt` даёт все эти файлы **без правок** —
командой `taktc compile -t <цель> lift.takt -o out/`. Код воспроизводим побайтно
(генерация детерминирована), поэтому его можно сравнивать между версиями.

## Цель `c` — прошивка МК (порты через HAL-колбэки)

Заголовок `lift.h`:

```c
{{#include ../18-showcase/generated/c/lift.h}}
```

Реализация `lift.c`:

```c
{{#include ../18-showcase/generated/c/lift.c}}
```

## Цель `c-hal` — прошивка с прямым доступом к регистрам

Отличается от `c` слоем аппаратной абстракции: адреса портов (`0x50000010` и др.)
превращаются в доступ по памяти. Заголовок `lift.h`:

```c
{{#include ../18-showcase/generated/c-hal/lift.h}}
```

Реализация `lift.c`:

```c
{{#include ../18-showcase/generated/c-hal/lift.c}}
```

## Цель `rust` — `no_std` Rust

```rust
{{#include ../18-showcase/generated/rust/lift.rs}}
```

## Цель `st` — ПЛК, Structured Text (IEC 61131-3)

```pascal
{{#include ../18-showcase/generated/st/lift.st}}
```

## Цель `sv` — синтезируемый SystemVerilog (FPGA/ASIC)

```systemverilog
{{#include ../18-showcase/generated/sv/lift.sv}}
```
