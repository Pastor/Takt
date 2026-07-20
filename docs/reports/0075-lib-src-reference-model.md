# Отчёт о тестировании фичи 0075: эталонная модель порождает компилируемый C

> Фича: [../features/0075-lib-src-reference-model.md](../features/0075-lib-src-reference-model.md) · тест-план: [../tests/0075-lib-src-reference-model.md](../tests/0075-lib-src-reference-model.md) · ADR: [../adr/0075-lib-src-reference-model.md](../adr/0075-lib-src-reference-model.md)

- **Дата:** 2026-07-20
- **Окружение:** macOS (darwin 25.5.0), cargo nightly, `cc` (Apple clang).
- **Вердикт:** **готово.** `reference_model_tests` (1) и `parse_simple` зелёные,
  `precheck.sh` зелёный. Язык и версия не изменились.

## Сверка с критериями приёмки (ADR 0075)

| Критерий | Проверка | Результат |
|---|---|---|
| **A1** `SYNTH_SRC` → `cc -c` | `reference_model_compiles_and_translates_state_ref` | ✅ rc=0 |
| **A2** перевод `S(Ping) = End` | тот же тест | ✅ `main->entry_parallel0.ping0.state == THIS_IS_MY_MODEL_PING_END` |
| **A3** разбор полного `SRC` | `parse_simple` | ✅ |
| **A4** прочее не задето | `precheck.sh` | ✅ зелёный |

## Замер (11 ошибок исходного `SRC`)

`cc -fsyntax-only` на выводе полного `SRC`:
- `no member named 'read_numeric'` — `out`-порт `A: u8` с бит-доступом
  (`A.0 := true`) зовёт `read_numeric` (RMW), а структура эмитит только
  `write_numeric` → **локальный баг генератора** (кандидат, вне 0078).
- `{…} >> 5` — `const MATRIX: u8 := {0,…}` печатается макросом-массивом, бит-доступ
  `MATRIX.5` трактует числом → **семантика `[bit;N]`** (0078).

Строка `S(Ping) = End` ошибок не даёт — подтверждено грепом позиций.

## Решение

Эталон **разделён** (Option A): `parse_simple` — покрытие парсера на полном
`SRC`; компиляционная проверка — на `SYNTH_SRC` (`cc -c`), сохранившей композицию
и `S(Ping) = End`. `syntax_simple` удалён из `lib.rs` (стоял на некомпилируемом
`SRC`); `lib.rs` уменьшен 1450 → 1405.

## Кандидаты (вне объёма)

- **A-2:** `read_numeric` для `out`-порта с бит-доступом — локальный фикс/0080.
- **A-3:** const `[bit;N]` бит-доступ — за 0078.
