# Тест-план 0331: именованное условие в теле

> Фича: [../features/0331-named-condition-in-body.md](../features/0331-named-condition-in-body.md) · ADR: [../adr/0331-named-condition-in-body.md](../adr/0331-named-condition-in-body.md) · отчёт: [../reports/0331-named-condition-in-body.md](../reports/0331-named-condition-in-body.md)

## Условия

| # | Условие | Проверка |
|---|---|---|
| П1 | Значения совпадают у эталона и RTL | `named_condition_matches_generated_sv` |
| П2 | Цель `c` подставляет условие, макроса в выводе нет | `generated_c_inlines_named_condition` |
| П3 | Регрессия и предкоммит | `cargo test --all-features`, `./scripts/precheck.sh` |

## Почему тесты именно такие

- **Условие меняется по тактам:** постоянное не показало бы подмены условия его
  отрицанием.
- **П2 проверяет текст:** неопределённый идентификатор — свойство **вывода**, а
  не значения; сверка его не увидит, потому что такой файл вообще не собрать.
- ⚠️ Проверка ищет `if (COND`, а не подстроку `COND_`: имя модели фикстуры само
  содержит `COND`, и наивная проверка падала на верном выводе.
