# Разработка 0342-01: список зарезервированных имён IEC

> Фича: [../features/0342-st-reserved-names.md](../features/0342-st-reserved-names.md) · ADR: [../adr/0342-st-reserved-names.md](../adr/0342-st-reserved-names.md)

## Состав правки

| Файл | Что сделано |
|---|---|
| `takt-lang/src/generator/st/st_reserved.rs` | список дополнен двадцатью именами; врезка о четырёх, которые `iec2c` принимает |
| `takt-lang/tests/targets/st_reserved_names_tests.rs` | **новый** сторож: прогон `iec2c` по каждой записи плюс контрольное имя |

## Проверено

- Сторож на первом прогоне нашёл четыре ложные записи — они сняты.
- `var ln: u8 := 1;` теперь даёт `ST-014` до порождения файла.
- Вывод корпуса не изменился.
