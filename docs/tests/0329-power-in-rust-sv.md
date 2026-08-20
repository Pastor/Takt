# Тест-план 0329: степень в целях `rust` и `sv`

> Фича: [../features/0329-power-in-rust-sv.md](../features/0329-power-in-rust-sv.md) · ADR: [../adr/0329-power-in-rust-sv.md](../adr/0329-power-in-rust-sv.md) · отчёт: [../reports/0329-power-in-rust-sv.md](../reports/0329-power-in-rust-sv.md)

## Условия

| # | Условие | Проверка |
|---|---|---|
| П1 | `rust` печатает `wrapping_pow` | `rust_and_sv_translate_power` |
| П2 | `sv` не печатает `**` | тот же тест |
| П3 | Оба инструмента SV принимают модуль | ручной прогон `verilator`/`yosys` |
| П4 | Сторож 0308 не использует `**` | `slice_in_statement_points_at_the_statement_for_sv` |
| П5 | Регрессия и предкоммит | `cargo test --all-features`, `./scripts/precheck.sh` |

## Почему тесты именно такие

- **П1 проверяет именно `wrapping_pow`:** обычный `pow` дал бы панику при
  переполнении, то есть другое поведение на том же входе.
- **П4 — следствие фичи:** прежний сторож брал `**` как пример непереводимого;
  оставить его значило бы проверять устаревшее утверждение.
