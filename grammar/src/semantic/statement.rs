//! Разрешение семантических операторов языка BuT.
//!
//! Основная функция [`resolve_statement`] преобразует «сырые» (`Unresolved`)
//! АСД-операторы в полностью разрешённые семантические варианты.
//!
//! ## Алгоритм
//!
//! Для каждого варианта `ast::Statement`:
//! 1. Рекурсивно разрешаются вложенные операторы.
//! 2. Выражения преобразуются через [`construct_expression`].
//! 3. Типы переменных разрешаются через [`construct_type`].
//!
//! Если выражение в операторе не может быть разрешено (например, ссылка
//! на необъявленную встроенную функцию), весь оператор сохраняется в виде
//! [`Statement::Unresolved`] — ошибка не пробрасывается наверх.
//! Это позволяет обрабатывать код с встроенными функциями (`debug`, `S` и т.п.)
//! без предварительной регистрации встроенных символов.

use crate::diagnostics::Diagnostic;
use crate::parser::ast;
use crate::semantic::expression::construct_expression;
use crate::semantic::tree::construct_type;
use crate::semantic::{ModelNode, Statement};
use std::cell::RefCell;
use std::rc::Rc;

/// Разрешает семантический оператор [`Statement`].
///
/// Для `Unresolved` вызывает [`resolve_ast_statement`].
/// Для `Block` рекурсивно разрешает каждый вложенный оператор.
/// Остальные варианты возвращаются без изменений.
///
/// При ошибке разрешения выражения оператор сохраняется как `Unresolved`.
pub fn resolve_statement(
    statement: &Statement,
    model: Rc<RefCell<ModelNode>>,
) -> Result<Statement, Diagnostic> {
    match statement {
        Statement::Unresolved(stmt) => Ok(resolve_ast_statement(stmt, model)
            .unwrap_or_else(|_| statement.clone())),
        Statement::None => Ok(Statement::None),
        Statement::Block(stmts) => {
            let resolved = stmts
                .iter()
                .map(|s| resolve_statement(s, model.clone()))
                .collect::<Result<Vec<_>, _>>()?;
            Ok(Statement::Block(resolved))
        }
        other => Ok(other.clone()),
    }
}

