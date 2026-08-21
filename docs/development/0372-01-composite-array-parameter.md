# Задача 0372-01: Составной элемент массива в параметре функции

> Фича: [../features/0372-composite-array-parameter.md](../features/0372-composite-array-parameter.md) · ADR: [../adr/0372-composite-array-parameter.md](../adr/0372-composite-array-parameter.md) · анализ: [../analyze/0372-composite-array-parameter.md](../analyze/0372-composite-array-parameter.md)

## Что было

- **Цель `sv`.** `flat_param_width` (0369) требовала **скалярной** ширины
  элемента, поэтому массив структур, перечислений и вложенный массив шли в
  параметр функции распакованным портом: yosys — «input/output/inout ports
  cannot have unpacked dimensions», verilator — молчит.
- **Цель `st`.** `array_form_name` (0348) строила имя из **текста** типа
  элемента: `[[u8; 2]; 2]` давал `TAKT_ARR_2_ARRAY_[0..1]_OF_USINT`, тогда как
  объявление печаталось верной многомерной формой (0363).

## Что сделано

**Цель `sv` (`generator/sv/`).**

- `sv_array::FlatParam` + `sv_array::flat_param` — раскладка параметра по
  плоскому вектору: спуск **только по распакованным размерностям**
  (`walk_dimensions`), частью становится элемент; ширина части считает
  `packed_width` (скаляр — `scalar_width`, перечисление — `enum_width`,
  структура — сумма полей рекурсивно). `None` — раскладки нет, поведение
  прежнее.
- `sv_array::flatten_argument` печатает конкатенацию **частей** в обратном
  порядке: первая ложится в младшие разряды.
- `sv_fsm` печатает сигнатуру по `flat_param.width` и пролог по её частям;
  часть-перечисление приводится к своему типу (`mode_e'(a_flat[1:0])`) — без
  приведения verilator отвечает **ошибкой** `ENUMVALUE`.
- `sv_expr` строит аргумент тем же носителем, передавая ему карты структур и
  перечислений области видимости.

⚠️ **Спуск останавливается на упакованном типе — это замер, а не вкус.** Первая
редакция раскладывала структуру **по полям** (обход `walk_type` фичи 0367), и
yosys отвечал «Latch inferred for signal `…a[0].lo`»: запись полей элемента
внутри `always_comb` он полным присваиванием не считает. Присваивание структуры
целиком приняли **оба** инструмента.

**Цель `st` (`generator/st/st_type.rs`).**

- Разбор массива вынесен в `array_dims_and_base` — **один** носитель для
  объявления (`array_type`) и для имени формы (`array_form_name`).
- Имя строится по размерностям и базовому типу: `TAKT_ARR_2_2_USINT`.
  Одномерная форма имени не изменилась (`TAKT_ARR_2_USINT`,
  `TAKT_ARR_2_CELL`) — вывод корпуса на месте.

**Статус по функциональности (правило 11).** Затронуты цели `sv`/`sv-mmio` и
`st`/`st-at`; `c`/`c-hal`, `rust`, `plantuml` и эталон — **н/п**: те же входы
они переводят и исполняют, замер это подтверждает.

## Проверки

```sh
cargo test                                  # 3399 тестов, 0 провалов
./scripts/probe.sh -n 3 <проба>.takt        # три вида элемента: все инструменты приняли
./scripts/precheck.sh
```

Мутации (каждая проверена):

| Мутация | Что падает |
|---|---|
| прямой порядок конкатенации | `composite_parameter_values_match_generated_sv` |
| спуск по полям структуры (первая редакция) | `composite_parameter_is_synthesizable` («Latch inferred») |
| имя формы по тексту типа элемента | `composite_parameter_st_trace_matches_reference`, `nested_array_form_name_follows_the_declaration` |
