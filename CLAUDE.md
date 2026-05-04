# CLAUDE.md

This file provides guidance to Claude Code (claude.ai/code) when working with code in this repository.

## Project

**Lam** (Language of Automata Models) — DSL for finite state machine (FSM) specification. Compiles to C. Single Rust
crate `grammar` in a workspace.

## Commands

```sh
# Build
cargo build --bin lamc
cargo build --features lsp --bin lam-lsp

# Test (run single-threaded to avoid race conditions)
cargo test -- --test-threads=1
cargo test --features lsp -- --test-threads=1   # includes LSP tests

# Run a single test by name
cargo test test_name -- --test-threads=1

# Pre-commit check (fmt + check + clippy + test + build examples)
./precheck.sh
```

## Architecture

Compilation pipeline: `.lam` source → **Lexer** (`parser/lexer.rs`) → **Parser** (LALRPOP LR(1), `grammar.lalrpop`) →
AST (`parser/ast.rs`) → **Semantic analysis** (7 passes, `semantic/`) → **C generator** (`generator/c/`).

### Semantic passes (in order)

| Pass | File                | Purpose                                                  |
|------|---------------------|----------------------------------------------------------|
| 0    | `tree.rs`           | Extract model/state names, load imports                  |
| 1    | `tree.rs`           | Resolve composite states (`M1 + M2`, `M1 \| M2`)         |
| 2    | `type_inference.rs` | Resolve variables, type inference, ports                 |
| 3    | `tree.rs`           | Resolve named conditions (`cond`)                        |
| 4    | `tree.rs`           | Resolve named blocks (`enter`/`exit`/`always`)           |
| 5    | `tree.rs`           | Resolve function bodies                                  |
| 6    | `tree.rs`           | Replace `Condition::Unresolved` with resolved conditions |

### Critical invariant: `ref` conditions on edges are NOT resolved

Conditions `ref Next: expr;` are stored as `Condition::Unresolved(ast::Condition)` and are intentionally not resolved by
the semantic pipeline. Do NOT add `reference.rs`/`resolve_state_references` — it breaks `S(Ping) = End`. The
`syntax_simple` test covers this.

### Operator semantics: `=` vs `==`

- `=` in **expressions** → `ast::Expression::Assign`
- `==` in **expressions** → `ast::Expression::Equal`
- `=` in **conditions** (`cond`, `ref`) → `ast::Condition::Equal`

### Key files

- `grammar/src/lib.rs` — public API: `parse`, `compile_to_c`, `unused_variable_warnings`,
  `nondeterministic_transition_warnings`
- `grammar/src/semantic/mod.rs` — `ModelNode`, `StateNode`, `TypeNode`
- `grammar/src/semantic/tree.rs` — `construct_model`, semantic passes
- `grammar/src/semantic/validate.rs` — `validate_model`, determinism checks
- `grammar/src/lsp.rs` — `position_to_offset`, `node_at_position`, `hover_info`
- `grammar/src/generator/c/mod.rs` — C code generator
- `grammar/src/bin/lamc.rs` — CLI: `lamc compile --target c input.lam -o output/`
- `grammar/tests/semantic_tests.rs` — semantic integration tests
- `grammar/tests/lsp_tests.rs` — LSP integration tests

## Development rules

- **Language**: Russian in comments, commit messages, docstrings, and changelog.
- **TDD**: write a failing test first, then minimal code to pass, then refactor.
- **Patch size**: group changes logically, max 300 lines per patch; create `changes/Changes-XX[-PYY].patch`.
- **Changelog**: prepend to `CHANGES.md` using [Keep a Changelog](https://keepachangelog.com/en/1.0.0/) format.
- Run `./precheck.sh` before committing.
- Use meaningful commit messages that describe the changes made.
- Use consistent naming conventions and follow established coding standards.
- Document your code with comments and docstrings to improve readability and maintainability.

## Testing conventions

- Use `tempfile` for temporary files, `rstest` for parameterized tests.
- Test naming: `test_<object>_<scenario>_<expected>`.
- Valid `.lam` fixtures: `grammar/tests/data/semantic/valid/`
- Invalid `.lam` fixtures (expected diagnostics): `grammar/tests/data/semantic/invalid/`
- LSP fixtures: `grammar/tests/data/lsp/`

## Adding a new language construct

1. Extend `grammar/src/grammar.lalrpop` and `lexer.rs`.
2. Add AST node to `parser/ast.rs`.
3. Implement semantic pass in `semantic/tree.rs`.
4. Extend C generator in `generator/c/` if needed.
5. Add test fixtures to `tests/data/semantic/valid/` and/or `invalid/`.

## Добавление

- Читай TASKS.md первым, перечисли все задачи, выполняй последовательно. После каждой задачи запускай cargo test перед
  переходом к следующей.
- Всегда запускай cargo test после изменения Rust-файлов. При написании новых тестов верифицируй ожидаемые значения по
  реальному выводу — не угадывай.
- При работе с путями используй std::path::Path (не разбивку строк). При генерации кода явно обрабатывай
  анонимные/корневые модели и пустые случаи.
- При изменении логики разрешения переменных проверяй и верхнеуровневый, и вложенный доступ к модели (включая
  tick/init) — исправления в одном пути часто ломают другой.
- После завершения каждой задачи из TASKS.md добавляй однострочную запись в STATUS.md (название задачи, изменённые
  файлы, тесты проходят). В начале сессии сначала читай STATUS.md, чтобы возобновить с последней контрольной точки.
- При написании новых тестов сначала запускай небольшой зонд для захвата реальных выходных значений, затем пиши
  assertions против этих захваченных значений. Не угадывай ожидаемые строки/адреса.
- Настрой CI-воркфлоу `.claude/ci-repair.sh`: (1) запускает `cargo test --no-fail-fast`, захватывает сбои в JSON, (2)
  для каждого сбоя вызывает `claude -p` с фокусированным промптом (только падающий тест + релевантные файлы + ошибка), 
  (3) агент исправляет, перезапускает тест, пишет патч в `.claude/patches/<name>.patch`, (4) применяет патчи, запускает
  полный тест-сьют, создаёт `gh pr create --draft`. Добавь `.claude/state.json` для возобновления прерванных запусков.
  Покажи скрипт и пробный прогон.
- Прочитай TASKS.md и определи все независимые задачи. Для каждой независимой задачи запусти параллельного субагента
  через Task tool: (1) создай git-воркдерево под .worktrees/<task-slug>, (2) реализуй задачу end-to-end, (3) запусти
  `cargo test`, захвати сбои, (4) итерируй пока тесты не пройдут или до 3 попыток, (5) верни unified diff и сводку.
  После всех субагентов проверь патчи, разреши конфликты, влей в main, запусти полный `cargo test`. Отчитайся таблицей
  задача → статус → количество тестов.
