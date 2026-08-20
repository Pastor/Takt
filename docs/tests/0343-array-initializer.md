# Тест-план 0343: инициализатор массива

> Фича: [../features/0343-array-initializer.md](../features/0343-array-initializer.md) · ADR: [../adr/0343-array-initializer.md](../adr/0343-array-initializer.md) · отчёт: [../reports/0343-array-initializer.md](../reports/0343-array-initializer.md)

## Условия

| # | Условие | Проверка |
|---|---|---|
| П1 | Значения ST совпадают с эталоном | `array_initializer_matches_generated_st` |
| П2 | `c`: элементы по полям, сборка чиста | `c_array_of_structs_compiles` |
| П3 | `rust`: литерал структуры, сборка чиста | `rust_array_of_structs_compiles` |
| П4 | Контроль: массив скаляров не изменился | в тех же тестах |
| П5 | Граница `ST-017` сохранена | `st_still_refuses_aggregate_parameter_argument` |
| П6 | Предкоммит | `./scripts/precheck.sh` |

## Почему тесты именно такие

Потерянный инициализатор **компилируется**: `iec2c` принимает объявление без
`:=`. Вердикт даёт только прогон значений — и он же отделяет «доехало» от
«доехало верно».
