# Задача 0269-01: Определения подсветки ST и EBNF

> Фича: [../features/0269-book-st-syntax-highlight.md](../features/0269-book-st-syntax-highlight.md) · ADR: [../adr/0269-book-st-syntax-highlight.md](../adr/0269-book-st-syntax-highlight.md) · анализ: [../analyze/0269-book-st-syntax-highlight.md](../analyze/0269-book-st-syntax-highlight.md)

## Что сделано

**`book/st.sublime-syntax`** — Structured Text: объявления POU и секций
(`FUNCTION_BLOCK`, `VAR`/`END_VAR`, `TYPE`/`STRUCT`, `AT`), управляющие
конструкции, словесные операторы (`AND`, `OR`, `XOR`, `NOT`, `MOD`),
элементарные типы, литералы, локации `%MB512`, комментарии `(* … *)`, строки
с экранированием `$`.

⚠️ Правила **нечувствительны к регистру** (`(?i:…)`): идентификаторы IEC
регистронезависимы, цель печатает ключевые слова верхним регистром, а
рукописный пример может быть строчным.

⚠️ **Правило литерала с основанием стоит до общего числового.** В первой
редакции его не было, и `16#FF` выходило как число `16` плюс бесцветное `#FF`
— поймано рендером страницы, а не чтением.

**`book/ebnf.sublime-syntax`** — EBNF в той форме, которой написано приложение
«Грамматика»: терминалы в кавычках (главный носитель смысла), `::=`/`=`, `|`,
`{}`, `[]`, `()`, `;`, нетерминалы, комментарии.

**`book/src/template.typ`** — оба файла подключены **списком**:
`syntaxes: ("/takt.sublime-syntax", "/st.sublime-syntax", "/ebnf.sublime-syntax")`.

## Проверка

Рендером страницы (машине цвет недоступен):

```sh
typst compile --root book --format png --ppi 90 book/src/main.typ out{n}.png
```

Настоящий блок приложения после правки: `VAR`/`END_VAR`/`END_IF` синим,
`IF`/`THEN` жирным, `TRUE`/`FALSE`/`T#180000ms` — цветом констант, комментарий
курсивом.
