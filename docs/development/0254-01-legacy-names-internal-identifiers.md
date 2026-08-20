# Задача 0254-01: Переименование служебных идентификаторов

> Фича: [../features/0254-legacy-names-internal-identifiers.md](../features/0254-legacy-names-internal-identifiers.md) · ADR: [../adr/0254-legacy-names-internal-identifiers.md](../adr/0254-legacy-names-internal-identifiers.md) · анализ: [../analyze/0254-legacy-names-internal-identifiers.md](../analyze/0254-legacy-names-internal-identifiers.md)

## Что переименовано

| Было | Стало | Где |
|---|---|---|
| `LAMC` | `TAKTC` | `scripts/precheck.sh` |
| `lam_file` | `takt_file` | `precheck.sh`, `run_simulations.sh` |
| `lam_*` (временные каталоги) | `takt_*` | тесты обоих крейтов |
| `BUT_KEYWORDS`, `BUT_BUILTIN_TYPES` | `TAKT_KEYWORDS`, `TAKT_BUILTIN_TYPES` | `takt-lang/src/lsp/` |
| `lam-generated-examples`, `lam_generated` | `takt-generated-examples`, `takt_generated` | `examples/generated/rust/` |
| `LAM_PROBE_OUT` | `TAKT_PROBE_OUT` | тест семантических токенов |
| `LAM_*` (ключи цветов) | `TAKT_*` | плагин IntelliJ: `.kt`, схемы, `plugin.xml` |
| `collect_lam_files`, `test_size_letter_comes_from_lam_type` | `collect_takt_files`, `…_from_takt_type` | `takt-lang/src/` |

Всего 114 вхождений в рабочих файлах.

## Что исправлено попутно

Комментарий `scripts/precheck.sh` адресовал сверку `sv-mmio` как
`simulation/tests/conformance_sv_mmio_tests.rs` — крейта `simulation` нет с
фичи 0100, а каталога `conformance/` там нет с 0244. Указан действующий путь.
Место названо **гейтом**, а не глазами: прежде файл был из проверки исключён
целиком.

## Проверка

```sh
sh scripts/check-legacy-names.sh
cargo test --all-features
cd extensions/intellij-takt && ./gradlew --offline test
```
