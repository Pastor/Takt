# Разработка 0074-01: канонизация скобок паттерна `S(Модель)` в `resolve_condition`

- **Фича:** [0074](../features/0074-parenthesised-state-of.md)
- **ADR:** [0074](../adr/0074-parenthesised-state-of.md) (Option C)
- **Дата:** 2026-07-20
- **Файл:** `grammar/src/semantic/condition.rs`

## Что сделано

Добавлена канонизация паттерна `S(Модель) = Состояние` в единой воронке разбора
условий — снятие прозрачных обёрток `ConditionNode::Parenthesis` в трёх позициях
паттерна.

### Новые помощники (`condition.rs`)

- `strip_cond_parens(ConditionNode) -> ConditionNode` — цикл снятия
  `Parenthesis`. Применяется **только** к операндам паттерна `S(…)` (в других
  местах скобки несут группировку/вывод — см. ADR, Option B отвергнут).
- `is_state_of(&ConditionNode) -> bool` — предикат «текущее состояние модели»:
  `Function(Builtin "S", …)` либо краткая форма `Model(…)`. Распознаётся так же,
  как в генераторе C (`c_expr::condition::state_of_model`).
- `canonicalize_state_of(left, right_ast, model) -> Option<(left, right)>` —
  если `strip_cond_parens(left)` — паттерн состояния, возвращает пару со снятыми
  скобками (слева и у имени состояния справа); иначе `None` (вызывающий сохраняет
  прежнюю логику, включая ветку `EnumVariant`).

### Точки вызова

- Ветка `ast::Condition::Function`: если разрешённая функция — `Builtin("S")`,
  снять `Parenthesis` у каждого аргумента (форма `S((Ping))`).
- Ветки `ast::Condition::Equal` / `NotEqual`: перед прежней логикой вызвать
  `canonicalize_state_of`; при `Some` вернуть каноничный `Equal`/`NotEqual`
  (формы `(S(Ping)) = End`, `S(Ping) = (End)` и любая их вложенность).

### Импорт

`use crate::semantic::{… FunctionDefinitionNode …}` добавлен (нужен предикату
`is_state_of` и снятию скобок у аргумента `S`).

## Почему не тронуты потребители

Все `ConditionNode` строит `resolve_condition` (проход 6 для ref-рёбер,
`extract_conditions` для `cond`, `tree.rs` для LTL/Guard). Каноничная форма
доходит до `validate/common.rs` и генератора C без их правок. Спецслучай
`SE-025`/`SE-033` в `validate` не менялся.

## Проверка (проба на цели `c`)

- `(S(Ping)) = End`, `S((Ping)) = End`, `S(Ping) = (End)`, `((S((Ping)))) =
  (End)` → C **байт-в-байт** равен выводу `S(Ping) = End` (одинаковое имя файла).
- `S(Ping) = NoSuch` (и скобочные) → `SE-033`, не `SE-025`.
- `rust`/`st`/`sv`: скобочные формы ведут себя как бесскобочная
  (RS-020/ST-011/SV-002) — паттерн там не поддержан, фикс лишь уравнял формы.

## Замечания

- Байтовая неизменность корпуса — следствие тождественности strip на
  бесскобочных формах; страхует гейт детерминизма `precheck.sh`.
- `Builtin` хранит `&'static str` → сравнение `*name == "S"` (не `name == "S"`).
