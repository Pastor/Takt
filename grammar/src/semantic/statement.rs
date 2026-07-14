//! Разрешение семантических операторов языка Lam.
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
//! ## Локальные переменные в блоках (С4)
//!
//! Объявления `var`/`const` внутри блоков (`if`, `loop`, `for`, `always` и др.)
//! разрешаются в [`StatementNode::Variable`] и временно регистрируются в
//! `model.variables` через [`register_local_var`]. Это позволяет последующим
//! операторам того же блока ссылаться на локальную переменную через обычный
//! механизм [`construct_expression`]. При выходе из блока [`unregister_local_vars`]
//! восстанавливает исходное состояние модели (с поддержкой затенения).
//!
//! Если выражение в операторе не может быть разрешено (например, ссылка
//! на необъявленную встроенную функцию), весь оператор сохраняется в виде
//! [`StatementNode::Unresolved`] — ошибка не пробрасывается наверх.
//! Это позволяет обрабатывать код с встроенными функциями (`debug`, `S` и т.п.)
//! без предварительной регистрации встроенных символов.

use crate::diagnostics::Diagnostic;
use crate::parser::ast;
use crate::semantic::condition::resolve_condition;
use crate::semantic::expression::construct_expression;
use crate::semantic::type_node::{TypeNode, construct_type};
use crate::semantic::{
    ExpressionNode, Formula, MatchArmNode, MatchPatternNode, ModelNode, StatementNode, VariableNode,
};
use std::cell::RefCell;
use std::rc::Rc;

/// Разрешает семантический оператор [`StatementNode`].
///
/// Для `Unresolved` вызывает [`resolve_ast_statement`].
/// Для `Block` рекурсивно разрешает каждый вложенный оператор.
/// Остальные варианты возвращаются без изменений.
///
/// При ошибке разрешения выражения оператор сохраняется как `Unresolved`.
pub fn resolve_statement(
    statement: &StatementNode,
    params: Vec<(String, TypeNode)>,
    model: Rc<RefCell<ModelNode>>,
) -> Result<StatementNode, Diagnostic> {
    match statement {
        StatementNode::Unresolved(stmt) => Ok(resolve_ast_statement(stmt, params.clone(), model)?),
        StatementNode::None => Ok(StatementNode::None),
        StatementNode::Block(stmts) => {
            let mut resolved = Vec::with_capacity(stmts.len());
            let mut locals: Vec<(String, Option<VariableNode>)> = Vec::new();
            for s in stmts {
                let r = resolve_statement(s, params.clone(), model.clone())?;
                if let Some(entry) = register_local_var(&r, &model) {
                    locals.push(entry);
                }
                resolved.push(r);
            }
            unregister_local_vars(locals, &model);
            Ok(StatementNode::Block(resolved))
        }
        other => Ok(other.clone()),
    }
}

