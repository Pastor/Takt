# История изменений

Все значимые изменения проекта фиксируются в этом файле.

Формат основан на [Keep a Changelog](https://keepachangelog.com/en/1.0.0/).
Версионирование следует [Semantic Versioning](https://semver.org/).

## [Не выпущено]

### Добавлено

- Внедрён единый процесс разработки через жизненный цикл фич (фича 0017):
  `FEATURES.md` (витрина незакрытых фич, три блока по правилам 10/17/21) и реестры
  стадий `docs/{features,adr,analyze,development,tests,reports,fixes}/README.md`
  (заголовки таблиц согласованы с `scripts/new-feature.sh --register`).
- Реконструирована история проекта в карточки фич `docs/features/`: `0001`–`0016`
  (статус `ГОТОВО`); открытые фичи `0018`–`0020` из бывших `TODO.md`/`STATUS.md`.
- Документация (`///`) для всего публичного API: `verification::ltl::Ltl`, `verification::buchi::BuchiAutomaton`,
  `semantic::formula::Formula`, `semantic::minimap::{Name, StateExtend, Element, Map}` и их методов.
- Крейт `simulation`: crate-level doc-comment в `lib.rs` и `bin/simulation.rs`.
- Type alias `PortMap` в `generator/c/c_header.rs` для упрощения сложного возвращаемого типа.
- Type alias `NodeMap` в `verification/buchi.rs` для типа таблицы узлов GPVW-алгоритма.

### Изменено

- Фича 0018, подзадача 0018-03: P11 — `Printer::calculate_padding` использует
  `" ".repeat(n)` (одна аллокация точной ёмкости) вместо посимвольного цикла;
  P04b — `#[non_exhaustive]` на публичных enum `parser::ast::{Type, ModelElement,
  StateElement, Expression, Statement}` и `semantic::type_node::TypeNode`
  (внешняя совместимость по правилу 11; внутрикрейтовая исчерпывающая проверка
  сохранена); P13 — аудит `new()`/`Default` показал адекватное покрытие,
  изменений не потребовалось. Поведение не изменилось.
- Фича 0018, подзадача 0018-02 (P05): «bool trap» `guard_enable` заменён на тип
  опций `generator::GenerateOptions` (`#[non_exhaustive]` + `Default` + `new()`,
  реэкспорт `grammar::GenerateOptions`). Проведён через трейт `Generator`,
  диспетчер `generator::generate`, публичную `compile_to_c(..., &GenerateOptions)`
  и `bin/lamc.rs`. P07 (Builder `GraphicsConfig`) признан ненужным по YAGNI
  (конфиг конструируется только из serde/`Default`). Поведение не изменилось.
- Фича 0018 (приведение кода к `docs/CODE.md`), подзадача 0018-01: срезовые типы
  в сигнатурах — `const_expr_string(name: &str)` (`generator/c/c_decl.rs`),
  `create_svg(edges_vec: &[…])` (`simulation/unit/viewport.rs`),
  `Comment::value() -> &str` (`parser/ast.rs`); `#[non_exhaustive]` на
  `generator::Language` и `diagnostics::ErrorType`; удалён неиспользуемый
  тип-алиас `generator::Source`. Поведение не изменилось, тесты зелёные.
- `docs/RULE.md` — содержит только правила (эталон процесса); секция контекста
  удалена. Процесс управления работой — исключительно через жизненный цикл фич.
- `CLAUDE.md` — переписан как контекст-only (архитектура, ключевые файлы,
  инварианты) со ссылкой на `docs/RULE.md` (правила) и `docs/CODE.md` (Rust-код);
  `AGENTS.md` остаётся симлинком на `CLAUDE.md`.
- Скрипты перенесены в `scripts/`: `precheck.sh`, `run_simulations.sh` — пути
  внутри исправлены на корень репозитория (работают из любого каталога).
- `PLAN.md` перенесён в `docs/analyze/0018-code-guidelines.md` (анализ фичи 0018).

### Удалено

- Плоские списки задач `TASKS.md`, `STATUS.md`, `TODO.md` — содержимое перенесено
  в карточки фич (`docs/features/`).
- Каталог `changes/` с ручными патчами — процесс перешёл на карточки фич + коммиты.

### Исправлено

- `semantic/minimap.rs`: видимость `StateExtend` изменена с `pub(crate)` на `pub`
  (устранено предупреждение `private_interfaces`).
- `verification/buchi.rs`: устранены предупреждения clippy — `needless_return`,
  `needless_range_loop`; добавлен `#[allow(clippy::too_many_arguments)]` для `expand`.
- `bin/snapshot.rs`: убрано лишнее приведение `t_start as f64`; добавлен
  `#[allow(clippy::too_many_arguments)]` для `rects_intersection_area` и `energy`.
- `semantic/condition.rs`, `semantic/tree.rs`, `semantic/validate.rs`,
  `generator/c/c_expr.rs`: схлопнуты вложенные `if let` в цепочки `if let ... && let ...`
  (устранены предупреждения `collapsible_if`).

### Изменено

- Улучшена отрисовка рёбер в `snapshot.rs`: добавлено разведение кратных (параллельных и обратных) рёбер
  и ребер из разных источников за счет индивидуальных коэффициентов изгиба и смещения точек входа/выхода.

### Добавлено

- Добавлен коэффициент искривления рёбер (`CURVE_COEFFICIENT`) в `snapshot.rs`.
- Добавлен параметр размера стрелки рёбер (`ARROW_SIZE`) в `snapshot.rs`.
- Добавлен предел приближения объектов (`MIN_DISTANCE`) в `snapshot.rs`, по умолчанию равный `RADIUS`.
- Белый фон для генерируемого SVG изображения в `snapshot.rs`.
- Генератор диаграмм состояний PlantUML (`--target plantuml`):
    - `generator/plantuml/puml_map.rs` — снимок семантической карты модели для PlantUML
    - `generator/plantuml/mod.rs` — генерация `.puml`-файла из `PumlMap`
    - `Language::PlantUML` в `generator/mod.rs`
    - Публичная функция `compile_to_plantuml()` в `lib.rs`
    - Поддержка `--target plantuml` в CLI (`lamc.rs`)

### Исправлено

- Исправлены ошибки компиляции в `snapshot.rs` для совместимости с `rand` 0.10.1 и `svg` 0.18.0.
- Обновлены вызовы генерации случайных чисел (переход на `rng()`, `random()`, `random_range()`).
- Исправлено использование элементов SVG (`Style`, `Marker`, `Text`, `Definitions`).
- Устранена неоднозначность типов в вызовах `sample` и `max`.
- Удалены неиспользуемые импорты.
