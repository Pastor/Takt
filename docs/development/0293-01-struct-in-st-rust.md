# Задача 0293-01: Структуры в цели `sv`

> Фича: [../features/0293-struct-in-st-rust.md](../features/0293-struct-in-st-rust.md) · ADR: [../adr/0293-struct-in-st-rust.md](../adr/0293-struct-in-st-rust.md) · анализ: [../analyze/0293-struct-in-st-rust.md](../analyze/0293-struct-in-st-rust.md)

## Что было

`sv_type` отображал `TypeNode::Struct` в имя `<имя>_t`, но **объявления не
эмитил**: `verilator` отвечал `Can't find typedef/interface: 'gains_t'` — при
**нулевом** коде возврата `taktc`. Агрегатный инициализатор отвергался `SV-002`.

## Что сделано

- `sv_type::emit_structs` печатает `typedef struct packed { … } <имя>_t;` для
  каждой структуры дерева; поле печатается тем же `sv_type`, что и переменная;
- `sv_const::struct_reset` кладёт агрегат в цепь сброса.

⚠️ **`packed` обязателен:** непакованная структура не синтезируется и не годится
на роль регистра, а именно регистром становится переменная модели.

⚠️ **Форма агрегата выбрана пробой обоих инструментов.** Именованную
(`'{kp: 2, ki: 3}`) `verilator` принимает, а **yosys отвергает**
(`syntax error, unexpected ':'`). Принята позиционная конкатенация
`{8'd2, 8'd3}` — тот же урок, что у `assert … else` (0235).

⚠️ **Размерные литералы обязательны всегда**, а не по нужде (в отличие от печати
литерала, 0157): `verilator -Wall` отвечает `WIDTHCONCAT` на любое безразмерное
число внутри `{…}`.

⚠️ Порядок полей — **объявленный**: в `struct packed` он определяет разряды.

## Проверки

```sh
cargo test --test conformance struct
./scripts/precheck.sh
```

`verilator --lint-only -Wall` и `yosys synth` принимают вывод; потактовая сверка
`conformance_sv_tests::struct_init` — трасса совпадает с эталоном.
