# Задача 0046-02: Механическая чистка остатка clippy + rustc

> Фича: [../features/0046-build-warnings-cleanup.md](../features/0046-build-warnings-cleanup.md) · ADR: [../adr/0046-build-warnings-cleanup.md](../adr/0046-build-warnings-cleanup.md)

## Что было

После [0046-01](0046-01-location-shrink.md) — ~135 clippy (обычных) + 3 rustc.

## Что сделано

- **`cargo clippy --fix --all-targets --all-features`** (два прохода): needless_ref
  (24), collapsible_if (18), clone_on_copy (16), needless_borrow (6),
  redundant_closure, map_or_else, … Вывод генераторов сверен — байт-в-байт неизменен.
- **Ручные `#[allow]` с обоснованием у места** (осознанные компромиссы, как ~38
  существующих): `too_many_arguments` (печатники `c_expr/fixed.rs`
  `binary`/`negate`/`cast`/`rescale`, `st_stmt::print_for`, `viewport::create_svg`,
  `SimulationRunner::new`); `large_enum_variant` (`UnitKind` — `Node` доминирует);
  `type_complexity` (`Predicate.func`); `upper_case_acronyms` (`Viewport::SVG`);
  `redundant_guards` (`if b == 0.0` — паттерн `0.0` дал бы
  `illegal_floating_point_literal_pattern`).
- **Ручные правки:** `doc_lazy_continuation` (×3 — `//` между doc-блоками сливал
  список и прозу → разделены пустым `///`); `field_reassign_with_default`
  (struct-update + сжат doc-хелпер, чтобы `c_header.rs` не вырос сверх реестра);
  `assertions_on_constants` (`assert!(false, …)` → `panic!`);
  `needless_range_loop` (→ `enumerate`); `get(k).is_none()` → `!contains_key(k)`.
- **rustc:** `unused_imports` (clippy --fix); `missing_docs`
  (`lsp::collect_diagnostics` — добавлен doc); `dead_code` (`Item.end` в
  `format/comments.rs` — мёртвое поле удалено).

## Проверки

- `cargo clippy --all-targets --all-features -- -D warnings` — EXIT=0.
- `git diff examples/generated` — пусто. `precheck.sh` — тесты зелёные.
- ⚠️ Ловушка: `grep -c warning` по кэшированному clippy дал ложный **0** —
  истинный остаток виден лишь на свежей компиляции / под `-D warnings`.
