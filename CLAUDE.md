# CLAUDE.md — контекст проекта Lam

Живой контекст проекта для сессий Claude Code (claude.ai/code) и других
AI-инструментов. Здесь — **только контекст** (что за проект, архитектура,
ключевые файлы, инварианты, состояние), без правил процесса.

- **Правила работы — эталон:** [`docs/RULE.md`](docs/RULE.md). Процесс = жизненный
  цикл фич; `FEATURES.md` — витрина незакрытых фич, карточки фич — в
  `docs/features/`. Плоских списков задач (`TASKS.md`/`STATUS.md`) нет.
- **Правила написания Rust-кода:** [`docs/CODE.md`](docs/CODE.md).
- **Журнал изменений:** [`CHANGES.md`](CHANGES.md) (формат Keep a Changelog).

## Проект

**Lam** (Language of Automata Models) — DSL для спецификации и синтеза конечных
автоматов (FSM) промышленных систем управления. Компилируется в C. Rust-workspace
из двух крейтов:

- `grammar` — компилятор (бинарник `lamc`) и LSP-сервер (`lam-lsp`).
- `simulation` — симулятор моделей и GIF-визуализация.

## Команды

```sh
# Сборка
cargo build --bin lamc
cargo build --features lsp --bin lam-lsp

# Тесты (однопоточно — иначе гонки за общие файлы)
cargo test -- --test-threads=1
cargo test --features lsp -- --test-threads=1   # включая тесты LSP
cargo test <имя> -- --test-threads=1            # один тест по имени

# Предкоммит-проверка (fmt + check + clippy + test + сборка примеров)
./scripts/precheck.sh

# Прогон всех симуляций из examples/simulations/
./scripts/run_simulations.sh
```

CI (`.github/workflows/ci.yml`): `cargo build --all-features --all-targets
--examples`, `cargo check`, `cargo test`.

## Архитектура

Конвейер компиляции: `.lam` → **лексер** (`parser/lexer.rs`) → **парсер**
(LALRPOP LR(1), `grammar.lalrpop`) → AST (`parser/ast.rs`) → **семантика**
(7 проходов, `semantic/`) → **генераторы** C (`generator/c/`) и PlantUML
(`generator/plantuml/`). Верификация свойств — `verification/` (LTL, автоматы Бюхи).

### Семантические проходы (по порядку)

| Проход | Файл | Назначение |
|---|---|---|
| 0 | `tree.rs` | Имена моделей/состояний, загрузка импортов |
| 1 | `tree.rs` | Составные состояния (`M1 + M2`, `M1 \| M2`) |
| 2 | `type_inference.rs` | Переменные, вывод типов, порты |
| 3 | `tree.rs` | Именованные условия (`cond`) |
| 4 | `tree.rs` | Именованные блоки (`enter`/`exit`/`always`) |
| 5 | `tree.rs` | Тела функций |
| 6 | `tree.rs` | Замена `Condition::Unresolved` разрешёнными условиями |

### Критические инварианты (НЕ нарушать)

- **Условия рёбер `ref` не разрешаются.** `ref Next: expr;` хранится как
  `Condition::Unresolved(ast::Condition)`. НЕ добавлять проход
  `resolve_state_references` — ломает `S(Ping) = End`. Охраняется тестом
  `syntax_simple`. (`semantic/reference.rs` — это структура данных
  `ReferenceNode`, а НЕ запрещённый проход разрешения.)
- **Операторы `:=` / `=` (фича 0021, версия языка 0.1.0):** присваивание —
  `:=` (`Expression::Assign`); сравнение на равенство — `=` в выражениях
  (`Expression::Equal`) **и** в условиях (`Condition::Equal`); `==` **выведен**
  из языка (ошибка разбора). Реляционный `<=` без изменений (`LessEqual`). Знак
  `=` также связка в определениях имён (`type`/`enum`/`model`/`state`/`cond`),
  инициализаторы `var`/`const`/портов — через `:=`. AST-узлы прежние — сменилась
  лишь привязка токенов в `grammar.lalrpop` (токен `:=` = `Token::ColonAssign`).
- **Адрес порта (фича 0020, версия языка 0.2.0):** три источника адреса с
  приоритетом **inline `:=` < оператор `address Имя = <адрес>;` < внешняя карта**
  (`--address-map`, `.ld`-подобный формат `address_map.rs`). Слой
  `AddressMap` (`resolve_addresses`). Диагностики: SE-048 (висячая привязка),
  SE-049 (конфликт inline + `address`), SE-050 (оверлей карты), SE-051 (висячая
  запись карты), SE-052 (used-порт без адреса), AM-001…006 (формат карты).
  Потребление — только цель `-t c-hal` (таблица адресов + дефолтный HAL через
  `*(volatile*)`); цель `c` адрес **не эмитит** (порты через HAL-колбэки) и
  байт-в-байт неизменна. `address` — жёсткое ключевое слово.

