# Задача 0283-01: Слияние `report_simple_result` и `report_hal_result`

> Фича: [../features/0283-cli-report-result-merge.md](../features/0283-cli-report-result-merge.md) · ADR: [../adr/0283-cli-report-result-merge.md](../adr/0283-cli-report-result-merge.md) · анализ: [../analyze/0283-cli-report-result-merge.md](../analyze/0283-cli-report-result-merge.md)

## Что сделано

Обе функции заменены на одну — `report_result` в `takt-lang/src/compile_cli/mod.rs`.
Восемь мест вызова (по одному на цель) зовут её.

Различия, которые были между копиями, сняты:

| Что | Было | Стало |
|---|---|---|
| `--verbose` | учитывался только «простой» ветвью | учитывается всеми |
| путь выхода в verbose-режиме | без слэша | со слэшем, как в обычном |

Без изменений: печать ошибки и выход с кодом 1, `print_warnings` с реестром
файлов (границы фич 0228 и 0275 на месте), собственное сообщение цели `c`
(оно перечисляет пути поиска — названное исключение).

## Сторож

`takt-lang/tests/targets/cli_report_result_tests.rs` — пять проверок:
`--verbose` у «простой» и у адресной цели, **контрпример** (без флага путь как
передан), глушение `--quiet` у обеих ветвей, форма пути выхода в обоих режимах.

⚠️ Контрпример обязателен: без него «печатает канонический путь» проходило бы и
у реализации, печатающей его всегда.

## Проверка

```sh
cargo test --test targets cli_report_result_tests::
cargo test --test targets cli_warning_position_tests::   # границы 0228 целы
```
