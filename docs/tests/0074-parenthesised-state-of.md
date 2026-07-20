# Тест-план 0074: Скобочная форма `S(Модель) = Состояние`

- **Фича:** [0074](../features/0074-parenthesised-state-of.md)
- **ADR:** [0074](../adr/0074-parenthesised-state-of.md) (Option C)
- **Дата:** 2026-07-20
- **Роль:** Тестировщик

## Стратегия

Фича меняет **семантику** (правило 16): скобки паттерна `S(…)` становятся
прозрачными. Доказательства — двухуровневые:

1. **Структура** (`grammar/src/semantic/condition.rs`, юнит) — резолвер даёт
   каноничную `ConditionNode` без обёрток `Parenthesis`.
2. **Трансляция** (`grammar/tests/c_state_ref_tests.rs`, интеграция) — вывод C
   **байт-в-байт** равен бесскобочной форме и компилируется `cc`.

Наблюдение берётся у потребителя: у `c` — текст C и компиляция `cc` (позиции на
вывод не влияют, поэтому на структуре — сравнение формы, а не `==` с `Location`).

## Примеры (правило 16)

Валидные (после фикса компилируются целью `c`, дают тот же C, что `S(Ping) = End`):

```lam
ref Stop: (S(Ping)) = End;
ref Stop: S((Ping)) = End;
ref Stop: S(Ping) = (End);
ref Stop: ((S((Ping)))) = (End);
ref Stop: (S(Ping)) != Done;
```

Контрпример (семантическая ошибка, не трансляционная):

```lam
ref Stop: (S(Ping)) = NoSuchState;   // SE-033: состояния нет у Ping
```

## Условия проверок и ожидаемые результаты

| № | Проверка | Ожидание | Где |
|---|---|---|---|
| T1 | `S(Ping) = End` (эталон) резолвится в `Equal(Function(S,[Model]), Unresolved(Variable "End"))` | каноничная форма | `condition::parenthesised_state_of_canonicalizes` |
| T2 | 4 скобочные формы резолвятся в ту же каноничную структуру (без `Parenthesis`) | канон | там же |
| T3 | Обычная скобка `(flag) = true` **сохраняется** (не пере-снятие) | `Equal(Parenthesis, …)` | `condition::ordinary_parentheses_are_preserved` |
| T4 | Ветка `EnumVariant` справа не задета канонизацией | зелёные | `enum_variant_resolves_over_state_in_equal/not_equal` |
| T5 | 4 скобочные формы → C **байт-в-байт** = `S(Ping) = Done`; `cc -c` OK | равно + компилируется | `c_state_ref::parenthesised_state_of_is_canonical` |
| T6 | Скобочные формы `!=` → C = бесскобочного `!=`; `cc -c` OK | равно | `c_state_ref::parenthesised_state_of_not_equal_is_canonical` |
| T7 | `(S(Ping)) = NoSuchState` → **`SE-033`**, не `SE-025` | `SE-033` | `c_state_ref::parenthesised_unknown_state_is_se033` |
| T8 | Инвариант `resolve_state_references` цел | зелёный | `reference_model_compiles_and_translates_state_ref` |
| T9 | Вывод корпуса `examples/generated/` **байт-в-байт** неизменен | нет диффа | гейт детерминизма `precheck.sh` |
| T10 | Весь `precheck.sh` (fmt/check/clippy/тесты/примеры) | зелёный | `./scripts/precheck.sh` |

## Границы

Старый сторож `parenthesised_state_of_is_rejected_by_semantics` (ожидал `SE-025`)
**инвертирован** в T5 — это заявленная граница ADR 0074.

Цели `rust`/`st`/`sv` паттерн `S(…)` не поддерживают (RS-020/ST-011/SV-002) — вне
объёма: фикс уравнял скобочные формы с бесскобочной, не добавляя поддержку.
