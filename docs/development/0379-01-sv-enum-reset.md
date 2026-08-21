# Задача 0379-01: Сброс перечислимого регистра у цели sv

> Фича: [../features/0379-sv-enum-reset.md](../features/0379-sv-enum-reset.md) · ADR: [../adr/0379-sv-enum-reset.md](../adr/0379-sv-enum-reset.md) · анализ: [../analyze/0379-sv-enum-reset.md](../analyze/0379-sv-enum-reset.md)

## Что было

`sv_const::reset_value` в ветви `ExpressionNode::None` печатала `'0` для
любого типа. Для перечислимого регистра verilator отвечал ошибкой
`%Error-ENUMVALUE`.

## Что сделано

- В ветви `None` добавлен разбор `TypeNode::Enum`: значение сброса —
  `enum_literal(ty, 0, enums)` (мнемоника варианта со значением 0), а при его
  отсутствии — приведение `<тип>'(0)`.
- Комментарий называет причину и **почему не первый вариант**: он изменил бы
  значение, а у эталона там ноль.

**Статус по функциональности (правило 11).** Затронуты `sv`/`sv-mmio`; прочие
цели и эталон — **н/п**.

## Проверки

```sh
cargo test --test conformance conformance_sv_enum_reset
./scripts/probe.sh -n 3 <проба>.takt      # sv: verilator принял · yosys синтезировал
./scripts/precheck.sh
```

Мутация «первый вариант вместо нуля» роняет сверку значений (`RTL=[[1, 0], …]`
против `sim=[[0, 0], …]`) и сверку текста.
