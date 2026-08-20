# Задача 0270-01: Сборка PDF без тегов доступности

> Фича: [../features/0270-book-pdf-size.md](../features/0270-book-pdf-size.md) · ADR: [../adr/0270-book-pdf-size.md](../adr/0270-book-pdf-size.md) · анализ: [../analyze/0270-book-pdf-size.md](../analyze/0270-book-pdf-size.md)

## Что сделано

`book/Makefile`: заведена переменная

```make
TYPST_PDF_FLAGS ?= --no-pdf-tags
```

и вызов `typst compile` берёт её. Переменная, а не литерал: доступный PDF
собирается `make -C book build TYPST_PDF_FLAGS=`, и это записано рядом.

Рядом же — замер, объясняющий флаг: основной вес давало дерево тегов
(14 208 объектов `StructElem`), а не шрифты (1.7 %) и не SVG (6 %).

## Результат

| | До | После |
|---|---|---|
| PDF | 3 076 713 Б | **946 850 Б** |
| объектов в файле | 16 050 | 1 386 |
| `Outlines` | есть | есть |
| объектов `/Link` | есть | 54 |

## Проверка

```sh
make -C book build && ls -la book/book/pdf/takt-language.pdf
```
