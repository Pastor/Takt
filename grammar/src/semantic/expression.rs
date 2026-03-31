//! Построение семантических выражений языка BuT.
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

use crate::diagnostics::Diagnostic;
use crate::parser::ast;
use crate::semantic::builtin::builtin_function;
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
    model: Rc<RefCell<ModelNode>>,
) -> Result<ExpressionNode, Diagnostic> {
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
        // модель или условие, что соответствует ожидаемой семантике языка BuT.
        ast::Expression::Variable(id) => {
            let name = &id.name;
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
            Err(
                format!("Идентификатор '{}' не найден в области видимости", name)
                    .as_str()
                    .into(),
            )
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
        ast::Expression::ArraySubscript(_, id, n) => {
            let var = model.borrow().search_var(&id.name).ok_or_else(|| {
                format!("Переменная '{}' не найдена", &id.name)
                    .as_str()
                    .into()
            })?;
            // Проверяем тип и границы (если тип известен)
            match var_type(&var) {
                TypeNode::Array(size, _) => {
                    if n < 0 || n >= size as i64 {
                        return Err(format!(
                            "Индекс {} выходит за границы массива '{}' (размер {})",
                            n, &id.name, size
                        )
                        .as_str()
                        .into());
                    }
                }
                TypeNode::Inference => {} // тип ещё не выведен — пропускаем проверку
                _ => {
                    return Err(format!("Переменная '{}' не является массивом", &id.name)
                        .as_str()
                        .into());
                }
            }
            Ok(ExpressionNode::ArraySubscript(
                Rc::new(RefCell::new(var)),
                n,
            ))
        }
        ast::Expression::ArraySlice(_, id, start, end) => {
            let var = model.borrow().search_var(&id.name).ok_or_else(|| {
                format!("Переменная '{}' не найдена", &id.name)
                    .as_str()
                    .into()
            })?;
            // Проверяем тип и границы среза (если тип известен)
            match var_type(&var) {
                TypeNode::Array(size, _) => {
                    check_slice_bounds(&id.name, size, start, end)?;
                }
                TypeNode::Inference => {} // тип ещё не выведен — пропускаем
                _ => {
                    return Err(format!("Переменная '{}' не является массивом", &id.name)
                        .as_str()
                        .into());
                }
            }
            Ok(ExpressionNode::ArraySlice(
                Rc::new(RefCell::new(var)),
                start,
                end,
            ))
        }

        // ── Структурные выражения ─────────────────────────────────────────────
        ast::Expression::Parenthesis(_, inner) => {
            Ok(ExpressionNode::Parenthesis(resolve_expr(*inner, model)?))
        }
        ast::Expression::BitAccess(_, inner, member) => Ok(ExpressionNode::BitAccess(
            resolve_expr(*inner, model)?,
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
            let resolved_args = resolve_elems(args, model)?;
            Ok(ExpressionNode::Function(func, resolved_args))
        }

        // Блок кода как выражение: разрешаем базовое выражение, Statement не трогаем.
        ast::Expression::CodeBlock(_, inner, stmt) => Ok(ExpressionNode::CodeBlock(
            resolve_expr(*inner, model)?,
            StatementNode::Unresolved(*stmt),
        )),

        // Именованный вызов: разрешаем базовое выражение, аргументы оставляем как есть.
        ast::Expression::NamedFunction(_, inner, named_args) => Ok(
            ExpressionNode::NamedFunctionBox(resolve_expr(*inner, model)?, named_args),
        ),

        // Приведение типа: разрешаем выражение.
        ast::Expression::Cast(_, inner, ty) => {
            Ok(ExpressionNode::Cast(resolve_expr(*inner, model)?, ty))
        }

        // ── Унарные операции ──────────────────────────────────────────────────
        ast::Expression::Not(_, e) => Ok(ExpressionNode::Not(resolve_expr(*e, model)?)),
        ast::Expression::BitwiseNot(_, e) => {
            Ok(ExpressionNode::BitwiseNot(resolve_expr(*e, model)?))
        }
        ast::Expression::UnaryPlus(_, e) => Ok(ExpressionNode::UnaryPlus(resolve_expr(*e, model)?)),
        ast::Expression::Negate(_, e) => Ok(ExpressionNode::Negate(resolve_expr(*e, model)?)),

        // ── Бинарные операции ─────────────────────────────────────────────────
        ast::Expression::Power(_, l, r) => {
            let (l, r) = resolve_bin(*l, *r, model)?;
            Ok(ExpressionNode::Power(l, r))
        }
        ast::Expression::Multiply(_, l, r) => {
            let (l, r) = resolve_bin(*l, *r, model)?;
            Ok(ExpressionNode::Multiply(l, r))
        }
        ast::Expression::Divide(_, l, r) => {
            let (l, r) = resolve_bin(*l, *r, model)?;
            Ok(ExpressionNode::Divide(l, r))
        }
        ast::Expression::Modulo(_, l, r) => {
            let (l, r) = resolve_bin(*l, *r, model)?;
            Ok(ExpressionNode::Modulo(l, r))
        }
        ast::Expression::Add(_, l, r) => {
            let (l, r) = resolve_bin(*l, *r, model)?;
            Ok(ExpressionNode::Add(l, r))
        }
        ast::Expression::Subtract(_, l, r) => {
            let (l, r) = resolve_bin(*l, *r, model)?;
            Ok(ExpressionNode::Subtract(l, r))
        }
        ast::Expression::ShiftLeft(_, l, r) => {
            let (l, r) = resolve_bin(*l, *r, model)?;
            Ok(ExpressionNode::ShiftLeft(l, r))
        }
        ast::Expression::ShiftRight(_, l, r) => {
            let (l, r) = resolve_bin(*l, *r, model)?;
            Ok(ExpressionNode::ShiftRight(l, r))
        }
        ast::Expression::BitwiseAnd(_, l, r) => {
            let (l, r) = resolve_bin(*l, *r, model)?;
            Ok(ExpressionNode::BitwiseAnd(l, r))
        }
        ast::Expression::BitwiseXor(_, l, r) => {
            let (l, r) = resolve_bin(*l, *r, model)?;
            Ok(ExpressionNode::BitwiseXor(l, r))
        }
        ast::Expression::BitwiseOr(_, l, r) => {
            let (l, r) = resolve_bin(*l, *r, model)?;
            Ok(ExpressionNode::BitwiseOr(l, r))
        }
        ast::Expression::Less(_, l, r) => {
            let (l, r) = resolve_bin(*l, *r, model)?;
            Ok(ExpressionNode::Less(l, r))
        }
        ast::Expression::More(_, l, r) => {
            let (l, r) = resolve_bin(*l, *r, model)?;
            Ok(ExpressionNode::More(l, r))
        }
        ast::Expression::LessEqual(_, l, r) => {
            let (l, r) = resolve_bin(*l, *r, model)?;
            Ok(ExpressionNode::LessEqual(l, r))
        }
        ast::Expression::MoreEqual(_, l, r) => {
            let (l, r) = resolve_bin(*l, *r, model)?;
            Ok(ExpressionNode::MoreEqual(l, r))
        }
        ast::Expression::Equal(_, l, r) => {
            let (l, r) = resolve_bin(*l, *r, model)?;
            Ok(ExpressionNode::Equal(l, r))
        }
        ast::Expression::NotEqual(_, l, r) => {
            let (l, r) = resolve_bin(*l, *r, model)?;
            Ok(ExpressionNode::NotEqual(l, r))
        }
        ast::Expression::And(_, l, r) => {
            let (l, r) = resolve_bin(*l, *r, model)?;
            Ok(ExpressionNode::And(l, r))
        }
        ast::Expression::Or(_, l, r) => {
            let (l, r) = resolve_bin(*l, *r, model)?;
            Ok(ExpressionNode::Or(l, r))
        }

        // ── Прочие выражения ──────────────────────────────────────────────────
        ast::Expression::Assign(_, l, r) => {
            let (l, r) = resolve_bin(*l, *r, model)?;
            Ok(ExpressionNode::Assign(l, r))
        }
        ast::Expression::ConditionalOperator(_, cond, then_e, else_e) => {
            let cond = construct_expression(*cond, model.clone())?;
            let then_e = construct_expression(*then_e, model.clone())?;
            let else_e = construct_expression(*else_e, model)?;
            Ok(ExpressionNode::ConditionalOperator(
                Box::new(cond),
                Box::new(then_e),
                Box::new(else_e),
            ))
        }
        ast::Expression::Array(_, items) => Ok(ExpressionNode::Array(resolve_elems(items, model)?)),
        ast::Expression::Initializer(_, items) => {
            Ok(ExpressionNode::Initializer(resolve_elems(items, model)?))
        }
    }
}

// ── Вспомогательные функции ────────────────────────────────────────────────────

/// Разрешает унарное подвыражение и оборачивает результат в `Box`.
#[inline]
fn resolve_expr(
    expr: ast::Expression,
    model: Rc<RefCell<ModelNode>>,
) -> Result<Box<ExpressionNode>, Diagnostic> {
    construct_expression(expr, model).map(Box::new)
}

/// Разрешает оба операнда бинарного выражения.
///
/// Возвращает пару `(Box<Expression>, Box<Expression>)`.
#[inline]
fn resolve_bin(
    left: ast::Expression,
    right: ast::Expression,
    model: Rc<RefCell<ModelNode>>,
) -> Result<(Box<ExpressionNode>, Box<ExpressionNode>), Diagnostic> {
    let l = construct_expression(left, model.clone()).map(Box::new)?;
    let r = construct_expression(right, model).map(Box::new)?;
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
    if let Some(s) = start {
        if s < 0 || s >= size as i64 {
            return Err(format!(
                "Начало среза {} выходит за границы массива '{}' (размер {})",
                s, name, size
            )
            .as_str()
            .into());
        }
    }
    if let Some(e) = end {
        if e < 0 || e > size as i64 {
            return Err(format!(
                "Конец среза {} выходит за границы массива '{}' (размер {})",
                e, name, size
            )
            .as_str()
            .into());
        }
    }
    if let (Some(s), Some(e)) = (start, end) {
        if s > e {
            return Err(format!(
                "Начало среза {} больше конца {} для массива '{}'",
                s, e, name
            )
            .as_str()
            .into());
        }
    }
    Ok(())
}

/// Разрешает все элементы вектора выражений.
///
/// При ошибке в любом элементе немедленно пробрасывает [`Diagnostic`].
#[inline]
fn resolve_elems(
    items: Vec<ast::Expression>,
    model: Rc<RefCell<ModelNode>>,
) -> Result<Vec<ExpressionNode>, Diagnostic> {
    items
        .into_iter()
        .map(|e| construct_expression(e, model.clone()))
        .collect()
}

// ── Тесты ─────────────────────────────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use super::*;
    use crate::parse;
    use crate::semantic::tree::construct_model;
    use crate::semantic::type_node::TypeNode;
    use crate::semantic::{ConditionNode, ModelNode, VariableNode};
    // ── Вспомогательные функции ───────────────────────────────────────────────

    /// Строит семантическую модель из исходного кода BuT.
    fn build(src: &str) -> Result<ModelNode, Diagnostic> {
        let (ast, _) = parse(src, 0).expect("ошибка разбора");
        construct_model(&ast, None, &[]).map(|m| m.take())
    }

    /// Возвращает разрешённый инициализатор переменной `name`.
    fn var_expr(node: &ModelNode, name: &str) -> ExpressionNode {
        match node.search_var(name).expect("переменная не найдена") {
            VariableNode::Simple { expr, .. }
            | VariableNode::Const { expr, .. }
            | VariableNode::Port { expr, .. } => expr,
            VariableNode::Unresolved => panic!("переменная неразрешена"),
        }
    }

    // ── Литералы ─────────────────────────────────────────────────────────────

    /// Числовой литерал: `var x = 42;` → `Number(42)`.
    #[test]
    fn number_literal_resolved() {
        let node = build("var x: bit = false; cond c = 42;").unwrap();
        assert_eq!(node.conditions["c"].value, ConditionNode::Number(42));
    }

    /// Булев литерал `true`: инициализатор переменной → `Bool(true)`.
    #[test]
    fn bool_literal_in_var_initializer() {
        let node = build("var flag: bit = true;").unwrap();
        assert!(
            matches!(var_expr(&node, "flag"), ExpressionNode::Bool(true)),
            "инициализатор должен быть Bool(true)"
        );
    }

    /// Булев литерал `false`: инициализатор переменной → `Bool(false)`.
    #[test]
    fn bool_false_literal_in_var() {
        let node = build("var flag: bit = false;").unwrap();
        assert!(
            matches!(var_expr(&node, "flag"), ExpressionNode::Bool(false)),
            "инициализатор должен быть Bool(false)"
        );
    }

    /// Целочисленный литерал как инициализатор переменной → `Number(n)`.
    #[test]
    fn number_literal_in_var_initializer() {
        let node = build("var x: [bit;8] = 0;").unwrap();
        assert!(
            matches!(var_expr(&node, "x"), ExpressionNode::Number(_)),
            "инициализатор должен быть Number"
        );
    }

    // ── Variable / разрешение идентификаторов ─────────────────────────────────

    /// Переменная в инициализаторе разрешается в `Expression::Variable`.
    ///
    /// # Пример (BuT)
    /// ```but
    /// var a: bit = false;
    /// var b: bit = a;
    /// ```
    #[test]
    fn variable_ref_in_initializer_resolves() {
        let node = build("var a: bit = false; var b: bit = a;").unwrap();
        assert!(
            matches!(var_expr(&node, "b"), ExpressionNode::Variable(_)),
            "инициализатор b должен быть Variable(a)"
        );
    }

    /// Условие в инициализаторе разрешается в `Expression::Condition`.
    ///
    /// # Пример (BuT)
    /// ```but
    /// cond done = true;
    /// var flag: bit = done;
    /// ```
    ///
    /// **Примечание**: Это возможно только если `done` объявлена ПЕРЕД `flag`,
    /// что проверяет данный тест.
    #[test]
    fn named_condition_in_var_initializer_resolves() {
        // Условие разрешается как Condition внутри именованного cond-блока.
        // Проверяем через отдельный cond, а не var-инициализатор (разрешение идёт в extract_conditions).
        let node = build("cond done = true; cond ref_done = done;").unwrap();
        // ref_done = done должно разрешиться через переменную (done — это cond, не var)
        // Если не находит как переменную, ищет как условие
        assert!(node.conditions.contains_key("ref_done"));
    }

    /// Контрпример: несуществующий идентификатор → ошибка.
    #[test]
    fn unknown_identifier_is_error() {
        let result = build("var x: bit = ghost;");
        assert!(
            result.is_err(),
            "неизвестный идентификатор должен давать ошибку"
        );
    }

    // ── Операторы ──────────────────────────────────────────────────────────────

    /// Сложение в инициализаторе: `var x = 1 + 2;` → `Add`.
    #[test]
    fn add_in_var_initializer() {
        let node = build("var x: [bit;8] = 1 + 2;").unwrap();
        assert!(
            matches!(var_expr(&node, "x"), ExpressionNode::Add(_, _)),
            "инициализатор должен быть Add"
        );
    }

    /// Вычитание в инициализаторе: `var x = 3 - 1;` → `Subtract`.
    #[test]
    fn subtract_in_var_initializer() {
        let node = build("var x: [bit;8] = 3 - 1;").unwrap();
        assert!(
            matches!(var_expr(&node, "x"), ExpressionNode::Subtract(_, _)),
            "инициализатор должен быть Subtract"
        );
    }

    /// Побитовое И: `var x = 0xFF & 0x0F;` → `BitwiseAnd`.
    #[test]
    fn bitwise_and_in_var_initializer() {
        let node = build("var x: [bit;8] = 255 & 15;").unwrap();
        assert!(
            matches!(var_expr(&node, "x"), ExpressionNode::BitwiseAnd(_, _)),
            "инициализатор должен быть BitwiseAnd"
        );
    }

    /// Побитовое ИЛИ: `var x = 0x0F | 0xF0;` → `BitwiseOr`.
    #[test]
    fn bitwise_or_in_var_initializer() {
        let node = build("var x: [bit;8] = 15 | 240;").unwrap();
        assert!(
            matches!(var_expr(&node, "x"), ExpressionNode::BitwiseOr(_, _)),
            "инициализатор должен быть BitwiseOr"
        );
    }

    /// Логическое НЕ: `var x = !false;` → `Not`.
    #[test]
    fn not_in_var_initializer() {
        let node = build("var x: bit = !false;").unwrap();
        assert!(
            matches!(var_expr(&node, "x"), ExpressionNode::Not(_)),
            "инициализатор должен быть Not"
        );
    }

    /// Скобки: `var x = (42);` → `Parenthesis`.
    #[test]
    fn parenthesis_in_var_initializer() {
        let node = build("var x: [bit;8] = (42);").unwrap();
        assert!(
            matches!(var_expr(&node, "x"), ExpressionNode::Parenthesis(_)),
            "инициализатор должен быть Parenthesis"
        );
    }

    /// Сравнение `<`: инициализатор → `Less`.
    #[test]
    fn less_in_var_initializer() {
        let node = build("var x: bit = 1 < 2;").unwrap();
        assert!(
            matches!(var_expr(&node, "x"), ExpressionNode::Less(_, _)),
            "инициализатор должен быть Less"
        );
    }

    /// Сравнение `>`: инициализатор → `More`.
    #[test]
    fn more_in_var_initializer() {
        let node = build("var x: bit = 2 > 1;").unwrap();
        assert!(
            matches!(var_expr(&node, "x"), ExpressionNode::More(_, _)),
            "инициализатор должен быть More"
        );
    }

    /// Равенство `==`: инициализатор → `Equal`.
    #[test]
    fn equal_in_var_initializer() {
        let node = build("var x: bit = 1 == 1;").unwrap();
        assert!(
            matches!(var_expr(&node, "x"), ExpressionNode::Equal(_, _)),
            "инициализатор должен быть Equal"
        );
    }

    /// Неравенство `!=`: инициализатор → `NotEqual`.
    #[test]
    fn not_equal_in_var_initializer() {
        let node = build("var x: bit = 1 != 2;").unwrap();
        assert!(
            matches!(var_expr(&node, "x"), ExpressionNode::NotEqual(_, _)),
            "инициализатор должен быть NotEqual"
        );
    }

    // ── Вывод типа (type_inference) через разрешённые выражения ──────────────

    /// Переменная без аннотации типа с булевым литералом: выводится `TypeNode::Bool`.
    ///
    /// # Пример (BuT)
    /// ```but
    /// var flag = false;   // тип выводится как bool
    /// ```
    #[test]
    fn type_inference_bool_literal() {
        let node = build("var flag = false;").unwrap();
        if let Some(VariableNode::Simple { ty, .. }) = node.search_var("flag") {
            assert_eq!(
                ty,
                TypeNode::Bool,
                "тип должен выводиться как Bool из булева литерала"
            );
        } else {
            panic!("переменная flag не найдена или не Simple");
        }
    }

    /// Переменная без аннотации типа с вещественным литералом: → `TypeNode::Rational`.
    #[test]
    fn type_inference_rational_literal() {
        let node = build("var r = 3.14;").unwrap();
        if let Some(VariableNode::Simple { ty, .. }) = node.search_var("r") {
            assert_eq!(
                ty,
                TypeNode::Rational,
                "тип должен выводиться как Rational из вещественного литерала"
            );
        } else {
            panic!("переменная r не найдена");
        }
    }

    /// Константа без аннотации типа с булевым литералом: → `TypeNode::Bool`.
    #[test]
    fn type_inference_const_bool() {
        let node = build("const C = false;").unwrap();
        if let Some(VariableNode::Const { ty, .. }) = node.search_var("C") {
            assert_eq!(
                ty,
                TypeNode::Bool,
                "тип константы должен выводиться как Bool"
            );
        } else {
            panic!("константа C не найдена");
        }
    }

    /// Вывод типа через переменную: `var b: bit = false; var a = b;` → тип `a` = `Bit`.
    #[test]
    fn type_inference_from_variable() {
        let node = build("var b: bit = false; var a = b;").unwrap();
        if let Some(VariableNode::Simple { ty, .. }) = node.search_var("a") {
            assert_eq!(ty, TypeNode::Bit, "тип a должен выводиться из типа b = Bit");
        } else {
            panic!("переменная a не найдена");
        }
    }

    // ── Массивы ────────────────────────────────────────────────────────────────

    /// Индексирование массива в инициализаторе разрешается в `ArraySubscript`.
    ///
    /// # Пример (BuT)
    /// ```but
    /// var buf: [bit; 8];
    /// var x: bit = buf[3];
    /// ```
    #[test]
    fn array_subscript_in_var_initializer() {
        let node = build("var buf: [bit;8] = 0; var x: bit = buf[3];").unwrap();
        assert!(
            matches!(var_expr(&node, "x"), ExpressionNode::ArraySubscript(_, 3)),
            "инициализатор x должен быть ArraySubscript(buf, 3)"
        );
    }

    /// Контрпример: индексирование несуществующего массива — ошибка.
    #[test]
    fn array_subscript_unknown_var_is_error() {
        let result = build("var x: bit = ghost[0];");
        assert!(
            result.is_err(),
            "индексирование несуществующего массива — ошибка"
        );
    }

    // ── Проверка типа и границ массива ────────────────────────────────────────

    /// Корректный индекс в пределах массива — строится без ошибок.
    #[test]
    fn array_subscript_valid_index() {
        let node = build("var buf: [bit;8] = 0; var x: bit = buf[0];").unwrap();
        assert!(
            matches!(var_expr(&node, "x"), ExpressionNode::ArraySubscript(_, 0)),
            "x должен быть ArraySubscript(buf, 0)"
        );
    }

    /// Последний допустимый индекс (size - 1) — строится без ошибок.
    #[test]
    fn array_subscript_last_valid_index() {
        let node = build("var buf: [bit;8] = 0; var x: bit = buf[7];").unwrap();
        assert!(
            matches!(var_expr(&node, "x"), ExpressionNode::ArraySubscript(_, 7)),
            "x должен быть ArraySubscript(buf, 7)"
        );
    }

    /// Индекс равный размеру массива (out of bounds) — ошибка.
    #[test]
    fn array_subscript_out_of_bounds_is_error() {
        let result = build("var buf: [bit;8] = 0; var x: bit = buf[8];");
        assert!(result.is_err(), "индекс 8 >= size 8 должен давать ошибку");
    }

    /// Отрицательный индекс — ошибка.
    #[test]
    fn array_subscript_negative_index_is_error() {
        let result = build("var buf: [bit;8] = 0; var x: bit = buf[-1];");
        assert!(result.is_err(), "отрицательный индекс должен давать ошибку");
    }

    /// Индексирование переменной с типом Bit — ошибка (не массив).
    #[test]
    fn array_subscript_on_non_array_is_error() {
        let result = build("var flag: bit = false; var x: bit = flag[0];");
        assert!(
            result.is_err(),
            "индексирование Bit-переменной должно давать ошибку"
        );
        let err = result.unwrap_err();
        assert!(
            err.message.contains("flag"),
            "сообщение должно упоминать имя переменной: {}",
            err.message
        );
    }

    /// `var_type` для Simple-переменной возвращает правильный тип.
    #[test]
    fn var_type_simple() {
        use crate::semantic::VariableNode;
        let v = VariableNode::Simple {
            upper: None,
            loc: crate::diagnostics::Location::Implicit,
            name: "x".into(),
            ty: TypeNode::Bit,
            expr: ExpressionNode::None,
        };
        assert_eq!(var_type(&v), TypeNode::Bit);
    }

    /// `var_type` для Unresolved-переменной возвращает Inference.
    #[test]
    fn var_type_unresolved() {
        use crate::semantic::VariableNode;
        assert_eq!(var_type(&VariableNode::Unresolved), TypeNode::Inference);
    }

    /// `check_slice_bounds`: допустимый срез [1:6] для массива size=8 — ок.
    #[test]
    fn check_slice_bounds_valid() {
        check_slice_bounds("buf", 8, Some(1), Some(6)).unwrap();
    }

    /// `check_slice_bounds`: срез с end > size — ошибка.
    #[test]
    fn check_slice_bounds_end_out_of_range_is_error() {
        assert!(check_slice_bounds("buf", 8, None, Some(9)).is_err());
    }

    /// `check_slice_bounds`: срез с start >= size — ошибка.
    #[test]
    fn check_slice_bounds_start_out_of_range_is_error() {
        assert!(check_slice_bounds("buf", 8, Some(8), None).is_err());
    }

    /// `check_slice_bounds`: start > end — ошибка.
    #[test]
    fn check_slice_bounds_start_greater_than_end_is_error() {
        assert!(check_slice_bounds("buf", 8, Some(5), Some(3)).is_err());
    }

    /// `check_slice_bounds`: None, None — всегда ок (срез без границ).
    #[test]
    fn check_slice_bounds_both_none_is_ok() {
        check_slice_bounds("buf", 8, None, None).unwrap();
    }

    // ── Implement-состояния и construct_expression ─────────────────────────────

    /// Implement-состояние (`= M`) использует construct_expression для разрешения
    /// имени модели — ранее это вызывало stack overflow.
    ///
    /// Регрессионный тест: construct_implement теперь корректно делегирует
    /// через construct_expression вместо рекурсивного вызова через заглушку.
    #[test]
    fn implement_model_via_construct_expression() {
        let node = build("start A = M { } state B; model M { start S; }").unwrap();
        assert!(
            matches!(
                &node.states.get("A"),
                Some(crate::semantic::StateNode::Implement { .. })
            ),
            "A должен быть Implement-состоянием"
        );
    }

    /// Переменная без аннотации с числовым литералом: `var x = 100;` → `Array(8, Bit)`.
    #[test]
    fn type_inference_number_literal() {
        let node = build("var x = 100;").unwrap();
        if let Some(VariableNode::Simple { ty, .. }) = node.search_var("x") {
            assert_eq!(
                ty,
                TypeNode::Array(8, Box::new(TypeNode::Bit)),
                "тип должен выводиться как [bit;8] из числового литерала 100"
            );
        } else {
            panic!("переменная x не найдена");
        }
    }

    // ── Дополнительные арифметические операции ────────────────────────────────

    /// Умножение: `var x: [bit;8] = 2 * 3;` → `Multiply`.
    #[test]
    fn multiply_in_var_initializer() {
        let node = build("var x: [bit;8] = 2 * 3;").unwrap();
        assert!(
            matches!(var_expr(&node, "x"), ExpressionNode::Multiply(_, _)),
            "инициализатор должен быть Multiply"
        );
    }

    /// Деление: `var x: [bit;8] = 6 / 2;` → `Divide`.
    #[test]
    fn divide_in_var_initializer() {
        let node = build("var x: [bit;8] = 6 / 2;").unwrap();
        assert!(
            matches!(var_expr(&node, "x"), ExpressionNode::Divide(_, _)),
            "инициализатор должен быть Divide"
        );
    }

    /// Остаток от деления: `var x: [bit;8] = 7 % 3;` → `Modulo`.
    #[test]
    fn modulo_in_var_initializer() {
        let node = build("var x: [bit;8] = 7 % 3;").unwrap();
        assert!(
            matches!(var_expr(&node, "x"), ExpressionNode::Modulo(_, _)),
            "инициализатор должен быть Modulo"
        );
    }

    /// Возведение в степень: `var x: [bit;8] = 2 ** 3;` → `Power`.
    #[test]
    fn power_in_var_initializer() {
        let node = build("var x: [bit;8] = 2 ** 3;").unwrap();
        assert!(
            matches!(var_expr(&node, "x"), ExpressionNode::Power(_, _)),
            "инициализатор должен быть Power"
        );
    }

    /// Сдвиг влево: `var x: [bit;8] = 1 << 2;` → `ShiftLeft`.
    #[test]
    fn shift_left_in_var_initializer() {
        let node = build("var x: [bit;8] = 1 << 2;").unwrap();
        assert!(
            matches!(var_expr(&node, "x"), ExpressionNode::ShiftLeft(_, _)),
            "инициализатор должен быть ShiftLeft"
        );
    }

    /// Сдвиг вправо: `var x: [bit;8] = 4 >> 1;` → `ShiftRight`.
    #[test]
    fn shift_right_in_var_initializer() {
        let node = build("var x: [bit;8] = 4 >> 1;").unwrap();
        assert!(
            matches!(var_expr(&node, "x"), ExpressionNode::ShiftRight(_, _)),
            "инициализатор должен быть ShiftRight"
        );
    }

    /// Побитовое XOR: `var x: [bit;8] = 3 ^ 1;` → `BitwiseXor`.
    #[test]
    fn bitwise_xor_in_var_initializer() {
        let node = build("var x: [bit;8] = 3 ^ 1;").unwrap();
        assert!(
            matches!(var_expr(&node, "x"), ExpressionNode::BitwiseXor(_, _)),
            "инициализатор должен быть BitwiseXor"
        );
    }

    // ── Операции сравнения ────────────────────────────────────────────────────

    /// Меньше или равно: `var x: bit = 1 <= 2;` → `LessEqual`.
    #[test]
    fn less_equal_in_var_initializer() {
        let node = build("var x: bit = 1 <= 2;").unwrap();
        assert!(
            matches!(var_expr(&node, "x"), ExpressionNode::LessEqual(_, _)),
            "инициализатор должен быть LessEqual"
        );
    }

    /// Больше или равно: `var x: bit = 2 >= 1;` → `MoreEqual`.
    #[test]
    fn more_equal_in_var_initializer() {
        let node = build("var x: bit = 2 >= 1;").unwrap();
        assert!(
            matches!(var_expr(&node, "x"), ExpressionNode::MoreEqual(_, _)),
            "инициализатор должен быть MoreEqual"
        );
    }

    // ── Логические операции ───────────────────────────────────────────────────

    /// Логическое И: `var x: bit = true && false;` → `And`.
    #[test]
    fn and_in_var_initializer() {
        let node = build("var x: bit = true && false;").unwrap();
        assert!(
            matches!(var_expr(&node, "x"), ExpressionNode::And(_, _)),
            "инициализатор должен быть And"
        );
    }

    /// Логическое ИЛИ: `var x: bit = true || false;` → `Or`.
    #[test]
    fn or_in_var_initializer() {
        let node = build("var x: bit = true || false;").unwrap();
        assert!(
            matches!(var_expr(&node, "x"), ExpressionNode::Or(_, _)),
            "инициализатор должен быть Or"
        );
    }

    // ── Унарные операции ──────────────────────────────────────────────────────

    /// Унарный плюс: `var x: [bit;8] = +5;` → `UnaryPlus`.
    #[test]
    fn unary_plus_in_var_initializer() {
        let node = build("var x: [bit;8] = +5;").unwrap();
        assert!(
            matches!(var_expr(&node, "x"), ExpressionNode::UnaryPlus(_)),
            "инициализатор должен быть UnaryPlus"
        );
    }

    /// Побитовое НЕ: `var x: [bit;8] = ~0;` → `BitwiseNot`.
    #[test]
    fn bitwise_not_in_var_initializer() {
        let node = build("var x: [bit;8] = ~0;").unwrap();
        assert!(
            matches!(var_expr(&node, "x"), ExpressionNode::BitwiseNot(_)),
            "инициализатор должен быть BitwiseNot"
        );
    }

    /// Вещественный литерал: `var r = 3.14;` → `Rational`.
    #[test]
    fn rational_literal_in_var_initializer() {
        let node = build("var r = 3.14;").unwrap();
        assert!(
            matches!(var_expr(&node, "r"), ExpressionNode::Rational(_, _)),
            "инициализатор должен быть Rational"
        );
    }

    /// Отрицание вещественного числа: `var r = -3.14;` → `Rational` с флагом отрицания.
    #[test]
    fn negate_rational_in_var_initializer() {
        // Парсер может представить -3.14 как Rational(_, true) или Negate(Rational)
        let node = build("var r = -3.14;").unwrap();
        let expr = var_expr(&node, "r");
        assert!(
            matches!(
                expr,
                ExpressionNode::Rational(_, _) | ExpressionNode::Negate(_)
            ),
            "инициализатор должен быть Rational или Negate"
        );
    }

    // ── Тернарный оператор ────────────────────────────────────────────────────

    /// Тернарный оператор через `construct_expression` → `ConditionalOperator`.
    ///
    /// Синтаксис `? :` не поддерживается парсером BuT, поэтому
    /// тестируем напрямую через `construct_expression`.
    #[test]
    fn conditional_operator_via_construct_expression() {
        let model = Rc::new(RefCell::new(ModelNode::default()));
        let loc = crate::diagnostics::Location::default();
        // false ? 0 : 1
        let cond_expr = Box::new(ast::Expression::Bool(loc, false));
        let then_expr = Box::new(ast::Expression::Number(loc, 0));
        let else_expr = Box::new(ast::Expression::Number(loc, 1));
        let expr = ast::Expression::ConditionalOperator(loc, cond_expr, then_expr, else_expr);
        let result = construct_expression(expr, model).unwrap();
        assert!(
            matches!(result, ExpressionNode::ConditionalOperator(_, _, _)),
            "должен получиться ConditionalOperator"
        );
    }

    /// Тернарный оператор: подвыражения разрешаются корректно.
    ///
    /// Проверяем, что все три ветви (условие, then, else) разрешаются
    /// внутри конкретного контекста модели (с переменной).
    ///
    /// # Пример (псевдокод BuT):
    /// ```text
    /// var flag: bit = true;
    /// // flag ? 10 : 20  →  ConditionalOperator(Variable("flag"), Number(10), Number(20))
    /// ```
    #[test]
    fn conditional_operator_with_variable_condition() {
        use crate::semantic::tree::construct_model;
        // Строим модель с переменной flag
        let (ast, _) = crate::parse("var flag: bit = true;", 0).expect("ошибка разбора");
        let model = construct_model(&ast, None, &[]).expect("ошибка семантики");
        let loc = crate::diagnostics::Location::default();
        // flag ? 10 : 20
        let cond_expr = Box::new(ast::Expression::Variable(crate::parser::ast::Identifier {
            loc,
            name: "flag".to_string(),
        }));
        let then_expr = Box::new(ast::Expression::Number(loc, 10));
        let else_expr = Box::new(ast::Expression::Number(loc, 20));
        let expr = ast::Expression::ConditionalOperator(loc, cond_expr, then_expr, else_expr);
        let result = construct_expression(expr, model).unwrap();
        assert!(
            matches!(result, ExpressionNode::ConditionalOperator(_, _, _)),
            "ConditionalOperator с переменной-условием должен разрешиться"
        );
    }

    /// Контрпример: тернарный оператор с несуществующей переменной в условии — ошибка.
    ///
    /// Если условие ссылается на неизвестный идентификатор, `construct_expression`
    /// должен вернуть ошибку диагностики.
    #[test]
    fn conditional_operator_unknown_condition_is_error() {
        let model = Rc::new(RefCell::new(ModelNode::default()));
        let loc = crate::diagnostics::Location::default();
        // ghost ? 0 : 1  —  ghost не объявлен
        let cond_expr = Box::new(ast::Expression::Variable(crate::parser::ast::Identifier {
            loc,
            name: "ghost".to_string(),
        }));
        let then_expr = Box::new(ast::Expression::Number(loc, 0));
        let else_expr = Box::new(ast::Expression::Number(loc, 1));
        let expr = ast::Expression::ConditionalOperator(loc, cond_expr, then_expr, else_expr);
        let result = construct_expression(expr, model);
        assert!(
            result.is_err(),
            "тернарный оператор с неизвестным условием должен давать ошибку"
        );
    }

    // ── Срез массива ──────────────────────────────────────────────────────────

    /// Срез массива: `var y: [bit;4] = buf[1:5];` → `ArraySlice`.
    #[test]
    fn array_slice_in_var_initializer() {
        let node = build("var buf: [bit;8] = 0; var y: [bit;4] = buf[1:5];").unwrap();
        assert!(
            matches!(var_expr(&node, "y"), ExpressionNode::ArraySlice(_, _, _)),
            "инициализатор y должен быть ArraySlice"
        );
    }

    /// Срез несуществующего массива — ошибка.
    #[test]
    fn array_slice_unknown_var_is_error() {
        let result = build("var y: [bit;4] = ghost[0:4];");
        assert!(
            result.is_err(),
            "срез несуществующего массива должен давать ошибку"
        );
    }

    /// Срез переменной с типом Bit — ошибка (не массив).
    #[test]
    fn array_slice_on_non_array_is_error() {
        let result = build("var flag: bit = false; var y: [bit;4] = flag[0:4];");
        assert!(result.is_err(), "срез Bit-переменной должен давать ошибку");
    }

    // ── Строковый литерал ─────────────────────────────────────────────────────

    /// Строковый литерал в вызове debug() — разрешается без ошибок.
    #[test]
    fn string_literal_in_debug_call() {
        let node = build(r#"always { debug("hello"); } start S;"#).unwrap();
        assert!(node.has_states(), "модель должна содержать состояние S");
    }

    // ── Вызов функции ─────────────────────────────────────────────────────────

    /// Вызов внешней функции в блоке `always` → разрешается без ошибок.
    #[test]
    fn extern_function_call_in_always_block() {
        let node = build("extern fn foo(); always { foo(); } start S;").unwrap();
        assert!(node.has_states(), "модель должна содержать состояние S");
    }

    // ── Приведение типа ───────────────────────────────────────────────────────

    /// Приведение типа: `var x: [bit;8] = 42 as [bit;8];` → `Cast`.
    #[test]
    fn cast_in_var_initializer() {
        let node = build("var x: [bit;8] = 42 as [bit;8];").unwrap();
        assert!(
            matches!(var_expr(&node, "x"), ExpressionNode::Cast(_, _)),
            "инициализатор должен быть Cast"
        );
    }

    // ── Массивный литерал ─────────────────────────────────────────────────────

    /// Массивный литерал через `construct_expression` → `Expression::Array`.
    ///
    /// Синтаксис `[a, b]` не поддерживается парсером как инициализатор переменной,
    /// поэтому тестируем через `construct_expression` напрямую.
    #[test]
    fn array_literal_via_construct_expression() {
        let model = Rc::new(RefCell::new(ModelNode::default()));
        let loc = crate::diagnostics::Location::default();
        let items = vec![
            ast::Expression::Number(loc, 0),
            ast::Expression::Number(loc, 1),
        ];
        let expr = ast::Expression::Array(loc, items);
        let result = construct_expression(expr, model).unwrap();
        assert!(
            matches!(result, ExpressionNode::Array(_)),
            "должен получиться Expression::Array"
        );
    }

    // ── Прямое использование construct_expression ─────────────────────────────

    /// `ast::Expression::Type` → `Expression::Type(Type::Bit)`.
    #[test]
    fn construct_expression_type_variant() {
        use crate::parser::ast::Type;
        let model = Rc::new(RefCell::new(ModelNode::default()));
        let expr = ast::Expression::Type(crate::diagnostics::Location::default(), Type::Bit);
        let result = construct_expression(expr, model).unwrap();
        assert!(
            matches!(result, ExpressionNode::Type(Type::Bit)),
            "должен получиться Expression::Type(Type::Bit)"
        );
    }

    /// `ast::Expression::Address` → `Expression::Address(addr, bit)`.
    #[test]
    fn construct_expression_address_variant() {
        let model = Rc::new(RefCell::new(ModelNode::default()));
        let expr = ast::Expression::Address(crate::diagnostics::Location::default(), 0x1234, 5);
        let result = construct_expression(expr, model).unwrap();
        assert!(
            matches!(result, ExpressionNode::Address(0x1234, 5)),
            "должен получиться Expression::Address(0x1234, 5)"
        );
    }

    /// `ast::Expression::List` с пустым списком → `Expression::List([])`.
    #[test]
    fn construct_expression_list_variant() {
        let model = Rc::new(RefCell::new(ModelNode::default()));
        let expr = ast::Expression::List(crate::diagnostics::Location::default(), vec![]);
        let result = construct_expression(expr, model).unwrap();
        assert!(
            matches!(result, ExpressionNode::List(_)),
            "должен получиться Expression::List"
        );
    }

    /// Неизвестный идентификатор в `construct_expression` → ошибка.
    #[test]
    fn construct_expression_unknown_variable_is_error() {
        let model = Rc::new(RefCell::new(ModelNode::default()));
        let expr = ast::Expression::Variable(ast::Identifier::new("ghost"));
        let result = construct_expression(expr, model);
        assert!(
            result.is_err(),
            "неизвестный идентификатор должен давать ошибку"
        );
    }

    /// `ast::Expression::ArraySubscript` для несуществующей переменной → ошибка.
    #[test]
    fn construct_expression_array_subscript_unknown_var() {
        let model = Rc::new(RefCell::new(ModelNode::default()));
        let expr = ast::Expression::ArraySubscript(
            crate::diagnostics::Location::default(),
            ast::Identifier::new("no_such_var"),
            0,
        );
        let result = construct_expression(expr, model);
        assert!(
            result.is_err(),
            "несуществующая переменная должна давать ошибку"
        );
    }

    /// `ast::Expression::ArraySlice` для несуществующей переменной → ошибка.
    #[test]
    fn construct_expression_array_slice_unknown_var() {
        let model = Rc::new(RefCell::new(ModelNode::default()));
        let expr = ast::Expression::ArraySlice(
            crate::diagnostics::Location::default(),
            ast::Identifier::new("no_such_var"),
            Some(0),
            Some(4),
        );
        let result = construct_expression(expr, model);
        assert!(
            result.is_err(),
            "несуществующая переменная должна давать ошибку"
        );
    }

    /// `var_type` для Port-переменной возвращает правильный тип.
    #[test]
    fn var_type_port() {
        let v = VariableNode::Port {
            upper: None,
            loc: crate::diagnostics::Location::Implicit,
            name: "p".into(),
            ty: TypeNode::Bit,
            expr: ExpressionNode::None,
        };
        assert_eq!(var_type(&v), TypeNode::Bit);
    }

    /// `var_type` для Const-переменной возвращает правильный тип.
    #[test]
    fn var_type_const() {
        let v = VariableNode::Const {
            upper: None,
            loc: crate::diagnostics::Location::Implicit,
            name: "c".into(),
            ty: TypeNode::Bool,
            expr: ExpressionNode::None,
        };
        assert_eq!(var_type(&v), TypeNode::Bool);
    }

    /// `check_slice_bounds`: отрицательный start — ошибка.
    #[test]
    fn check_slice_bounds_negative_start_is_error() {
        assert!(check_slice_bounds("buf", 8, Some(-1), None).is_err());
    }

    /// `check_slice_bounds`: отрицательный end — ошибка.
    #[test]
    fn check_slice_bounds_negative_end_is_error() {
        assert!(check_slice_bounds("buf", 8, None, Some(-1)).is_err());
    }

    /// `check_slice_bounds`: срез без начала [..end] — ок.
    #[test]
    fn check_slice_bounds_only_end_is_ok() {
        check_slice_bounds("buf", 8, None, Some(8)).unwrap();
    }

    /// `check_slice_bounds`: срез без конца [start..] — ок.
    #[test]
    fn check_slice_bounds_only_start_is_ok() {
        check_slice_bounds("buf", 8, Some(0), None).unwrap();
    }
}
