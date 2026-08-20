# Тест-план 0317: представление `q` в общем слое

> Фича: [../features/0317-fixed-cast-shared-layer.md](../features/0317-fixed-cast-shared-layer.md) · ADR: [../adr/0317-fixed-cast-shared-layer.md](../adr/0317-fixed-cast-shared-layer.md) · отчёт: [../reports/0317-fixed-cast-shared-layer.md](../reports/0317-fixed-cast-shared-layer.md)

## Условия

| # | Условие | Проверка |
|---|---|---|
| П1 | Дробное приведение сворачивается | `fractional_cast_is_folded` |
| П2 | Целое масштабируется сдвигом | `integer_cast_is_folded` |
| П3 | Округление floor к −∞ (на отрицательном) | `rounding_is_floor_towards_minus_infinity` |
| П4 | Перенос и насыщение расходятся | `overflow_wraps_or_saturates_by_format` |
| П5 | Носитель реализует ADR 0061/0170 | `carrier_implements_adr_0061` |
| П6 | **Контроль:** литерал прежним путём, `SE-058` в силе | `plain_literal_is_unchanged` |
| П7 | Мутация floor→усечение ловится | ручной прогон |
| П8 | Регрессия и предкоммит | `cargo test --all-features`, `./scripts/precheck.sh` |

## Почему тесты именно такие

- **Отрицательное значение обязательно** (П3): на положительных floor и
  усечение совпадают, и дефект был бы невидим (урок 0061, T9).
- **Контроль (П6)** отделяет «приведение считается» от «свёртка трогает всё
  подряд»: авторский литерал по-прежнему обязан быть точным.
- **Мутация (П7)** доказывает, что сторож смотрит на правило, а не на число.
