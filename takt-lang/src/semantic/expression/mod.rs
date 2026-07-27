//! Построение семантических выражений языка Takt.
//!
//! Основная функция [`construct_expression`] преобразует АСД-выражение
//! [`ast::Expression`] в разрешённое семантическое [`ExpressionNode`].
//!
//! ## Разрешение идентификаторов
//!
//! Для [`ast::Expression::Variable`] поиск ведётся в следующем порядке:
//!
//! 1. `search_var` → [`ExpressionNode::Variable`]
//! 2. `search_model` → [`ExpressionNode::Model`]
//! 3. `search_cond` → [`ExpressionNode::Condition`]
//! 4. Иначе → [`Diagnostic`]-ошибка
//!
//! ## Вспомогательные функции
//!
//! - [`resolve_expr`] — разрешает унарное выражение и оборачивает в `Box`.
//! - [`resolve_bin`] — разрешает бинарное выражение (оба операнда).
//! - [`resolve_elems`] — разрешает все элементы вектора выражений.

use crate::diagnostics::{Diagnostic, Location};
use crate::parser::ast;
use crate::semantic::builtin::builtin_function;
use crate::semantic::type_inference::ast_type_to_node;
use crate::semantic::type_node::TypeNode;
use crate::semantic::{ExpressionNode, ModelNode, StatementNode, VariableNode};
use std::cell::RefCell;
use std::rc::Rc;

