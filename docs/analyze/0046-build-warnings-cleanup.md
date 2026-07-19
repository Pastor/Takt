# Анализ фичи 0046: Устранение всех предупреждений сборки (rustc + clippy)

> Фича: [../features/0046-build-warnings-cleanup.md](../features/0046-build-warnings-cleanup.md) · ADR: [../adr/0046-build-warnings-cleanup.md](../adr/0046-build-warnings-cleanup.md) · тест-план: [../tests/README.md](../tests/README.md)

## Цель и контекст

Свести предупреждения сборки обоих крейтов к нулю и закрепить результат, не
сломав осознанные подавления и инвариант `deny(clippy::wildcard_enum_match_arm)`
([ADR 0025](../adr/0025-simulator-expression-eval.md)). Инвентарь предупреждений —
в [карточке фичи](../features/0046-build-warnings-cleanup.md).

## Зависимости фичи (правило 17/19)

> **Обязательный раздел аналитика.** Заполнить на стадии анализа: проверить
> зависимости и проставить «Зависит от» в карточке и в `FEATURES.md`.

- **Зависит от:** **0036** (закрыта 2026-07-19) — она забрала 19
  `private_interfaces` из объёма 0046. Прочих зависимостей нет: 0027 (деление
  модулей) уже закрыта, конфликта по файлам не осталось.
- **Влияние на порядок разработки:** 0046 берётся сразу после 0036 (правило 19).

## ⚠️ Инвентарь пересчитан по факту (2026-07-19)

Снимок карточки (2026-07-15, ~125) **устарел**: свежий прогон дал **19 rustc → 3**
(0036 убрала 11 `private_interfaces`, ещё несколько — попутные фичи) и **~106
clippy → 549**. Взрыв clippy — **один класс**: `clippy::result_large_err` (**414**),
включённый по умолчанию в clippy 0.1.99 (в снимке его не было). Причина —
`Diagnostic` = 136 байт > порога 128 на каждом из 203+ `Result<_, Diagnostic>`.
Разрешение (решение заказчика) — ужать `Location::Source` до `u32×3`
(`Diagnostic` → 120 байт); детали и отвергнутые альтернативы — в
[ADR](../adr/0046-build-warnings-cleanup.md), раздел «Решение по `result_large_err`».

## Декомпозиция (правило 17)

- **0046-01** — ужатие `Location::Source(u64,usize,usize) → (u32,u32,u32)`:
  хелпер-конструктор `Location::source`, касты в аксессорах, ~200 сайтов
  (конструирование через хелпер/sed, разбор — `as usize`). Закрывает 414.
- **0046-02** — механическая чистка остатка: `clippy --fix` (needless_ref,
  collapsible_if, clone_on_copy, …) + ручные (`too_many_arguments` → `#[allow]`,
  `doc_lazy_continuation`, `redundant_guards`, `field_reassign_with_default`,
  `assertions_on_constants`, `needless_range_loop`) + 3 rustc (`unused_imports`,
  `missing_docs`, `dead_code`).
- **0046-03** — закрепление: `-D warnings` на clippy в `precheck.sh` и CI.

## Требования и проверяемые условия

- **R1. Ноль rustc-предупреждений.** `cargo build --all-features --all-targets`.
- **R2. Ноль clippy-предупреждений.** `cargo clippy --all-features --all-targets`.
- **R3. Инварианты целы.** `#[allow(...)]` и `deny(wildcard_enum_match_arm)` не
  сняты без обоснования; вывод C-генератора байт-в-байт неизменен.
- **R4. Долг закреплён.** Преграда в CI/`precheck.sh` (способ — за ADR).

## Критерии приёмки и способ проверки

| # | Критерий | Способ проверки |
|---|---|---|
| A1 | 0 rustc-warnings | `cargo build --all-features --all-targets 2>&1 \| grep -c warning` = 0 |
| A2 | 0 clippy-warnings | `cargo clippy --all-features --all-targets 2>&1 \| grep -c warning` = 0 |
| A3 | Codegen не изменился | сверка снапшотов `examples/generated/`, тесты conformance |
| A4 | Инвариант 0025 цел | негативная проба: `_ =>` в `eval/` валит сборку |

## Особенности по обратной функциональности

Язык не тронут; версия языка не растёт (правило 22). Возможен бамп версий крейтов
при расширении публичного API (`private_interfaces` → `pub`).

## Риски и зависимости

- Массовые автоправки clippy в `generator/c/` рискуют задеть вывод C — сверять.
- Снятие `#[allow]` может обнажить настоящий долг (например, `too_many_arguments`)
  → решать точечно, а не флагом.

<!-- Правило 17: при большом объёме декомпозировать на docs/analyze/0046-YY-slug.md. -->
