# Задача 0172-01: Правило языка «перечисление обязано иметь вариант» (`SE-105`)

> Фича: [../features/0172-empty-enum-semantics.md](../features/0172-empty-enum-semantics.md) · ADR: [../adr/0172-empty-enum-semantics.md](../adr/0172-empty-enum-semantics.md) · анализ: [../analyze/0172-empty-enum-semantics.md](../analyze/0172-empty-enum-semantics.md)

## Что было

Запись `enum E { }` отвергал **парсер**: правило `EnumDefine` требовало
`CommaOne<EnumVariant>`. Правило языка было свойством квантификатора в
грамматике, а текст сообщения — внутренностью LR-разбора:

```
model.takt:1:10: Ошибка компиляции [SY-002]: нераспознанный токен '}',
  ожидалось identifier, "X", "F", "G", "U", "R", "LTL", "Guard"
```

Ни правила, ни способа исправить, ни позиции объявления автору не сообщалось, а
`None`-ветви четырёх целей (`c`/`st`/`rust` → 8 бит без знака, `sv` → `SV-004`)
оставались недостижимыми **по случайности грамматики**.

## Что сделано

- **Грамматика** (`takt-lang/src/grammar.lalrpop`): `EnumDefine` принимает
  пустой список вариантов (`CommaOne<EnumVariant>` → `Comma<EnumVariant>`).
  Конфликтов LR нет; ветвь восстановления `enum <имя> { ! }` сохранена и
  по-прежнему отвечает за **мусор** внутри скобок. Побочный эффект — в списке
  ожидаемого у `SY-002` появилось `"}"`.
- **Семантика** (`semantic/validate/enums.rs`): новая проверка
  `validate_empty_enums` — **`SE-105`** на каждое перечисление без вариантов;
  позиция — объявление (имя перечисления), текст называет правило и способ
  исправить. Проверка накопительная (правило 0151), рекурсию не ведёт —
  `validate_model_all` обходит вложенные модели сам.
- **Подключение** (`semantic/validate/mod.rs`): проверка добавлена в массив
  `validate_model_all` (11 → 12 элементов) рядом с прочими проверками
  перечислений. ⚠️ Порядок в массиве на порядок сообщений **не влияет** —
  выдачу упорядочивает `diagnostics::normalize` по позиции в тексте; это
  измерено мутацией, а первоначальный комментарий «стоит раньше намеренно» был
  снят как неверный.
- **Ветви целей не тронуты**: `enum_facts` → `None` и трактовки
  `c`/`st`/`rust`/`sv` остались как есть (ADR 0060, правило 3). Изменилась
  **причина** их недостижимости.

Функциональности вне Rust-крейтов: `н/п` — правка целиком в `takt-lang`.

## Проверки

```sh
cargo test --all-features --test empty_enum_tests   # 9 тестов
./scripts/precheck.sh
```

Сторож — `takt-lang/tests/empty_enum_tests.rs` (фикстуры
`tests/data/semantic/invalid/empty_enum.takt` и `empty_enum_many.takt`):

| Тест | Условие анализа |
|---|---|
| `empty_enum_is_se105` | R1 (код `SE-105`, не `SY-002`) |
| `se105_points_at_the_declaration` | R1 (позиция — имя перечисления, 1:6) |
| `se105_text_names_the_rule_and_the_way_out` | R2 (имя, правило, выход) |
| `every_empty_enum_speaks_for_itself` | R3 (три диагностики, включая вложенную модель) |
| `cause_precedes_consequence` | R3 (причина раньше следствия — через `normalize`) |
| `garbage_inside_braces_is_still_a_parse_error` | R4 (`enum E { 42 }` → `SY-002`) |
| `formatter_round_trip_is_stable` | R5 (`enum Mode {}`, идемпотентность) |
| `editor_shows_se105` | R6 (правило 29: LSP отдаёт `SE-105`) |
| `single_variant_enum_is_valid` | контр-пример: один вариант законен |

**Мутации (проверка сторожа, а не кода):**

| Мутация | Ответ тестов |
|---|---|
| `if en.variants.is_empty()` → `if false` | 5 падений |
| Грамматика возвращена к `CommaOne` | 5 падений |
| Перестановка `validate_empty_enums` и `validate_enum_values` | **0 падений** → утверждение о порядке проверок снято из комментария и анализа |
