# Тест-план 0310: общий носитель правила приведения

> Фича: [../features/0310-int-cast-shared-layer.md](../features/0310-int-cast-shared-layer.md) · ADR: [../adr/0310-int-cast-shared-layer.md](../adr/0310-int-cast-shared-layer.md) · отчёт: [../reports/0310-int-cast-shared-layer.md](../reports/0310-int-cast-shared-layer.md)

## Условия

| # | Условие | Проверка |
|---|---|---|
| П1 | Беззнаковое приведение сворачивается обёрткой | `unsigned_cast_is_folded_with_wrap` |
| П2 | Знаковое переполнение — `SE-121` с текстом | `signed_overflow_is_refused` |
| П3 | **Контроль:** тождественное приведение работает | `identity_cast_still_works` |
| П4 | Носитель реализует ADR 0127 | `carrier_implements_adr_0127` |
| П5 | **Граница:** дробная цель остаётся эталону | `fixed_point_cast_is_still_left_to_the_reference` |
| П6 | Цель `sv` печатает обёрнутое значение | `value_changing_cast_is_folded` |
| П7 | Цель `sv` отвергает знаковое переполнение | `signed_overflow_cast_is_refused` |
| П8 | Реестр и приложение | `check-book-diagnostics.py`, `check-diagnostic-codes.sh` |
| П9 | Регрессия и предкоммит | `cargo test --all-features`, `./scripts/precheck.sh` |

## Почему тесты именно такие

- **Мутация обязательна** (снять маску обёртки): без неё «тест на 44» прошёл бы
  и в мире, где приведение просто не меняет значения.
- **Контроль и граница** (П3, П5) отделяют «правило переехало» от «правило
  переписано»: тождественное приведение и дробная цель обязаны вести себя
  по-прежнему.
- **Проверка через цель `sv`** (П6, П7) — потому что именно она отвергала вход;
  проверяется **значение** в выводе, а не отсутствие отказа.