/// Преобразует `ast::Statement` в разрешённый [`StatementNode`].
///
/// При ошибке разрешения выражения возвращает `Err` (вызывающий код может
/// обернуть в `Unresolved`).
fn resolve_ast_statement(
    stmt: &ast::Statement,
    params: Vec<(String, TypeNode)>,
    model: Rc<RefCell<ModelNode>>,
) -> Result<StatementNode, Diagnostic> {
    match stmt {
        // ── Блок операторов ────────────────────────────────────────────────────
        //
        // С4: после разрешения каждого оператора, если он объявляет локальную
        // переменную, та временно регистрируется в модели, чтобы последующие
        // операторы блока могли её использовать. При выходе из блока
        // все локальные переменные удаляются (затенённые — восстанавливаются).
        ast::Statement::Block { statements, .. } => {
            let mut resolved = Vec::with_capacity(statements.len());
            let mut locals: Vec<(String, Option<VariableNode>)> = Vec::new();
            for s in statements {
                let r = resolve_ast_statement(s, params.clone(), model.clone())?;
                if let Some(entry) = register_local_var(&r, &model) {
                    locals.push(entry);
                }
                resolved.push(r);
            }
            unregister_local_vars(locals, &model);
            Ok(StatementNode::Block(resolved))
        }

        // ── Оператор-выражение (присваивание, вызов функции и т.п.) ───────────
        ast::Statement::Expression(_, expr) => {
            let resolved = construct_expression(expr.clone(), params.clone(), model)?;
            Ok(StatementNode::Expression(Box::new(resolved)))
        }

        // ── Условный оператор if ───────────────────────────────────────────────
        ast::Statement::If(_, cond, then_, else_) => {
            let cond = construct_expression(cond.clone(), params.clone(), model.clone())?;
            let then_ = resolve_ast_statement(then_, params.clone(), model.clone())
                .unwrap_or_else(|_| StatementNode::Unresolved(then_.as_ref().clone()));
            let else_ = else_
                .as_ref()
                .map(|e| {
                    resolve_ast_statement(e, params.clone(), model.clone())
                        .unwrap_or_else(|_| StatementNode::Unresolved(e.as_ref().clone()))
                })
                .map(Box::new);
            Ok(StatementNode::If {
                cond: Box::new(cond),
                then_: Box::new(then_),
                else_,
            })
        }

        // ── Цикл loop ────────────────────────────────────────────────────────
        // `loop { тело }` — бесконечный цикл (cond = None)
        // `loop условие { тело }` — продолжается, пока условие истинно
        // Ключевое слово (`loop`/`while`) на семантику не влияет — синонимы.
        ast::Statement::Loop(_, cond, body, _) => {
            let cond = cond
                .as_ref()
                .map(|c| construct_expression(c.clone(), params.clone(), model.clone()))
                .transpose()?
                .map(Box::new);
            let body = resolve_ast_statement(body, params.clone(), model)
                .unwrap_or_else(|_| StatementNode::Unresolved(body.as_ref().clone()));
            Ok(StatementNode::Loop {
                cond,
                body: Box::new(body),
            })
        }

        // ── Цикл for ──────────────────────────────────────────────────────────
        //
        // С4: если инициализация объявляет переменную (`for var i: bit = 0; ...`),
        // та регистрируется в модели до разрешения условия, шага и тела цикла,
        // чтобы они могли на неё ссылаться. После разрешения всей конструкции
        // переменная удаляется из модели.
        ast::Statement::For(_, init, cond, step, body) => {
            let init_resolved = init
                .as_ref()
                .map(|s| {
                    resolve_ast_statement(s, params.clone(), model.clone())
                        .unwrap_or_else(|_| StatementNode::Unresolved(s.as_ref().clone()))
                })
                .map(Box::new);

            // Регистрируем переменную из init для cond/step/body
            let mut for_locals: Vec<(String, Option<VariableNode>)> = Vec::new();
            if let Some(init_stmt) = &init_resolved
                && let Some(entry) = register_local_var(init_stmt, &model)
            {
                for_locals.push(entry);
            }

            let cond = cond
                .as_ref()
                .map(|e| construct_expression(*e.clone(), params.clone(), model.clone()))
                .transpose()?
                .map(Box::new);
            let step = step
                .as_ref()
                .map(|e| construct_expression(*e.clone(), params.clone(), model.clone()))
                .transpose()?
                .map(Box::new);
            let body = body
                .as_ref()
                .map(|s| {
                    resolve_ast_statement(s, params.clone(), model.clone())
                        .unwrap_or_else(|_| StatementNode::Unresolved(s.as_ref().clone()))
                })
                .map(Box::new)
                .unwrap_or_else(|| Box::new(StatementNode::None));

            unregister_local_vars(for_locals, &model);

            Ok(StatementNode::For {
                init: init_resolved,
                cond,
                step,
                body,
            })
        }

        // ── Объявление локальной переменной ────────────────────────────────────
        //
        // С4: инициализатор берётся из def.initializer (поле внутри VariableDefine),
        // поскольку после исправления грамматики LocalVariableDefine передаёт
        // инициализатор именно туда, а третье поле Statement::Variable всегда None.
        ast::Statement::Variable(_, def, _extra_init) => {
            let (name, ty, def_init) = match def.as_ref() {
                ast::VariableDefine::Variable {
                    name,
                    typ,
                    initializer,
                    ..
                } => {
                    let n = name.as_ref().map(|i| i.name.clone()).unwrap_or_default();
                    let t = construct_type(typ.clone(), model.clone())?;
                    (n, t, initializer.clone())
                }
                ast::VariableDefine::Constant {
                    name,
                    typ,
                    initializer,
                    ..
                } => {
                    let n = name.as_ref().map(|i| i.name.clone()).unwrap_or_default();
                    let t = construct_type(typ.clone(), model.clone())?;
                    (n, t, Some(initializer.clone()))
                }
                ast::VariableDefine::Port {
                    name,
                    typ,
                    initializer,
                    ..
                } => {
                    let n = name.as_ref().map(|i| i.name.clone()).unwrap_or_default();
                    let t = construct_type(typ.clone(), model.clone())?;
                    (n, t, initializer.clone())
                }
            };
            let init = def_init
                .map(|e| construct_expression(e, params.clone(), model))
                .transpose()?
                .map(Box::new);
            Ok(StatementNode::Variable(name, ty, init))
        }

        // ── Оператор return ────────────────────────────────────────────────────
        ast::Statement::Return(_, expr) => {
            let expr = expr
                .as_ref()
                .map(|e| construct_expression(e.clone(), params.clone(), model))
                .transpose()?
                .map(Box::new);
            Ok(StatementNode::Return(expr))
        }

        // ── Простые операторы без выражений ───────────────────────────────────
        ast::Statement::Continue(_) => Ok(StatementNode::Continue),
        ast::Statement::Break(_) => Ok(StatementNode::Break),

        // ── Встроенная формула ─────────────────────────────────────────────────
        ast::Statement::InlineFormula(inline) => {
            match &**inline {
                ast::InlineFormulaDefine::Guard { conditions, .. } => {
                    let resolved: Vec<Formula> = conditions
                        .iter()
                        .filter_map(|c| resolve_condition(c, model.clone()).ok())
                        .map(|cn| crate::semantic::formula::condition_to_formula(&cn))
                        .collect();
                    Ok(StatementNode::InlineFormula(resolved))
                }
                ast::InlineFormulaDefine::Ltl { .. } => {
                    // TODO: Реализовать поддержку LTL в блоках кода
                    Ok(StatementNode::InlineFormula(Vec::new()))
                }
            }
        }

        // ── Оператор match ─────────────────────────────────────────────────────
        ast::Statement::Match(_, expr, ast_arms) => {
            let resolved_expr = construct_expression(*expr.clone(), params.clone(), model.clone())?;
            let mut arms: Vec<MatchArmNode> = Vec::new();
            for arm in ast_arms {
                let mut patterns: Vec<MatchPatternNode> = Vec::new();
                for pat in &arm.patterns {
                    let pat_node = match pat {
                        ast::MatchPattern::Wildcard(_) => MatchPatternNode::Wildcard,
                        ast::MatchPattern::Value(e) => {
                            let pexpr =
                                construct_expression(e.clone(), params.clone(), model.clone())?;
                            MatchPatternNode::Value(Box::new(pexpr))
                        }
                    };
                    patterns.push(pat_node);
                }
                let body_node = resolve_statement(
                    &StatementNode::Unresolved(*arm.body.clone()),
                    params.clone(),
                    model.clone(),
                )?;
                arms.push(MatchArmNode {
                    patterns,
                    body: Box::new(body_node),
                });
            }
            Ok(StatementNode::Match {
                expr: Box::new(resolved_expr),
                arms,
            })
        }

        // ── Прочие варианты: оставляем как Unresolved ─────────────────────────
        //
        // Assembly и Formula — встроенные низкоуровневые блоки, пропускаются.
        // Args, Error, StraySemicolon — служебные варианты, не требуют разрешения.
        _ => Ok(StatementNode::Unresolved(stmt.clone())),
    }
}

