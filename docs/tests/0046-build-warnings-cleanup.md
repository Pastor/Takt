# Тест-план — Фича 0046: Устранение всех предупреждений сборки

> Фича: [../features/0046-build-warnings-cleanup.md](../features/0046-build-warnings-cleanup.md) · ADR: [../adr/0046-build-warnings-cleanup.md](../adr/0046-build-warnings-cleanup.md) · анализ: [../analyze/0046-build-warnings-cleanup.md](../analyze/0046-build-warnings-cleanup.md)

## Критерии и проверки

| # | Критерий (R/A) | Проверка | Ожидание |
|---|---|---|---|
| T1 | A1 — 0 rustc | `cargo build --all-targets --all-features 2>&1 \| grep -c "^warning"` (без `src/grammar.rs`) | 0 |
| T2 | A2 — 0 clippy | `cargo clippy --all-targets --all-features -- -D warnings` | EXIT=0 |
| T3 | A3 — codegen не изменился | `git diff examples/generated` после регенерации | пусто |
| T4 | A3 — поведение | `cargo test -- --test-threads=1`; `conformance_{c,rust,st,sv}` | зелёные |
| T5 | A4 — инвариант 0025 цел | `deny(clippy::wildcard_enum_match_arm)` в `eval/mod.rs` на месте; проба `_ =>` в `eval/` валит сборку | сохранён |
| T6 | R4 — закрепление | `grep "\-D warnings" scripts/precheck.sh .github/workflows/ci.yml` | найдено |
| T7 | R4 — защёлка работает | временно вернуть предупреждение → `precheck`/clippy падает | падает |
| T8 | `result_large_err` | `cargo clippy … 2>&1 \| grep -c "result_large_err"` | 0 |
| T9 | `Diagnostic` < 128 | `result_large_err` не срабатывает (косвенно) | подтверждено |
| T10 | Позиции целы | `cargo test -p grammar --all-features` (LSP/диагностики/позиции по смещениям) | зелёные |
| T11 | Осознанные `#[allow]` | новые `#[allow(...)]` имеют обоснование у места | да |
| T12 | precheck | `./scripts/precheck.sh` | EXIT=0 |

## Направление ошибки

Усечение `usize → u32` для смещений/номеров файлов безопасно для `.lam` (влезают
с запасом); ошибка в сторону паники (`as` усечёт, но значения малы) — не тихого
неверного результата. Сторож — `cargo test -p grammar` (позиции по смещениям:
`line_column`, LSP goto, диагностики).
