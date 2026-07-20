# Задача 0070-01: снять `SE-035` с инициализатора порта

> Фича: [../features/0070-port-initializer-address-role.md](../features/0070-port-initializer-address-role.md) · ADR: [../adr/0070-port-initializer-address-role.md](../adr/0070-port-initializer-address-role.md) · анализ: [../analyze/0070-port-initializer-address-role.md](../analyze/0070-port-initializer-address-role.md)

## Что было

`grammar/src/semantic/validate/enums.rs` — `validate_bit_values` применяет
`check_bit_variable_value` к трём вариантам **одинаково**:

```rust
VariableNode::Simple { name, ty, expr, .. }
| VariableNode::Const  { name, ty, expr, .. }
| VariableNode::Port   { name, ty, expr, .. } => check_bit_variable_value(name, ty, expr, var.loc())?,
```

Для `Port` инициализатор — это **адрес** (ADR 0020, поле `mod.rs:883` — «Адрес
порта»), а `check_bit_variable_value` трактует его как значение бита → `SE-035` на
`bit := 0xADDR`. Асимметрия: `u8 := 0xADDR` проходит (SE-035 только для `bit`).

## Что сделано

Вариант `VariableNode::Port` **выведен** из-под `check_bit_variable_value`:
проверка значения бита остаётся только для `Simple`/`Const` (переменные/константы,
где инициализатор — действительно значение). Точечная правка одного матч-арма;
`check_bit_variable_value` не трогается (её ещё зовут enum-ветки того же файла).

Диагностика прочих проходов не затрагивается: `resolve_addresses`
(`address_map/resolve.rs`) читает `Port.expr` как адрес независимо и уже
покрывает случаи «нет адреса» (`SE-052`) и «конфликт» (`SE-049`).

**Обратная совместимость** (правило 11): множество принятых программ расширяется
(`bit := 0xADDR` был ошибкой — станет валиден); не-порт `var/const: bit := N`
сохраняет `SE-035`; вывод целей на корпусе байт-в-байт прежний. Версия языка не
поднимается (обоснование — в анализе, прецедент 0098).

## Фикстуры (правило 16 — примеры/контрпримеры)

- **Пример** `tests/data/semantic/valid/port_bit_address.lam` — `in BTN: bit :=
  0x00100000;` (+ рабочая `out … := 0xADDR:бит`) — валиден.
- **Контрпример** `tests/data/semantic/invalid/bit_var_value.lam` — `var flag:
  bit := 5;` (не порт) — `SE-035`.
- Тесты в `grammar/tests/semantic_tests.rs`: порт `bit := 0xADDR` без `SE-035`;
  не-порт `bit := 5` с `SE-035`; `bit := 0/1` на порту валидны.

## Проверки

- **A1/A2/A3:** новые семантические тесты (порт без SE-035; не-порт с SE-035;
  0/1 валидны).
- **A4:** зонд-инспекция `c-hal` для `bit := 0xADDR` → `{(uintptr_t)0xADDRu, -1, …}`.
- **A5/A6/A7:** `./scripts/precheck.sh` зелёный (правило 5); `git diff --stat
  examples/generated/` пуст; гейт воспроизводимости 0048 не нарушен.
- Полная сборка + тесты (`cargo test -- --test-threads=1`).