// ── Вспомогательные функции для управления областью видимости (С4) ────────────

/// Временно регистрирует локальную переменную из разрешённого оператора в модели.
///
/// Если оператор — [`StatementNode::Variable`], вставляет [`VariableNode::Simple`]
/// в `model.variables` и возвращает имя вместе с предыдущим значением (если имя
/// уже было занято — для поддержки затенения). Для остальных операторов
/// возвращает `None`.
///
/// Регистрация происходит ПОСЛЕ разрешения оператора, поэтому самоссылающийся
/// инициализатор (`var x = x`) корректно завершится ошибкой поиска переменной
/// при разрешении выражения.
fn register_local_var(
    stmt: &StatementNode,
    model: &Rc<RefCell<ModelNode>>,
) -> Option<(String, Option<VariableNode>)> {
    if let StatementNode::Variable(name, ty, _) = stmt {
        if name.is_empty() {
            return None;
        }
        // Сохраняем предыдущее значение для восстановления при выходе из блока
        let prev = model.borrow().variables.get(name).cloned();
        let node = VariableNode::Simple {
            upper: Some(Rc::downgrade(model)),
            loc: crate::diagnostics::Location::Implicit,
            name: name.clone(),
            ty: ty.clone(),
            // Expression::None — заглушка; инициализатор уже сохранён в
            // Statement::Variable и не нужен в узле переменной для разрешения выражений
            expr: ExpressionNode::None,
        };
        model.borrow_mut().variables.insert(name.clone(), node);
        Some((name.clone(), prev))
    } else {
        None
    }
}

