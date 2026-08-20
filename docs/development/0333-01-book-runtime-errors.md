# Разработка 0333-01: подраздел об ошибках программы

> Фича: [../features/0333-book-runtime-errors.md](../features/0333-book-runtime-errors.md) · ADR: [../adr/0333-book-runtime-errors.md](../adr/0333-book-runtime-errors.md)

## Состав правки

| Файл | Что сделано |
|---|---|
| `book/src/15-simulation/index.typ` | подраздел «Где симулятор строже прошивки»: таблица трёх случаев, врезка о разделении обязанностей, оговорка о компиляции |

## Проверено

- Прогон эталона на всех трёх формах: `SIM-001`, `SIM-003`, `SIM-011`.
- `make -C book build`, `check-book-glyphs.py`, `check-book-code-langs.py`.
