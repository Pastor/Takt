# Задача 0296-01: Одна воронка стадий для пути импорта

> Фича: [../features/0296-semantic-stages-single-source.md](../features/0296-semantic-stages-single-source.md) · ADR: [../adr/0296-semantic-stages-single-source.md](../adr/0296-semantic-stages-single-source.md) · анализ: [../analyze/0296-semantic-stages-single-source.md](../analyze/0296-semantic-stages-single-source.md)

## Что было

`semantic/tree.rs::construct_model_impl` — путь, которым строится **каждый**
подключаемый файл (все три формы `import`), — перечислял стадии сам:
`0 → 1 → 2 → 3 → 5 → 4 → 6`, затем `validate_model`. Это вторая копия знания,
объявленного единственным в шапке `semantic/stages/mod.rs::construct_stages`.

Копия отстала на три прохода: `collect_clock`, `specialize_instantiations` и
`constify_parameters` подключаемому файлу не доставались вовсе.

## Что сделано

- `construct_stages` разделена на публичный вход (создаёт пустой стек импорта)
  и `construct_stages_within` — ту же функцию со **стеком вызывающего**. Стек
  нужен для обнаружения циклов `a → b → a`; отдельная функция, а не параметр
  публичного входа, — чтобы корневой вызов не мог передать чужой стек;
- `construct_model_impl` заменён на `build_imported_file`: перечисления стадий
  он не содержит, зовёт `construct_stages_within` и сводит список диагностик к
  первой после `normalize` (контракт стадии 0 импортёра — одна диагностика);
- флаг `specialize` протащен в `construct_model_stage0` и передаётся
  подключаемым файлам: режим сборки обязан быть у них тот же.

**Функциональность:**

- `takt-lang` — правка целиком здесь (`semantic/{stages/mod,tree}.rs`);
- `takt-sim` — н/п: эталон строит дерево через публичный вход, сигнатура
  которого не изменилась;
- LSP и плагины — н/п: `lsp/diagnostics.rs` зовёт `construct_stages` напрямую,
  то есть правится тот самый носитель (правило 29, разобрано в анализе).

## Проверки

```sh
cargo build --bin taktc
cargo test --all-features
```

Проба (ADR 0296, записи 1–3): файл с `cond T = after 5s;`, поданный корнем и
подключённый через `import`, даёт `SE-068` в обоих случаях; прежде подключённый
принимался молча. Автотесты — T1–T3 набора
`takt-lang/tests/semantic/stage_order_single_source_tests.rs`.
