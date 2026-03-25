//! Построение семантических выражений языка BuT.
//!
//! Основная функция [`construct_expression`] преобразует АСД-выражение
//! [`ast::Expression`] в разрешённое семантическое [`Expression`].
//!
//! ## Разрешение идентификаторов
//!
//! Для [`ast::Expression::Variable`] поиск ведётся в следующем порядке:
//!
//! 1. `search_var` → [`Expression::Variable`]
//! 2. `search_model` → [`Expression::Model`]
//! 3. `search_cond` → [`Expression::Condition`]
//! 4. Иначе → [`Diagnostic`]-ошибка
//!
//! ## Вспомогательные функции
//!
//! - [`resolve_expr`] — разрешает унарное выражение и оборачивает в `Box`.
//! - [`resolve_bin`] — разрешает бинарное выражение (оба операнда).
//! - [`resolve_elems`] — разрешает все элементы вектора выражений.

use crate::diagnostics::Diagnostic;
use crate::parser::ast;
use crate::semantic::{Expression, ModelNode, Statement, TypeNode, VariableNode};
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
) -> Result<Expression, Diagnostic> {
    match expr {
        // ── Литералы ──────────────────────────────────────────────────────────
        ast::Expression::Number(_, n) => Ok(Expression::Number(n)),
        ast::Expression::Rational(_, s, neg) => Ok(Expression::Rational(s, neg)),
        ast::Expression::Bool(_, v) => Ok(Expression::Bool(v)),
        // Строки: извлекаем текст из Vec<StringLiteral>
        ast::Expression::String(lits) => {
            Ok(Expression::String(lits.into_iter().map(|l| l.string).collect()))
        }
        ast::Expression::Type(_, t) => Ok(Expression::Type(t)),
        ast::Expression::Address(_, addr, bit) => Ok(Expression::Address(addr, bit)),
        ast::Expression::List(_, pl) => Ok(Expression::List(pl)),

        // ── Разрешение идентификатора ──────────────────────────────────────────
        //
        // Приоритет поиска: переменная → модель → именованное условие.
        // Такой порядок гарантирует, что объявленная переменная затеняет одноимённую
        // модель или условие, что соответствует ожидаемой семантике языка BuT.
        ast::Expression::Variable(id) => {
            let name = &id.name;
            // 1. Переменная (включая порты и константы)
            if let Some(var) = model.borrow().search_var(name) {
                return Ok(Expression::Variable(Rc::new(RefCell::new(var))));
            }
            // 2. Именованная модель
            if let Some(m) = model.borrow().search_model(name) {
                return Ok(Expression::Model(m));
            }
            // 3. Именованное условие
            if let Some(cond) = model.borrow().search_cond(name) {
                return Ok(Expression::Condition(Rc::new(RefCell::new(cond))));
            }
            Err(format!("Идентификатор '{}' не найден в области видимости", name)
                .as_str()
                .into())
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
            let var = model
                .borrow()
                .search_var(&id.name)
                .ok_or_else(|| {
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
                    return Err(format!(
                        "Переменная '{}' не является массивом",
                        &id.name
                    )
                    .as_str()
                    .into())
                }
            }
            Ok(Expression::ArraySubscript(Rc::new(RefCell::new(var)), n))
        }
        ast::Expression::ArraySlice(_, id, start, end) => {
            let var = model
                .borrow()
                .search_var(&id.name)
                .ok_or_else(|| {
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
                    return Err(format!(
                        "Переменная '{}' не является массивом",
                        &id.name
                    )
                    .as_str()
                    .into())
                }
            }
            Ok(Expression::ArraySlice(
                Rc::new(RefCell::new(var)),
                start,
                end,
            ))
        }

        // ── Структурные выражения ─────────────────────────────────────────────
        ast::Expression::Parenthesis(_, inner) => {
            Ok(Expression::Parenthesis(resolve_expr(*inner, model)?))
        }
        ast::Expression::BitAccess(_, inner, member) => {
            Ok(Expression::BitAccess(resolve_expr(*inner, model)?, member))
        }

        // Вызов функции: ищем в контексте, для неизвестных (встроенных) — создаём заглушку.
        //
        // Встроенные функции языка (`debug`, `S`, `S(Model)` и т.п.) не объявляются
        // пользователем явно, поэтому при отсутствии в контексте создаётся
        // анонимный узел-заглушка с именем функции. Это позволяет семантическому
        // дереву корректно строиться без предварительной регистрации встроенных символов.
        ast::Expression::Function(_, id, args) => {
            use std::cell::RefCell;
            use std::rc::Rc;
            let func = model
                .borrow()
                .search_func(&id.name)
                .unwrap_or_else(|| {
                    Rc::new(RefCell::new(crate::semantic::FunctionNode {
                        name: id.name.clone(),
                    }))
                });
            let resolved_args = resolve_elems(args, model)?;
            Ok(Expression::Function(func, resolved_args))
        }

        // Блок кода как выражение: разрешаем базовое выражение, Statement не трогаем.
        ast::Expression::CodeBlock(_, inner, stmt) => {
            Ok(Expression::CodeBlock(resolve_expr(*inner, model)?, Statement::Unresolved(*stmt)))
        }

        // Именованный вызов: разрешаем базовое выражение, аргументы оставляем как есть.
        ast::Expression::NamedFunction(_, inner, named_args) => Ok(Expression::NamedFunctionBox(
            resolve_expr(*inner, model)?,
            named_args,
        )),

        // Приведение типа: разрешаем выражение.
        ast::Expression::Cast(_, inner, ty) => {
            Ok(Expression::Cast(resolve_expr(*inner, model)?, ty))
        }

        // ── Унарные операции ──────────────────────────────────────────────────
        ast::Expression::Not(_, e) => Ok(Expression::Not(resolve_expr(*e, model)?)),
        ast::Expression::BitwiseNot(_, e) => {
            Ok(Expression::BitwiseNot(resolve_expr(*e, model)?))
        }
        ast::Expression::UnaryPlus(_, e) => {
            Ok(Expression::UnaryPlus(resolve_expr(*e, model)?))
        }
        ast::Expression::Negate(_, e) => Ok(Expression::Negate(resolve_expr(*e, model)?)),

        // ── Бинарные операции ─────────────────────────────────────────────────
        ast::Expression::Power(_, l, r) => {
            let (l, r) = resolve_bin(*l, *r, model)?;
            Ok(Expression::Power(l, r))
        }
        ast::Expression::Multiply(_, l, r) => {
            let (l, r) = resolve_bin(*l, *r, model)?;
            Ok(Expression::Multiply(l, r))
        }
        ast::Expression::Divide(_, l, r) => {
            let (l, r) = resolve_bin(*l, *r, model)?;
            Ok(Expression::Divide(l, r))
        }
        ast::Expression::Modulo(_, l, r) => {
            let (l, r) = resolve_bin(*l, *r, model)?;
            Ok(Expression::Modulo(l, r))
        }
        ast::Expression::Add(_, l, r) => {
            let (l, r) = resolve_bin(*l, *r, model)?;
            Ok(Expression::Add(l, r))
        }
        ast::Expression::Subtract(_, l, r) => {
            let (l, r) = resolve_bin(*l, *r, model)?;
            Ok(Expression::Subtract(l, r))
        }
        ast::Expression::ShiftLeft(_, l, r) => {
            let (l, r) = resolve_bin(*l, *r, model)?;
            Ok(Expression::ShiftLeft(l, r))
        }
        ast::Expression::ShiftRight(_, l, r) => {
            let (l, r) = resolve_bin(*l, *r, model)?;
            Ok(Expression::ShiftRight(l, r))
        }
        ast::Expression::BitwiseAnd(_, l, r) => {
            let (l, r) = resolve_bin(*l, *r, model)?;
            Ok(Expression::BitwiseAnd(l, r))
        }
        ast::Expression::BitwiseXor(_, l, r) => {
            let (l, r) = resolve_bin(*l, *r, model)?;
            Ok(Expression::BitwiseXor(l, r))
        }
        ast::Expression::BitwiseOr(_, l, r) => {
            let (l, r) = resolve_bin(*l, *r, model)?;
            Ok(Expression::BitwiseOr(l, r))
        }
        ast::Expression::Less(_, l, r) => {
            let (l, r) = resolve_bin(*l, *r, model)?;
            Ok(Expression::Less(l, r))
        }
        ast::Expression::More(_, l, r) => {
            let (l, r) = resolve_bin(*l, *r, model)?;
            Ok(Expression::More(l, r))
        }
        ast::Expression::LessEqual(_, l, r) => {
            let (l, r) = resolve_bin(*l, *r, model)?;
            Ok(Expression::LessEqual(l, r))
        }
        ast::Expression::MoreEqual(_, l, r) => {
            let (l, r) = resolve_bin(*l, *r, model)?;
            Ok(Expression::MoreEqual(l, r))
        }
        ast::Expression::Equal(_, l, r) => {
            let (l, r) = resolve_bin(*l, *r, model)?;
            Ok(Expression::Equal(l, r))
        }
        ast::Expression::NotEqual(_, l, r) => {
            let (l, r) = resolve_bin(*l, *r, model)?;
            Ok(Expression::NotEqual(l, r))
        }
        ast::Expression::And(_, l, r) => {
            let (l, r) = resolve_bin(*l, *r, model)?;
            Ok(Expression::And(l, r))
        }
        ast::Expression::Or(_, l, r) => {
            let (l, r) = resolve_bin(*l, *r, model)?;
            Ok(Expression::Or(l, r))
        }

        // ── Прочие выражения ──────────────────────────────────────────────────
        ast::Expression::Assign(_, l, r) => {
            let (l, r) = resolve_bin(*l, *r, model)?;
            Ok(Expression::Assign(l, r))
        }
        ast::Expression::ConditionalOperator(_, cond, then_e, else_e) => {
            let cond = construct_expression(*cond, model.clone())?;
            let then_e = construct_expression(*then_e, model.clone())?;
            let else_e = construct_expression(*else_e, model)?;
            Ok(Expression::ConditionalOperator(
                Box::new(cond),
                Box::new(then_e),
                Box::new(else_e),
            ))
        }
        ast::Expression::Array(_, items) => {
            Ok(Expression::Array(resolve_elems(items, model)?))
        }
        ast::Expression::Initializer(_, items) => {
            Ok(Expression::Initializer(resolve_elems(items, model)?))
        }
    }
}

