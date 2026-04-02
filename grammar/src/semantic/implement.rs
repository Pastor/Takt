use crate::diagnostics::Diagnostic;
use crate::parser::ast;
use crate::semantic::{ExpressionNode, ModelNode};
use std::cell::RefCell;
use std::rc::Rc;

/// Реализация модели: описывает, как состояние или корневой автомат
/// составлен из именованных моделей.
///
/// - [`Unresolved`](Implement::Unresolved) — временная заглушка до второго прохода.
/// - [`Model`](Implement::Model) — ссылка на конкретную именованную модель.
/// - [`Parentless`](Implement::Parentless) — обёртка без родителя (скобки).
/// - [`Add`](Implement::Add) — последовательная компоновка `A + B`.
/// - [`Or`](Implement::Or) — параллельная компоновка `A | B`.
#[derive(Default, Debug, PartialEq, Eq, Clone)]
pub enum Implement {
    /// Реализация не задана (значение по умолчанию для безымянной корневой модели).
    #[default]
    None,
    /// «Сырое» АСД-выражение реализации, ожидающее разрешения на этапе stage1.
    Unresolved(ast::Expression),
    /// Ссылка на конкретную именованную модель.
    Model(Rc<RefCell<ModelNode>>),
    /// Скобочная группировка: `(реализация)`.
    Parentless(Box<Implement>),
    /// Последовательная компоновка: `левое + правое`.
    Add(Box<Implement>, Box<Implement>),
    /// Параллельная компоновка: `левое | правое`.
    Or(Box<Implement>, Box<Implement>),

    /// Последовательная компоновка: `левое + правое + ...`.
    Sequence(Vec<Box<Implement>>),
    /// Параллельная компоновка: `левое | правое | ...`.
    Parallel(Vec<Box<Implement>>),
}

/// Строит [`Implement`] из семантического выражения [`ExpressionNode`].
///
/// Обрабатывает разрешённые семантические выражения: `Model`, `Add`, `BitwiseOr`,
/// `Parenthesis`. Для ещё не разрешённых АСД-выражений (`Unresolved`) делегирует
/// в [`construct_implement_ast`], который напрямую обходит структуру АСД,
/// минуя полный цикл `construct_expression`. Это оптимизация: выражения реализации
/// (`A + B`, `A | B`) используют лишь ограниченное подмножество языка,
/// и специализированный обход работает быстрее и без лишних аллокаций.
///
/// # Ошибки
///
/// Возвращает [`Diagnostic`], если встречается неподдерживаемое выражение
/// (например, числовой литерал там, где ожидается имя модели).
pub fn construct_implement(
    expression: ExpressionNode,
    model: Rc<RefCell<ModelNode>>,
) -> Result<Implement, Diagnostic> {
    let model = Rc::clone(&model);
    match expression {
        // Ещё не разрешённое АСД-выражение: обходим напрямую без полного construct_expression
        ExpressionNode::Unresolved(expr) => construct_implement_ast(expr, model),
        // Разрешённая модель
        ExpressionNode::Model(model) => Ok(Implement::Model(Rc::clone(&model))),
        ExpressionNode::Parenthesis(expression) => construct_implement(*expression, model),
        ExpressionNode::Add(left, right) => {
            let left = construct_implement(*left, model.clone())?;
            let right = construct_implement(*right, model.clone())?;
            Ok(Implement::Add(Box::new(left), Box::new(right)))
        }
        ExpressionNode::BitwiseOr(left, right) => {
            let left = construct_implement(*left, model.clone())?;
            let right = construct_implement(*right, model.clone())?;
            Ok(Implement::Or(Box::new(left), Box::new(right)))
        }
        other => Err(format!("Неизвестное выражение реализации: {:?}", other)
            .as_str()
            .into()),
    }
}

/// Строит [`Implement`] непосредственно из АСД-выражения [`ast::Expression`].
///
/// Используется из [`construct_implement`] для обработки варианта
/// [`ExpressionNode::Unresolved`]. Напрямую обходит АСД, не вызывая полный
/// [`construct_expression`], что является оптимизацией для ограниченного
/// подмножества выражений реализации.
///
/// Поддерживаемые варианты:
/// - `Variable(id)` → именованная модель `id`;
/// - `Add(left, right)` → последовательная компоновка `left + right`;
/// - `BitwiseOr(left, right)` → параллельная компоновка `left | right`;
/// - `Parenthesis(inner)` → группировка `(inner)`.
///
/// # Ошибки
///
/// Возвращает [`Diagnostic`], если модель не найдена или встречается
/// неподдерживаемый вид выражения (например, числовой литерал).
fn construct_implement_ast(
    expr: ast::Expression,
    model: Rc<RefCell<ModelNode>>,
) -> Result<Implement, Diagnostic> {
    match expr {
        ast::Expression::Variable(id) => {
            let borrowed = model.as_ref().borrow();
            let found = borrowed.search_model(&id.name).ok_or_else(|| {
                Diagnostic::error(id.loc, format!("Модель '{}' не найдена", &id.name))
            })?;
            Ok(Implement::Model(Rc::clone(&found)))
        }
        ast::Expression::Parenthesis(_, inner) => construct_implement_ast(*inner, model),
        ast::Expression::Add(_, left, right) => {
            let left = construct_implement_ast(*left, model.clone())?;
            let right = construct_implement_ast(*right, model.clone())?;
            Ok(Implement::Add(Box::new(left), Box::new(right)))
        }
        ast::Expression::BitwiseOr(_, left, right) => {
            let left = construct_implement_ast(*left, model.clone())?;
            let right = construct_implement_ast(*right, model.clone())?;
            Ok(Implement::Or(Box::new(left), Box::new(right)))
        }
        other => Err(
            format!("Выражение реализации не поддерживается: {:?}", other)
                .as_str()
                .into(),
        ),
    }
}

