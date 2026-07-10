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
