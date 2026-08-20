# Тест-план 0330: присваивание агрегата

> Фича: [../features/0330-aggregate-assignment.md](../features/0330-aggregate-assignment.md) · ADR: [../adr/0330-aggregate-assignment.md](../adr/0330-aggregate-assignment.md) · отчёт: [../reports/0330-aggregate-assignment.md](../reports/0330-aggregate-assignment.md)

## Условия

| # | Условие | Проверка |
|---|---|---|
| П1 | Значения совпадают у эталона и RTL | `array_assignment_matches_generated_sv` |
| П2 | `st` печатает поэлементно, `iec2c` принимает | `generated_st_assigns_elements_one_by_one` |
| П3 | Регрессия и предкоммит | `cargo test --all-features`, `./scripts/precheck.sh` |

## Почему тесты именно такие

- **Разные значения по элементам** (`{i, i + 10}`): одинаковые не показали бы
  перепутанного порядка.
- **Значения меняются по тактам:** неизменные не показали бы потери
  присваивания.
- **Прогон `iec2c`, а не текст:** агрегатной формы в IEC нет, и «похоже на
  правду» здесь ничего не значит.
