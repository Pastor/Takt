# Тест-план фичи 0252: Однопоточность в CI и в замере покрытия

> Фича: [../features/0252-ci-windows-test-threads.md](../features/0252-ci-windows-test-threads.md) · анализ: [../analyze/0252-ci-windows-test-threads.md](../analyze/0252-ci-windows-test-threads.md) · отчёт: [../reports/0252-ci-windows-test-threads.md](../reports/0252-ci-windows-test-threads.md)

## Условия проверок

| # | Условие | Как проверяется | Ожидаемый результат |
|---|---|---|---|
| П1 | Флага нет в CI | `grep` по `ci.yml` | только упоминание в комментарии-истории |
| П2 | Флага нет в замере покрытия | `grep` по `coverage.sh` | то же |
| П3 | Профиль покрытия не изменился | `scripts/coverage.sh --files` до и после | те же проценты по файлам |
| П4 | Тесты параллельно зелёные | `cargo test --all-features` | провалов нет |
| П5 | Предкоммит | `./scripts/precheck.sh` | код 0 |

## Границы

Прогон CI не проверяется: задания Actions не стартуют (фича 0175). Правка
обоснована **правилом** (0190 и 0090), и это сказано прямо в комментарии рядом
с изменением.
