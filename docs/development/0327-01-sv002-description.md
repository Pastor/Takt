# Разработка 0327-01: описание `SV-002`

> Фича: [../features/0327-sv002-description.md](../features/0327-sv002-description.md) · ADR: [../adr/0327-sv002-description.md](../adr/0327-sv002-description.md)

## Состав правки

| Файл | Что сделано |
|---|---|
| `docs/diagnostics/README.md` | описание `SV-002` переформулировано по признаку; названы фичи, менявшие состав |
| `book/src/appendix-errors/index.typ` | та же формулировка в сводной таблице |

## Проверено

- `scripts/check-book-diagnostics.py` — 268 кодов сверены.
- `scripts/check-diagnostic-descriptions.py` — безликих нет.
- `make -C book build` — документ собран.
