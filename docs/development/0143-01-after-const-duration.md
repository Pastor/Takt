# Задача 0143-01: грамматика, узел АСД и исчерпывающие обходы

> Фича: [../features/0143-after-const-duration.md](../features/0143-after-const-duration.md) · ADR: [../adr/0143-after-const-duration.md](../adr/0143-after-const-duration.md) · анализ: [../analyze/0143-after-const-duration.md](../analyze/0143-after-const-duration.md)

## Что было

После `after` грамматика допускала **только** терминал `duration` либо `ticks`.
Проба 2026-07-29:

```
probe.takt:16:25: Ошибка компиляции [SY-002]: нераспознанный токен 'OVERRUN',
ожидалось duration, ticks
```

## Что сделано

- **Грамматика** (`takt-lang/src/grammar.lalrpop`, правило `ConditionExpression`)
  — две новые ветви:
  - `"after" <Identifier>` → `Condition::AfterExpr(loc, Variable(имя))`;
  - `"after" "(" <Condition> ")"` → `Condition::AfterExpr(loc,
    Parenthesis(выражение))`.

  Прежние ветви `"after" duration` и `"after" ticks` **не тронуты** — отсюда
  обратная совместимость по построению. Конфликта LR(1) нет: после `after`
  различающий токен — терминал длительности, терминал тактов, идентификатор или
  `(`.

  ⚠️ **Скобки сохраняются в дереве** (узел `Parenthesis`), а не снимаются при
  разборе: `after` связывает крепче арифметики, поэтому напечатанное без скобок
  `after A + 1s` разобралось бы как `(after A) + 1s` — форматтер испортил бы
  программу.

- **Узел АСД** (`takt-lang/src/parser/ast.rs`): вариант
  `Condition::AfterExpr(Location, Box<Condition>)` + строка в `loc()`.
  Наносекунд в узле нет — значение вычисляет семантика (задача 0143-02).

- **Исчерпывающие обходы.** Компилятор указал ровно шесть мест, где новый вариант
  обязан быть разобран (это и есть страховка проекта — ни одно не пропущено
  молча):

  | Файл | Что решено |
  |---|---|
  | `format/expr.rs` | печать `after <вложенное условие>` (рекурсивно) |
  | `semantic/time_ast.rs::find_after` | позиция для `SE-068` — выдержка вне ребра остаётся ошибкой |
  | `semantic/time_ast.rs::raw_has_after_kind` | вид выдержки — **длительностный** (тактовых констант не бывает) |
  | `semantic/validate/implicit_bool.rs` | результат выдержки булев, неявным приведением не является |
  | `semantic/usages/walk.rs` | рекурсивный обход: имена внутри — **использования** констант |
  | `semantic/condition.rs` | ветвь-вызов вычислителя (задача 0143-02) |

- **LSP** (`lsp/keywords.rs`): текст hover-подсказки `after` перечисляет новые
  формы. Разбор токенов и список автодополнения правок не требуют — `after` в них
  уже есть (правило 29; полный ответ по редакторскому слою — в анализе).

## Проверки

```sh
cargo build --bin taktc
cargo test -p takt-lang --lib after_const -- --test-threads=1
cargo test -p takt-lang --test after_const_duration_tests --features lsp -- --test-threads=1
```

- R1 (форма разбирается) — проба:
  `taktc compile -t c --tick-hz=1000` на модели с `after OVERRUN` компилируется и
  печатает `SE-071` (`180000000000 нс = 180000 тактов`).
- R2/R9 (обратная совместимость, печать) — `taktc fmt --stdin` возвращает
  `ref Idle: after ((BASE + TRIM) - 30s);` байт-в-байт как написано.
- R7 (`SE-068`) — тест
  `after_const::tests::named_dwell_outside_reference_is_se068`.
- R10 (использования) — тесты `editor::references_include_name_inside_after`,
  `editor::rename_updates_name_inside_after`.