/// Преобразует `ast::Statement` в разрешённый [`Statement`].
///
/// При ошибке разрешения выражения возвращает `Err` (вызывающий код может
/// обернуть в `Unresolved`).
fn resolve_ast_statement(
    stmt: &ast::Statement,
    model: Rc<RefCell<ModelNode>>,
) -> Result<Statement, Diagnostic> {
    match stmt {
        // ── Блок операторов ────────────────────────────────────────────────────
        ast::Statement::Block { statements, .. } => {
            let resolved = statements
                .iter()
                .map(|s| {
                    // Каждый вложенный оператор разрешается лучшим усилием
                    resolve_ast_statement(s, model.clone())
                        .unwrap_or_else(|_| Statement::Unresolved(s.clone()))
                })
                .collect();
            Ok(Statement::Block(resolved))
        }

        // ── Оператор-выражение (присваивание, вызов функции и т.п.) ───────────
        ast::Statement::Expression(_, expr) => {
            let resolved = construct_expression(expr.clone(), model)?;
            Ok(Statement::Expression(Box::new(resolved)))
        }

        // ── Условный оператор if ───────────────────────────────────────────────
        ast::Statement::If(_, cond, then_, else_) => {
            let cond = construct_expression(cond.clone(), model.clone())?;
            let then_ = resolve_ast_statement(then_, model.clone())
                .unwrap_or_else(|_| Statement::Unresolved(then_.as_ref().clone()));
            let else_ = else_
                .as_ref()
                .map(|e| {
                    resolve_ast_statement(e, model.clone())
                        .unwrap_or_else(|_| Statement::Unresolved(e.as_ref().clone()))
                })
                .map(Box::new);
            Ok(Statement::If {
                cond: Box::new(cond),
                then_: Box::new(then_),
                else_,
            })
        }

        // ── Цикл while ────────────────────────────────────────────────────────
        ast::Statement::While(_, cond, body) => {
            let cond = construct_expression(cond.clone(), model.clone())?;
            let body = resolve_ast_statement(body, model)
                .unwrap_or_else(|_| Statement::Unresolved(body.as_ref().clone()));
            Ok(Statement::While {
                cond: Box::new(cond),
                body: Box::new(body),
            })
        }

        // ── Цикл for ──────────────────────────────────────────────────────────
        ast::Statement::For(_, init, cond, step, body) => {
            let init = init
                .as_ref()
                .map(|s| {
                    resolve_ast_statement(s, model.clone())
                        .unwrap_or_else(|_| Statement::Unresolved(s.as_ref().clone()))
                })
                .map(Box::new);
            let cond = cond
                .as_ref()
                .map(|e| construct_expression(*e.clone(), model.clone()))
                .transpose()?
                .map(Box::new);
            let step = step
                .as_ref()
                .map(|e| construct_expression(*e.clone(), model.clone()))
                .transpose()?
                .map(Box::new);
            let body = body
                .as_ref()
                .map(|s| {
                    resolve_ast_statement(s, model.clone())
                        .unwrap_or_else(|_| Statement::Unresolved(s.as_ref().clone()))
                })
                .map(Box::new)
                .unwrap_or_else(|| Box::new(Statement::None));
            Ok(Statement::For {
                init,
                cond,
                step,
                body,
            })
        }

        // ── Цикл do...while ───────────────────────────────────────────────────
        ast::Statement::DoWhile(_, body, cond) => {
            let body = resolve_ast_statement(body, model.clone())
                .unwrap_or_else(|_| Statement::Unresolved(body.as_ref().clone()));
            let cond = construct_expression(cond.clone(), model)?;
            Ok(Statement::DoWhile {
                body: Box::new(body),
                cond: Box::new(cond),
            })
        }

        // ── Объявление локальной переменной ────────────────────────────────────
        ast::Statement::Variable(_, def, init) => {
            let types = model.borrow().types.clone();
            let (name, ty) = match def.as_ref() {
                ast::VariableDefine::Variable { name, typ, .. }
                | ast::VariableDefine::Constant { name, typ, .. } => {
                    let n = name.as_ref().map(|i| i.name.clone()).unwrap_or_default();
                    let t = construct_type(typ.clone(), &types)?;
                    (n, t)
                }
                ast::VariableDefine::Port { name, typ, .. } => {
                    let n = name.as_ref().map(|i| i.name.clone()).unwrap_or_default();
                    let t = construct_type(typ.clone(), &types)?;
                    (n, t)
                }
            };
            let init = init
                .as_ref()
                .map(|e| construct_expression(e.clone(), model))
                .transpose()?
                .map(Box::new);
            Ok(Statement::Variable(name, ty, init))
        }

        // ── Оператор return ────────────────────────────────────────────────────
        ast::Statement::Return(_, expr) => {
            let expr = expr
                .as_ref()
                .map(|e| construct_expression(e.clone(), model))
                .transpose()?
                .map(Box::new);
            Ok(Statement::Return(expr))
        }

        // ── Простые операторы без выражений ───────────────────────────────────
        ast::Statement::Continue(_) => Ok(Statement::Continue),
        ast::Statement::Break(_) => Ok(Statement::Break),

        // ── Прочие варианты: оставляем как Unresolved ─────────────────────────
        //
        // Assembly и Formula — встроенные низкоуровневые блоки, пропускаются.
        // Args, Error, StraySemicolon — служебные варианты, не требуют разрешения.
        _ => Ok(Statement::Unresolved(stmt.clone())),
    }
}

