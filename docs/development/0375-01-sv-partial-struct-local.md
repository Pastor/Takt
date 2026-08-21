# Задача 0375-01: Частично использованная локальная структура у цели sv

> Фича: [../features/0375-sv-partial-struct-local.md](../features/0375-sv-partial-struct-local.md) · ADR: [../adr/0375-sv-partial-struct-local.md](../adr/0375-sv-partial-struct-local.md) · анализ: [../analyze/0375-sv-partial-struct-local.md](../analyze/0375-sv-partial-struct-local.md)

## Что было

Фича 0373 подняла локальную переменную со структурой в начало `always_comb`.
Если тело пишет поле, но не читает его, verilator отвечает `UNUSEDSIGNAL`, а
гейт цели считает предупреждение ошибкой.

## Что сделано

- `sv_locals::emit_declarations` печатает рядом с объявлением поднятой
  переменной объявление поглотителя `logic _unused_<имя>;`.
- `sv_locals::emit_defaults` печатает присваивание поглотителя после нулевых
  умолчаний: `_unused_<имя> = &{1'b0, …};`.
- **Операнды редукции выбираются по форме значения:** упакованное — целиком,
  распакованный массив — поэлементно (список берётся у тех же умолчаний, что
  собрал `leaf_zero_defaults`).

**Статус по функциональности (правило 11).** Затронуты `sv`/`sv-mmio`; прочие
цели и эталон — **н/п**.

## Проверки

```sh
cargo test
./scripts/probe.sh -n 2 <проба>.takt     # шесть входов: verilator и yosys приняли
./scripts/precheck.sh
```

Мутации (каждая проверена):

| Мутация | Что падает |
|---|---|
| поглотитель всегда «целиком» | `absorber_of_unpacked_array_is_elementwise`, `struct_local_is_synthesizable` |
| поглотителя нет вовсе | `partially_used_struct_local_passes_the_lint`, `absorber_of_unpacked_array_is_elementwise` |
