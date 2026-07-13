# Задача 0022-02: JFlex-лексер и SyntaxHighlighter Lam

> Фича: [../features/0022-intellij-syntax-highlight.md](../features/0022-intellij-syntax-highlight.md) · ADR: [../adr/0022-intellij-syntax-highlight.md](../adr/0022-intellij-syntax-highlight.md) · анализ: [../analyze/0022-intellij-syntax-highlight.md](../analyze/0022-intellij-syntax-highlight.md)

**Статус:** ЗАПЛАНИРОВАНО (разработка не начата).

## Что было

После 0022-01 есть каркас плагина и распознавание `.lam`, но нет лексера — файлы
не раскрашиваются. Источник истины лексики — `grammar/src/parser/lexer.rs`
(таблица `KEYWORDS`) и операторы фичи 0021 (`:=` присваивание, `=` сравнение,
`<=` реляционный, `==` выведен).

## Что план (объём задачи)

Ядро подсветки (R2, R3):

- **`Lam.flex`** (JFlex) → генерируемый `LamLexer` (`FlexAdapter`). Токены:
  - **ключевые слова** — весь набор `KEYWORDS`: `model state start ref next cond
    var const fn type enum struct import from as if else match for while loop
    break continue return in out inout true false extern template assembly
    formula string _ X F G U R LTL Guard` (перечень зеркалит `parser/lexer.rs`);
  - **операторы** — `:=`, `=`, `<=`, `>=`, `<`, `>`, `+ - * / %`, `&& || !`,
    `|`, `&`, `->`, `..`; последовательность `==` — как **BAD_CHARACTER**/два `=`
    (в 0021 выведена, не легализуем визуально);
  - **литералы** — целые/hex/bin числа, строки `"…"` (с эскейпами), `true/false`;
  - **комментарии** — строчный `//…` и doc `///…`;
  - **идентификаторы**, пунктуация `{ } ( ) [ ] ; : , .`.
- **`LamTokenTypes`** — `IElementType`-константы по категориям.
- **`LamSyntaxHighlighter`** (`SyntaxHighlighterBase`) — маппинг токен →
  `TextAttributesKey` (KEYWORD, OPERATOR, NUMBER, STRING, LINE_COMMENT,
  DOC_COMMENT, IDENTIFIER, PARENTHESES/BRACES/BRACKETS, SEMICOLON/COMMA/DOT,
  BAD_CHARACTER), + `LamSyntaxHighlighterFactory` в `plugin.xml`.
- **Регресс-тест соответствия** (R3): константа набора ключевых слов плагина
  сверяется с эталонным списком из `parser/lexer.rs` (список фиксируется в
  тест-плане; при добавлении слова в язык тест краснеет).

Функциональность по стекам (правило 11): язык/компилятор — **н/п**.

## Проверки (план)

- `LexerTestCase` (IntelliJ test framework): для набора примеров/контрпримеров из
  тест-плана поток токенов совпадает с ожидаемым; `x := 1;`, `a = b`, `x <= y`
  дают ASSIGN/EQ/LE, `x == y` — BAD_CHARACTER.
- Highlighter-тест: диапазоны образцового `.lam` → ожидаемые `TextAttributesKey`.
- Регресс-тест ключевых слов (A3) зелёный.
- Соответствие R2/R3 и критериям приёмки A2/A3/A4 (анализ).
