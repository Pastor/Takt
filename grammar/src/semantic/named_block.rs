//! Разрешение именованных блоков кода языка BuT.
//!
//! Функция [`resolve_named_blocks`] преобразует список [`NamedCodeBlock`],
//! вызывая [`resolve_statement`] для каждого блока с его [`Statement`].
//!
//! Вариант [`NamedCodeBlock::Unresolved`] (имя + сырой АСД-оператор) разрешается
//! в конкретный вариант: `Enter`, `Exit`, `Always` или `Unknown`.

use crate::diagnostics::Diagnostic;
use crate::semantic::statement::resolve_statement;
use crate::semantic::{ModelNode, NamedCodeBlock, Statement};
use std::cell::RefCell;
use std::rc::Rc;

pub fn resolve_named_blocks(
    named_blocks: Vec<NamedCodeBlock>,
    model: Rc<RefCell<ModelNode>>,
) -> Result<Vec<NamedCodeBlock>, Diagnostic> {
    let mut blocks = Vec::with_capacity(named_blocks.len());
    for nb in named_blocks {
        let block = match nb {
            NamedCodeBlock::None => return Err("Statement должен быть определен".into()),
            NamedCodeBlock::Unresolved(name, stmt) => {
                let stmt = resolve_statement(&Statement::Unresolved(stmt), model.clone())?;
                match name.as_str() {
                    "enter" => NamedCodeBlock::Enter(stmt),
                    "exit" => NamedCodeBlock::Exit(stmt),
                    "always" => NamedCodeBlock::Always(stmt),
                    name => NamedCodeBlock::Unknown(name.to_string(), stmt),
                }
            }
            NamedCodeBlock::Enter(stmt) => {
                NamedCodeBlock::Enter(resolve_statement(&stmt, model.clone())?)
            }
            NamedCodeBlock::Exit(stmt) => {
                NamedCodeBlock::Exit(resolve_statement(&stmt, model.clone())?)
            }
            NamedCodeBlock::Always(stmt) => {
                NamedCodeBlock::Always(resolve_statement(&stmt, model.clone())?)
            }
            NamedCodeBlock::Unknown(name, stmt) => {
                NamedCodeBlock::Unknown(name.clone(), resolve_statement(&stmt, model.clone())?)
            }
        };
        blocks.push(block);
    }
    Ok(blocks)
}

