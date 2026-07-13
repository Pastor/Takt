# Реестр разработки

Стадия 4 жизненного цикла (правило 17 в [../RULE.md](../RULE.md)). Подзадачи
разработки `XXXX-YY-slug.md` (нумерация `YY` внутри фичи). Крупные фичи
декомпозируются на несколько подзадач.

Заготовка создаётся из шаблона [`../templates/development.md`](../templates/development.md).

| Задача | Фича | Заголовок | Документ |
|--------|------|-----------|----------|
| 0018-01 | 0018 | Срезовые типы, `#[non_exhaustive]`, чистка мёртвого кода | [0018-01-slice-types-and-api.md](0018-01-slice-types-and-api.md) |
| 0018-02 | 0018 | Опции генератора (`GenerateOptions`); Builder — не требуется | [0018-02-generate-options.md](0018-02-generate-options.md) |
| 0018-03 | 0018 | `with_capacity` в отступах, `#[non_exhaustive]` на AST/IR, аудит конструкторов | [0018-03-with-capacity-nonexhaustive-ast.md](0018-03-with-capacity-nonexhaustive-ast.md) |
| 0018-04 | 0018 | Компилируемые doctests (P12); возврат владения при Err (P09) | [0018-04-doctests-and-ownership.md](0018-04-doctests-and-ownership.md) |
| 0018-05 | 0018 | Аудит `.clone()` (P08); `mem::take` (P10) | [0018-05-clone-audit.md](0018-05-clone-audit.md) |
| 0019-01 | 0019 | Устранение дубликата LoopCond (ЗАПЛАНИРОВАНО) | [0019-01-loopcond-dedup.md](0019-01-loopcond-dedup.md) |
| 0020-01 | 0020 | Грамматика и AST оператора `address` (ЗАПЛАНИРОВАНО) | [0020-01-address-grammar.md](0020-01-address-grammar.md) |
| 0021-01 | 0021 | Лексер + грамматика: `:=` присваивание, `=` равенство (ЗАПЛАНИРОВАНО) | [0021-01-lexer-grammar.md](0021-01-lexer-grammar.md) |
| 0021-02 | 0021 | Семантика/C-генератор/LSP под новую семантику (ЗАПЛАНИРОВАНО) | [0021-02-semantics-codegen-lsp.md](0021-02-semantics-codegen-lsp.md) |
| 0021-03 | 0021 | Мигратор `.lam` + миграция фикстур/примеров (ЗАПЛАНИРОВАНО) | [0021-03-migrator.md](0021-03-migrator.md) |
| 0021-04 | 0021 | Документация и повышение версии языка (ЗАПЛАНИРОВАНО) | [0021-04-docs-version.md](0021-04-docs-version.md) |
| 0022-01 | 0022 | Каркас плагина: Gradle, FileType, регистрация `.lam` (ЗАПЛАНИРОВАНО) | [0022-01-plugin-skeleton.md](0022-01-plugin-skeleton.md) |
| 0022-02 | 0022 | JFlex-лексер + SyntaxHighlighter (ЗАПЛАНИРОВАНО) | [0022-02-lexer-highlighter.md](0022-02-lexer-highlighter.md) |
| 0022-03 | 0022 | ColorSettingsPage, commenter, brace matcher, доки (ЗАПЛАНИРОВАНО) | [0022-03-color-settings-docs.md](0022-03-color-settings-docs.md) |

