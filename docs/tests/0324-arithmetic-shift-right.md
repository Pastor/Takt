# Тест-план 0324: арифметический сдвиг вправо

> Фича: [../features/0324-arithmetic-shift-right.md](../features/0324-arithmetic-shift-right.md) · ADR: [../adr/0324-arithmetic-shift-right.md](../adr/0324-arithmetic-shift-right.md) · отчёт: [../reports/0324-arithmetic-shift-right.md](../reports/0324-arithmetic-shift-right.md)

## Условия

| # | Условие | Проверка |
|---|---|---|
| П1 | `-7 >> 1 = -4` совпадает у эталона и RTL | `shifts_match_generated_sv` |
| П2 | Левый сдвиг не изменился (`3 << 2 = 12`) | тот же тест |
| П3 | `sv` печатает `>>>` при знаковом операнде | `signed_shift_uses_arithmetic_operator` |
| П4 | Порождённый ST принимается `iec2c` | гейт цели `st` |
| П5 | Регрессия и предкоммит | `cargo test --all-features`, `./scripts/precheck.sh` |

## Почему тесты именно такие

- **Нечётное значение обязательно:** на чётном floor и усечение к нулю
  совпадают, и подмена сдвига делением осталась бы незамеченной.
- **П3 проверяет текст**, потому что `>>` и `>>>` расходятся значением
  **только на отрицательных**: на другой фикстуре подмена прошла бы мимо.
