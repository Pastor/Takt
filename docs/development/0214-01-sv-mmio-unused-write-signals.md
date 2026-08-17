# Задача 0214-01: Сигналы записи `sv-mmio` — по составу портов

> Фича: [../features/0214-sv-mmio-unused-write-signals.md](../features/0214-sv-mmio-unused-write-signals.md) · ADR: [../adr/0214-sv-mmio-unused-write-signals.md](../adr/0214-sv-mmio-unused-write-signals.md) · анализ: [../analyze/0214-sv-mmio-unused-write-signals.md](../analyze/0214-sv-mmio-unused-write-signals.md)

## Что было

`emit_reg_iface_lines` печатала все четыре сигнала регистрового интерфейса, если
у модели есть хоть один адресованный порт. На модели без **входных** портов
`reg_wdata`/`reg_wen` оставались неподключёнными: `verilator --lint-only -Wall`
— два `UNUSEDSIGNAL` и ненулевой код возврата.

## Что сделано

**`takt-lang/src/generator/sv/sv_mmio.rs`:** заведён предикат
`Mmio::has_writable` — есть ли регистр, который пишет шина. Сигналы записи
эмитятся только при нём. ⚠️ Правило не новое, а достроенное: при
`Mmio::is_empty` интерфейс не эмитился и раньше — не хватало различения по
**направлению**.

**`takt-lang/src/generator/sv/sv_apb.rs`:** адаптер следует за ядром — не
объявляет внутренних проводов записи, не подключает `.reg_wdata`/`.reg_wen` и
честно поглощает сигналы записи протокола
(`wire _unused_write = &{1'b0, pwdata, pwrite, psel, penable};`). ⚠️ Без этого
verilator отвечал `PINNOTFOUND`: пара «ядро + обёртка» не собиралась вовсе.
Состав интерфейса самой обёртки не меняется — он задан шиной APB, а не моделью.

**`takt-sim/tests/conformance/conformance_sv_mmio_tests.rs`:** сверка строит
тестбенч кодом и подключала сигналы записи безусловно. Теперь читает
порождённый модуль и подключает то, что он объявил. ⚠️ Это **третий**
потребитель интерфейса — ADR назвал два.

**Гейт:** `SV_MMIO_TRANSLATABLE="stacker regulator"`. Прежде список состоял из
одного примера со входными портами, поэтому класс был невидим.

**Тестбенч `examples/generated/sv-mmio/tb/regulator_apb_tb.sv`** (рукописный,
как принято для APB): чтение выходного регистра до и после завершения автомата,
цикл записи по нему и проверка, что значение не изменилось, чтение адреса без
порта. ⚠️ Первая редакция падала: `regulator` сходится за считанные такты, и
попытка записи **до** завершения ничего не доказывала — значение менял сам
автомат. Порядок проверок переставлен, причина записана в шапке.

**README:** раздел о цели `sv-mmio` дополнен — состав интерфейса зависит от
модели, адаптер следует за ядром, цикл записи завершается штатно.

## Проверки

```sh
cargo test --all-features --test targets sv_mmio_write_signals_tests::  # 3 сторожа
cargo test --all-features --test conformance conformance_sv_mmio        # 2 сверки
./scripts/precheck.sh
```

| Что | До | После |
|---|---|---|
| `regulator` целью `sv-mmio`, `verilator -Wall` | **2** `UNUSEDSIGNAL`, ненулевой код | **0** |
| Пара «ядро + APB» на `regulator` | `PINNOTFOUND` ×2 | линт и синтез проходят |
| `stacker` (ядро и адаптер) | — | байт-в-байт прежний |
| Тестбенч `regulator_apb` | не существовал | 4 проверки пройдены |
