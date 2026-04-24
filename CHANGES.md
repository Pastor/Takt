# История изменений

Все значимые изменения проекта фиксируются в этом файле.

Формат основан на [Keep a Changelog](https://keepachangelog.com/en/1.0.0/).
Версионирование следует [Semantic Versioning](https://semver.org/).

## [Не выпущено]

### Добавлено

- Документация (`///`) для всего публичного API: `verification::ltl::Ltl`, `verification::buchi::BuchiAutomaton`,
  `semantic::formula::Formula`, `semantic::minimap::{Name, StateExtend, Element, Map}` и их методов.
- Крейт `simulation`: crate-level doc-comment в `lib.rs` и `bin/simulation.rs`.
- Type alias `PortMap` в `generator/c/c_header.rs` для упрощения сложного возвращаемого типа.
- Type alias `NodeMap` в `verification/buchi.rs` для типа таблицы узлов GPVW-алгоритма.

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
