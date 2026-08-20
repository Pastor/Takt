# Тест-план 0314: предупреждение о выброшенном вызове

> Фича: [../features/0314-c-dropped-builtin-warning.md](../features/0314-c-dropped-builtin-warning.md) · ADR: [../adr/0314-c-dropped-builtin-warning.md](../adr/0314-c-dropped-builtin-warning.md) · отчёт: [../reports/0314-c-dropped-builtin-warning.md](../reports/0314-c-dropped-builtin-warning.md)

## Условия

| # | Условие | Проверка |
|---|---|---|
| П1 | `CC-024` с именем функции | `dropped_debug_call_is_reported` |
| П2 | Вывод не изменился — вызова в нём нет | тот же тест |
| П3 | Предупреждение на **каждый** вызов | `every_dropped_call_is_reported` |
| П4 | **Контроль:** чистая модель молчит | `model_without_builtin_calls_is_silent` |
| П5 | `--quiet` глушит | прогон CLI |
| П6 | Реестр и приложение | `check-book-diagnostics.py` |
| П7 | Регрессия и предкоммит | `cargo test --all-features`, `./scripts/precheck.sh` |

## Почему тесты именно такие

- **Счёт (П3) обязателен:** одно предупреждение на модель означало бы, что
  второй выброшенный вызов теряется молча — ровно тот класс, который фича
  лечит (образец `ST-022`, фича 0235).
- **П2 сторожит поведение:** предупреждение не должно превратиться в печать
  вызова — решение 0189 не меняется.
- **Контроль (П4)** отделяет «появилось где надо» от «появляется всегда».
