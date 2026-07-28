# Задача 0178-01: Исчерпывающий разбор токена и догон подсветки

> Фича: [../features/0178-editor-layer-language-sync.md](../features/0178-editor-layer-language-sync.md) · ADR: [../adr/0178-editor-layer-language-sync.md](../adr/0178-editor-layer-language-sync.md) · анализ: [../analyze/0178-editor-layer-language-sync.md](../analyze/0178-editor-layer-language-sync.md)

## Что было

Разбор `Token` в `takt-lang/src/lsp/semantic_tokens.rs` заканчивался
`_ => continue`. Токен, не перечисленный в ветках, **молча** не подсвечивался, и
сборка при этом оставалась зелёной.

Непокрытыми были 11 значимых токенов: `Invariant`; LTL-операторы `LtlNext`,
`LtlFinally`, `LtlGlobally`, `LtlUntil`, `LtlRelease`; виды формул `TypeLtl`,
`TypeGuard`; операторы `Arrow` (`->`), `FatArrow` (`=>`), `Question` (`?`).
При этом `PeirceArrow` (`-->`) подсвечивался — то есть непоследовательность была
случайной, а не решённой.

Зонд до правки: `cond Ok = 1 = 1;` → 3 токена `TT_KEYWORD`,
`invariant Ok = 1 = 1;` → **2**.

## Что сделано

**Ядро языка не тронуто** (R6): изменён только слой LSP.

- **Подстановочная ветка снята.** Разбор перечисляет все варианты `Token`;
  модуль объявляет `#![deny(clippy::wildcard_enum_match_arm)]`, чтобы `_` нельзя
  было вернуть. ⚠️ Замер показал, что гарантия **сильнее задуманной**: мутация
  (убрать `Token::Invariant`) валит сборку не линтом, а **самим rustc** —
  `error[E0004]: non-exhaustive patterns`. То есть инвариант держится и без
  clippy, на обычной `cargo build`; `deny` защищает от возврата `_`, а не от
  пропуска варианта.
- **Ключевые слова догнаны:** `Invariant`, пять LTL-операторов и два вида формул
  → `TT_KEYWORD`.
- **Операторы догнаны** (A-2 ADR): `Arrow`, `FatArrow`, `Question` →
  `TT_OPERATOR`, единообразно с уже подсвеченным `PeirceArrow`.
- **Пунктуация перечислена явно:** `Sharp`, `Semicolon`, `Comma`, `Colon` и
  шесть видов скобок → `continue`. «Не подсвечиваем» стало решением, а не
  умолчанием: новый знак препинания придётся внести в эту ветку руками.
- **Причина зафиксирована в шапке модуля:** почему разбор исчерпывающ, что было
  до, и что тот же приём применён в `eval/` симулятора и `semantic/usages/walk.rs`.

⚠️ `Token::Pragma` оставлен в ветке ключевых слов **как есть** (A-3 ADR): токен
разобран, но лексером не порождается — `"pragma"` в таблице `KEYWORDS`
отсутствует. Ветка мёртвая; это вопрос к языку, а не к подсветке, и в объём
фичи не берётся.

## Проверки

Три поведенческих сторожа в `lsp/mod.rs`, все — **сравнением с эталоном**, а не
«есть хотя бы один токен категории» (слабая форма зеленела бы и на непокрытом
токене: в источнике всегда есть `model`/`start`):

- `test_semantic_tokens_highlight_invariant_like_cond` — число `TT_KEYWORD` у
  `invariant` равно числу у структурно равного `cond`;
- `test_semantic_tokens_highlight_ltl_operators` — формула `: [LTL] G flag;`
  добавляет ровно **2** ключевых слова к эталону без формулы;
- `test_semantic_tokens_highlight_arrows_as_operators` — `-> u8` добавляет ровно
  **1** оператор.

**Мутационные проверки (обязательны, урок 0056):**

| Мутация | Ожидание | Результат |
|---|---|---|
| убрать `Token::Invariant` из разбора | сборка красная | `error[E0004]: non-exhaustive patterns` ✅ |
| перевести `Invariant` и LTL в ветку `continue` | оба теста красные | `..._invariant_like_cond` FAILED, `..._ltl_operators` FAILED ✅ |
| перевести `Arrow` в ветку `continue` | тест стрелок красный | `..._arrows_as_operators` FAILED ✅ |

Команда: `cargo test --features lsp --lib lsp::` — 54 пройдено, 0 упало.
