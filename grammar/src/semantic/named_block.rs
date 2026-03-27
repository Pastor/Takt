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
                    "enter" => NamedCodeBlock::Enter { upper: None, body: stmt },
                    "exit" => NamedCodeBlock::Exit { upper: None, body: stmt },
                    "always" => NamedCodeBlock::Always { upper: None, body: stmt },
                    name => NamedCodeBlock::Unknown { upper: None, name: name.to_string(), body: stmt },
                }
            }
            NamedCodeBlock::Enter { upper, body } => {
                NamedCodeBlock::Enter { upper, body: resolve_statement(&body, model.clone())? }
            }
            NamedCodeBlock::Exit { upper, body } => {
                NamedCodeBlock::Exit { upper, body: resolve_statement(&body, model.clone())? }
            }
            NamedCodeBlock::Always { upper, body } => {
                NamedCodeBlock::Always { upper, body: resolve_statement(&body, model.clone())? }
            }
            NamedCodeBlock::Unknown { upper, name, body } => {
                NamedCodeBlock::Unknown { upper, name: name.clone(), body: resolve_statement(&body, model.clone())? }
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
    /// NamedCodeBlock::Unresolved("enter", stmt) → Enter { body: resolved_stmt, .. }
    /// ```
    #[test]
    fn unresolved_enter_resolves_to_enter() {
        let nb = NamedCodeBlock::Unresolved("enter".into(), noop_stmt());
        let result = resolve_named_blocks(vec![nb], empty_model()).unwrap();
        assert!(
            matches!(result[0], NamedCodeBlock::Enter { .. }),
            "ожидался Enter, получен {:?}",
            result[0]
        );
    }

    /// `Unresolved("exit", ...)` → `NamedCodeBlock::Exit`.
    #[test]
    fn unresolved_exit_resolves_to_exit() {
        let nb = NamedCodeBlock::Unresolved("exit".into(), noop_stmt());
        let result = resolve_named_blocks(vec![nb], empty_model()).unwrap();
        assert!(matches!(result[0], NamedCodeBlock::Exit { .. }));
    }

    /// `Unresolved("always", ...)` → `NamedCodeBlock::Always`.
    #[test]
    fn unresolved_always_resolves_to_always() {
        let nb = NamedCodeBlock::Unresolved("always".into(), noop_stmt());
        let result = resolve_named_blocks(vec![nb], empty_model()).unwrap();
        assert!(matches!(result[0], NamedCodeBlock::Always { .. }));
    }

    /// `Unresolved("custom", ...)` → `NamedCodeBlock::Unknown { name: "custom", .. }`.
    ///
    /// Пользовательские именованные блоки сохраняются как `Unknown`.
    #[test]
    fn unresolved_custom_resolves_to_unknown() {
        let nb = NamedCodeBlock::Unresolved("custom".into(), noop_stmt());
        let result = resolve_named_blocks(vec![nb], empty_model()).unwrap();
        assert!(
            matches!(&result[0], NamedCodeBlock::Unknown { name, .. } if name == "custom"),
            "ожидался Unknown {{ custom }}, получен {:?}",
            result[0]
        );
    }

    // ── Уже разрешённые блоки ─────────────────────────────────────────────────

    /// Уже разрешённый `Enter { body: Unresolved(..) }` ещё раз разрешается.
    #[test]
    fn already_enter_re_resolves() {
        let stmt = Statement::Unresolved(noop_stmt());
        let nb = NamedCodeBlock::Enter { upper: None, body: stmt };
        let result = resolve_named_blocks(vec![nb], empty_model()).unwrap();
        assert!(matches!(result[0], NamedCodeBlock::Enter { .. }));
    }

    /// Уже разрешённый `Exit` ещё раз разрешается.
    #[test]
    fn already_exit_re_resolves() {
        let nb = NamedCodeBlock::Exit { upper: None, body: Statement::Unresolved(noop_stmt()) };
        let result = resolve_named_blocks(vec![nb], empty_model()).unwrap();
        assert!(matches!(result[0], NamedCodeBlock::Exit { .. }));
    }

    /// Уже разрешённый `Always` ещё раз разрешается.
    #[test]
    fn already_always_re_resolves() {
        let nb = NamedCodeBlock::Always { upper: None, body: Statement::Unresolved(noop_stmt()) };
        let result = resolve_named_blocks(vec![nb], empty_model()).unwrap();
        assert!(matches!(result[0], NamedCodeBlock::Always { .. }));
    }

    /// Уже разрешённый `Unknown` ещё раз разрешается, имя сохраняется.
    #[test]
    fn already_unknown_re_resolves() {
        let nb = NamedCodeBlock::Unknown {
            upper: None,
            name: "tick".into(),
            body: Statement::Unresolved(noop_stmt()),
        };
        let result = resolve_named_blocks(vec![nb], empty_model()).unwrap();
        assert!(
            matches!(&result[0], NamedCodeBlock::Unknown { name, .. } if name == "tick"),
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
        assert!(matches!(result[0], NamedCodeBlock::Enter { .. }));
        assert!(matches!(result[1], NamedCodeBlock::Exit { .. }));
        assert!(matches!(result[2], NamedCodeBlock::Always { .. }));
    }
}
