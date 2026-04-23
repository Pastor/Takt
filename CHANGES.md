# История изменений

Все значимые изменения проекта фиксируются в этом файле.

Формат основан на [Keep a Changelog](https://keepachangelog.com/en/1.0.0/).
Версионирование следует [Semantic Versioning](https://semver.org/).

## [Не выпущено]

### Добавлено
- Генератор диаграмм состояний PlantUML (`--target plantuml`):
  - `generator/plantuml/puml_map.rs` — снимок семантической карты модели для PlantUML
  - `generator/plantuml/mod.rs` — генерация `.puml`-файла из `PumlMap`
  - `Language::PlantUML` в `generator/mod.rs`
  - Публичная функция `compile_to_plantuml()` в `lib.rs`
  - Поддержка `--target plantuml` в CLI (`lamc.rs`)
