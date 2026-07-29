# Задача 0155-02: Сторожа на шесть точек глотания

> Фича: [../features/0155-semantic-nested-statement-resolution.md](../features/0155-semantic-nested-statement-resolution.md) · ADR: [../adr/0155-semantic-nested-statement-resolution.md](../adr/0155-semantic-nested-statement-resolution.md) · анализ: [../analyze/0155-semantic-nested-statement-resolution.md](../analyze/0155-semantic-nested-statement-resolution.md)

## Что было

Покрытия не было вовсе: ни один тест не проверял, что оператор во вложенном теле
доезжает до диагностики или до порождённого кода. Возврат глотания — правка в
одну строку на каждую точку, и она прошла бы незамеченной.

## Что сделано

**Фикстуры** — `takt-lang/tests/data/nested0155/` (правило 16: примеры и
контрпримеры): восемь контрпримеров (по одному на точку плюс вложенность в теле
функции и в плече `match`) и один пример корректных вложенных тел.

**Тесты** — `takt-lang/tests/nested_statement_resolution_tests.rs`, 12 штук:

| Тест | Что доказывает |
|---|---|
| `unknown_identifier_in_if_then_is_diagnosed` | точка 1 |
| `unknown_identifier_in_if_else_is_diagnosed` | точка 2 |
| `unknown_identifier_in_while_body_is_diagnosed` | точка 3 |
| `unknown_identifier_in_loop_body_is_diagnosed` | точка 3 через синоним `loop` |
| `unknown_identifier_in_for_init_is_diagnosed` | точка 4 |
| `unknown_identifier_in_for_body_is_diagnosed` | точка 5 |
| `resolution_error_in_inline_guard_is_diagnosed` | точка 6 (через `SE-062` — см. оговорку 0155-01) |
| `unknown_identifier_nested_in_function_body_is_diagnosed` | вложенность внутри `fn` |
| `unknown_identifier_nested_in_match_arm_is_diagnosed` | вложенность внутри плеча `match` |
| `valid_nested_bodies_are_still_accepted` | сторож направления: корректное не отвергается |
| `valid_nested_body_is_emitted_into_generated_c` | **тело доезжает до C** |
| `nested_diagnostic_reaches_lsp` | правило 29: диагностика доходит до редактора |

⚠️ **`valid_nested_body_is_emitted_into_generated_c` — не декорация.**
Диагностика доказывает половину: что ошибка не молчит. Вторая половина — что
корректное тело **не выброшено**; ровно эта половина и была сломана. Тест читает
порождённый `.c` и требует в нём тела обеих ветвей.

**Достижимость сторожа глубины** — `takt-lang/tests/deep_nesting_tests.rs`:
добавлены `deep_statement_nesting_is_diagnosed` (60 вложенных `if` → `SE-062`) и
сторож направления `statement_nesting_within_limit_is_accepted`. Заметка в шапке
файла про недостижимость сторожа операторов исправлена — после 0155 она неверна.

## Проверки

**Мутационная проверка взведённости ловушки** (урок 0056: «сторож, не
проверяющий взведённость ловушки, проверяет собственную удачу»). Глотание
возвращено в точку 1 (`if`-then):

```
test unknown_identifier_in_if_then_is_diagnosed ... FAILED
test unknown_identifier_nested_in_function_body_is_diagnosed ... FAILED
test unknown_identifier_nested_in_match_arm_is_diagnosed ... FAILED
test nested_diagnostic_reaches_lsp ... FAILED
test result: FAILED. 7 passed; 4 failed
```

Мутация снята, все 12 зелёные; `deep_nesting_tests` — 8 из 8.
