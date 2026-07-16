# Задача 0044-05: Форматтер — печать узла `InvariantDefine`

> Фича: [../features/0044-sim-assert-invariant.md](../features/0044-sim-assert-invariant.md) · ADR: [../adr/0044-sim-assert-invariant.md](../adr/0044-sim-assert-invariant.md) · анализ: [../analyze/0044-sim-assert-invariant.md](../analyze/0044-sim-assert-invariant.md)

## Что было

*Реальное состояние кода на 2026-07-15 (ветка `v2`).*

> **Задача обязательна, а не опциональна.** `CLAUDE.md` и ADR 0024 прямо
> предупреждают: добавляешь узел АСД — добавь и его печать в
> `grammar/src/format/`, иначе `format_source` начнёт **отказывать** на файлах с
> этим узлом (по замыслу: молча потерять кусок исходника хуже), а
> `grammar/tests/format_tests.rs` **завалит сборку** при появлении нового
> непокрытого узла.

- **Ядро печати** — `grammar/src/format/` (`mod.rs` 543 стр., `expr.rs` 376,
  `stmt.rs` 176, `comments.rs` 136). Публичная точка — `format_source`.
  Потребители: `lamc fmt` (`--check`/`--stdin`) и LSP `textDocument/formatting`.
- **`FormatError::Unsupported(String)`** — `format/mod.rs:36–44`: явный отказ на
  непокрытом узле. Живой пример отказа **уже есть**:
  `ast::ModelElement::Formula(_) => Err(FormatError::Unsupported("Formula".to_string()))`
  (`mod.rs:343`) — конструкция `formula "диалект" { … }` форматтером не
  поддерживается. Корпус её не содержит, поэтому тесты зелёные; **0044 это не
  чинит** (вынесено в кандидаты).
- **Образец печати — `cond`** (`mod.rs:344–353`):

  ```rust
  ast::ModelElement::Condition(c) => {
      // `cond Имя = условие;` — печатается ПЕЧАТЬЮ УСЛОВИЙ, а не выражений:
      // `=` здесь равенство (инвариант ADR 0019).
      let name = c.name.as_ref().map(|n| n.name.as_str()).unwrap_or("");
      out.node_line(&loc, &format!("cond {name} = {};", expr::condition(&c.value)?));
      Ok(())
  }
  ```

  Этот комментарий — прямая инструкция для 0044: правая часть инварианта
  печатается через `expr::condition`, **не** `expr::expression`.
- **Второй образец — `address`** (`mod.rs:358–366`): печатается через
  `expr::expression` (там действительно выражение). Не путать.
- **Печать элемента состояния** — `print_state_element_inner` (`mod.rs:452+`),
  `match` по `ast::StateElement` (5 вариантов).
- **Позиции для пустых строк и комментариев** — `mod.rs:237` (элемент модели),
  `state_element_loc` (`mod.rs:440–449`). Новый вариант обязан быть добавлен в
  **оба** `match` — иначе сборка не пройдёт (перечисление не исчерпано).
- **Тест по корпусу** — `grammar/tests/format_tests.rs`: собирает **158** файлов
  `.lam` из `examples/`, `grammar/tests/data`, `simulation/tests/data`
  (`corpus()`, стр. 23–34), проверяет **A1 — идемпотентность** (`fmt(fmt(x)) ==
  fmt(x)`) и **A3 — семантическую нейтральность** (`parse(fmt(x))` структурно
  равен `parse(x)`), печатает сводку покрытия и падает на новом непокрытом узле.
- **Синонимы не канонизируются** (ADR 0024): `Guard::explicit` (`ast.rs:862–866`)
  хранит, написал ли автор `: c;` или `: [Guard] c;`; `ast::LoopKeyword` — то же
  для `while`/`loop`. Печать `inline_formula` — `format/expr.rs:319–357`.

## Что сделано

> **Выполнено** (2026-07-16).

1. **Печать элемента модели** — ветка `ast::ModelElement::Invariant(i)` рядом с
   `Condition` (`mod.rs:344`), по тому же образцу:
   `format!("invariant {name} = {};", expr::condition(&i.value)?)`.
2. **Печать элемента состояния** — ветка `ast::StateElement::Invariant(i)` в
   `print_state_element_inner` (`mod.rs:452+`).
3. **Позиции** — новый вариант в `match` на `mod.rs:237` и в `state_element_loc`
   (`mod.rs:440–449`); по образцу `inline_formula_loc` (`mod.rs:257`,
   `expr.rs:351`).
4. **Никакой канонизации** (ADR 0024): `invariant P = C;` печатается как
   `invariant`, **не** разворачивается в `cond P = C; : [Guard] P;` — именно
   поэтому десахаризация (0044-02) живёт в семантике, а не в АСД.
5. **`ModelElement::Formula` не трогаем** — остаётся `Unsupported` (`mod.rs:343`).

**Статус по функциональности (правило 11):**

- `grammar/src/format/` — основная работа.
- `lamc fmt` и LSP `textDocument/formatting` — получают поддержку автоматически
  (общее ядро `format_source`).
- `simulation` — н/п.
- **Регресс:** корпус (158 файлов) обязан форматироваться как прежде; фикстуры
  `grammar/tests/data/` намеренно **не** нормализованы (тесты завязаны на их
  позиции) — `precheck.sh` проверяет канон только для `examples/`.

## Проверки

> **Планируется (разработка не начата).**

- **T28 (A16)** — `format_source` печатает `invariant` в обеих позициях; **не**
  возвращает `FormatError::Unsupported`.
- **T29** — идемпотентность: `fmt(fmt(x)) == fmt(x)`.
- **T30** — семантическая нейтральность: `parse(fmt(x))` ≡ `parse(x)`.
- **T31 (A17)** — `invariant P = x = 1;` печатается с `=`, **не** `:=`
  (правая часть — условие, инвариант ADR 0019).
- **T32** — **корпус целиком** (158 файлов) зелёный; сводка покрытия не
  показывает новых непокрытых узлов.
- **T33** — синонимы: файл с `: c;` и файл с `: [Guard] c;` печатаются каждый в
  своей форме (`Guard::explicit` не потерян).

```sh
cargo test --test format_tests -- --test-threads=1
cargo test --features lsp -- --test-threads=1     # LSP-форматирование
cargo run --bin lamc -- fmt --check examples/     # канон examples/
./scripts/precheck.sh
```

> **Историческая заметка (0024):** тесты LSP-форматирования не собирались без
> `--features lsp` (коммит `6984471`). Прогонять обе команды.

Соответствие анализу: **R21, R22, R23** → критерии **A16, A17**.
