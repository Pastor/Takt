# Тест-план 0325: семантика сдвигов в документе

> Фича: [../features/0325-book-shift-semantics.md](../features/0325-book-shift-semantics.md) · ADR: [../adr/0325-book-shift-semantics.md](../adr/0325-book-shift-semantics.md) · отчёт: [../reports/0325-book-shift-semantics.md](../reports/0325-book-shift-semantics.md)

## Условия

| # | Условие | Проверка |
|---|---|---|
| П1 | Утверждения примера верны | прогон эталона на записи раздела |
| П2 | Документ собирается | `make -C book build` |
| П3 | Символы в шрифте | `scripts/check-book-glyphs.py` |
| П4 | Подсветка блоков объявлена | `scripts/check-book-code-langs.py` |
| П5 | Предкоммит | `./scripts/precheck.sh` |

## Почему тесты именно такие

- **П1 — главное:** документ, утверждающий неверное, хуже отсутствующего.
  Пример прогоняется на эталоне, а не сверяется глазами.
- Прочие условия держат гейты документа, заведённые фичами 0133, 0146, 0269.
