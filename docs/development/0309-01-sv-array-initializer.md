# Разработка 0309-01: агрегат массива в ветви сброса цели `sv`

> Фича: [../features/0309-sv-array-initializer.md](../features/0309-sv-array-initializer.md) · ADR: [../adr/0309-sv-array-initializer.md](../adr/0309-sv-array-initializer.md)

## Состав правки

| Файл | Что сделано |
|---|---|
| `takt-lang/src/generator/sv/sv_const.rs` | ветвь агрегата массива + `array_reset` (шаблон присваивания, размерные литералы, проверка размера) |
| `takt-sim/tests/data/eval/conformance_sv_array.takt` | фикстура: агрегат, чтение и запись элемента, накапливающее тело |
| `takt-sim/tests/conformance/conformance_sv_array_tests.rs` | потактовая сверка + проверка отказа на несовпадении размера |
| `takt-lang/tests/targets/bit_write_targets_tests.rs` | граница 0250 переписана: цель переводит |
| `takt-sim/tests/sim/aggregate_argument_tests.rs` | граница 0209 переписана: проверяется значение в выводе |

## Проба формы (2026-08-20)

```systemverilog
logic [7:0] a [0:2];
always_ff @(posedge clk) if (!rst_n) a <= '{8'd1, 8'd2, 8'd3};
```

- `verilator --lint-only -Wall` — принимает;
- `yosys -p "read_verilog -sv …; synth"` — принимает (предупреждение о замене
  памяти списком регистров).

## Проверено

- `cargo test --test conformance conformance_sv_array` — 2/2.
- `cargo test --all-features` — провалов нет.
- Проба `scripts/probe.sh`: до правки `sv`/`sv-mmio` отвергали, после — все
  восемь целей переводят.
