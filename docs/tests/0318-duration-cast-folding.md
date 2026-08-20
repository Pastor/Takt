# Тест-план 0318: мост «число ↔ длительность»

> Фича: [../features/0318-duration-cast-folding.md](../features/0318-duration-cast-folding.md) · ADR: [../adr/0318-duration-cast-folding.md](../adr/0318-duration-cast-folding.md) · отчёт: [../reports/0318-duration-cast-folding.md](../reports/0318-duration-cast-folding.md)

## Условия

| # | Условие | Проверка |
|---|---|---|
| П1 | `ЦЕЛОЕ as duration` сворачивается | `integer_to_duration_is_folded` |
| П2 | `duration as ЦЕЛОЕ` сворачивается | `duration_to_integer_is_folded` |
| П3 | **Контроль:** литерал длительности прежним путём | `plain_duration_literal_is_unchanged` |
| П4 | **Граница:** переполнение судит `SE-121` | `overflowing_duration_uses_the_integer_rule` |
| П5 | Регрессия и предкоммит | `cargo test --all-features`, `./scripts/precheck.sh` |

## Почему тесты именно такие

- **Граница (П4)** доказывает, что второго правила переноса не заведено:
  ответ даёт тот же код, что и обычное целочисленное приведение.
- **Контроль (П3)** отделяет «приведение считается» от «свёртка трогает всё».
