# Задача 0050-01: Каркас бэкенда — `Language::Rust`, `generator/rust/`, цель `-t rust`

> Фича: [../features/0050-rust-backend.md](../features/0050-rust-backend.md) · ADR: [../adr/0050-rust-backend.md](../adr/0050-rust-backend.md) · анализ: [../analyze/0050-rust-backend.md](../analyze/0050-rust-backend.md) · тест-план: [../tests/0050-rust-backend.md](../tests/0050-rust-backend.md)
>
> Требования: **R1**. Критерии: **A1**, **A11**.
> Порядок: **первая**; за ней — [0050-02](0050-02-gate.md) (гейт **до** эмиссии).

## Что было

*Проверено чтением кода 2026-07-16.*

- `generator/mod.rs:13` — `enum Language` помечен `#[non_exhaustive]`, варианты
  `C`, `PlantUML`, `ST`. Комментарий прямо разрешает расширение.
- `generator/mod.rs:95` — трейт `Generator` (`generate(&self, model, output_path,
  options) -> Result<(), Diagnostic>`).
- `generator/mod.rs:105` — диспетчер `generate(l: Language, …)`: `match` по
  варианту, по генератору на ветку.
- `generator/indent.rs` — общий слой печати с отступами (переиспользуется).
- `semantic/minimap.rs` — снимок достижимых состояний/моделей (`BTreeMap` →
  детерминизм, фича 0048).
- `lib.rs` — по функции на цель: `compile_to_c`, `compile_to_c_hal`,
  `compile_to_st`, `compile_to_st_at`, `compile_to_plantuml`.
- `bin/lamc.rs` — цели `c`/`c-hal`/`plantuml`/`st`/`st-at` в `parse_compile_args`
  и `print_usage`.

**Общего слоя бэкендов нет и он не заводится** — решение из `FEATURES.md`
(«0045 каркаса не ждёт»): `Language` уже расширяем; вычленять каркас на пятом
бэкенде — преждевременное обобщение.

## Что сделано

> **Планируется (разработка не начата).** План по ADR 0050.

1. `Language::Rust` + ветка в диспетчере `generate()`.
2. Модуль `grammar/src/generator/rust/` — сразу дробно (правило «≤ ~1000 строк»,
   образец — `generator/c/`): `mod.rs` (`Generator`, отображение типов),
   `rust_map.rs` (`RustMap` — снимок по образцу `CMap`), далее по задачам
   `rust_decl.rs` / `rust_expr.rs` / `rust_model.rs`.
3. `grammar::compile_to_rust(filename, source, output_path, search_paths,
   options)` — по образцу `compile_to_c`.
4. CLI: значение `rust` для `-t` (`parse_compile_args`), строка в `print_usage`,
   расширение (`.rs`) в выборе имени выходного файла.
5. Каркас эмитит **минимальный компилируемый** модуль (шапка `#![no_std]` +
   пустая `struct` модели) — наполнение в 0050-03…07. Заглушка **не молчит**: не
   реализованная конструкция даёт `Diagnostic`, а не комментарий-заглушку
   (наследие [ADR 0028](../adr/0028-c-generator-stubs.md): проглоченная ошибка +
   код возврата 0 = мёртвый автомат на объекте).

## Проверки

- `lamc compile -t rust examples/elevator_mini.lam` создаёт `.rs`; `rustc
  --crate-type=lib` его принимает (каркас обязан компилироваться с первого дня —
  иначе гейт 0050-02 нечем включать).
- **A11:** `git diff examples/generated/` по целям `c`/`c-hal`/`plantuml`/`st`
  пуст; `grammar.lalrpop` и версия языка не тронуты.
- Тесты аргументов CLI: `-t rust` разобран; неизвестная цель по-прежнему ошибка.
- `./scripts/precheck.sh` зелёный.
