# Отчёт о тестировании 0074: Скобочная форма `S(Модель) = Состояние`

- **Фича:** [0074](../features/0074-parenthesised-state-of.md)
- **ADR:** [0074](../adr/0074-parenthesised-state-of.md) (Option C)
- **Тест-план:** [0074](../tests/0074-parenthesised-state-of.md)
- **Дата:** 2026-07-20
- **Вердикт:** ✅ **ГОТОВО**

## Окружение

- macOS (darwin 25.5), Rust workspace `grammar` 0.7.0 / `simulation` 0.4.0.
- `./scripts/precheck.sh` — **EXIT=0** (fmt, check, clippy `-D warnings`, тесты,
  сборка примеров, гейт детерминизма, ST `iec2c` 8/8, c-hal, sv-тестбенчи).
- `cargo test`: **2141 passed, 6 ignored** (46 наборов).

## Сводка

Дефект: скобки вокруг операндов паттерна `S(Модель) = Состояние`
(`(S(Ping)) = End`, `S((Ping)) = End`, `S(Ping) = (End)`) давали `SE-025`.
Причина — распознавание паттерна сопоставлением формы у нескольких потребителей,
которое рвёт обёртка `ConditionNode::Parenthesis`.

Решение (Option C): канонизация в **единой воронке** `resolve_condition` —
снятие прозрачных скобок в трёх позициях паттерна. Одна правка чинит семантику,
все генераторы и симулятор; вывод корпуса **байт-в-байт неизменен**.

## Сверка с тест-планом

| № | Проверка | Результат |
|---|---|---|
| T1 | Эталон `S(Ping) = End` каноничен | ✅ `parenthesised_state_of_canonicalizes` |
| T2 | 4 скобочные формы → та же каноничная структура (без `Parenthesis`) | ✅ там же |
| T3 | Обычная скобка `(flag) = true` сохранена (не пере-снятие) | ✅ `ordinary_parentheses_are_preserved` |
| T4 | Ветка `EnumVariant` справа не задета | ✅ `enum_variant_resolves_over_state_in_{equal,not_equal}` |
| T5 | 4 формы → C байт-в-байт = `S(Ping) = Done`; `cc -c` OK | ✅ `parenthesised_state_of_is_canonical` |
| T6 | Скобочные `!=` → C = бесскобочного `!=`; `cc -c` OK | ✅ `parenthesised_state_of_not_equal_is_canonical` |
| T7 | `(S(Ping)) = NoSuchState` → `SE-033`, не `SE-025` | ✅ `parenthesised_unknown_state_is_se033` |
| T8 | Инвариант `resolve_state_references` цел | ✅ `reference_model_compiles_and_translates_state_ref` |
| T9 | Вывод `examples/generated/` неизменен | ✅ `git status` пуст; гейт детерминизма зелёный |
| T10 | Весь `precheck.sh` | ✅ EXIT=0 |

## Примеры и контрпримеры (правило 16)

Валидные (дают тот же C, что `S(Ping) = Done`, компилируются `cc`):
`(S(Ping)) = Done`, `S((Ping)) = Done`, `S(Ping) = (Done)`,
`((S((Ping)))) = (Done)`, `(S(Ping)) != Done`, `S((Ping)) != (Done)`.

Контрпример: `(S(Ping)) = NoSuchState` → **`SE-033`** (состояния нет), а не
`SE-025` — диагностика по существу сохранена.

## Границы и отклонения

- Сторож `parenthesised_state_of_is_rejected_by_semantics` (ожидал `SE-025`)
  **инвертирован** в `parenthesised_state_of_is_canonical` — заявленная граница
  ADR 0074.
- **Находка пробы:** карточка называла две скобочные формы; проба нашла **третью**
  (`S(Ping) = (End)`, скобки вокруг имени состояния) — она в объёме фикса.
- Комментарий генератора C (`c_expr/condition.rs`), ссылавшийся на `SE-025`,
  обновлён под 0074.
- Цели `rust`/`st`/`sv` паттерн `S(…)` не поддерживают (RS-020/ST-011/SV-002) —
  предсуществующее ограничение, вне объёма; фикс уравнял скобочные формы с
  бесскобочной (обе одинаково не транслируются), проверено пробой на всех целях.

## Найденные дефекты

Нет. Фиксов (`docs/fixes/`) не заводилось.
