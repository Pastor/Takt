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

| TASKS.md §1 | builder.rs: построение дерева Unit + ModelNodeContext с иерархическим контекстом (Rc&lt;RefCell&lt;ModelNode&gt;&gt;, lazy copy, parent chain) | `simulation/src/unit/builder.rs` | ✅ (118 тестов simulation, 1302 workspace) |
| TASKS.md §3 | GIF: имя модели сверху, удалена символьная легенда (Sn/Pn), полные имена на рёбрах/узлах, цветовая легенда | `viewport.rs`, `runner.rs`, `bin/simulation.rs` | ✅ (1329 тестов) |
| TASKS.md §1 | GIF: highlight-кадр сработавшего перехода (оранжевое ребро + метка); `last_transition` в Unit::Node | `unit/mod.rs`, `viewport.rs`, `runner.rs` | ✅ (1329 тестов) |
| TASKS.md §2 | Сохранение/загрузка состояния: `state_io.rs`, CLI `--save-state`/`--load-state` | `state_io.rs`, `lib.rs`, `bin/simulation.rs` | ✅ (1329 тестов) |

| predicate.rs | Читаемые метки рёбер: `condition_label()` генерирует `button != 0` вместо Debug-строки | `predicate.rs` | ✅ (1329 тестов) |
| TASKS.md §4 | Параметры GIF вынесены в `gif_config::GifConfig` (serde JSON); CLI `--gif-config FILE`; примеры в `examples/gif-configs/` (default, dark, compact, large, monochrome) | `gif_config.rs`, `viewport.rs`, `runner.rs`, `bin/simulation.rs`, `lib.rs`, `examples/gif-configs/*.json` | ✅ (1335 тестов) |

## Пропущенные задачи (высокий риск)

- 11: Адрес порта отдельно от объявления — архитектурное изменение, поломка существующих `.lam` файлов
- 15: Унификация Condition/Expression — рискованный рефакторинг грамматики, возможны LR-конфликты
