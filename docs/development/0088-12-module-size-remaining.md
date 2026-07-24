# Задача 0088-12: Симуляция `unit/viewport` — вынос подмодуля `graph`

> Фича: [../features/0088-module-size-remaining.md](../features/0088-module-size-remaining.md) · ADR: [../adr/0088-module-size-remaining.md](../adr/0088-module-size-remaining.md) · анализ: [../analyze/0088-module-size-remaining.md](../analyze/0088-module-size-remaining.md)

## Что было

`simulation/src/unit/viewport.rs` — **1455 строк** (нарушитель, последний из
безопасной части). Врезка `FEATURES.md` пессимистично оценивала его как «одна
функция `create_svg` ~1128 строк, требует разбиения тела на под-функции (риск
изменить SVG)». **Факт при осмотре иной:** `create_svg` — ~550 строк, а внутри
файла есть самодостаточный **inline-подмодуль** `pub(super) mod graph` (~567
строк вместе со своими тестами), зависящий только от `super::Positions` и типов
крейта.

## Что сделано

Подмодуль `graph` вынесен в **файл-модуль** `simulation/src/unit/viewport/graph.rs`
(строки 735–1299 → отдельный файл, дедент 4 пробела; `viewport.rs` объявляет
`pub(super) mod graph;`, doc-комментарий сохранён над объявлением):

- **`create_svg` не тронут вовсе** → SVG-вывод байт-в-байт неизменен, риск из
  врезки **не возникает** (нужды дробить тело функции нет — естественная граница
  модуля решает задачу).
- Публичные пути неизменны: `graph::unit_to_graph`/`graph::calculate_graph`
  зовутся из `compute_layout` как прежде (`super` внутри `graph` по-прежнему =
  `viewport`, `use super::Positions` цел).
- Тесты подмодуля (`test_dist_*`, `test_segments_*`, `test_unit_to_graph_*`,
  `test_calculate_graph_*`) переехали вместе с ним.

**Чистое перемещение:** `viewport.rs` **1455 → 889**; `viewport/graph.rs` — 567;
оба ≤ 1000 → запись удалена из реестра (**7 → 6**). `cargo fmt -p simulation`
применён к вынесенному файлу.

Стеки: только `simulation`. `grammar` — н/п.

## Проверки

- `cargo test -p simulation --lib unit::viewport` — **31 passed, 0 failed**
  (тесты `viewport` + подмодуля `graph`).
- `./scripts/precheck.sh` — зелёный (`check-module-size.sh` −1, все тесты,
  `run_simulations.sh` не в precheck, но GIF-путь кода не менялся).

## Итог по безопасной части фичи 0088

Реестр **18 → 6**. Оставшиеся 6 — **ядро семантики** (`tree`, `mod`,
`expression`, `type_inference`) и **пришпиленные** (`lamc`, `lib`): это scope
**фичи-преемника** (ADR 0088, Option B), заводится при закрытии 0088.
