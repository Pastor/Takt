# Задача 0380-01: Fixed-point в параметре функции

> Фича: [../features/0380-fixed-point-parameter.md](../features/0380-fixed-point-parameter.md) · ADR: [../adr/0380-fixed-point-parameter.md](../adr/0380-fixed-point-parameter.md) · анализ: [../analyze/0380-fixed-point-parameter.md](../analyze/0380-fixed-point-parameter.md)

## Что было

`fn half(v: q(8, 8))` давала `SE-119` — отказ о дефекте компилятора — у всех
девяти потребителей.

## Что сделано

- `semantic/function.rs`: ветвь `ast::Expression::Function` в разборе типа
  параметра. Помощник `fixed_from_call` опознаёт форму «идентификатор + два
  числовых аргумента» и строит `ast::Type::Fixed`; дальше работает прежний
  `construct_type` → `construct_fixed` (имя `q` и границы `m`/`n` проверяет
  он).
- Форма, не похожая на конструктор типа, даёт **`SE-034`** с названным
  ожиданием («имя типа, `[тип; N]` или `q(m, n)`») вместо `SE-119`.
- `generator/st/st_fixed.rs`: `insert_helper` вставляет хелперы перед **первым
  POU** (`first_pou` — строка, начинающаяся с `FUNCTION`), а не перед первым
  `FUNCTION_BLOCK`.

**Статус по функциональности (правило 11).** Семантика — общая для всех
потребителей; отдельно правлена цель `st` (порядок хелперов). Прочие цели
изменений не потребовали: форму они получают уже разрешённым типом.

## Проверки

```sh
cargo test                                    # 3376 тестов, 0 провалов
./scripts/probe.sh -n 3 <проба>.takt          # восемь целей: инструменты приняли
./scripts/precheck.sh
```

Мутация «вставка перед `FUNCTION_BLOCK`» роняет оба теста порядка хелперов.