/// Строит разрешённое семантическое выражение из АСД-выражения.
///
/// Рекурсивно обходит дерево [`ast::Expression`], разрешая все идентификаторы
/// (переменные, функции, условия, модели) в контексте [`ModelNode`] и его
/// родителей (цепочка `upper`).
///
/// # Ошибки
///
/// Возвращает [`Diagnostic`], если:
/// - переменная, условие, функция или модель не найдены в области видимости;
/// - любое подвыражение не разрешается (ошибка пробрасывается).
pub fn construct_expression(
    expr: ast::Expression,
    params: Vec<(String, TypeNode)>,
    model: Rc<RefCell<ModelNode>>,
) -> Result<ExpressionNode, Diagnostic> {
    // Сторож глубины (фича 0129): рекурсия идёт по структуре текста, и на
    // глубине порядка трёхсот кончался стек — без диагностики, SIGABRT.
    let _depth = crate::semantic::validate::depth::enter(Some(expr.loc()))?;
    match expr {
        // ── Литералы ──────────────────────────────────────────────────────────
        ast::Expression::Number(_, n) => Ok(ExpressionNode::Number(n)),
        ast::Expression::Rational(_, s, neg) => Ok(ExpressionNode::Rational(s, neg)),
        ast::Expression::Bool(_, v) => Ok(ExpressionNode::Bool(v)),
        // Строки: извлекаем текст из Vec<StringLiteral>
        ast::Expression::String(lits) => Ok(ExpressionNode::String(
            lits.into_iter().map(|l| l.string).collect(),
        )),
        ast::Expression::Type(_, t) => Ok(ExpressionNode::Type(t)),
        ast::Expression::Address(_, addr, bit) => Ok(ExpressionNode::Address(addr, bit)),
        ast::Expression::List(_, pl) => Ok(ExpressionNode::List(pl)),

        // ── Разрешение идентификатора ──────────────────────────────────────────
        //
        // Приоритет поиска: переменная → модель → именованное условие.
        // Такой порядок гарантирует, что объявленная переменная затеняет одноимённую
        // модель или условие, что соответствует ожидаемой семантике языка Takt.
        ast::Expression::Variable(id) => {
            let name = &id.name;
            for (param_name, param_type) in params {
                if param_name == *name {
                    return Ok(ExpressionNode::Variable(Rc::new(RefCell::new(
                        VariableNode::Simple {
                            upper: Some(Rc::downgrade(&model)),
                            loc: Location::Implicit,
                            name: name.clone(),
                            ty: param_type.clone(),
                            expr: Default::default(),
                        },
                    ))));
                }
            }
            // 1. Переменная (включая порты и константы)
            if let Some(var) = model.borrow().search_var(name) {
                return Ok(ExpressionNode::Variable(Rc::new(RefCell::new(var))));
            }
            // 2. Именованная модель
            if let Some(m) = model.borrow().search_model(name) {
                return Ok(ExpressionNode::Model(m));
            }
            // 3. Именованное условие
            if let Some(cond) = model.borrow().search_cond(name) {
                return Ok(ExpressionNode::Condition(Rc::new(RefCell::new(cond))));
            }
            // 4. Вариант перечисления (NI6): имя является вариантом доступного перечисления
            if let Some((_enum_name, value)) = model.borrow().search_enum_variant(name) {
                return Ok(ExpressionNode::Number(value));
            }
            Err(Diagnostic::from(
                format!("Идентификатор '{}' не найден в области видимости", name).as_str(),
            )
            .with_code("SE-003"))
        }

        // ── Обращение к массиву ────────────────────────────────────────────────
        //
        // Для операций ArraySubscript и ArraySlice:
        //   1. Переменная должна существовать в области видимости.
        //   2. Тип переменной должен быть массивом (`TypeNode::Array`).
        //   3. Индексы/границы должны находиться в допустимом диапазоне.
        //
        // Если тип переменной ещё не выведен (`TypeNode::Inference`), структурная
        // проверка пропускается — она будет повторно вычислена после вывода типов.
        ast::Expression::ArraySubscript(_, id, idx_expr) => {
            let var = model.borrow().search_var(&id.name).ok_or_else(|| {
                Diagnostic::from(format!("Переменная '{}' не найдена", id.name).as_str())
                    .with_code("SE-003")
            })?;
            // Проверяем тип (для динамических индексов проверку границ пропускаем)
            match var_type(&var) {
                TypeNode::Array(size, _) => {
                    // Статическая проверка границ только для числовых литералов
                    if let ast::Expression::Number(_, n) = idx_expr.as_ref()
                        && (*n < 0 || *n >= size as i64)
                    {
                        return Err(Diagnostic::from(
                            format!(
                                "Индекс {} выходит за границы массива '{}' (размер {})",
                                n, id.name, size
                            )
                            .as_str(),
                        )
                        .with_code("SE-028"));
                    }
                }
                TypeNode::Inference => {} // тип ещё не выведен — пропускаем проверку
                _ => {
                    return Err(Diagnostic::from(
                        format!("Переменная '{}' не является массивом", id.name).as_str(),
                    )
                    .with_code("SE-030"));
                }
            }
            let resolved_idx = construct_expression(*idx_expr.clone(), vec![], model.clone())?;
            Ok(ExpressionNode::ArraySubscript(
                Rc::new(RefCell::new(var)),
                Box::new(resolved_idx),
            ))
        }
        ast::Expression::ArraySlice(_, id, start, end) => {
            let var = model.borrow().search_var(&id.name).ok_or_else(|| {
                Diagnostic::from(format!("Переменная '{}' не найдена", id.name).as_str())
                    .with_code("SE-003")
            })?;
            // Проверяем тип и границы среза (если тип известен)
            match var_type(&var) {
                TypeNode::Array(size, _) => {
                    check_slice_bounds(&id.name, size, start, end)?;
                }
                TypeNode::Inference => {} // тип ещё не выведен — пропускаем
                _ => {
                    return Err(Diagnostic::from(
                        format!("Переменная '{}' не является массивом", id.name).as_str(),
                    )
                    .with_code("SE-030"));
                }
            }
            Ok(ExpressionNode::ArraySlice(
                Rc::new(RefCell::new(var)),
                start,
                end,
            ))
        }

        // ── Структурные выражения ─────────────────────────────────────────────
        ast::Expression::Parenthesis(_, inner) => Ok(ExpressionNode::Parenthesis(resolve_expr(
            *inner,
            params.clone(),
            model,
        )?)),
        ast::Expression::BitAccess(_, inner, member) => Ok(ExpressionNode::BitAccess(
            resolve_expr(*inner, params.clone(), model)?,
            member,
        )),

        // Вызов функции: ищем в контексте, для неизвестных (встроенных) — создаём заглушку.
        //
        // Встроенные функции языка (`debug`, `S`, `S(Model)` и т.п.) не объявляются
        // пользователем явно, поэтому при отсутствии в контексте создаётся
        // анонимный узел-заглушка с именем функции. Это позволяет семантическому
        // дереву корректно строиться без предварительной регистрации встроенных символов.
        ast::Expression::Function(_, id, args) => {
            let func = if let Some(func) = model.borrow().search_func(&id.name) {
                func
            } else {
                Rc::new(RefCell::new(builtin_function(&id.name)?.clone()))
            };
            let resolved_args = resolve_elems(args, params.clone(), model)?;
            Ok(ExpressionNode::Function(func, resolved_args))
        }

        // Блок кода как выражение: разрешаем базовое выражение, Statement не трогаем.
        ast::Expression::CodeBlock(_, inner, stmt) => Ok(ExpressionNode::CodeBlock(
            resolve_expr(*inner, params.clone(), model)?,
            StatementNode::Unresolved(*stmt),
        )),

        // Именованный вызов: разрешаем базовое выражение, аргументы оставляем как есть.
        ast::Expression::NamedFunction(_, inner, named_args) => {
            Ok(ExpressionNode::NamedFunctionBox(
                resolve_expr(*inner, params.clone(), model)?,
                named_args,
            ))
        }

        // Приведение типа: разрешаем выражение.
        ast::Expression::Cast(_, inner, ty) => Ok(ExpressionNode::Cast(
            resolve_expr(*inner, params.clone(), model)?,
            ast_type_to_node(&ty),
        )),

        // ── Унарные операции ──────────────────────────────────────────────────
        ast::Expression::Not(_, e) => Ok(ExpressionNode::Not(resolve_expr(
            *e,
            params.clone(),
            model,
        )?)),
        ast::Expression::BitwiseNot(_, e) => Ok(ExpressionNode::BitwiseNot(resolve_expr(
            *e,
            params.clone(),
            model,
        )?)),
        ast::Expression::UnaryPlus(_, e) => Ok(ExpressionNode::UnaryPlus(resolve_expr(
            *e,
            params.clone(),
            model,
        )?)),
        ast::Expression::Negate(_, e) => Ok(ExpressionNode::Negate(resolve_expr(
            *e,
            params.clone(),
            model,
        )?)),

        // ── Бинарные операции ─────────────────────────────────────────────────
        ast::Expression::Power(_, l, r) => {
            let (l, r) = resolve_bin(*l, *r, params.clone(), model)?;
            Ok(ExpressionNode::Power(l, r))
        }
        ast::Expression::Multiply(_, l, r) => {
            let (l, r) = resolve_bin(*l, *r, params.clone(), model)?;
            Ok(ExpressionNode::Multiply(l, r))
        }
        ast::Expression::Divide(_, l, r) => {
            let (l, r) = resolve_bin(*l, *r, params.clone(), model)?;
            Ok(ExpressionNode::Divide(l, r))
        }
        ast::Expression::Modulo(_, l, r) => {
            let (l, r) = resolve_bin(*l, *r, params.clone(), model)?;
            Ok(ExpressionNode::Modulo(l, r))
        }
        ast::Expression::Add(_, l, r) => {
            let (l, r) = resolve_bin(*l, *r, params.clone(), model)?;
            Ok(ExpressionNode::Add(l, r))
        }
        ast::Expression::Subtract(_, l, r) => {
            let (l, r) = resolve_bin(*l, *r, params.clone(), model)?;
            Ok(ExpressionNode::Subtract(l, r))
        }
        ast::Expression::ShiftLeft(_, l, r) => {
            let (l, r) = resolve_bin(*l, *r, params.clone(), model)?;
            Ok(ExpressionNode::ShiftLeft(l, r))
        }
        ast::Expression::ShiftRight(_, l, r) => {
            let (l, r) = resolve_bin(*l, *r, params.clone(), model)?;
            Ok(ExpressionNode::ShiftRight(l, r))
        }
        ast::Expression::BitwiseAnd(_, l, r) => {
            let (l, r) = resolve_bin(*l, *r, params.clone(), model)?;
            Ok(ExpressionNode::BitwiseAnd(l, r))
        }
        ast::Expression::BitwiseXor(_, l, r) => {
            let (l, r) = resolve_bin(*l, *r, params.clone(), model)?;
            Ok(ExpressionNode::BitwiseXor(l, r))
        }
        ast::Expression::BitwiseOr(_, l, r) => {
            let (l, r) = resolve_bin(*l, *r, params.clone(), model)?;
            Ok(ExpressionNode::BitwiseOr(l, r))
        }
        ast::Expression::Less(_, l, r) => {
            let (l, r) = resolve_bin(*l, *r, params.clone(), model)?;
            Ok(ExpressionNode::Less(l, r))
        }
        ast::Expression::More(_, l, r) => {
            let (l, r) = resolve_bin(*l, *r, params.clone(), model)?;
            Ok(ExpressionNode::More(l, r))
        }
        ast::Expression::LessEqual(_, l, r) => {
            let (l, r) = resolve_bin(*l, *r, params.clone(), model)?;
            Ok(ExpressionNode::LessEqual(l, r))
        }
        ast::Expression::MoreEqual(_, l, r) => {
            let (l, r) = resolve_bin(*l, *r, params.clone(), model)?;
            Ok(ExpressionNode::MoreEqual(l, r))
        }
        ast::Expression::Equal(_, l, r) => {
            let (l, r) = resolve_bin(*l, *r, params.clone(), model)?;
            Ok(ExpressionNode::Equal(l, r))
        }
        ast::Expression::NotEqual(_, l, r) => {
            let (l, r) = resolve_bin(*l, *r, params.clone(), model)?;
            Ok(ExpressionNode::NotEqual(l, r))
        }
        ast::Expression::And(_, l, r) => {
            let (l, r) = resolve_bin(*l, *r, params.clone(), model)?;
            Ok(ExpressionNode::And(l, r))
        }
        ast::Expression::Or(_, l, r) => {
            let (l, r) = resolve_bin(*l, *r, params.clone(), model)?;
            Ok(ExpressionNode::Or(l, r))
        }

        // ── Прочие выражения ──────────────────────────────────────────────────
        ast::Expression::Assign(_, l, r) => {
            let (l, r) = resolve_bin(*l, *r, params.clone(), model)?;
            Ok(ExpressionNode::Assign(l, r))
        }
        ast::Expression::ConditionalOperator(_, cond, then_e, else_e) => {
            let cond = construct_expression(*cond, params.clone(), model.clone())?;
            let then_e = construct_expression(*then_e, params.clone(), model.clone())?;
            let else_e = construct_expression(*else_e, params.clone(), model)?;
            Ok(ExpressionNode::ConditionalOperator(
                Box::new(cond),
                Box::new(then_e),
                Box::new(else_e),
            ))
        }
        ast::Expression::Array(_, items) => Ok(ExpressionNode::Array(resolve_elems(
            items,
            params.clone(),
            model,
        )?)),
        ast::Expression::Initializer(_, items) => Ok(ExpressionNode::Initializer(resolve_elems(
            items,
            params.clone(),
            model,
        )?)),
    }
}