### Ключевые файлы

- `grammar/src/lib.rs` — публичный API: `parse`, `compile_to_c`,
  `compile_to_plantuml`, `unused_variable_warnings`,
  `nondeterministic_transition_warnings`.
- `grammar/src/semantic/{mod,tree,validate,type_inference,index}.rs` — семантика.
- `grammar/src/generator/{c,plantuml}/` — генераторы кода/диаграмм.
- `grammar/src/verification/{ltl,buchi}.rs` — верификация LTL/Бюхи.
- `grammar/src/lsp.rs` — LSP (`position_to_offset`, `node_at_position`, `hover_info`).
- `grammar/src/bin/lamc.rs` — CLI компилятора (`lamc compile -t c|plantuml …`).
- `simulation/src/{lib,runner,gif,state_io}.rs`, `simulation/src/unit/` — симуляция.
- Тесты: `grammar/tests/{semantic_tests,lsp_tests}.rs`; фикстуры — `grammar/tests/data/`.

## Технические подводные камни

- **Пути** — через `std::path::Path` (не разбивка строк). При генерации C явно
  обрабатывай анонимные/корневые модели (получают имя `Root`) и пустые случаи.
- **Генератор C дефектен на `Array`/`Bit`/`Rational`** (выявлено анализом 0025;
  в бэклоге `FEATURES.md`). `get_c_type` (`generator/c/mod.rs:150`): `Array(size,
  elem)` → `uint{size}_t`, где `size` — **число элементов**, а не разрядность
  (`[u8; 4]` → невалидный `uint4_t`), и переменная-массив **молча исчезает** из
  структуры; `Bit` → `int`; `Rational` → `float` (f32). **Не используй вывод C как
  эталон** для этих типов (сверка симулятора с C — только `Integer`/`Enum`/`Bool`).
- **Разрешение переменных** проверяй в обоих путях — верхнеуровневый и вложенный
  доступ к модели (включая `tick`/`init`): исправление в одном пути часто ломает
  другой.
- **Размер модуля** — не больше ~1000 строк (вместе с тестами); дели по логике.
- **Новые тесты** — сперва зонд для захвата реального вывода, затем assertions
  против захваченных значений (не угадывать строки/адреса).
- **Симулятор почти починен (фича 0025, в работе).** Выражения, условия,
  операторы (`var`, `while`, `for`, `match`) и **вызовы функций** работают
  (задачи 0025-01/02a/02b-1/02b-2/03). Вся семантика значений — **только** в
  `simulation/src/eval/` (`ops.rs` — операции, `mod.rs::coerce_to_type` —
  приведение по типу, `error.rs` — `EvalError`/`SIM-0xx`); адаптеры
  (`predicate.rs`, `expression.rs`) семантики не содержат; операторы и тела `fn`
  исполняет **один** интерпретатор `unit/statement.rs::exec_statement`. Новую
  семантику вычислений добавляй в `eval/`, а не в адаптеры. `Value` — в
  `crate::eval::value::Value`. Модуль `eval` под
  `deny(clippy::wildcard_enum_match_arm)`: ветка `_` по вариантам узла завалит
  сборку (так и задумано; исключение — `coerce_to_type`, т.к. `TypeNode` помечен
  `#[non_exhaustive]`). **Ещё не сделано:** `enter` стартового состояния не
  исполняется (Д5, задача 04); ошибка вычисления **неотличима** от ложного условия
  (`Predicate` → `bool`, задача 05) — до неё ошибки **печатаются** в stderr, а не
  теряются. Новые тесты симуляции пиши на вычисленные значения, а не на факт
  перехода (тесты были зелёными именно потому, что значений не сверяли).
  **Ловушки языка** (обе в бэклоге): `examples/comprehensive.lam` дефектен —
  заявленный в комментарии сценарий `→ Cooling → Done` недостижим по его же
  логике, не принимай за эталон; **функции не могут вызывать функции**
  (семантика отвергает), поэтому рекурсия невозможна.
## Добавление конструкции языка

1. `grammar.lalrpop` + `lexer.rs`. 2. AST-узел в `parser/ast.rs`. 3. Семантический
проход в `semantic/tree.rs`. 4. Генератор в `generator/`. 5. Фикстуры в
`tests/data/semantic/{valid,invalid}/`. Изменение языка → рост версии языка
(правило 22).

## Состояние

Актуальное состояние и последнее изменение — в `CHANGES.md` (раздел
`[Не выпущено]`). Незакрытые фичи и порядок работ — в `FEATURES.md`.
