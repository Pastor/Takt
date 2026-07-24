# Задача 0088-09: Генератор `c_source` — разбиение inline-тестов на подмодуль

> Фича: [../features/0088-module-size-remaining.md](../features/0088-module-size-remaining.md) · ADR: [../adr/0088-module-size-remaining.md](../adr/0088-module-size-remaining.md) · анализ: [../analyze/0088-module-size-remaining.md](../analyze/0088-module-size-remaining.md)

## Что было

`grammar/src/generator/c/c_source.rs` — **1169 строк** (нарушитель). Из них
собственно код — **30 строк** (`generate_source`), остальное — inline-модуль
`#[cfg(test)] mod tests { … }` (~1136 строк, helpers `make_map_and_owner`/
`expr_to_str` + 67 плоских `#[test]`).

## Что сделано

Вторая половина тестов (с `test_generate_source_functions`) вынесена в
**вложенный** подмодуль `mod part2` внутри `mod tests` (приём 0088-06/08,
адаптированный для inline-тестов в `src`):

- Файл — `grammar/src/generator/c/c_source/tests/part2.rs` (**естественный**
  путь модуля: `c_source.rs` → `c_source/` → `tests/` → `part2.rs`; **без**
  `#[path]`, объявление — голое `mod part2;` внутри `mod tests`).
- Helpers и импорты — из родителя через `use super::*` (glob): `super` для
  вложенного `part2` — это модуль `tests`, где живут helpers и все `use`.
- Резал по границе **полного `#[test]`** (урок 0088-07), а не по строке.

**Чистое перемещение:** ни одно утверждение не менялось, только адрес. Код
`generate_source` не тронут → вывод генератора `c` байт-в-байт неизменен.

- `c_source.rs`: **1169 → 604** (30 строк кода + первая половина тестов);
- `c_source/tests/part2.rs` — **573**;

оба ≤ 1000 → запись удалена из реестра (**10 → 9**).

Стеки: только `grammar`. `simulation` — н/п.

## Проверки

- `cargo test -p grammar --lib generator::c::c_source` — **67 passed, 0 failed**
  (оба подмодуля `tests` + `tests::part2`).
- `./scripts/precheck.sh` — зелёный (`check-module-size.sh` −1, все тесты,
  детерминизм-гейт → вывод генераторов неизменен).