// ── Вспомогательные функции ────────────────────────────────────────────────────

/// Разрешает унарное подвыражение и оборачивает результат в `Box`.
#[inline]
fn resolve_expr(
    expr: ast::Expression,
    params: Vec<(String, TypeNode)>,
    model: Rc<RefCell<ModelNode>>,
) -> Result<Box<ExpressionNode>, Diagnostic> {
    construct_expression(expr, params.clone(), model).map(Box::new)
}

/// Разрешает оба операнда бинарного выражения.
///
/// Возвращает пару `(Box<Expression>, Box<Expression>)`.
#[inline]
fn resolve_bin(
    left: ast::Expression,
    right: ast::Expression,
    params: Vec<(String, TypeNode)>,
    model: Rc<RefCell<ModelNode>>,
) -> Result<(Box<ExpressionNode>, Box<ExpressionNode>), Diagnostic> {
    let l = construct_expression(left, params.clone(), model.clone()).map(Box::new)?;
    let r = construct_expression(right, params.clone(), model).map(Box::new)?;
    Ok((l, r))
}

/// Возвращает [`TypeNode`] переменной из её [`VariableNode`].
///
/// Если переменная не разрешена ([`VariableNode::Unresolved`]),
/// возвращает [`TypeNode::Inference`].
#[inline]
fn var_type(var: &VariableNode) -> TypeNode {
    match var {
        VariableNode::Simple { ty, .. }
        | VariableNode::Port { ty, .. }
        | VariableNode::Const { ty, .. } => ty.clone(),
        VariableNode::Unresolved => TypeNode::Inference,
    }
}