/// Восстанавливает состояние модели после выхода из блока.
///
/// Для каждой записи из `locals`:
/// - Если до регистрации была переменная с тем же именем — восстанавливает её.
/// - Если не было — удаляет имя из `model.variables`.
///
/// Порядок восстановления не важен, поскольку имена уникальны внутри одного блока.
fn unregister_local_vars(
    locals: Vec<(String, Option<VariableNode>)>,
    model: &Rc<RefCell<ModelNode>>,
) {
    for (name, prev) in locals {
        match prev {
            Some(node) => {
                model.borrow_mut().variables.insert(name, node);
            }
            None => {
                model.borrow_mut().variables.remove(&name);
            }
        }
    }
}

// ── Тесты ─────────────────────────────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use super::*;
    use crate::parse;
    use crate::semantic::tree::construct_model;

    /// Строит модель и возвращает корневой ModelNode.
    fn build(src: &str) -> ModelNode {
        let (ast, _) = parse(src, 0).expect("ошибка разбора");
        construct_model(&ast, None, &[])
            .map(|m| m.take())
            .expect("ошибка построения")
    }

    // ─── Тесты resolve_statement ────────────────────────────────────────────

    /// `Statement::None` → возвращается без изменений.
    #[test]
    fn resolve_none_returns_none() {
        let m = Rc::new(RefCell::new(ModelNode::default()));
        let result = resolve_statement(&StatementNode::None, vec![], m).unwrap();
        assert_eq!(result, StatementNode::None);
    }

    /// Уже разрешённый `Statement::Continue` → возвращается без изменений.
    #[test]
    fn resolve_already_resolved_passthrough() {
        let m = Rc::new(RefCell::new(ModelNode::default()));
        let stmt = StatementNode::Continue;
        let result = resolve_statement(&stmt, vec![], m).unwrap();
        assert_eq!(result, StatementNode::Continue);
    }

    /// `Statement::Block([Continue, Break])` рекурсивно разрешается.
    #[test]
    fn resolve_block_recursively() {
        let m = Rc::new(RefCell::new(ModelNode::default()));
        let stmt = StatementNode::Block(vec![StatementNode::Continue, StatementNode::Break]);
        let result = resolve_statement(&stmt, vec![], m).unwrap();
        assert_eq!(
            result,
            StatementNode::Block(vec![StatementNode::Continue, StatementNode::Break])
        );
    }

    // ─── Интеграционные тесты через construct_model ──────────────────────────

    /// `always { it = it; }` с объявленной переменной `it` — разрешается в Block.
    #[test]
    fn model_level_always_block_with_known_var_resolves() {
        let node = build("var it: bit := 0; always { it := it; } start S;");
        let nb = node
            .get_named_block("always")
            .expect("блок always должен быть");
        let stmt = nb.statement().expect("оператор должен быть");
        // Должен быть разрешён (не Unresolved)
        assert!(
            !matches!(stmt, StatementNode::Unresolved(_)),
            "блок always должен быть разрешён, получен: {:?}",
            stmt
        );
    }

    /// `always { debug("msg"); }` с необъявленной функцией — хранится как Unresolved (без паники).
    #[test]
    fn model_level_always_block_with_unknown_func_does_not_panic() {
        // `debug` не объявлена — после изменения expression.rs создаётся заглушка,
        // поэтому оператор может быть разрешён или нет, но паники быть не должно
        let node = build(r#"always { debug("msg"); } start S;"#);
        let nb = node
            .get_named_block("always")
            .expect("блок always должен быть");
        let _ = nb.statement(); // просто доступ без паники
    }

    /// `enter { A = 0; }` внутри состояния — блок хранится в state.named_blocks.
    #[test]
    fn state_level_named_block_is_populated() {
        let node = build("var A: bit := false; start S { enter { A := A; } }");
        let state = node.states.get("S").expect("состояние S не найдено");
        assert!(
            state.get_named_block("enter").is_some(),
            "enter должен быть в named_blocks состояния"
        );
    }

    /// `enter { A = A; }` с известной переменной — разрешается.
    #[test]
    fn state_level_named_block_resolves_known_var() {
        let node = build("var A: bit := false; start S { enter { A := A; } }");
        let state = node.states.get("S").expect("состояние S не найдено");
        let enter = state.get_named_block("enter").expect("enter не найден");
        let stmt = enter.statement().expect("оператор должен быть");
        assert!(
            !matches!(stmt, StatementNode::Unresolved(_)),
            "enter должен быть разрешён"
        );
    }

    /// Несколько named blocks в одном состоянии: enter и exit.
    #[test]
    fn state_level_multiple_named_blocks() {
        let node = build("var A: bit := false; start S { enter { A := A; } exit { A := A; } }");
        let state = node.states.get("S").unwrap();
        assert!(
            state.get_named_block("enter").is_some(),
            "enter должен быть"
        );
        assert!(state.get_named_block("exit").is_some(), "exit должен быть");
    }

    /// Разрешение оператора `if` в блоке состояния.
    #[test]
    fn state_named_block_if_statement_resolves() {
        let node = build("var x: bit := false; start S { always { if x { x := x; } } }");
        let state = node.states.get("S").unwrap();
        let always = state.get_named_block("always").expect("always не найден");
        let stmt = always.statement().expect("оператор должен быть");
        assert!(
            matches!(stmt, StatementNode::Block(_) | StatementNode::If { .. }),
            "оператор if должен быть разрешён: {:?}",
            stmt
        );
    }

    /// Оператор `return` в named block разрешается.
    #[test]
    fn return_statement_resolves() {
        let node = build("var x: bit := false; always { return x; } start S;");
        let nb = node.get_named_block("always").expect("always не найден");
        let stmt = nb.statement().expect("оператор должен быть");
        assert!(
            !matches!(stmt, StatementNode::Unresolved(_)),
            "return должен быть разрешён: {:?}",
            stmt
        );
    }

    /// `continue` и `break` разрешаются в соответствующие варианты.
    #[test]
    fn continue_break_resolve() {
        let ast_continue = ast::Statement::Continue(crate::diagnostics::Location::default());
        let ast_break = ast::Statement::Break(crate::diagnostics::Location::default());
        let m = Rc::new(RefCell::new(ModelNode::default()));
        let r1 =
            resolve_statement(&StatementNode::Unresolved(ast_continue), vec![], m.clone()).unwrap();
        let r2 = resolve_statement(&StatementNode::Unresolved(ast_break), vec![], m).unwrap();
        assert_eq!(r1, StatementNode::Continue);
        assert_eq!(r2, StatementNode::Break);
    }

    // ─── Циклы ────────────────────────────────────────────────────────────────

    // ── Вспомогательная функция ───────────────────────────────────────────────

    /// Возвращает первый реальный оператор из блока (вложенного в always-блок).
    ///
    /// `always { <stmt> }` оборачивает оператор в `Block([<stmt>])`.
    /// Эта функция раскрывает один уровень вложенности.
    fn first_in_block(stmt: &StatementNode) -> &StatementNode {
        match stmt {
            StatementNode::Block(stmts) => stmts.first().expect("блок пуст"),
            other => other,
        }
    }

    // ── Циклы ─────────────────────────────────────────────────────────────────

    /// `loop условие { }` разрешается в `Statement::Loop` с условием.
    ///
    /// # Пример (Lam)
    /// ```but
    /// var flag: bit = false;
    /// always {
    ///     loop flag { flag = flag; }
    /// }
    /// ```
    #[test]
    fn loop_with_condition_resolves() {
        let node = build(
            // loop с условием — аналог while
            "var flag: bit := false; always { loop flag { flag := flag; } } start S;",
        );
        let nb = node.get_named_block("always").expect("always не найден");
        let stmt = first_in_block(nb.statement().expect("оператор должен быть"));
        assert!(
            matches!(stmt, StatementNode::Loop { cond: Some(_), .. }),
            "ожидался Loop с условием, получен: {:?}",
            stmt
        );
    }

    /// `loop { }` без условия разрешается в `Statement::Loop` с `cond = None`.
    ///
    /// # Пример (Lam)
    /// ```but
    /// always {
    ///     loop { break; }
    /// }
    /// ```
    #[test]
    fn loop_without_condition_resolves() {
        let node = build(
            // loop без условия — бесконечный цикл
            "always { loop { break; } } start S;",
        );
        let nb = node.get_named_block("always").expect("always не найден");
        let stmt = first_in_block(nb.statement().expect("оператор должен быть"));
        assert!(
            matches!(stmt, StatementNode::Loop { cond: None, .. }),
            "ожидался Loop без условия, получен: {:?}",
            stmt
        );
    }

    /// `for` с инициализацией, условием и шагом разрешается.
    ///
    /// # Пример (Lam)
    /// ```but
    /// var i: bit = false;
    /// always {
    ///     // С1 (вариант A): for без скобок вокруг заголовка
    ///     for i = false; i; i = false { }
    /// }
    /// ```
    #[test]
    fn for_loop_resolves() {
        let node = build(
            // С1 (вариант A): for без скобок вокруг заголовка
            "var i: bit := false; always { for i := false; i; i := false { } } start S;",
        );
        let nb = node.get_named_block("always").expect("always не найден");
        let stmt = first_in_block(nb.statement().expect("оператор должен быть"));
        assert!(
            matches!(stmt, StatementNode::For { .. }),
            "ожидался For, получен: {:?}",
            stmt
        );
    }

    /// `for` без инициализации и шага разрешается.
    ///
    /// # Пример (Lam)
    /// ```but
    /// // С1 (вариант A): for без скобок, все части пусты
    /// for ;; { }
    /// ```
    #[test]
    fn for_loop_empty_parts_resolves() {
        // С1 (вариант A): for без скобок вокруг заголовка
        let node = build("var i: bit := false; always { for ;; { } } start S;");
        let nb = node.get_named_block("always").expect("always не найден");
        let stmt = first_in_block(nb.statement().expect("оператор должен быть"));
        assert!(
            matches!(
                stmt,
                StatementNode::For {
                    init: None,
                    cond: None,
                    step: None,
                    ..
                }
            ),
            "ожидался For{{None, None, None}}, получен: {:?}",
            stmt
        );
    }

    /// Оператор `return` без значения разрешается.
    #[test]
    fn return_without_value_resolves() {
        let node = build("always { return; } start S;");
        let nb = node.get_named_block("always").expect("always не найден");
        let stmt = first_in_block(nb.statement().expect("оператор должен быть"));
        assert!(
            matches!(stmt, StatementNode::Return(None)),
            "ожидался Return(None), получен: {:?}",
            stmt
        );
    }

    /// Оператор `if` с `else`-веткой разрешается.
    ///
    /// # Пример (Lam)
    /// ```but
    /// var x: bit = false;
    /// always {
    ///     if x { x = x; } else { x = false; }
    /// }
    /// ```
    #[test]
    fn if_else_resolves() {
        let node =
            build("var x: bit := false; always { if x { x := x; } else { x := false; } } start S;");
        let nb = node.get_named_block("always").expect("always не найден");
        let stmt = first_in_block(nb.statement().expect("оператор должен быть"));
        assert!(
            matches!(stmt, StatementNode::If { else_: Some(_), .. }),
            "ожидался If{{else_: Some}}, получен: {:?}",
            stmt
        );
    }

    /// `if` без `else` имеет `else_ = None`.
    #[test]
    fn if_without_else_has_none_else() {
        let node = build("var x: bit := false; always { if x { x := x; } } start S;");
        let nb = node.get_named_block("always").expect("always не найден");
        let stmt = first_in_block(nb.statement().expect("оператор должен быть"));
        assert!(
            matches!(stmt, StatementNode::If { else_: None, .. }),
            "ожидался If{{else_: None}}, получен: {:?}",
            stmt
        );
    }

    // ─── С4: локальные переменные в блоках ────────────────────────────────────

    /// `always { var x: bit = false; x = true; }` — локальная переменная
    /// объявляется и используется в том же блоке.
    ///
    /// # Пример (Lam)
    /// ```but
    /// always { var x: bit = false; x = true; }
    /// start S;
    /// ```
    #[test]
    fn local_var_in_block_resolves() {
        let node = build("always { var x: bit := false; x := true; } start S;");
        let nb = node.get_named_block("always").expect("always не найден");
        let stmt = nb.statement().expect("оператор должен быть");
        // Блок должен быть разрешён
        assert!(
            !matches!(stmt, StatementNode::Unresolved(_)),
            "блок с локальной var должен быть разрешён: {:?}",
            stmt
        );
        // Первый оператор — Statement::Variable(x, ...)
        if let StatementNode::Block(stmts) = stmt {
            assert!(
                matches!(stmts.first(), Some(StatementNode::Variable(n, _, _)) if n == "x"),
                "первый оператор должен быть Variable(x): {:?}",
                stmts.first()
            );
        } else {
            panic!("ожидался Statement::Block, получен: {:?}", stmt);
        }
    }

    /// `always { var x: bit = false; x = true; }` — инициализатор `false`
    /// сохраняется в `Statement::Variable`.
    ///
    /// # Пример (Lam)
    /// ```but
    /// always { var x: bit = false; x = true; }
    /// start S;
    /// ```
    #[test]
    fn local_var_initializer_is_preserved() {
        let node = build("always { var x: bit := false; x := true; } start S;");
        let nb = node.get_named_block("always").expect("always не найден");
        let stmt = nb.statement().expect("оператор должен быть");
        if let StatementNode::Block(stmts) = stmt {
            // Инициализатор должен быть Some(Bool(false))
            assert!(
                matches!(stmts.first(), Some(StatementNode::Variable(_, _, Some(_)))),
                "у переменной x должен быть инициализатор: {:?}",
                stmts.first()
            );
        }
    }

    /// `always { const C: bit = true; C; }` — константа внутри блока разрешается.
    ///
    /// # Пример (Lam)
    /// ```but
    /// always { const C: bit = true; C; }
    /// start S;
    /// ```
    #[test]
    fn local_const_in_block_resolves() {
        let node = build("always { const C: bit := true; C; } start S;");
        let nb = node.get_named_block("always").expect("always не найден");
        let stmt = nb.statement().expect("оператор должен быть");
        assert!(
            !matches!(stmt, StatementNode::Unresolved(_)),
            "блок с локальной const должен быть разрешён: {:?}",
            stmt
        );
        if let StatementNode::Block(stmts) = stmt {
            assert!(
                matches!(stmts.first(), Some(StatementNode::Variable(n, _, _)) if n == "C"),
                "первый оператор должен быть Variable(C): {:?}",
                stmts.first()
            );
        }
    }

    /// Локальная переменная затеняет переменную уровня модели.
    ///
    /// Внутри блока `x` ссылается на локальную переменную, после выхода — на model-level.
    ///
    /// # Пример (Lam)
    /// ```but
    /// var x: bit = true;
    /// always { var x: bit = false; x = false; }
    /// start S;
    /// ```
    #[test]
    fn local_var_shadows_model_var() {
        let node =
            build("var x: bit := true; always { var x: bit := false; x := false; } start S;");
        let nb = node.get_named_block("always").expect("always не найден");
        let stmt = nb.statement().expect("оператор должен быть");
        // Блок разрешается (затенение работает)
        assert!(
            !matches!(stmt, StatementNode::Unresolved(_)),
            "блок с затенённой переменной должен быть разрешён: {:?}",
            stmt
        );
        // После выхода из блока model-level x должна остаться в модели
        assert!(
            node.search_var("x").is_some(),
            "model-level переменная x должна сохраниться после выхода из блока"
        );
    }

    /// После выхода из блока локальная переменная удаляется из модели.
    ///
    /// Проверяет механизм [`unregister_local_vars`] напрямую: после разрешения
    /// `{ var inner: bit = false; }` переменная `inner` не должна находиться
    /// в `model.variables`.
    ///
    /// # Контрпример (Lam)
    /// ```but
    /// { var inner: bit = false; }
    /// inner = true;   // ошибка: inner вне области видимости
    /// ```
    #[test]
    fn local_var_scope_exits_block() {
        use crate::diagnostics::Location;
        use crate::parser::ast::{Identifier, VariableDefine};

        let loc = Location::default();
        let def = Box::new(VariableDefine::Variable {
            loc,
            typ: Some(crate::parser::ast::Type::Bit),
            name: Some(Identifier {
                loc,
                name: "inner".into(),
            }),
            initializer: None,
        });
        let var_stmt = ast::Statement::Variable(loc, def, None);
        let block = ast::Statement::Block {
            loc,
            unchecked: false,
            statements: vec![var_stmt],
        };

        let m = Rc::new(RefCell::new(ModelNode::default()));
        let resolved = resolve_ast_statement(&block, vec![], m.clone()).unwrap();

        // После разрешения блока `inner` не должна оставаться в модели
        assert!(
            m.borrow().variables.get("inner").is_none(),
            "inner не должна быть в модели после выхода из блока"
        );
        // Но Statement::Variable(inner) должен быть внутри блока
        if let StatementNode::Block(stmts) = resolved {
            assert!(
                matches!(stmts.first(), Some(StatementNode::Variable(n, _, _)) if n == "inner"),
                "разрешённый блок должен содержать Variable(inner): {:?}",
                stmts.first()
            );
        } else {
            panic!("ожидался Statement::Block");
        }
    }

    /// `for var i: bit = false; i; i = false { }` — переменная из init for
    /// видна в условии и шаге цикла.
    ///
    /// # Пример (Lam)
    /// ```but
    /// always { for var i: bit = false; i; i = false { } }
    /// start S;
    /// ```
    #[test]
    fn local_var_in_for_init_resolves() {
        let node = build("always { for var i: bit := false; i; i := false { } } start S;");
        let nb = node.get_named_block("always").expect("always не найден");
        let stmt = first_in_block(nb.statement().expect("оператор должен быть"));
        assert!(
            matches!(
                stmt,
                StatementNode::For {
                    init: Some(_),
                    cond: Some(_),
                    ..
                }
            ),
            "ожидался For{{init: Some, cond: Some}}: {:?}",
            stmt
        );
        if let StatementNode::For {
            init: Some(init_box),
            ..
        } = stmt
        {
            assert!(
                matches!(init_box.as_ref(), StatementNode::Variable(n, _, _) if n == "i"),
                "init for должен быть Variable(i): {:?}",
                init_box
            );
        }
    }

    /// Параметр функции виден в операторе-выражении (`Statement::Expression`).
    ///
    /// Регрессионный тест для ошибки «Идентификатор 'value' не найден в области видимости».
    /// До исправления `construct_expression` в ветке `Expression` вызывалась с `vec![]`
    /// вместо `params.clone()`, из-за чего параметры функции были невидимы в операторах вида
    /// `value > 100` или `out = value`.
    ///
    /// # Пример (Lam)
    /// ```but
    /// fn clamp_temp(value: u8): u8 {
    ///     if value > 100 { return 100; }
    ///     return value;
    /// }
    /// start S;
    /// ```
    #[test]
    fn function_param_visible_in_expression_statement() {
        use crate::semantic::FunctionDefinitionNode;
        // `result = value` — оператор-выражение (присваивание), где `value` — параметр функции.
        // До исправления это приводило к ошибке LSP «Идентификатор 'value' не найден».
        let node = build(concat!(
            "type u8 = [bit;8]; ",
            "var result: u8 := 0; ",
            "fn clamp(value: u8) -> u8 { result := value; return value; } ",
            "start S;"
        ));
        // Проверяем, что модель успешно построена и функция clamp присутствует
        assert!(
            node.functions.contains_key("clamp"),
            "функция clamp должна быть в модели"
        );
        // Проверяем, что тело функции разрешено (нет Unresolved на верхнем уровне)
        let func = node
            .functions
            .get("clamp")
            .expect("функция clamp не найдена");
        if let FunctionDefinitionNode::Local { body, .. } = func {
            assert!(
                !matches!(body, StatementNode::Unresolved(_)),
                "тело функции clamp должно быть разрешено, получен: {:?}",
                body
            );
        } else {
            panic!("функция clamp должна быть Local, получен: {:?}", func);
        }
    }

    /// Объявление переменной в операторе `var` разрешается в `Statement::Variable`.
    ///
    /// Тест проверяет семантический уровень напрямую (через `ast::Statement::Variable`).
    #[test]
    fn variable_statement_resolves() {
        use crate::diagnostics::Location;
        use crate::parser::ast::{Identifier, VariableDefine};
        let loc = Location::default();
        let def = Box::new(VariableDefine::Variable {
            loc,
            typ: Some(crate::parser::ast::Type::Bit),
            name: Some(Identifier {
                loc,
                name: "tmp".into(),
            }),
            initializer: None,
        });
        let stmt = ast::Statement::Variable(loc, def, None);
        let m = Rc::new(RefCell::new(ModelNode::default()));
        let result = resolve_statement(&StatementNode::Unresolved(stmt), vec![], m).unwrap();
        assert!(
            matches!(result, StatementNode::Variable(ref n, _, None) if n == "tmp"),
            "ожидался Variable(tmp, _, None), получен: {:?}",
            result
        );
    }
}
