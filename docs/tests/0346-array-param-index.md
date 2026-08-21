# Тест-план 0346: индексация параметра-массива

> Фича: [../features/0346-array-param-index.md](../features/0346-array-param-index.md) · ADR: [../adr/0346-array-param-index.md](../adr/0346-array-param-index.md) · отчёт: [../reports/0346-array-param-index.md](../reports/0346-array-param-index.md)

## Условия

| # | Условие | Проверка |
|---|---|---|
| П1 | `a[0]` при параметре-массиве строится | `indexed_array_parameter_resolves` |
| П2 | `a[i]` при параметре-индексе строится | `parameter_used_as_index_resolves` |
| П3 | Неизвестное имя — `SE-003` | `unknown_name_is_still_se003` |
| П4 | Индексация не массива — `SE-030` | `non_array_parameter_is_se030` |
| П5 | Значения совпадают с эталоном | `array_parameter_index_matches_simulator_and_generated_c` |
| П6 | Предкоммит | `./scripts/precheck.sh` |

## Почему тесты именно такие

Отказ был **семантическим** — значит расхождения между потребителями не
существовало, и сверка нужна не для сравнения целей, а чтобы доказать, что
исправленное поведение верно (`first` даёт 7, `at_index` — 9).

⚠️ Имя `at` для функции брать нельзя: это ключевое слово языка (адрес порта), и
отказ пришёл бы от парсера — первая редакция теста так и упала.
