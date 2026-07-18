# Отчёт о тестировании фичи 0058: хвостовой `if/else` с `return` в цели `rust`

> Фича: [../features/0058-rust-tail-return-if-else.md](../features/0058-rust-tail-return-if-else.md) · тест-план: [../tests/0058-rust-tail-return-if-else.md](../tests/0058-rust-tail-return-if-else.md)

- **Дата:** 2026-07-18
- **Окружение:** macOS (darwin 25.5.0), rustc/clippy-driver nightly 1.99 (edition 2021).
- **Вердикт:** **готово.** 1985 тестов зелёные; свёрнутая форма принята `clippy
  -D warnings`; `examples/generated/rust` побайтово неизменны.

## Сверка с критериями приёмки

| # | Критерий | Результат | Способ |
|---|---|---|---|
| A1 | `if a>b {return a;} else {return b;}` → `if a>b {a} else {b}`, clippy принимает | ✅ | зонд + `clippy-driver -D warnings` (обёртка `#![no_std]` как в precheck); тест `tail_if_else_folds_to_expression` |
| A2 | `examples/generated/rust/*.rs` побайтово прежние | ✅ | `git diff --stat examples/generated/rust` пуст после регенерации всего корпуса |
| A3 | `if` в хвосте без `else` — как сегодня (`return` остаётся) | ✅ | предикат-тест `if_without_else_is_not_foldable`; codegen `non_tail_if_keeps_return` |
| A4 | Смешанные ветки не сворачиваются | ✅ | предикат-тест `mixed_branches_are_not_foldable` |
| A5 | `if / else if / else` с `return` сворачивается целиком, clippy принимает | ✅ | зонд (`fn grade`) + тест `tail_else_if_chain_folds` |
| A6 | Гейт цели `rust` (rustc + clippy) зелёный | ✅ | вывод побайтово прежний (A2) → результат гейта неизменен; свёрнутая форма проверена зондом |
| A7 | `conformance_rust_tests` не меняет вердиктов | ✅ | `cargo test --test conformance_rust_tests` — 4 passed |

## Примеры и контрпримеры (правило 16)

- **Пример (свёртка):** `fn pick(a,b){ if a>b {return a;} else {return b;} }` →
  `if a > b { a } else { b }`.
- **Пример (цепочка):** `fn grade(a){ if a>10 {return 3;} else if a>5 {return 2;}
  else {return 1;} }` → вложенный `if/else` без единого `return`.
- **Контрпример (не сворачивается):** `fn clip(a){ if a>10 {return 3;} return 1; }`
  → ранний `if` без `else` сохраняет `return 3;`, сворачивается лишь хвостовой
  `return 1;` → `1`.

## Найденные дефекты

Нет. Побочная находка проработки (`clippy::new_without_default` на модели без
портов) — вне объёма 0058, живёт кандидатом в `FEATURES.md`.

## Замечание об эталоне (уроки 0045/0050)

Гейт (`clippy -D warnings`) доказывает **компилируемость**, но не верность —
поэтому A7 (потактовая сверка) обязателен и выполнен: разворот синтаксический,
поведение неизменно.
