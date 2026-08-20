# Тест-план 0334: сдвиг на величину не меньше ширины типа

> Фича: [../features/0334-rust-variable-shift-width.md](../features/0334-rust-variable-shift-width.md) · ADR: [../adr/0334-rust-variable-shift-width.md](../adr/0334-rust-variable-shift-width.md) · отчёт: [../reports/0334-rust-variable-shift-width.md](../reports/0334-rust-variable-shift-width.md)

## Условия

| # | Условие | Проверка |
|---|---|---|
| П1 | Эталон: `0`, `−1`, `0`, `25` | `shift_by_type_width_matches_simulator_and_generated_rust` |
| П2 | Порождённый Rust собирается и даёт то же | там же (настоящий `rustc`, отладочная сборка) |
| П3 | Контрольный вход (`n < ширины`) не изменился | `ctl = 25` в том же ожидании |
| П4 | Приведение к `u32` печатается по нужде | `shift_amount_cast_is_printed_only_when_needed` |
| П5 | Мутация ловится | снятие ветви валит оба теста |
| П6 | Предкоммит | `./scripts/precheck.sh` |

## Почему тесты именно такие

Сборка доказывает лишь **валидность**: переменная величина собиралась и прежде,
считая другое. Поэтому сверяются числа. Драйвер собирается **без `-O`**:
отладочный режим — тот, где прежний вывод паниковал.