// ── Тесты ─────────────────────────────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use super::*;
    use crate::diagnostics::Location;
    use crate::parser::ast;

    fn empty_model() -> Rc<RefCell<ModelNode>> {
        Rc::new(RefCell::new(ModelNode::default()))
    }

    // ── NamedCodeBlock::None → ошибка ─────────────────────────────────────────

    /// `NamedCodeBlock::None` в списке — ошибка (блок не определён).
    ///
    /// # Контрпример
    /// Блок без оператора недопустим и должен производить диагностику.
    #[test]
    fn none_block_returns_error() {
        let result = resolve_named_blocks(vec![NamedCodeBlock::None], empty_model());
        assert!(
            result.is_err(),
            "NamedCodeBlock::None должен давать ошибку"
        );
    }

    // ── NamedCodeBlock::Unresolved → разрешение ───────────────────────────────

    fn noop_stmt() -> ast::Statement {
        ast::Statement::Continue(Location::default())
    }

    /// `Unresolved("enter", ...)` → `NamedCodeBlock::Enter`.
    ///
    /// # Пример
    /// ```text
    /// NamedCodeBlock::Unresolved("enter", stmt) → Enter(resolved_stmt)
    /// ```
    #[test]
    fn unresolved_enter_resolves_to_enter() {
        let nb = NamedCodeBlock::Unresolved("enter".into(), noop_stmt());
        let result = resolve_named_blocks(vec![nb], empty_model()).unwrap();
        assert!(
            matches!(result[0], NamedCodeBlock::Enter(_)),
            "ожидался Enter, получен {:?}",
            result[0]
        );
    }

    /// `Unresolved("exit", ...)` → `NamedCodeBlock::Exit`.
    #[test]
    fn unresolved_exit_resolves_to_exit() {
        let nb = NamedCodeBlock::Unresolved("exit".into(), noop_stmt());
        let result = resolve_named_blocks(vec![nb], empty_model()).unwrap();
        assert!(matches!(result[0], NamedCodeBlock::Exit(_)));
    }

    /// `Unresolved("always", ...)` → `NamedCodeBlock::Always`.
    #[test]
    fn unresolved_always_resolves_to_always() {
        let nb = NamedCodeBlock::Unresolved("always".into(), noop_stmt());
        let result = resolve_named_blocks(vec![nb], empty_model()).unwrap();
        assert!(matches!(result[0], NamedCodeBlock::Always(_)));
    }

    /// `Unresolved("custom", ...)` → `NamedCodeBlock::Unknown("custom", ...)`.
    ///
    /// Пользовательские именованные блоки сохраняются как `Unknown`.
    #[test]
    fn unresolved_custom_resolves_to_unknown() {
        let nb = NamedCodeBlock::Unresolved("custom".into(), noop_stmt());
        let result = resolve_named_blocks(vec![nb], empty_model()).unwrap();
        assert!(
            matches!(&result[0], NamedCodeBlock::Unknown(n, _) if n == "custom"),
            "ожидался Unknown(custom), получен {:?}",
            result[0]
        );
    }

    // ── Уже разрешённые блоки ─────────────────────────────────────────────────

    /// Уже разрешённый `Enter(Unresolved(...))` ещё раз разрешается.
    #[test]
    fn already_enter_re_resolves() {
        let stmt = Statement::Unresolved(noop_stmt());
        let nb = NamedCodeBlock::Enter(stmt);
        let result = resolve_named_blocks(vec![nb], empty_model()).unwrap();
        assert!(matches!(result[0], NamedCodeBlock::Enter(_)));
    }

    /// Уже разрешённый `Exit` ещё раз разрешается.
    #[test]
    fn already_exit_re_resolves() {
        let nb = NamedCodeBlock::Exit(Statement::Unresolved(noop_stmt()));
        let result = resolve_named_blocks(vec![nb], empty_model()).unwrap();
        assert!(matches!(result[0], NamedCodeBlock::Exit(_)));
    }

    /// Уже разрешённый `Always` ещё раз разрешается.
    #[test]
    fn already_always_re_resolves() {
        let nb = NamedCodeBlock::Always(Statement::Unresolved(noop_stmt()));
        let result = resolve_named_blocks(vec![nb], empty_model()).unwrap();
        assert!(matches!(result[0], NamedCodeBlock::Always(_)));
    }

    /// Уже разрешённый `Unknown` ещё раз разрешается, имя сохраняется.
    #[test]
    fn already_unknown_re_resolves() {
        let nb = NamedCodeBlock::Unknown("tick".into(), Statement::Unresolved(noop_stmt()));
        let result = resolve_named_blocks(vec![nb], empty_model()).unwrap();
        assert!(
            matches!(&result[0], NamedCodeBlock::Unknown(n, _) if n == "tick"),
            "ожидался Unknown(tick)"
        );
    }

    /// Несколько блоков разрешаются последовательно.
    #[test]
    fn multiple_blocks_all_resolve() {
        let blocks = vec![
            NamedCodeBlock::Unresolved("enter".into(), noop_stmt()),
            NamedCodeBlock::Unresolved("exit".into(), noop_stmt()),
            NamedCodeBlock::Unresolved("always".into(), noop_stmt()),
        ];
        let result = resolve_named_blocks(blocks, empty_model()).unwrap();
        assert_eq!(result.len(), 3);
        assert!(matches!(result[0], NamedCodeBlock::Enter(_)));
        assert!(matches!(result[1], NamedCodeBlock::Exit(_)));
        assert!(matches!(result[2], NamedCodeBlock::Always(_)));
    }
}
