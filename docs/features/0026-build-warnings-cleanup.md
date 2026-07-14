# Фича 0026: Устранение всех предупреждений сборки (rustc + clippy)

- **Номер:** 0026
- **Статус:** СОЗДАНА
- **Зависит от:** <уточняет аналитик на стадии анализа, правило 17 — предварительно
  `нет`; пересекается с кандидатом «`validate.rs` — 3648 строк» только косвенно>
- **Связанные issue (анализ):** новая фича (из бэклога кандидатов `FEATURES.md`;
  выявлено при обзоре 2026-07-15)
- **Крейт:** `grammar` и `simulation` (плюс тестовые цели обоих)

## Ссылки на артефакты (жизненный цикл, правило 17)

| Стадия | Артефакт |
|---|---|
| Архитектура (ADR) | [`docs/adr/0026-build-warnings-cleanup.md`](../adr/0026-build-warnings-cleanup.md) |
| Анализ | [`docs/analyze/0026-build-warnings-cleanup.md`](../analyze/0026-build-warnings-cleanup.md) |
| Разработка | [`docs/development/`](../development/README.md) (задачи `0026-YY-*`) |
| Тест-план | [`docs/tests/0026-build-warnings-cleanup.md`](../tests/README.md) |
| Отчёт о тестировании | [`docs/reports/0026-build-warnings-cleanup.md`](../reports/README.md) |
| Исправления | [`docs/fixes/`](../fixes/README.md) (при необходимости `0026-YY-*`) |

## Краткое описание

Привести сборку обоих крейтов к нулю предупреждений. `cargo build --all-features
--all-targets` выдаёт **19** предупреждений rustc, `cargo clippy --all-features
--all-targets` — **~106** (суммарно ~125). Это фоновый шум, скрывающий новые,
осмысленные предупреждения, и накопленный технический долг стиля/видимости. Цель —
вычистить его и закрепить результат в CI (`-D warnings`), чтобы долг не
накапливался снова.

> Фича зарегистрирована из бэклога кандидатов `FEATURES.md`; далее проходит
> жизненный цикл по правилу 17: архитектура (ADR) → анализ → разработка.

## Мотивация / контекст

Предупреждения органичны и большинство давно висят, но их масса (~125) означает,
что **новое** предупреждение — например, признак свежего дефекта — тонет в шуме и
не замечается. CI сегодня (`cargo build`/`check`/`test`) предупреждения **не**
проваливает, поэтому долг растёт бесконтрольно. `precheck.sh` гоняет clippy, но
без `-D warnings`.

## Инвентарь предупреждений (снимок 2026-07-15, до починки)

### rustc — 19

| Линт | Кол-во | Места |
|---|---|---|
| `private_interfaces` | 11 | `simulation/src/unit/mod.rs` (`Context`/`Value`/`Predicate`/`Flow` — `pub(crate)` при публичных полях `Unit`) |
| `unused_imports` | 2 | `simulation/src/unit/builder.rs:9`; `grammar/tests/lsp_tests.rs:12-13` |
| `missing_docs` | 1 | `grammar/src/lsp.rs:157` |
| `dead_code` | 1 | `grammar/src/format/comments.rs:27` |

### clippy — ~106 (крупнейшие группы)

| Линт | Кол-во | Основные места |
|---|---|---|
| `needless_ref` | 23 | `grammar/src/generator/c/c_expr.rs` |
| `collapsible_if` | 18 | `grammar/src/lsp.rs` |
| `clone_on_copy` | 16 | `grammar/tests/lexer_tests.rs` (`Location` — `Copy`) |
| `private_interfaces` | 8 | `simulation/src/unit/mod.rs` |
| `needless_borrow` | 6 | `grammar/src/lsp.rs` |
| `map_or_else` | 5 | `lexer_tests.rs`, `unit/mod.rs`, `unit/viewport.rs` |
| `assertions_on_constants` | 3 | `grammar/tests/parser_tests.rs` |
| прочее (единичное) | ~27 | `derivable_impl`, `redundant_guard` (`eval/ops.rs`), `map_values`, `missing_docs_in_private_items` и др. |

## Объём (предварительно, уточняет аналитик)

1. **rustc-долг** — 19 предупреждений; частный, но заметный кусок —
   `private_interfaces` в `simulation/src/unit/mod.rs`: `Context`, `Value`,
   `Predicate`, `Flow` остаются `pub(crate)` при публичных полях `Unit`; чинится
   согласованием видимости. `TickResult` уже исправлен попутно задачей 0025-05.
2. **clippy-долг** — ~106 предупреждений, в основном механические автоправки
   (`needless_ref`/`collapsible_if`/`clone_on_copy`); часть в тестовых целях.
3. **CI-закрепление** — рассмотреть `-D warnings` (или `RUSTFLAGS`/`clippy
   --deny warnings`) в `.github/workflows/ci.yml` и/или `precheck.sh`, чтобы
   ноль-долг не разъехался снова.

## Осторожно (не сломать)

- В коде **~38 директив `#[allow(...)]` в 20 файлах** — это осознанные
  компромиссы (`large_enum_variant`, `too_many_arguments`, `dead_code`,
  `type_complexity`). **Не снимать** без разбора: часть удерживает инварианты.
- `deny(clippy::wildcard_enum_match_arm)` в `simulation/src/eval/` —
  **инвариант из [ADR 0025](../adr/0025-simulator-expression-eval.md)**, не
  трогать (исключение — `coerce_to_type` для `#[non_exhaustive] TypeNode`).
- Правки в `grammar/src/generator/c/` могут менять **вывод C**; правки, задевающие
  фикстуры `tests/data/`-снапшотов, — сверять codegen побайтно (`git diff`
  порождённого кода).

## Обратная совместимость (правило 11)

Язык и его семантика **не меняются** — чистка внутренняя. Версия языка не растёт
(правило 22). Видимая часть — возможный бамп версий крейтов при расширении
публичного API (`private_interfaces` → `pub`), решает аналитик.

<!-- При ЗАКРЫТИИ фичи (стадия 8, статус ГОТОВО) сюда добавляется раздел
     «## Итог (что сделано)» —
     ссылки на отчёт/фиксы (правило 21). Незакрытые фичи «Итога» не имеют. -->