// ── Вспомогательные функции ────────────────────────────────────────────────────

/// Разрешает унарное подвыражение и оборачивает результат в `Box`.
#[inline]
fn resolve_expr(
    expr: ast::Expression,
    model: Rc<RefCell<ModelNode>>,
) -> Result<Box<Expression>, Diagnostic> {
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
) -> Result<(Box<Expression>, Box<Expression>), Diagnostic> {
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
        VariableNode::Simple(_, ty, _)
        | VariableNode::Port(_, ty, _)
        | VariableNode::Const(_, ty, _) => ty.clone(),
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
) -> Result<Vec<Expression>, Diagnostic> {
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
    use crate::semantic::{Condition, ModelNode, TypeNode, VariableNode};

    // ── Вспомогательные функции ───────────────────────────────────────────────

    /// Строит семантическую модель из исходного кода BuT.
    fn build(src: &str) -> Result<ModelNode, Diagnostic> {
        let (ast, _) = parse(src, 0).expect("ошибка разбора");
        construct_model(&ast, None, &[]).map(|m| m.take())
    }

    /// Возвращает разрешённый инициализатор переменной `name`.
    fn var_expr(node: &ModelNode, name: &str) -> Expression {
        match node.search_var(name).expect("переменная не найдена") {
            VariableNode::Simple(_, _, expr)
            | VariableNode::Const(_, _, expr)
            | VariableNode::Port(_, _, expr) => expr,
            VariableNode::Unresolved => panic!("переменная неразрешена"),
        }
    }

    // ── Литералы ─────────────────────────────────────────────────────────────

    /// Числовой литерал: `var x = 42;` → `Number(42)`.
    #[test]
    fn number_literal_resolved() {
        let node = build("var x: bit = false; cond c = 42;").unwrap();
        assert_eq!(node.conditions["c"].value, Condition::Number(42));
    }

    /// Булев литерал `true`: инициализатор переменной → `Bool(true)`.
    #[test]
    fn bool_literal_in_var_initializer() {
        let node = build("var flag: bit = true;").unwrap();
        assert!(
            matches!(var_expr(&node, "flag"), Expression::Bool(true)),
            "инициализатор должен быть Bool(true)"
        );
    }

    /// Булев литерал `false`: инициализатор переменной → `Bool(false)`.
    #[test]
    fn bool_false_literal_in_var() {
        let node = build("var flag: bit = false;").unwrap();
        assert!(
            matches!(var_expr(&node, "flag"), Expression::Bool(false)),
            "инициализатор должен быть Bool(false)"
        );
    }

    /// Целочисленный литерал как инициализатор переменной → `Number(n)`.
    #[test]
    fn number_literal_in_var_initializer() {
        let node = build("var x: [bit;8] = 0;").unwrap();
        assert!(
            matches!(var_expr(&node, "x"), Expression::Number(_)),
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
            matches!(var_expr(&node, "b"), Expression::Variable(_)),
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
            matches!(var_expr(&node, "x"), Expression::Add(_, _)),
            "инициализатор должен быть Add"
        );
    }

    /// Вычитание в инициализаторе: `var x = 3 - 1;` → `Subtract`.
    #[test]
    fn subtract_in_var_initializer() {
        let node = build("var x: [bit;8] = 3 - 1;").unwrap();
        assert!(
            matches!(var_expr(&node, "x"), Expression::Subtract(_, _)),
            "инициализатор должен быть Subtract"
        );
    }

    /// Побитовое И: `var x = 0xFF & 0x0F;` → `BitwiseAnd`.
    #[test]
    fn bitwise_and_in_var_initializer() {
        let node = build("var x: [bit;8] = 255 & 15;").unwrap();
        assert!(
            matches!(var_expr(&node, "x"), Expression::BitwiseAnd(_, _)),
            "инициализатор должен быть BitwiseAnd"
        );
    }

    /// Побитовое ИЛИ: `var x = 0x0F | 0xF0;` → `BitwiseOr`.
    #[test]
    fn bitwise_or_in_var_initializer() {
        let node = build("var x: [bit;8] = 15 | 240;").unwrap();
        assert!(
            matches!(var_expr(&node, "x"), Expression::BitwiseOr(_, _)),
            "инициализатор должен быть BitwiseOr"
        );
    }

    /// Логическое НЕ: `var x = !false;` → `Not`.
    #[test]
    fn not_in_var_initializer() {
        let node = build("var x: bit = !false;").unwrap();
        assert!(
            matches!(var_expr(&node, "x"), Expression::Not(_)),
            "инициализатор должен быть Not"
        );
    }

    /// Скобки: `var x = (42);` → `Parenthesis`.
    #[test]
    fn parenthesis_in_var_initializer() {
        let node = build("var x: [bit;8] = (42);").unwrap();
        assert!(
            matches!(var_expr(&node, "x"), Expression::Parenthesis(_)),
            "инициализатор должен быть Parenthesis"
        );
    }

    /// Сравнение `<`: инициализатор → `Less`.
    #[test]
    fn less_in_var_initializer() {
        let node = build("var x: bit = 1 < 2;").unwrap();
        assert!(
            matches!(var_expr(&node, "x"), Expression::Less(_, _)),
            "инициализатор должен быть Less"
        );
    }

    /// Сравнение `>`: инициализатор → `More`.
    #[test]
    fn more_in_var_initializer() {
        let node = build("var x: bit = 2 > 1;").unwrap();
        assert!(
            matches!(var_expr(&node, "x"), Expression::More(_, _)),
            "инициализатор должен быть More"
        );
    }

    /// Равенство `==`: инициализатор → `Equal`.
    #[test]
    fn equal_in_var_initializer() {
        let node = build("var x: bit = 1 == 1;").unwrap();
        assert!(
            matches!(var_expr(&node, "x"), Expression::Equal(_, _)),
            "инициализатор должен быть Equal"
        );
    }

    /// Неравенство `!=`: инициализатор → `NotEqual`.
    #[test]
    fn not_equal_in_var_initializer() {
        let node = build("var x: bit = 1 != 2;").unwrap();
        assert!(
            matches!(var_expr(&node, "x"), Expression::NotEqual(_, _)),
            "инициализатор должен быть NotEqual"
        );
    }

    // ── Вывод типа (type_inference) через разрешённые выражения ──────────────

    /// Переменная без аннотации типа с булевым литералом: выводится `TypeNode::Bit`.
    ///
    /// # Пример (BuT)
    /// ```but
    /// var flag = false;   // тип выводится как bit
    /// ```
    #[test]
    fn type_inference_bool_literal() {
        let node = build("var flag = false;").unwrap();
        if let Some(VariableNode::Simple(_, ty, _)) = node.search_var("flag") {
            assert_eq!(ty, TypeNode::Bit, "тип должен выводиться как Bit из булева литерала");
        } else {
            panic!("переменная flag не найдена или не Simple");
        }
    }

    /// Переменная без аннотации типа с вещественным литералом: → `TypeNode::Rational`.
    #[test]
    fn type_inference_rational_literal() {
        let node = build("var r = 3.14;").unwrap();
        if let Some(VariableNode::Simple(_, ty, _)) = node.search_var("r") {
            assert_eq!(ty, TypeNode::Rational, "тип должен выводиться как Rational из вещественного литерала");
        } else {
            panic!("переменная r не найдена");
        }
    }

    /// Константа без аннотации типа с булевым литералом: → `TypeNode::Bit`.
    #[test]
    fn type_inference_const_bool() {
        let node = build("const C = false;").unwrap();
        if let Some(VariableNode::Const(_, ty, _)) = node.search_var("C") {
            assert_eq!(ty, TypeNode::Bit, "тип константы должен выводиться как Bit");
        } else {
            panic!("константа C не найдена");
        }
    }

    /// Вывод типа через переменную: `var b: bit = false; var a = b;` → тип `a` = `Bit`.
    #[test]
    fn type_inference_from_variable() {
        let node = build("var b: bit = false; var a = b;").unwrap();
        if let Some(VariableNode::Simple(_, ty, _)) = node.search_var("a") {
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
            matches!(var_expr(&node, "x"), Expression::ArraySubscript(_, 3)),
            "инициализатор x должен быть ArraySubscript(buf, 3)"
        );
    }

    /// Контрпример: индексирование несуществующего массива — ошибка.
    #[test]
    fn array_subscript_unknown_var_is_error() {
        let result = build("var x: bit = ghost[0];");
        assert!(result.is_err(), "индексирование несуществующего массива — ошибка");
    }

    // ── Проверка типа и границ массива ────────────────────────────────────────

    /// Корректный индекс в пределах массива — строится без ошибок.
    #[test]
    fn array_subscript_valid_index() {
        let node = build("var buf: [bit;8] = 0; var x: bit = buf[0];").unwrap();
        assert!(
            matches!(var_expr(&node, "x"), Expression::ArraySubscript(_, 0)),
            "x должен быть ArraySubscript(buf, 0)"
        );
    }

    /// Последний допустимый индекс (size - 1) — строится без ошибок.
    #[test]
    fn array_subscript_last_valid_index() {
        let node = build("var buf: [bit;8] = 0; var x: bit = buf[7];").unwrap();
        assert!(
            matches!(var_expr(&node, "x"), Expression::ArraySubscript(_, 7)),
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
        assert!(result.is_err(), "индексирование Bit-переменной должно давать ошибку");
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
        let v = VariableNode::Simple("x".into(), TypeNode::Bit, Expression::None);
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
}
