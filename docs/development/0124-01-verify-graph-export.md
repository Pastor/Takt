# Задача 0124-01: Экспорт графов верификации (Крипке/Бюхи/произведение) в Graphviz DOT

> Фича: [../features/0124-verify-graph-export.md](../features/0124-verify-graph-export.md) · ADR: [../adr/0124-verify-graph-export.md](../adr/0124-verify-graph-export.md) · анализ: [../analyze/0124-verify-graph-export.md](../analyze/0124-verify-graph-export.md)

## Что было

Движок верификации строил `Kripke`/`BuchiAutomaton`/`Product` в памяти и отдавал
наружу только текст (вердикт, `--trace`-дампы, контрпример). Графического вывода
не было — раздел документа «Практический пример» не мог показать наглядное
построение модели Крипке и автомата Бюхи настоящим выводом инструмента.

## Что сделано

- **`verify::select_kripke`** (`verification/verify.rs`) — извлечён из `run`
  общий отбор «управляющая (0049) vs данными (0068)» Крипке; им пользуются и
  проверка, и экспорт (единый источник истины — диаграмма = проверявшийся граф).
- **`verify::build_graphs`/`build_control_kripke`** — публичные строители
  промежуточных структур (`VerificationGraphs { kripke, automaton, product }`)
  для экспорта; `Err` несёт вердикт-отказ.
- **`verification/dot.rs`** (новый модуль) — эмиттеры `kripke_to_dot`/
  `buchi_to_dot`/`product_to_dot`. Соглашения: старт — `shape=point`; принимающее
  — `doublecircle`; id узла — индекс вершины (в пути данных имя состояния делят
  несколько вершин); метки — компонентно экранированы, разделитель `\n` —
  настоящий перенос строки DOT.
- **CLI** (`bin/taktc.rs`) — `GraphKind` + флаг `--emit-graph <kind>` (раздельная
  и слитная формы, валидация значения) в `VerifyOptions`; ветка `run_emit_graph`
  в `run_verify` печатает DOT и возвращает код. `buchi`/`product` без `--property`
  → отказ.

Стек: **Rust** (движок + CLI) — основная работа. C/ST/rust/sv-генераторы,
симулятор, LSP — **н/п** (фича не трогает ни язык, ни цели, ни симуляцию).

## Проверки

- `cargo test -p takt-lang --lib verification::dot -- --test-threads=1` — 6/6.
- `cargo test --bin taktc verify_args -- --test-threads=1` — 12/12 (в т.ч. 2 новых
  на `--emit-graph`).
- Ручной рендер: `taktc verify --emit-graph kripke dispenser.takt | dot -Tsvg` —
  валидный SVG. `--emit-graph buchi -p "F Filling"` — принимающее состояние
  `doublecircle`. `--emit-graph buchi` без `-p` — отказ, код 1.
- `./scripts/precheck.sh` — зелёный (прежний вывод `verify`/`--trace` неизменен).
