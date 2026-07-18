# Отчёт о тестировании фичи 0059: структура `Shared` в цели `rust`

> Фича: [../features/0059-rust-shared-struct.md](../features/0059-rust-shared-struct.md) · тест-план: [../tests/0059-rust-shared-struct.md](../tests/0059-rust-shared-struct.md)

- **Дата:** 2026-07-18
- **Окружение:** macOS (darwin 25.5.0), rustc/clippy-driver nightly 1.99 (edition 2021).
- **Вердикт:** **готово.** 1988 тестов зелёные; корпус проходит `rustc` + `clippy
  -D warnings` без заглушек; `conformance_rust` вердикты не изменились.

## Сверка с критериями приёмки

| # | Критерий | Результат | Способ |
|---|---|---|---|
| A1 | `#[allow(too_many_arguments)]` в порождённом коде — нет | ✅ | `grep -rn` по `examples/generated/rust/` — пусто |
| A2 | генератор не эмитит `#[allow]` вовсе | ✅ | `grep 'p.ident("#[allow'` по `generator/rust/` — пусто |
| A3 | гейт `rust` (rustc + clippy `-D warnings`) на всём корпусе зелёный | ✅ | обёртка `#![no_std]` по 5 файлам — все OK |
| A4 | `conformance_rust_tests` — вердикты не изменились | ✅ | `cargo test --test conformance_rust_tests` — 4 passed |
| A5 | такт под-модели ≤ 3 параметров | ✅ | тест `submodel_tick_has_at_most_three_params`; `MovementController` было 10, стало 3 |
| A6 | `comprehensive` без `Shared` | ✅ | `grep "struct Shared" comprehensive.rs` пусто; тест `model_without_submodels_has_no_shared_struct` |
| A7 | переменная, не нужная под-моделям, не в `Shared` | ✅ | тест `shared_union_excludes_variables_no_submodel_needs` |
| A8 | `Shared` приватна | ✅ | `grep "pub struct Shared"` пусто |
| A9 | многоуровневая композиция (`extend_complex`) компилируется | ✅ | гейт зелёный; один `ExtendComplexShared` ретранслируется root→C→CC1 |
| A10 | `no_std` держится, `unsafe` отсутствует | ✅ | гейт под `#![no_std]`; `grep unsafe` пусто |

## Отклонения от проработки (с обоснованием)

- **Тип `Shared` — `<Root>Shared`, а не голое `Shared`.** В плоском модуле цели
  `rust` модели соседствуют, и голое имя столкнулось бы (ADR приводил `Shared`
  условно). Уточнение реализации, критериям не противоречит.
- **Правило 4 ADR («Shared на каждом уровне») — вырожденно.** Общие переменные
  root-центричны (`shared_variables` считает нужды от корня), поэтому владелец —
  только корень, а промежуточные уровни `Shared` ретранслируют. Многоуровневый
  случай (`extend_complex`) компилируется и верен (A9).
- **Вынос в новый модуль `rust_shared.rs`** сверх задач 0059-01/02 — потребовал
  храповик размера (`rust_model.rs` вышел бы за baseline). Baseline обновлён.

## Найденные дефекты

Нет. Побочная находка (5 `#[allow]` в самом генераторе) — вне объёма, кандидат.

## Примечание об эталоне (уроки 0045/0050)

Правка меняет **способ доступа** к переменным (`busy` → `shared.busy`) — класс,
где ошибка тихая. Поэтому основной критерий — A4 (потактовая сверка), а не гейт:
гейт доказывает компилируемость, но не верность. A4 выполнен.
