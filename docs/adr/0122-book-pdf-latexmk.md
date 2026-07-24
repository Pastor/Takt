# ADR 0122: Сборка PDF через latexmk (корректные кросс-ссылки) + Makefile

- **Status:** Accepted
- **Date:** 2026-07-24
- **Authors:** Архитектор, лид документации
- **Related issues:** [Фича 0122](../features/0122-book-pdf-latexmk.md); инфраструктура сборки `book/` (ADR 0101)

## Context

С появлением кросс-главных ссылок (раздел → приложение «Ошибки», правило 26) и
оглавления в PDF нужны **перекрёстные ссылки**, которые LaTeX разрешает только за
**несколько проходов** движка. Одиночный вызов `xelatex` оставляет «??»/устаревшие
номера. Профиль `pdf` в `book.toml` был настроен на `pdf-engine = "xelatex"` —
сходимость ссылок не гарантировалась при `make build`.

## Decision Drivers

1. **Корректные кросс-ссылки** из штатного `make build`, без ручных повторных
   проходов.
2. **Минимум изменений**; не заводить самодельный цикл xelatex в Makefile.
3. **Находимость инструментов** (TeX не всегда в PATH у `make`).

## Considered Options

- **(A) `pdf-engine = "latexmk"` + `-xelatex`.** latexmk перезапускает xelatex до
  сходимости меток. Один флаг в профиле; `mdbook build` = корректный PDF.
  **Выбрано.**
- **(B) Профиль `to = "latex"` + отдельный таргет Makefile с 3× xelatex.**
  Дублирует конвейер, хрупко (число проходов «на глаз»).
- **(C) Оставить xelatex, полагаться на внутренний ре-ран pandoc.** Не
  гарантирует сходимость для кросс-глав.

## Decision

`book.toml` профиль `pdf`: `pdf-engine = "latexmk"`,
`pdf-engine-opts = ["-xelatex", "-interaction=nonstopmode"]`. `Makefile`: в `deps`
добавлен `latexmk`; в PATH добавлены типичные каталоги TeX
(`/Library/TeX/texbin`, `…/texlive/2026/…`) — `make build` находит движок даже
когда TeX не в PATH. Шапка Makefile описывает механизм.

## Consequences

### Положительные

- `make build` даёт PDF с **разрешёнными** кросс-ссылками (проверено:
  `pdftotext | grep '??'` = 0).
- Не нужен ручной многопроходный прогон.

### Отрицательные / Action items

- Добавлена зависимость **latexmk** (входит в TeX Live/MacTeX — рядом с xelatex).
- Пути TeX в PATH — под macOS/MacTeX; на других ОС полагаемся на системный PATH.

### Acceptance criteria

1. `book.toml`: движок latexmk (`-xelatex`).
2. `Makefile`: `deps` требует latexmk; PATH включает каталоги TeX; шапка описывает
   сходимость ссылок.
3. `make build` собирает PDF; `pdftotext` не содержит «??»; сборка чистая.
