# Разработка 0018-01: срезовые типы, `#[non_exhaustive]`, чистка мёртвого кода

- **Фича:** [0018](../features/0018-code-guidelines.md)
- **Подзадача:** 0018-01
- **Статус:** ВЫПОЛНЕНО
- **Анализ:** [docs/analyze/0018-code-guidelines.md](../analyze/0018-code-guidelines.md)

## Область

Первая, безопасная и легко тестируемая партия задач плана: P01–P04, P06.

## Сделано

| Задача | Изменение | Файл |
|--------|-----------|------|
| P01 | `const_expr_string(name: &String)` → `name: &str` | `grammar/src/generator/c/c_decl.rs` |
| P02 | `create_svg(edges_vec: &Vec<…>)` → `&[…]` | `simulation/src/unit/viewport.rs` |
| P03 | `Comment::value(&self) -> &String` → `-> &str` (снят лишний `const`) | `grammar/src/parser/ast.rs` |
| P04 | `#[non_exhaustive]` на `Language` и `ErrorType` | `grammar/src/generator/mod.rs`, `grammar/src/diagnostics.rs` |
| P06 | Удалён мёртвый тип-алиас `Source = (Option<String>, Option<String>)` | `grammar/src/generator/mod.rs` |

## Решения

- **P06 — удаление вместо Newtype.** План предполагал замену кортежа-алиаса на
  Newtype, но алиас `Source` оказался **полностью неиспользуемым** (генераторы
  возвращают `header`/`source` как отдельные `String`). По YAGNI (CODE.md,
  «Не вводи преждевременные абстракции») алиас удалён, а не завёрнут в Newtype.
- **P04 — осознанный подбор.** `#[non_exhaustive]` добавлен только к реально
  расширяемым `Language` (целевые языки) и `ErrorType` (категории ошибок).
  `Level` **намеренно пропущен**: это стабильный enum серьёзности из 4 значений,
  плотно используемый во внешних сравнениях (`grammar/tests/diagnostics_tests.rs`),
  расширять его не планируется. Внешнее конструирование известных вариантов
  (`Level::Debug` и т.п.) `#[non_exhaustive]` не ломает; исчерпывающих внешних
  `match` по `Language`/`ErrorType` в проекте нет — проверено grep'ом по
  `grammar/tests`, `grammar/src/bin`, `simulation/src`.

## Проверка

- `cargo build --all-targets --all-features` — успешно.
- `cargo test --features lsp -- --test-threads=1` — все наборы зелёные, падений нет.
- Изменения только на уровне типов/сигнатур; поведение генерации и диагностик
  не менялось (регресс отсутствует).

## Осталось в фиче 0018

P05 (опции генератора вместо `guard_enable: bool`), P07 (Builder `GraphicsConfig` —
с проверкой YAGNI), P08/P10 (аудит `.clone()`/`mem::take`), P09, P11 (`with_capacity`),
P12 (doctests), P13 (`new()`/`Default`), P04b (`#[non_exhaustive]` на узлах AST/`TypeNode`).
