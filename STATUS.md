# STATUS — выполненные задачи TASKS.md

| Задача | Описание | Изменённые файлы | Тесты |
|--------|----------|-----------------|-------|
| 1 | `cond` требует `;` | `grammar.lalrpop` | ✅ |
| 3 | Индекс массива — любое выражение (`arr[i]`) | `ast.rs`, `grammar.lalrpop`, `semantic/mod.rs`, `condition.rs`, `expression.rs`, `validate.rs`, `c_expr.rs` | ✅ |
| 4+5 | Встроенные типы u8/i8…u64/i64 → `TypeNode::Integer` | `type_node.rs`, `function.rs`, `c/mod.rs` | ✅ |
| 6 | `struct` C-генерация (`typedef struct`) | `c_header.rs`, `c_map.rs` | ✅ |
| 8 | `while` синтаксис (синоним `loop`) | `lexer.rs`, `grammar.lalrpop` | ✅ |
| 12 | `inout` порт (двунаправленный) | `ast.rs`, `lexer.rs`, `grammar.lalrpop`, `c/mod.rs`, `c_header.rs` | ✅ |
| 13 | `from` — зарезервированное слово | `lexer.rs`, `grammar.lalrpop` | ✅ |
| 14 | Предупреждение при неизвестном имени именованного блока (SE-045) | `lib.rs` | ✅ |
| 16 | Предупреждение `StraySemicolon` (SE-044) | `lib.rs` | ✅ |
| 17 | Диагностика недостижимых состояний (SE-046) | `validate.rs`, `lib.rs` | ✅ |
| 10 | `match`/`switch` — полная реализация (лексер, грамматика, AST, семантика, C-генератор) | `lexer.rs`, `grammar.lalrpop`, `ast.rs`, `semantic/mod.rs`, `statement.rs`, `c_expr.rs` | ✅ |
| 18 | SE-047: анализ константных условий переходов | `validate.rs`, `lib.rs` | ✅ |

## Пропущенные задачи (высокий риск)

- 11: Адрес порта отдельно от объявления — архитектурное изменение, поломка существующих `.lam` файлов
- 15: Унификация Condition/Expression — рискованный рефакторинг грамматики, возможны LR-конфликты
