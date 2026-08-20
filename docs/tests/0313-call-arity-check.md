# Тест-план 0313: арность вызова

> Фича: [../features/0313-call-arity-check.md](../features/0313-call-arity-check.md) · ADR: [../adr/0313-call-arity-check.md](../adr/0313-call-arity-check.md) · отчёт: [../reports/0313-call-arity-check.md](../reports/0313-call-arity-check.md)

## Условия

| # | Условие | Проверка |
|---|---|---|
| П1 | Локальная функция: недостача → `SE-122` с обоими числами | `local_function_arity_is_checked` |
| П2 | Встроенная: та же проверка | `builtin_function_arity_is_checked` |
| П3 | Лишний аргумент ловится | `extra_argument_is_checked` |
| П4 | **Контроль:** согласованный вызов законен | `matching_calls_are_accepted` |
| П5 | **Граница:** неразрешённое имя судит `SE-004` | `unknown_function_keeps_its_own_diagnostic` |
| П6 | Реестр и приложение | `check-book-diagnostics.py` |
| П7 | Регрессия и предкоммит | `cargo test --all-features`, `./scripts/precheck.sh` |

## Почему тесты именно такие

- **Контроль (П4) обязателен:** без него «недостача отвергается» означало бы
  «отвергается любой вызов».
- **Граница (П5)** сторожит от второго ответа на один вход: `SE-004` уже
  говорит, что функции нет вовсе.
- **Текст проверяется на оба числа** (П1): сообщение без них заставляет автора
  считать параметры самому.
