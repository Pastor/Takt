# Тест-план 0320: длина агрегата

> Фича: [../features/0320-aggregate-length-check.md](../features/0320-aggregate-length-check.md) · ADR: [../adr/0320-aggregate-length-check.md](../adr/0320-aggregate-length-check.md) · отчёт: [../reports/0320-aggregate-length-check.md](../reports/0320-aggregate-length-check.md)

## Условия

| # | Условие | Проверка |
|---|---|---|
| П1 | Лишний элемент — `SE-123` с обоими числами | `extra_element_is_refused` |
| П2 | Недостача — тот же код | `missing_element_is_refused` |
| П3 | Структура судится тем же правилом | `struct_field_count_is_checked` |
| П4 | **Контроль:** верная длина законна | `matching_lengths_are_accepted` |
| П5 | **Граница:** скаляр не задет | `scalar_initializer_is_untouched` |
| П6 | Реестр и приложение | `check-book-diagnostics.py` |
| П7 | Регрессия и предкоммит | `cargo test --all-features`, `./scripts/precheck.sh` |

## Почему тесты именно такие

- **Оба направления (П1, П2)**: усечение и расширение одинаково не определены,
  и проверка, ловящая только лишнее, оставила бы половину класса.
- **Контроль (П4) обязателен:** без него правило читалось бы как «агрегаты
  запрещены».
- **Текст с обоими числами (П1):** сообщение без них заставляет автора считать
  элементы вручную.
