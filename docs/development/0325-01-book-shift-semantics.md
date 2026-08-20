# Разработка 0325-01: подраздел «Сдвиги»

> Фича: [../features/0325-book-shift-semantics.md](../features/0325-book-shift-semantics.md) · ADR: [../adr/0325-book-shift-semantics.md](../adr/0325-book-shift-semantics.md)

## Состав правки

| Файл | Что сделано |
|---|---|
| `book/src/05-expressions/index.typ` | подраздел «Сдвиги»: таблица по знаку типа, пример, врезка «сдвиг вправо — не деление» |

## Проверено

- `make -C book build` — документ собран.
- `scripts/check-book-glyphs.py` — символов вне шрифта нет.
- `scripts/check-book-code-langs.py` — подсветка объявлена.
- Прогон эталона на примере раздела: `-7 >> 1 = -4`, `-7 / 2 = -3`.
