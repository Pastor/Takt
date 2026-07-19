# Задача 0046-03: Закрепление ноль-долга (`-D warnings` в precheck + CI)

> Фича: [../features/0046-build-warnings-cleanup.md](../features/0046-build-warnings-cleanup.md) · ADR: [../adr/0046-build-warnings-cleanup.md](../adr/0046-build-warnings-cleanup.md)

## Что было

`precheck.sh` гонял `cargo clippy --all-targets --all-features` **без** `-D
warnings` (информационно); CI — `build`/`check`/`test` без гейта предупреждений.
Долг копился молча (549 к 2026-07-19).

## Что сделано

- **`scripts/precheck.sh`:** шаг clippy → `cargo clippy --all-targets
  --all-features -- -D warnings`. Clippy гоняет и clippy-, и rustc-линты — один
  флаг закрывает оба набора.
- **`.github/workflows/ci.yml`:** новый шаг «Линты (0 предупреждений, фича 0046)»
  — `clippy --all-targets --all-features -- -D warnings` после «Проверка».
- **Почему CLI-уровень, а не `#![deny(warnings)]`:** запрет `docs/CODE.md` —
  `deny(warnings)` в коде ломает сборку **у пользователя** при обновлении
  компилятора. Флаг в скрипте ломает лишь `precheck`/CI — что и требуется:
  предупреждение устраняют, а не копят. Точечные исключения — `#[allow(...)]` с
  обоснованием у места (прецедент — `deny(clippy::wildcard_enum_match_arm)` в
  `eval/mod.rs`, ADR 0025).

## Проверки

- `cargo clippy --all-targets --all-features -- -D warnings` — EXIT=0.
- `./scripts/precheck.sh` — EXIT=0 (шаг clippy проходит).
- Проба: временно вернуть предупреждение → `precheck`/CI падает (защёлка работает).