// ── Тесты ─────────────────────────────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use super::*;
    use crate::parse;
    use crate::semantic::tree::construct_model;

    /// Строит модель и возвращает корневой ModelNode.
    fn build(src: &str) -> crate::semantic::ModelNode {
        let (ast, _) = parse(src, 0).expect("ошибка разбора");
        construct_model(&ast, None, &[]).map(|m| m.take()).expect("ошибка построения")
    }

    // ─── Тесты resolve_statement ────────────────────────────────────────────

    /// `Statement::None` → возвращается без изменений.
    #[test]
    fn resolve_none_returns_none() {
        let m = Rc::new(RefCell::new(ModelNode::default()));
        let result = resolve_statement(&Statement::None, m).unwrap();
        assert_eq!(result, Statement::None);
    }

    /// Уже разрешённый `Statement::Continue` → возвращается без изменений.
    #[test]
    fn resolve_already_resolved_passthrough() {
        let m = Rc::new(RefCell::new(ModelNode::default()));
        let stmt = Statement::Continue;
        let result = resolve_statement(&stmt, m).unwrap();
        assert_eq!(result, Statement::Continue);
    }

    /// `Statement::Block([Continue, Break])` рекурсивно разрешается.
    #[test]
    fn resolve_block_recursively() {
        let m = Rc::new(RefCell::new(ModelNode::default()));
        let stmt = Statement::Block(vec![Statement::Continue, Statement::Break]);
        let result = resolve_statement(&stmt, m).unwrap();
        assert_eq!(result, Statement::Block(vec![Statement::Continue, Statement::Break]));
    }

    // ─── Интеграционные тесты через construct_model ──────────────────────────

    /// `always { it = it; }` с объявленной переменной `it` — разрешается в Block.
    #[test]
    fn model_level_always_block_with_known_var_resolves() {
        let node = build("var it: bit = 0; always { it = it; } start S;");
        let nb = node.named_blocks.get("always").expect("блок always должен быть");
        // Должен быть разрешён (не Unresolved)
        assert!(
            !matches!(nb.statement, Statement::Unresolved(_)),
            "блок always должен быть разрешён, получен: {:?}", nb.statement
        );
    }

    /// `always { debug("msg"); }` с необъявленной функцией — хранится как Unresolved (без паники).
    #[test]
    fn model_level_always_block_with_unknown_func_does_not_panic() {
        // `debug` не объявлена — после изменения expression.rs создаётся заглушка,
        // поэтому оператор может быть разрешён или нет, но паники быть не должно
        let node = build(r#"always { debug("msg"); } start S;"#);
        let nb = node.named_blocks.get("always").expect("блок always должен быть");
        let _ = &nb.statement; // просто доступ без паники
    }

    /// `enter { A = 0; }` внутри состояния — блок хранится в state.named_blocks.
    #[test]
    fn state_level_named_block_is_populated() {
        let node = build("var A: bit = false; start S { enter { A = A; } }");
        let state = node.states.get("S").expect("состояние S не найдено");
        let nbs = match state {
            crate::semantic::StateNode::Simple { named_blocks, .. } => named_blocks,
            crate::semantic::StateNode::Implement { named_blocks, .. } => named_blocks,
            _ => panic!("неожиданный вид состояния"),
        };
        assert!(nbs.contains_key("enter"), "enter должен быть в named_blocks состояния");
    }

    /// `enter { A = A; }` с известной переменной — разрешается.
    #[test]
    fn state_level_named_block_resolves_known_var() {
        let node = build("var A: bit = false; start S { enter { A = A; } }");
        let state = node.states.get("S").expect("состояние S не найдено");
        let nbs = match state {
            crate::semantic::StateNode::Simple { named_blocks, .. } => named_blocks,
            crate::semantic::StateNode::Implement { named_blocks, .. } => named_blocks,
            _ => panic!("неожиданный вид состояния"),
        };
        let enter = nbs.get("enter").expect("enter не найден");
        assert!(
            !matches!(enter.statement, Statement::Unresolved(_)),
            "enter должен быть разрешён"
        );
    }

    /// Несколько named blocks в одном состоянии: enter и exit.
    #[test]
    fn state_level_multiple_named_blocks() {
        let node = build("var A: bit = false; start S { enter { A = A; } exit { A = A; } }");
        let state = node.states.get("S").unwrap();
        let nbs = match state {
            crate::semantic::StateNode::Simple { named_blocks, .. } => named_blocks,
            _ => panic!("неожиданный вид состояния"),
        };
        assert!(nbs.contains_key("enter"), "enter должен быть");
        assert!(nbs.contains_key("exit"), "exit должен быть");
    }

    /// Разрешение оператора `if` в блоке состояния.
    #[test]
    fn state_named_block_if_statement_resolves() {
        let node = build("var x: bit = false; start S { always { if x { x = x; } } }");
        let state = node.states.get("S").unwrap();
        let nbs = match state {
            crate::semantic::StateNode::Simple { named_blocks, .. } => named_blocks,
            _ => panic!("неожиданный вид состояния"),
        };
        let always = nbs.get("always").expect("always не найден");
        assert!(matches!(always.statement, Statement::Block(_) | Statement::If { .. }),
            "оператор if должен быть разрешён: {:?}", always.statement);
    }

    /// Оператор `return` в named block разрешается.
    #[test]
    fn return_statement_resolves() {
        let node = build("var x: bit = false; always { return x; } start S;");
        let nb = node.named_blocks.get("always").expect("always не найден");
        assert!(!matches!(nb.statement, Statement::Unresolved(_)),
            "return должен быть разрешён: {:?}", nb.statement);
    }

    /// `continue` и `break` разрешаются в соответствующие варианты.
    #[test]
    fn continue_break_resolve() {
        // Создаём Statement::Unresolved напрямую и вызываем resolve_statement
        let ast_continue = ast::Statement::Continue(crate::diagnostics::Location::default());
        let ast_break = ast::Statement::Break(crate::diagnostics::Location::default());
        let m = Rc::new(RefCell::new(ModelNode::default()));
        let r1 = resolve_statement(&Statement::Unresolved(ast_continue), m.clone()).unwrap();
        let r2 = resolve_statement(&Statement::Unresolved(ast_break), m).unwrap();
        assert_eq!(r1, Statement::Continue);
        assert_eq!(r2, Statement::Break);
    }
}
