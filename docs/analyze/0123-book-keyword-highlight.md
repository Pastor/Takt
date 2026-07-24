# Анализ фичи 0123: Подсветка ключевых слов языка в тексте документа

> Фича: [../features/0123-book-keyword-highlight.md](../features/0123-book-keyword-highlight.md) · ADR: [../adr/0123-book-keyword-highlight.md](../adr/0123-book-keyword-highlight.md) · тест-план: [../tests/README.md](../tests/README.md)

## Цель

Ключевые слова языка в прозе документа — моноширинным + жирным + цветом, чтобы
отличать от постороннего слова. Инфраструктура/стиль (не язык, Tier 3).

## Список ключевых слов (источник — лексер)

Взят из `takt-lang/src/parser/lexer.rs` таблица `KEYWORDS`:
`address as assembly break cond const continue else enum extern false fn for
formula from if import in inout invariant loop match model next out ref return
start state string struct template true type var while LTL Guard`.

**Исключены** из подсветки: одиночные `X`/`F`/`G`/`U`/`R` (операторы LTL — высок
риск ложных срабатываний, напр. состояние с именем `F`) и `_` (wildcard). Типы
`bit`/`bool`/`float`/`u8`… — **не** ключевые слова лексера (резолвятся как
псевдонимы/примитивы), поэтому не подсвечиваются.

## Механизм (проверено)

- Профиль mdbook-pandoc = pandoc defaults-файл → принимает `filters = […]`.
- `book/keywords.lua`: функция `Code(el)` — если `el.text` в множестве ключевых
  слов и `FORMAT` = latex, вернуть `RawInline('latex', '\textbf{\textcolor[HTML]
  {204A87}{\texttt{…}}}')`. Цвет — tango KeywordTok (как в блоках кода).
- Блоки кода — `CodeBlock`, фильтром **не** трогаются (уже подсвечены skylighting).

## Проверка

- Фильтр применяется: в сгенерированном `.tex` — **80** вхождений
  `\textcolor[HTML]{204A87}` (образцы: `model`, `state`).
- Разметка валидна: xelatex компилирует её без ошибок (единственные ошибки
  latex-профиля — про `dispenser-filling.svg`, артефакт `to=latex` без rsvg; в
  реальном `pdf`-профиле SVG обрабатывается).
- `make build` (pdf-профиль, latexmk) — PDF собран без ошибок; `pdftotext | grep
  '??'` = 0 (кросс-ссылки целы).

## Ведение

При добавлении ключевого слова в лексер — обновить `keywords.lua` (кандидат:
скрипт-гейт сверки множества `keywords.lua` с `KEYWORDS`).