fn unroll_implement_expression_ast(
    expr: ast::Expression,
    model: Rc<RefCell<ModelNode>>,
) -> Result<ExpressionNode, Diagnostic> {
    match expr {
        ast::Expression::Variable(id) => {
            let borrowed = model.as_ref().borrow();
            let found = borrowed.search_model(&id.name).ok_or_else(|| {
                Diagnostic::error(id.loc, format!("Модель '{}' не найдена", &id.name))
            })?;
            Ok(ExpressionNode::Model(Rc::clone(&found)))
        }
        ast::Expression::Parenthesis(_, inner) => unroll_implement_expression_ast(*inner, model),
        ast::Expression::Add(_, left, right) => {
            let left = unroll_implement_expression_ast(*left, model.clone())?;
            let right = unroll_implement_expression_ast(*right, model.clone())?;
            Ok(ExpressionNode::Add(Box::new(left), Box::new(right)))
        }
        ast::Expression::BitwiseOr(_, left, right) => {
            let left = unroll_implement_expression_ast(*left, model.clone())?;
            let right = unroll_implement_expression_ast(*right, model.clone())?;
            Ok(ExpressionNode::BitwiseOr(Box::new(left), Box::new(right)))
        }
        other => Err(
            format!("Выражение реализации не поддерживается: {:?}", other)
                .as_str()
                .into(),
        ),
    }
}

pub fn unroll_implement_expression(
    expression: ExpressionNode,
    model: Rc<RefCell<ModelNode>>,
) -> Result<Implement, Diagnostic> {
    let model = Rc::clone(&model);
    match expression {
        ExpressionNode::Unresolved(expr) => {
            let unrolled = unroll_implement_expression_ast(expr, model.clone())?;
            unroll_implement_expression(unrolled, model)
        }
        ExpressionNode::Model(model) => Ok(Implement::Model(Rc::clone(&model))),
        ExpressionNode::Parenthesis(expression) => unroll_implement_expression(*expression, model),
        ExpressionNode::Add(left, right) => {
            let left = unroll_implement_expression(*left, model.clone())?;
            let right = unroll_implement_expression(*right, model.clone())?;
            if let Implement::Sequence(mut seq) = left.clone()
                && let Implement::Model(_) = right.clone()
            {
                seq.push(Box::new(right));
                return Ok(Implement::Sequence(seq));
            } else if let Implement::Model(_) = left.clone()
                && let Implement::Sequence(mut seq) = right
            {
                seq.push(Box::new(left));
                return Ok(Implement::Sequence(seq));
            } else if let Implement::Sequence(seq1) = left.clone()
                && let Implement::Sequence(mut seq) = right
            {
                seq.extend(seq1);
                return Ok(Implement::Sequence(seq));
            } else if let Implement::Model(_) = left.clone()
                && let Implement::Model(_) = right.clone()
            {
                return Ok(Implement::Sequence(vec![Box::new(left), Box::new(right)]));
            }
            todo!("Not implemented")
        }
        ExpressionNode::BitwiseOr(left, right) => {
            let left = unroll_implement_expression(*left, model.clone())?;
            let right = unroll_implement_expression(*right, model.clone())?;
            if let Implement::Parallel(mut parallel) = left.clone()
                && let Implement::Model(_) = right.clone()
            {
                parallel.push(Box::new(right));
                return Ok(Implement::Parallel(parallel));
            } else if let Implement::Model(_) = left.clone()
                && let Implement::Parallel(mut parallel) = right
            {
                parallel.push(Box::new(left));
                return Ok(Implement::Parallel(parallel));
            } else if let Implement::Parallel(other) = left.clone()
                && let Implement::Parallel(mut parallel) = right
            {
                parallel.extend(other);
                return Ok(Implement::Parallel(parallel));
            } else if let Implement::Model(_) = left.clone()
                && let Implement::Model(_) = right.clone()
            {
                return Ok(Implement::Parallel(vec![Box::new(left), Box::new(right)]));
            }
            todo!("Not implemented")
        }
        other => Err(format!("Неизвестное выражение реализации: {:?}", other)
            .as_str()
            .into()),
    }
}

#[cfg(test)]
mod tests {
    use crate::diagnostics::Location;
    use crate::parse;
    use crate::parser::ast;
    use crate::semantic::ExpressionNode;
    use crate::semantic::implement::{Implement, unroll_implement_expression};
    use crate::semantic::tree::construct_model;

    #[test]
    fn test_unroll_implement_expression() {
        let src = r#"
model A {
    start Start;
}
model B {
    start Start;
}
start Entry = A;
"#;
        let (ast, _) = parse(src, 0).unwrap();
        let model_rc = construct_model(&ast, None, &[]).unwrap();

        let implement = unroll_implement_expression(
            ExpressionNode::Unresolved(ast::Expression::Variable(ast::Identifier::new("A"))),
            model_rc.clone(),
        )
        .unwrap();
        assert!(matches!(implement, Implement::Model(_)));
        let implement = unroll_implement_expression(
            ExpressionNode::Unresolved(ast::Expression::BitwiseOr(
                Location::Implicit,
                Box::new(ast::Expression::Variable(ast::Identifier::new("A"))),
                Box::new(ast::Expression::Variable(ast::Identifier::new("B"))),
            )),
            model_rc.clone(),
        )
        .unwrap();
        assert!(matches!(implement, Implement::Parallel(_)));
    }
}