/// Проверяет допустимость границ среза массива.
///
/// # Правила проверки
///
/// - `start` (если задан): `0 ≤ start < size`
/// - `end` (если задан): `0 ≤ end ≤ size`
/// - Если заданы оба: `start ≤ end`
///
/// # Ошибки
///
/// Возвращает [`Diagnostic`], если любое условие нарушено.
fn check_slice_bounds(
    name: &str,
    size: u16,
    start: Option<i64>,
    end: Option<i64>,
) -> Result<(), Diagnostic> {
    if let Some(s) = start
        && (s < 0 || s >= size as i64)
    {
        return Err(Diagnostic::from(
            format!(
                "Начало среза {} выходит за границы массива '{}' (размер {})",
                s, name, size
            )
            .as_str(),
        )
        .with_code("SE-029"));
    }
    if let Some(e) = end
        && (e < 0 || e > size as i64)
    {
        return Err(Diagnostic::from(
            format!(
                "Конец среза {} выходит за границы массива '{}' (размер {})",
                e, name, size
            )
            .as_str(),
        )
        .with_code("SE-029"));
    }
    if let (Some(s), Some(e)) = (start, end)
        && s > e
    {
        return Err(Diagnostic::from(
            format!(
                "Начало среза {} больше конца {} для массива '{}'",
                s, e, name
            )
            .as_str(),
        )
        .with_code("SE-029"));
    }
    Ok(())
}

/// Разрешает все элементы вектора выражений.
///
/// При ошибке в любом элементе немедленно пробрасывает [`Diagnostic`].
#[inline]
fn resolve_elems(
    items: Vec<ast::Expression>,
    params: Vec<(String, TypeNode)>,
    model: Rc<RefCell<ModelNode>>,
) -> Result<Vec<ExpressionNode>, Diagnostic> {
    items
        .into_iter()
        .map(|e| construct_expression(e, params.clone(), model.clone()))
        .collect()
}

// ── Тесты ─────────────────────────────────────────────────────────────────────

#[cfg(test)]
mod tests;
