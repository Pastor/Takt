# Тест-план фичи 0283: Печать результата компиляции

> Фича: [../features/0283-cli-report-result-merge.md](../features/0283-cli-report-result-merge.md) · анализ: [../analyze/0283-cli-report-result-merge.md](../analyze/0283-cli-report-result-merge.md) · отчёт: [../reports/0283-cli-report-result-merge.md](../reports/0283-cli-report-result-merge.md)

## Условия проверок

| # | Условие | Как проверяется | Ожидаемый результат |
|---|---|---|---|
| П1 | `--verbose` у цели `st` | `verbose_prints_canonical_input_for_simple_target` | абсолютный путь входа |
| П2 | `--verbose` у цели `st-at` | `verbose_prints_canonical_input_for_address_target` | абсолютный путь входа |
| П3 | **Контрпример:** без флага | `without_verbose_input_path_is_as_given` | путь как передан, строка результата есть |
| П4 | `--quiet` у обеих ветвей | `quiet_suppresses_result_for_both_kinds` | строки результата нет |
| П5 | Форма пути выхода | `output_path_is_printed_with_slash_in_both_modes` | `…/out/ (` в обоих режимах |
| П6 | Границы 0228 целы | `cli_warning_position_tests::` | зелёные |
| П7 | Регрессия | `cargo test --all-features` | провалов нет |
| П8 | Предкоммит | `./scripts/precheck.sh` | код 0 |

## Мутационные проверки

- **М1.** Вернуть игнорирование `--verbose` для адресных целей → П2 падает.
- **М2.** Печатать канонический путь всегда → П3 падает (для того контрпример и
  заведён).
