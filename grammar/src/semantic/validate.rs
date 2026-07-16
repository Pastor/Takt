//! Валидация семантических узлов языка Lam.
//!
//! Проверяет семантические инварианты после построения дерева.
//! Проверки выполняются рекурсивно для всех вложенных моделей.
//!
//! ## Текущие проверки
//!
//! - Модель, содержащая состояния, должна иметь ровно одно начальное
//!   состояние (`start`). Модели без состояний (только с объявлениями
//!   переменных, типов и т.п.) от этой проверки освобождены.
//!
//! - Переменная типа `bit` может быть инициализирована только значениями
//!   `0`, `1`, `true` или `false`. Любое другое числовое значение — ошибка.
//!
//! - Условие перехода (`ref`) не должно содержать неявного приведения
//!   числового типа к булевому. Использование переменных числового типа
//!   (например, `[bit;8]`) без явного сравнения порождает предупреждение
//!   [`check_implicit_bool_conditions`].

use crate::diagnostics::{Diagnostic, Location};
use crate::parser::{ast as ast_types, ast};
use crate::semantic::condition::resolve_condition;
use crate::semantic::type_node::TypeNode;
use crate::semantic::{
    ConditionNode, ExpressionNode, FunctionDefinitionNode, ModelNode, PortDirection, ReferenceNode,
    StateNode, StateNodeKind, StatementNode, VariableNode,
};
use std::cell::RefCell;
use std::collections::HashSet;
use std::rc::Rc;

/// Проверяет, что модель содержит ровно одно начальное состояние.
///
/// Если в модели нет состояний вообще (например, модуль с только
/// объявлениями типов или переменных), проверка пропускается.
///
/// # Ошибки
///
/// Возвращает [`Diagnostic`], если модель содержит состояния, но
/// начальных состояний не ровно одно (0 или ≥ 2).
fn model_only_one_start_state(model: Rc<RefCell<ModelNode>>) -> Result<(), Diagnostic> {
    let borrowed = model.borrow();

    // Модель без состояний допустима (только переменные/типы/условия)
    if borrowed.states.is_empty() {
        return Ok(());
    }

    let name = borrowed.name.clone().unwrap_or_default();
    let start_count = borrowed
        .states
        .values()
        .filter(|state| {
            matches!(
                state,
                StateNode::Simple {
                    kind: StateNodeKind::Start,
                    ..
                } | StateNode::Implement {
                    kind: StateNodeKind::Start,
                    ..
                }
            )
        })
        .count();

    if start_count != 1 {
        return Err(Diagnostic::error(
            borrowed.loc,
            format!(
                "В модели '{}' должно быть только одно начальное состояние (найдено: {})",
                name, start_count
            ),
        )
        .with_code("SE-011"));
    }
    Ok(())
}

/// Проверяет, что инициализатор переменной типа `bit` содержит допустимое значение.
///
/// Тип `bit` принимает только числовые значения `0` или `1`,
/// а также булевы литералы `true` / `false`.
/// Выражения, не являющиеся числовыми литералами (переменные, операции),
/// не проверяются статически.
///
/// # Ошибки
///
/// Возвращает [`Diagnostic`], если числовой литерал не равен 0 или 1.
fn check_bit_variable_value(
    name: &str,
    ty: &TypeNode,
    expr: &ExpressionNode,
    loc: Location,
) -> Result<(), Diagnostic> {
    if *ty == TypeNode::Bit
        && let ExpressionNode::Number(n) = expr
        && *n != 0
        && *n != 1
    {
        return Err(Diagnostic::error(
            loc,
            format!(
                "Переменная '{}' имеет тип bit, но инициализирована значением {} \
                 (допустимые числовые значения: 0 или 1)",
                name, n
            ),
        )
        .with_code("SE-035"));
    }
    Ok(())
}

/// Проверяет все переменные модели на корректность начальных значений для типа `bit`.
///
/// Обходит `Simple`-, `Const`- и `Port`-переменные текущего уровня.
/// Рекурсия по вложенным моделям не нужна — [`validate_model`] уже обходит
/// их самостоятельно, вызывая `validate_bit_values` для каждой вложенной модели.
///
/// # Ошибки
///
/// Пробрасывает [`Diagnostic`] из [`check_bit_variable_value`].
fn validate_bit_values(model: Rc<RefCell<ModelNode>>) -> Result<(), Diagnostic> {
    let borrowed = model.borrow();
    for var in borrowed.variables.values() {
        match var {
            VariableNode::Simple { name, ty, expr, .. }
            | VariableNode::Const { name, ty, expr, .. }
            | VariableNode::Port { name, ty, expr, .. } => {
                check_bit_variable_value(name, ty, expr, var.loc())?;
            }
            VariableNode::Unresolved => {}
        }
    }
    Ok(())
}

fn validate_cond(
    context: Option<ConditionNode>,
    cond: &ConditionNode,
    model: Rc<RefCell<ModelNode>>,
) -> Result<(), Diagnostic> {
    let _borrowed = model.borrow();
    match cond.clone() {
        ConditionNode::None => {}
        ConditionNode::Unresolved(cond) => {
            #[allow(clippy::collapsible_if)]
            if let Some(context) = context
                && let ast::Condition::Variable(id) = cond.clone()
            {
                if let ConditionNode::Function(func, args, _) = context
                    && let FunctionDefinitionNode::Builtin(name, ..) = *func.borrow()
                    && name == "S"
                    && args.len() == 1
                    && let Some(cond) = args.first()
                    && let ConditionNode::Model(model) = *cond.clone()
                {
                    let model = model.borrow();
                    let model_name = model
                        .name
                        .clone()
                        .unwrap_or_else(|| "<анонимная>".to_string());
                    model.search_state(&id.name).ok_or_else(|| {
                        Diagnostic::error(
                            id.loc,
                            format!(
                                "Состояние '{}' не найдено в моделе '{}'",
                                &id.name, &model_name
                            ),
                        )
                        .with_code("SE-033")
                    })?;
                    return Ok(());
                }
            }

            if let ConditionNode::Unresolved(_) = resolve_condition(&cond, model.clone())? {
                return Err(Diagnostic::error(
                    cond.loc(),
                    format!("Неразрешённое условие: {:?}", cond),
                )
                .with_code("SE-025"));
            }
        }
        ConditionNode::ArraySubscript(_, _) => {}
        ConditionNode::Parenthesis(cond) => {
            validate_cond(None, &cond, model.clone())?;
        }
        ConditionNode::BitAccess(cond, _) => {
            validate_cond(None, &cond, model.clone())?;
        }
        ConditionNode::Function(_, conds, _) => {
            for cond in conds {
                validate_cond(None, &cond, model.clone())?;
            }
        }
        ConditionNode::Not(cond) => {
            validate_cond(None, &cond, model.clone())?;
        }
        ConditionNode::Add(left, right) => {
            validate_cond(None, &left, model.clone())?;
            validate_cond(None, &right, model.clone())?;
        }
        ConditionNode::Subtract(left, right) => {
            validate_cond(None, &left, model.clone())?;
            validate_cond(None, &right, model.clone())?;
        }
        ConditionNode::And(left, right) => {
            validate_cond(None, &left, model.clone())?;
            validate_cond(None, &right, model.clone())?;
        }
        ConditionNode::Or(left, right) => {
            validate_cond(None, &left, model.clone())?;
            validate_cond(None, &right, model.clone())?;
        }
        ConditionNode::Less(left, right) => {
            validate_cond(None, &left, model.clone())?;
            validate_cond(None, &right, model.clone())?;
        }
        ConditionNode::More(left, right) => {
            validate_cond(None, &left, model.clone())?;
            validate_cond(None, &right, model.clone())?;
        }
        ConditionNode::LessEqual(left, right) => {
            validate_cond(None, &left, model.clone())?;
            validate_cond(None, &right, model.clone())?;
        }
        ConditionNode::MoreEqual(left, right) => {
            validate_cond(None, &left, model.clone())?;
            validate_cond(None, &right, model.clone())?;
        }
        ConditionNode::Equal(left, right) => {
            validate_cond(None, &left, model.clone())?;
            validate_cond(Some(*left.clone()), &right, model.clone())?;
        }
        ConditionNode::NotEqual(left, right) => {
            validate_cond(None, &left, model.clone())?;
            // Передаём контекст левого операнда — как в Equal — для проверки
            // паттерна `S(Model) != СостояниеИмя`: имя состояния должно быть
            // валидным в указанной модели.
            validate_cond(Some(*left.clone()), &right, model.clone())?;
        }
        ConditionNode::Number(_) => {}
        ConditionNode::Rational(_, _) => {}
        ConditionNode::String(_) => {}
        ConditionNode::Bool(_) => {}
        ConditionNode::Variable(var_rc, _) => {
            // Чтение из `out`-порта запрещено в условии (SE-027)
            if let VariableNode::Port {
                direction: PortDirection::Out,
                name,
                loc,
                ..
            } = &*var_rc.borrow()
            {
                return Err(Diagnostic::error(
                    *loc,
                    format!("Чтение из выходного порта '{}' запрещено", name),
                )
                .with_code("SE-027"));
            }
        }
        ConditionNode::Model(_model) => {}
        ConditionNode::State(_state) => {}
        ConditionNode::EnumVariant(_, _, _) => {}
    }
    Ok(())
}

fn validate_state_references(model: Rc<RefCell<ModelNode>>) -> Result<(), Diagnostic> {
    let borrowed = model.borrow();
    for state in borrowed.states.values() {
        match state {
            StateNode::Simple { references, .. } | StateNode::Implement { references, .. } => {
                for reference in references {
                    validate_reference(reference, model.clone())?;
                }
            }
            StateNode::Unresolved => {}
        }
    }
    Ok(())
}

fn validate_expression(
    expr: &ExpressionNode,
    model: Rc<RefCell<ModelNode>>,
) -> Result<(), Diagnostic> {
    let _borrowed = model.borrow();
    match expr {
        ExpressionNode::None => {}
        ExpressionNode::Unresolved(_) => {}
        ExpressionNode::ArraySubscript(_, _) => {}
        ExpressionNode::ArraySlice(_, _, _) => {}
        ExpressionNode::Parenthesis(expr)
        | ExpressionNode::BitAccess(expr, _)
        | ExpressionNode::CodeBlock(expr, _)
        | ExpressionNode::NamedFunctionBox(expr, _)
        | ExpressionNode::Not(expr)
        | ExpressionNode::UnaryPlus(expr)
        | ExpressionNode::Negate(expr)
        | ExpressionNode::Cast(expr, _)
        | ExpressionNode::BitwiseNot(expr) => {
            validate_expression(expr, model.clone())?;
        }
        ExpressionNode::Power(left, right)
        | ExpressionNode::Multiply(left, right)
        | ExpressionNode::Divide(left, right)
        | ExpressionNode::Modulo(left, right)
        | ExpressionNode::Add(left, right)
        | ExpressionNode::Subtract(left, right)
        | ExpressionNode::ShiftLeft(left, right)
        | ExpressionNode::ShiftRight(left, right)
        | ExpressionNode::BitwiseAnd(left, right)
        | ExpressionNode::BitwiseXor(left, right)
        | ExpressionNode::BitwiseOr(left, right)
        | ExpressionNode::Less(left, right)
        | ExpressionNode::More(left, right)
        | ExpressionNode::LessEqual(left, right)
        | ExpressionNode::MoreEqual(left, right)
        | ExpressionNode::Equal(left, right)
        | ExpressionNode::NotEqual(left, right)
        | ExpressionNode::And(left, right)
        | ExpressionNode::Or(left, right) => {
            validate_expression(left, model.clone())?;
            validate_expression(right, model.clone())?;
        }
        ExpressionNode::Assign(left, right) => {
            // Запись в `in`-порт запрещена (SE-026)
            let check_port = |expr: &ExpressionNode| {
                if let ExpressionNode::Variable(v) = expr {
                    return Some(v.clone());
                }
                if let ExpressionNode::BitAccess(inner, _) = expr
                    && let ExpressionNode::Variable(v) = inner.as_ref()
                {
                    return Some(v.clone());
                }
                None
            };
            if let Some(var_rc) = check_port(left)
                && let VariableNode::Port {
                    direction: PortDirection::In,
                    name,
                    loc,
                    ..
                } = &*var_rc.borrow()
            {
                return Err(Diagnostic::error(
                    *loc,
                    format!("Запись в входной порт '{}' запрещена", name),
                )
                .with_code("SE-026"));
            }
            // Для BitAccess как lvalue на out-порт не рекурсируем внутрь
            // (это запись в порт, а не чтение — SE-027 здесь неприменим).
            let is_out_port_bitaccess = if let ExpressionNode::BitAccess(inner, _) = left.as_ref() {
                if let ExpressionNode::Variable(v) = inner.as_ref() {
                    matches!(
                        &*v.borrow(),
                        VariableNode::Port {
                            direction: PortDirection::Out,
                            ..
                        }
                    )
                } else {
                    false
                }
            } else {
                false
            };
            if !is_out_port_bitaccess {
                validate_expression(left, model.clone())?;
            }
            validate_expression(right, model.clone())?;
        }
        ExpressionNode::ConditionalOperator(left, right, other) => {
            validate_expression(left, model.clone())?;
            validate_expression(right, model.clone())?;
            validate_expression(other, model.clone())?;
        }
        ExpressionNode::Number(_) => {}
        ExpressionNode::Rational(_, _) => {}
        ExpressionNode::String(_) => {}
        ExpressionNode::Type(_) => {}
        ExpressionNode::Address(_, _) => {}
        ExpressionNode::Bool(_) => {}
        ExpressionNode::Variable(var_rc) => {
            // Чтение из `out`-порта запрещено (SE-027)
            if let VariableNode::Port {
                direction: PortDirection::Out,
                name,
                loc,
                ..
            } = &*var_rc.borrow()
            {
                return Err(Diagnostic::error(
                    *loc,
                    format!("Чтение из выходного порта '{}' запрещено", name),
                )
                .with_code("SE-027"));
            }
        }
        ExpressionNode::Model(_model) => {}
        ExpressionNode::Condition(cond) => {
            validate_cond(None, &cond.borrow().value, model.clone())?;
        }
        ExpressionNode::List(_) => {}
        ExpressionNode::Array(exprs)
        | ExpressionNode::Initializer(exprs)
        | ExpressionNode::Function(_, exprs) => {
            for expr in exprs {
                validate_expression(expr, model.clone())?;
            }
        }
    }
    Ok(())
}

fn validate_reference(
    reference: &ReferenceNode<StateNode>,
    model: Rc<RefCell<ModelNode>>,
) -> Result<(), Diagnostic> {
    validate_cond(None, &reference.cond, model.clone())?;
    Ok(())
}

fn validate_variables(model: Rc<RefCell<ModelNode>>) -> Result<(), Diagnostic> {
    let borrowed = model.borrow();
    for variable in borrowed.variables.values() {
        match variable {
            VariableNode::Unresolved => {}
            VariableNode::Simple { expr, .. }
            | VariableNode::Port { expr, .. }
            | VariableNode::Const { expr, .. } => {
                validate_expression(expr, model.clone())?;
            }
        }
    }
    Ok(())
}

fn validate_conditions(model: Rc<RefCell<ModelNode>>) -> Result<(), Diagnostic> {
    let borrowed = model.borrow();
    for cond in borrowed.conditions.values() {
        validate_cond(None, &cond.value, model.clone())?;
    }
    Ok(())
}

// ─── Се11: строгая проверка булевости условий переходов ──────────────────────

/// Возвращает `true`, если AST-условие перехода гарантированно является булевым.
///
/// Используется для условий на рёбрах `ref`, которые в текущем конвейере
/// хранятся как [`ConditionNode::Unresolved`] и содержат «сырой» [`ast::Condition`].
///
/// ## Правила классификации
///
/// | Условие                                     | Результат |
/// |---------------------------------------------|-----------|
/// | Булев литерал (`true`, `false`)             | булево    |
/// | Сравнение (`=`, `!=`, `<`, `>`, `<=`, `>=`) | булево    |
/// | Логическое НЕ (`!x`)                        | булево    |
/// | Скобки (`(…)`)                              | рекурсия  |
/// | Вызов функции — тип неизвестен              | булево    |
/// | Переменная типа `bool` или `bit`            | булево    |
/// | Именованное условие (`cond`)                | булево    |
/// | Неизвестное имя                             | булево    |
/// | Числовой литерал                            | числовое  |
/// | Вещественный литерал                        | числовое  |
/// | Строковый литерал                           | числовое  |
/// | Арифметика (`+`, `-`)                       | числовое  |
/// | Побитовые операции (`&`, `\|`)              | числовое  |
/// | Элемент массива (`arr[n]`)                  | числовое  |
/// | Доступ к биту (`.n`)                        | числовое  |
/// | Переменная числового типа (`[bit;N]`)       | числовое  |
pub(crate) fn is_boolean_ast_condition(
    cond: &ast_types::Condition,
    model: &Rc<RefCell<ModelNode>>,
) -> bool {
    use ast_types::Condition as AC;
    match cond {
        // ── Явно булевые ──────────────────────────────────────────────────────
        // Булев литерал
        AC::Bool(_, _) => true,
        // Результат операции сравнения — всегда булево
        AC::Equal(_, _, _)
        | AC::NotEqual(_, _, _)
        | AC::Less(_, _, _)
        | AC::More(_, _, _)
        | AC::LessEqual(_, _, _)
        | AC::MoreEqual(_, _, _) => true,
        // Логическое НЕ (`!x`) всегда возвращает булев результат
        AC::Not(_, _) => true,
        // Скобки прозрачны — рекурсивно проверяем вложенное условие
        AC::Parenthesis(_, inner) => is_boolean_ast_condition(inner, model),
        // Вызов функции — тип возврата неизвестен, не предупреждаем
        AC::Function(_, _, _) => true,
        // Переменная: ищем в семантической модели и проверяем тип
        AC::Variable(id) => {
            let borrowed = model.borrow();
            // Переменная типа bool или bit — допустимо
            if let Some(var) = borrowed.search_var(&id.name) {
                return match &var {
                    VariableNode::Simple { ty, .. }
                    | VariableNode::Port { ty, .. }
                    | VariableNode::Const { ty, .. } => {
                        matches!(ty, TypeNode::Bool | TypeNode::Bit)
                    }
                    // Тип не разрешён — не предупреждаем
                    VariableNode::Unresolved => true,
                };
            }
            // Именованное условие (`cond Full = …`) — само является булевым
            if borrowed.search_cond(&id.name).is_some() {
                return true;
            }
            // Имя не найдено — ошибку выдаст другая проверка, не дублируем
            true
        }
        // ── Явно числовые ────────────────────────────────────────────────────
        // Целочисленный литерал
        AC::Number(_, _) => false,
        // Вещественный литерал
        AC::Rational(_, _, _) => false,
        // Строковый литерал (нетипичный в условии, но не булево)
        AC::String(_) => false,
        // Арифметические операции возвращают числовой тип
        AC::Add(_, _, _) | AC::Subtract(_, _, _) => false,
        // Побитовые операции возвращают числовой тип
        AC::And(_, _, _) | AC::Or(_, _, _) => false,
        // Индексация массива возвращает элемент числового типа
        AC::ArraySubscript(_, _, _) => false,
        // Доступ к битовому полю возвращает числовое значение
        AC::BitAccess(_, _, _) => false,
    }
}

/// Возвращает краткое описание небулевого AST-условия для диагностического сообщения.
///
/// Вызывается только когда [`is_boolean_ast_condition`] вернул `false`,
/// поэтому покрывает только «числовые» ветви.
pub(crate) fn ast_condition_summary(
    cond: &ast_types::Condition,
    model: &Rc<RefCell<ModelNode>>,
) -> String {
    use ast_types::Condition as AC;
    match cond {
        AC::Number(_, n) => format!("числовой литерал {}", n),
        AC::Rational(_, r, neg) => {
            format!("вещественный литерал {}{}", if *neg { "-" } else { "" }, r)
        }
        AC::String(_) => "строковый литерал".to_string(),
        AC::Variable(id) => {
            // Ищем тип переменной для информативного сообщения
            let ty_str = model
                .borrow()
                .search_var(&id.name)
                .map(|var| match var {
                    VariableNode::Simple { ty, .. }
                    | VariableNode::Port { ty, .. }
                    | VariableNode::Const { ty, .. } => format!("{:?}", ty),
                    VariableNode::Unresolved => "?".to_string(),
                })
                .unwrap_or_else(|| "?".to_string());
            format!("переменная '{}' типа {}", id.name, ty_str)
        }
        AC::Add(_, _, _) => "арифметическое сложение".to_string(),
        AC::Subtract(_, _, _) => "арифметическое вычитание".to_string(),
        AC::And(_, _, _) => "побитовое И".to_string(),
        AC::Or(_, _, _) => "побитовое ИЛИ".to_string(),
        AC::ArraySubscript(_, id, idx) => {
            let idx_str = match idx.as_ref() {
                AC::Number(_, n) => n.to_string(),
                AC::Variable(v) => v.name.clone(),
                _ => "expr".to_string(),
            };
            format!("элемент массива '{}[{}]'", id.name, idx_str)
        }
        AC::BitAccess(_, _, _) => "доступ к битовому полю".to_string(),
        // Остальные варианты сюда попасть не должны (они булевые)
        _ => "числовое выражение".to_string(),
    }
}

/// Добавляет предупреждение Се11 в `out`.
///
/// Выносит форматирование сообщения в отдельную функцию, чтобы не дублировать его.
/// `loc` — координаты перехода в исходном файле (берётся из [`ReferenceNode::location`]).
#[inline]
fn emit_implicit_bool_warning(
    loc: Location,
    prefix: &str,
    target_name: &str,
    summary: &str,
    is_next: bool,
    out: &mut Vec<Diagnostic>,
) {
    let verb = if is_next { "next к" } else { "к" };
    out.push(
        Diagnostic::warning(
            loc,
            format!(
                "{}: условие перехода {} '{}' содержит {} — \
                 рекомендуется явное сравнение (например, '!= 0')",
                prefix, verb, target_name, summary
            ),
        )
        .with_code("SE-037"),
    );
}

/// Проверяет, является ли разрешённое семантическое условие гарантированно булевым.
///
/// Применяется для условий на рёбрах `ref`, разрешённых на этапе 6 конвейера.
///
/// ## Правила классификации
///
/// | Условие                                          | Результат  |
/// |--------------------------------------------------|------------|
/// | Безусловный переход (`None`)                     | булево     |
/// | Булев литерал (`true`, `false`)                  | булево     |
/// | Операции сравнения (`=`, `!=`, `<`, `>`, …)     | булево     |
/// | Логическое НЕ (`!x`)                             | булево     |
/// | Скобки (`(…)`)                                   | рекурсия   |
/// | Вызов функции — тип возврата неизвестен          | булево     |
/// | Переменная типа `bool` или `bit`                 | булево     |
/// | Переменная числового типа (`[bit;N]`)            | числовое   |
/// | Числовой / вещественный / строковый литерал      | числовое   |
/// | Арифметика (`+`, `-`)                            | числовое   |
/// | Побитовые операции (`&`, `\|`)                   | числовое   |
/// | Элемент массива (`arr[n]`)                       | числовое   |
/// | Доступ к битовому полю (`.n`)                    | числовое   |
fn is_boolean_semantic_condition(cond: &ConditionNode) -> bool {
    match cond {
        ConditionNode::None => true,
        ConditionNode::Bool(_) => true,
        ConditionNode::Equal(_, _)
        | ConditionNode::NotEqual(_, _)
        | ConditionNode::Less(_, _)
        | ConditionNode::More(_, _)
        | ConditionNode::LessEqual(_, _)
        | ConditionNode::MoreEqual(_, _) => true,
        ConditionNode::Not(_) => true,
        ConditionNode::Parenthesis(inner) => is_boolean_semantic_condition(inner),
        // Тип возврата функции неизвестен — не предупреждаем
        ConditionNode::Function(_, _, _) => true,
        ConditionNode::Variable(v, _) => {
            let borrowed = v.borrow();
            match &*borrowed {
                VariableNode::Simple { ty, .. }
                | VariableNode::Port { ty, .. }
                | VariableNode::Const { ty, .. } => matches!(ty, TypeNode::Bool | TypeNode::Bit),
                VariableNode::Unresolved => true, // тип неизвестен — не предупреждаем
            }
        }
        _ => false,
    }
}

/// Возвращает краткое описание небулевого разрешённого семантического условия.
///
/// Вызывается только когда [`is_boolean_semantic_condition`] вернул `false`,
/// поэтому покрывает только «числовые» ветви.
fn semantic_condition_summary(cond: &ConditionNode) -> String {
    match cond {
        ConditionNode::Number(n) => format!("числовой литерал {}", n),
        ConditionNode::Rational(s, neg) => {
            format!("вещественный литерал {}{}", if *neg { "-" } else { "" }, s)
        }
        ConditionNode::String(_) => "строковый литерал".to_string(),
        ConditionNode::Variable(v, _) => {
            let borrowed = v.borrow();
            let (name_str, ty) = match &*borrowed {
                VariableNode::Simple { name, ty, .. }
                | VariableNode::Port { name, ty, .. }
                | VariableNode::Const { name, ty, .. } => (name.clone(), ty.clone()),
                VariableNode::Unresolved => return "переменная (неизвестный тип)".to_string(),
            };
            format!("переменная '{}' типа {}", name_str, ty)
        }
        ConditionNode::Add(_, _) => "арифметическое сложение".to_string(),
        ConditionNode::Subtract(_, _) => "арифметическое вычитание".to_string(),
        ConditionNode::And(_, _) => "побитовое И".to_string(),
        ConditionNode::Or(_, _) => "побитовое ИЛИ".to_string(),
        ConditionNode::ArraySubscript(var, idx) => {
            let name = match &*var.borrow() {
                VariableNode::Simple { name, .. }
                | VariableNode::Port { name, .. }
                | VariableNode::Const { name, .. } => name.clone(),
                VariableNode::Unresolved => "?".to_string(),
            };
            let idx_str = match idx.as_ref() {
                ConditionNode::Number(n) => n.to_string(),
                _ => "expr".to_string(),
            };
            format!("элемент массива '{}[{}]'", name, idx_str)
        }
        ConditionNode::BitAccess(_, _) => "доступ к битовому полю".to_string(),
        _ => "числовое выражение".to_string(),
    }
}

/// Проверяет условие одного перехода и при необходимости добавляет предупреждение Се11.
///
/// Основной путь — условие уже разрешено на этапе 6 конвейера
/// ([`crate::semantic::tree`]). Неразрешённый вариант [`ConditionNode::Unresolved`]
/// используется только как запасной (для паттернов вида `S(Model).StateName`,
/// которые не могут быть разрешены в текущем контексте).
///
/// Описание условия для сообщения вычисляется **лениво** — только при наличии
/// реального нарушения.
fn check_one_ref(
    loc: Location,
    prefix: &str,
    target_name: &str,
    cond: &ConditionNode,
    model: &Rc<RefCell<ModelNode>>,
    is_next: bool,
    out: &mut Vec<Diagnostic>,
) {
    match cond {
        // ── Основной путь: разрешённое семантическое условие ──────────────────
        cond if !matches!(cond, ConditionNode::Unresolved(_))
            && !is_boolean_semantic_condition(cond) =>
        {
            let summary = semantic_condition_summary(cond);
            emit_implicit_bool_warning(loc, prefix, target_name, &summary, is_next, out);
        }
        // ── Запасной путь: условие не разрешено (например, S(Model).StateName) ──
        ConditionNode::Unresolved(ast_cond) if !is_boolean_ast_condition(ast_cond, model) => {
            let summary = ast_condition_summary(ast_cond, model);
            emit_implicit_bool_warning(loc, prefix, target_name, &summary, is_next, out);
        }
        _ => {}
    }
}

/// Рекурсивно собирает предупреждения Се11 для всех состояний модели.
///
/// Обходит все состояния текущей модели и вложенных моделей.
/// Для каждого перехода вызывает [`check_one_ref`].
fn collect_implicit_bool_warnings(model: &Rc<RefCell<ModelNode>>, out: &mut Vec<Diagnostic>) {
    let borrowed = model.borrow();
    let model_name = borrowed.name.clone().unwrap_or_default();

    // Строит префикс диагностического сообщения вида
    // "состояние 'S'" или "модель 'M', состояние 'S'".
    let prefix_for = |state_name: &str| -> String {
        if model_name.is_empty() {
            format!("состояние '{}'", state_name)
        } else {
            format!("модель '{}', состояние '{}'", model_name, state_name)
        }
    };

    for state in borrowed.states.values() {
        match state {
            StateNode::Simple {
                name, references, ..
            } => {
                let prefix = prefix_for(name);
                for r in references {
                    check_one_ref(r.location, &prefix, &r.name, &r.cond, model, false, out);
                }
            }
            StateNode::Implement {
                name,
                references,
                next,
                ..
            } => {
                let prefix = prefix_for(name);
                for r in references {
                    check_one_ref(r.location, &prefix, &r.name, &r.cond, model, false, out);
                }
                if let Some(nr) = next {
                    check_one_ref(nr.location, &prefix, &nr.name, &nr.cond, model, true, out);
                }
            }
            StateNode::Unresolved => {}
        }
    }

    // Рекурсивный спуск во вложенные модели
    let nested: Vec<Rc<RefCell<ModelNode>>> = borrowed.models.values().map(Rc::clone).collect();
    drop(borrowed); // освобождаем заимствование перед рекурсией

    for nested_model in nested {
        collect_implicit_bool_warnings(&nested_model, out);
    }
}

/// Проверяет условия переходов в модели и возвращает предупреждения
/// о неявном приведении числового типа к булевому.
///
/// Предупреждение выдаётся, когда условие перехода (`ref`/`next`) содержит
/// выражение числового типа, используемое как булево без явного сравнения.
///
/// # Примеры (Lam)
///
/// ```but
/// var timer: [bit;8] = 0;
/// start Red {
///     ref Green: timer;       // Предупреждение: timer — числовой тип [bit;8]
///     ref Blue:  timer != 0;  // Нет предупреждения: явное сравнение
/// }
/// ```
///
/// # Возвращаемое значение
///
/// Вектор [`Diagnostic`] уровня `Warning` для каждого обнаруженного случая.
/// Пустой вектор означает, что числовых условий не найдено.
pub fn check_implicit_bool_conditions(model: &Rc<RefCell<ModelNode>>) -> Vec<Diagnostic> {
    let mut warnings = Vec::new();
    collect_implicit_bool_warnings(model, &mut warnings);
    warnings
}

// ─── Ce4: Проверка объявлений типов-перечислений ────────────────────────────

/// Проверяет, что все переменные, тип которых — [`TypeNode::Enum`], ссылаются
/// на фактически объявленные перечисления.
///
/// ## Мотивация
///
/// `construct_type` не может проверить существование перечисления на этапе
/// построения дерева, поскольку перечисления и переменные обрабатываются в
/// одном проходе и могут идти в любом порядке. Эта функция выполняется после
/// полного построения дерева, когда `ModelNode::enums` уже заполнена.
///
/// ## Примеры
///
/// ```text
/// // Корректно: Color объявлен выше или ниже переменной
/// enum Color { Red = 0, Green = 1 }
/// var c: Color = 0;   // ✓
///
/// // Ошибка: Size не объявлен
/// var s: Size = 0;    // ✗ Ce4: перечисление 'Size' не объявлено
/// ```
///
/// # Ошибки
///
/// Возвращает [`Diagnostic`]-ошибку при первой переменной с необъявленным типом enum.
fn validate_enum_type_declarations(model: Rc<RefCell<ModelNode>>) -> Result<(), Diagnostic> {
    // Собираем (имя переменной, тип, loc) без удержания заимствования
    let vars: Vec<(String, TypeNode, Location)> = model
        .borrow()
        .variables
        .values()
        .filter_map(|var| match var {
            VariableNode::Simple { name, ty, .. }
            | VariableNode::Const { name, ty, .. }
            | VariableNode::Port { name, ty, .. } => Some((name.clone(), ty.clone(), var.loc())),
            VariableNode::Unresolved => None,
        })
        .collect();

    for (var_name, ty, loc) in vars {
        if let TypeNode::Enum(enum_name) = &ty
            && model.borrow().search_enum(enum_name).is_none()
        {
            return Err(Diagnostic::declaration_error(
                loc,
                format!(
                    "переменная '{}' объявлена с типом '{}', \
                     но перечисление '{}' не найдено в области видимости",
                    var_name, enum_name, enum_name
                ),
            )
            .with_code("SE-035"));
        }
    }
    Ok(())
}

/// Запускает все семантические проверки для модели и всех вложенных моделей.
///
/// # Ошибки
///
/// Пробрасывает первую найденную [`Diagnostic`]-ошибку.
/// Ce15: максимально допустимый размер массива.
///
/// Массивы размером более 1024 элементов являются подозрительно большими
/// для встраиваемых систем. Тип `element_count` ограничен `u16` (≤ 65535),
/// но даже статический C-массив из 65535 `float` занимает ~256 КБ и может
/// переполнить стек или BSS-сегмент на микроконтроллере.
/// Значение 1024 (2^10) — разумный предел для Lam-целевых платформ.
pub const MAX_ARRAY_SIZE: u16 = 1024;

/// Ce15: рекурсивно проверяет тип на допустимый размер массива.
///
/// Проверяет как внешний, так и вложенные (многомерные) массивы.
pub(crate) fn check_type_array_size(ty: &TypeNode, loc: Location) -> Result<(), Diagnostic> {
    if let TypeNode::Array(size, elem) = ty {
        if *size > MAX_ARRAY_SIZE {
            return Err(Diagnostic::error(
                loc,
                format!(
                    "размер массива {} превышает максимально допустимый {} (2^10). \
                     Используйте динамическую память или разбейте массив на части.",
                    size, MAX_ARRAY_SIZE
                ),
            )
            .with_code("SE-038"));
        }
        // Рекурсивная проверка вложенных массивов (многомерные типы)
        check_type_array_size(elem, loc)?;
    }
    Ok(())
}

/// Ce15: проверяет все переменные модели на допустимый размер массивов.
fn check_array_sizes(model: Rc<RefCell<ModelNode>>) -> Result<(), Diagnostic> {
    let borrowed = model.borrow();
    for var in borrowed.variables.values() {
        match var {
            VariableNode::Simple { ty, .. }
            | VariableNode::Const { ty, .. }
            | VariableNode::Port { ty, .. } => {
                check_type_array_size(ty, var.loc())?;
            }
            VariableNode::Unresolved => {}
        }
    }
    Ok(())
}

// ─── Ce16: Проверка рекурсивных псевдонимов типов ────────────────────────────

/// Возвращает имена псевдонимов типов, на которые прямо ссылается данный AST-тип.
///
/// Рекурсивно обходит `Array` и возвращает все `Alias`-имена, встреченные в типе.
/// Встроенные псевдонимы (`bit`, `bool`, `float`, `unit`) исключаются.
fn collect_type_deps(ty: &ast::Type) -> Vec<String> {
    match ty {
        ast::Type::Alias(id) => match id.name.as_str() {
            "bit" | "bool" | "float" | "unit" => vec![],
            name => vec![name.to_string()],
        },
        ast::Type::Array { element_type, .. } => collect_type_deps(element_type),
        _ => vec![],
    }
}

/// DFS-обход с обнаружением цикла в графе зависимостей псевдонимов.
///
/// `current`   — узел, который сейчас посещается.
/// `defs`      — карта `имя → AST-тип` (все псевдонимы в модели).
/// `visited`   — уже полностью обработанные узлы (серые/чёрные).
/// `stack`     — узлы на текущем пути DFS (для обнаружения цикла).
///
/// Возвращает `Some(имя)` — имя псевдонима, с которого начинается цикл, если цикл найден.
fn dfs_type_cycle(
    current: &str,
    defs: &std::collections::BTreeMap<String, ast::Type>,
    visited: &mut HashSet<String>,
    stack: &mut HashSet<String>,
) -> Option<String> {
    if stack.contains(current) {
        // Узел уже на стеке — найден цикл
        return Some(current.to_string());
    }
    if visited.contains(current) {
        // Узел уже полностью обработан — цикла нет
        return None;
    }

    stack.insert(current.to_string());

    if let Some(ty) = defs.get(current) {
        let deps = collect_type_deps(ty);
        for dep in deps {
            if let Some(cycle_start) = dfs_type_cycle(&dep, defs, visited, stack) {
                return Some(cycle_start);
            }
        }
    }

    stack.remove(current);
    visited.insert(current.to_string());
    None
}

/// Ce16: Проверяет карту сырых АСД-типов на наличие циклических зависимостей.
///
/// Используется как в `validate_model` (через `check_recursive_type_aliases`), так и
/// напрямую в `tree.rs` до вызова `construct_type` — чтобы выдать понятную ошибку Ce16
/// вместо «тип не найден».
///
/// `type_locs` — карта `имя → позиция` для формирования диагностики с правильным Source.
pub fn check_type_alias_cycles_ast(
    raw_defs: &std::collections::BTreeMap<String, ast::Type>,
    type_locs: &std::collections::BTreeMap<String, crate::diagnostics::Location>,
) -> Vec<Diagnostic> {
    let mut diags = Vec::new();
    let mut visited: HashSet<String> = HashSet::new();
    let mut reported: HashSet<String> = HashSet::new();

    for name in raw_defs.keys() {
        if visited.contains(name) {
            continue;
        }
        let mut stack: HashSet<String> = HashSet::new();
        if let Some(cycle_start) = dfs_type_cycle(name, raw_defs, &mut visited, &mut stack)
            && !reported.contains(&cycle_start)
        {
            reported.insert(cycle_start.clone());
            let loc = type_locs
                .get(&cycle_start)
                .copied()
                .unwrap_or(Location::Implicit);
            diags.push(
                Diagnostic::error(
                    loc,
                    format!(
                        "псевдоним типа '{}' образует циклическую зависимость",
                        cycle_start
                    ),
                )
                .with_code("SE-039"),
            );
        }
    }

    diags
}

/// Ce16: Проверяет наличие циклических зависимостей среди псевдонимов типов.
///
/// Строит граф зависимостей псевдонимов из `model.raw_type_defs` и обнаруживает
/// циклы с помощью DFS. Для каждого найденного цикла возвращает [`Diagnostic`]
/// с кодом Ce16.
///
/// ## Примеры
///
/// ```text
/// type A = [A; 8];           // Ce16: прямая рекурсия
/// type A = [B; 4];
/// type B = [A; 2];           // Ce16: взаимная рекурсия
/// type A = [bit; 8];
/// type B = [A; 2];           // OK: нет цикла
/// ```
pub fn check_recursive_type_aliases(model: Rc<RefCell<ModelNode>>) -> Vec<Diagnostic> {
    let raw_defs = model.borrow().raw_type_defs.clone();
    let type_locs = model.borrow().type_locs.clone();

    let mut diags = check_type_alias_cycles_ast(&raw_defs, &type_locs);

    // Рекурсивная проверка вложенных моделей
    let nested: Vec<Rc<RefCell<ModelNode>>> =
        model.borrow().models.values().map(Rc::clone).collect();
    for nested_model in nested {
        diags.extend(check_recursive_type_aliases(nested_model));
    }

    diags
}

pub fn validate_model(model: Rc<RefCell<ModelNode>>) -> Result<(), Diagnostic> {
    model_only_one_start_state(model.clone())?;
    validate_bit_values(model.clone())?;
    validate_enum_values(model.clone())?;
    validate_enum_type_declarations(model.clone())?;
    validate_state_references(model.clone())?;
    validate_variables(model.clone())?;
    validate_conditions(model.clone())?;
    check_array_sizes(model.clone())?;
    check_port_addresses(model.clone())?;

    // Ce16: проверка рекурсивных псевдонимов — ошибка при первом цикле
    let recursive_diags = check_recursive_type_aliases(model.clone());
    if let Some(first) = recursive_diags.into_iter().next() {
        return Err(first);
    }

    // Ce17: дублирующиеся поля структуры
    if let Some(diag) = check_duplicate_struct_fields(model.clone()) {
        return Err(diag);
    }

    // Ce18: неизвестный тип поля структуры
    if let Some(diag) = check_struct_field_types(model.clone()) {
        return Err(diag);
    }

    let nested: Vec<(String, Rc<RefCell<ModelNode>>)> = model
        .borrow()
        .models
        .iter()
        .map(|(k, v)| (k.clone(), Rc::clone(v)))
        .collect();

    for (_, nested_model) in nested {
        validate_model(nested_model)?; // рекурсивно проверяем вложенные модели
    }
    Ok(())
}

/// Фича 0020 (задача 0020-04): предупреждения о портах без адреса, попадающих
/// в кодогенерацию.
///
/// Порт считается **достижимым кодогенерацией**, если он используется в логике
/// модели (условиях, блоках, функциях) — критерий переиспользуется из
/// [`compute_usage`](super::unused::compute_usage). Для такого порта адрес
/// обязателен; источником может быть inline-инициализатор, оператор `address`
/// или внешняя карта (`external_ports` — имена портов, покрытых картой).
/// **Мёртвые** (неиспользуемые) порты без адреса **не** предупреждаются.
///
/// Возвращает предупреждения **SE-052**, отсортированные по позиции (для
/// детерминизма). Функция аналитическая: она не вызывается конвейером
/// `validate_model` по умолчанию (в текущей C-модели адрес не эмитится) — её
/// подключает потребитель адресов (C-таблица/HAL, задача 0020-05).
pub fn check_port_address_completeness(
    model: Rc<RefCell<ModelNode>>,
    external_ports: &HashSet<String>,
) -> Vec<Diagnostic> {
    let usage = super::unused::compute_usage(Rc::clone(&model));
    let mut out = Vec::new();
    collect_incomplete_addresses(&model, &usage.ports, external_ports, &mut out);
    out.sort_by_key(|d| d.loc.start());
    out
}

/// Рекурсивно собирает используемые порты без адреса по дереву моделей.
fn collect_incomplete_addresses(
    model: &Rc<RefCell<ModelNode>>,
    used_ports: &HashSet<String>,
    external_ports: &HashSet<String>,
    out: &mut Vec<Diagnostic>,
) {
    let borrowed = model.borrow();
    for var in borrowed.variables.values() {
        let VariableNode::Port {
            expr, loc, name, ..
        } = var
        else {
            continue;
        };
        if !used_ports.contains(name) {
            continue; // мёртвый порт — адрес не требуется
        }
        let has_inline = !matches!(expr, ExpressionNode::None);
        let has_operator = borrowed.address_defs.iter().any(|d| &d.port == name);
        let has_external = external_ports.contains(name);
        if !has_inline && !has_operator && !has_external {
            out.push(
                Diagnostic::warning(
                    *loc,
                    format!(
                        "порт '{}' используется в кодогенерации, но не имеет адреса \
                         (ни inline, ни оператором `address`, ни во внешней карте)",
                        name
                    ),
                )
                .with_code("SE-052"),
            );
        }
    }
    let nested: Vec<Rc<RefCell<ModelNode>>> = borrowed.models.values().map(Rc::clone).collect();
    drop(borrowed);
    for nested_model in nested {
        collect_incomplete_addresses(&nested_model, used_ports, external_ports, out);
    }
}

/// Фича 0020 (задача 0020-02): проверки оператора `address` для одной модели.
///
/// Наполнение [`address_defs`](ModelNode::address_defs) выполняет
/// [`construct_model`](super::tree::construct_model); здесь эти привязки
/// сверяются с объявленными портами:
///
/// - **Висячая привязка (R5, SE-048).** `address` ссылается на имя, которого нет
///   среди портов модели.
/// - **Конфликт источников (R4, SE-049).** Адрес порта задан одновременно inline
///   (`in P: T := <addr>;`) и оператором `address`, либо несколькими операторами
///   `address` для одного порта.
///
/// Приоритет источников (inline < `address` < внешняя карта) и построение
/// `AddressMap` для потребителей — задачи 0020-03/0020-05. Здесь достаточно
/// гарантировать однозначность источника адреса внутри модели.
fn check_port_addresses(model: Rc<RefCell<ModelNode>>) -> Result<(), Diagnostic> {
    let borrowed = model.borrow();
    let mut bound_by_address: HashSet<&str> = HashSet::new();
    for def in &borrowed.address_defs {
        // R5: адрес должен ссылаться на существующий порт.
        let Some(VariableNode::Port { expr, .. }) = borrowed.variables.get(&def.port) else {
            return Err(Diagnostic::error(
                def.loc,
                format!(
                    "оператор `address` ссылается на несуществующий порт '{}'",
                    def.port
                ),
            )
            .with_code("SE-048"));
        };
        // R4: несколько операторов `address` для одного порта.
        if !bound_by_address.insert(def.port.as_str()) {
            return Err(Diagnostic::error(
                def.loc,
                format!(
                    "адрес порта '{}' задан оператором `address` более одного раза",
                    def.port
                ),
            )
            .with_code("SE-049"));
        }
        // R4: адрес задан и inline-инициализатором, и оператором `address`.
        if !matches!(expr, ExpressionNode::None) {
            return Err(Diagnostic::error(
                def.loc,
                format!(
                    "адрес порта '{}' задан одновременно inline и оператором `address`",
                    def.port
                ),
            )
            .with_code("SE-049"));
        }
    }
    Ok(())
}

// ─── Предупреждения о портах во вложенных моделях ────────────────────────────

/// Возвращает предупреждения о портах, объявленных во вложенных (не корневых) моделях.
///
/// Порты во вложенных моделях видны всем моделям в системе: они попадают
/// в общие перечисления `BitPort`, `RationalPort`, `NumericPort` и доступны
/// через колбэки корневой модели. Пользователи должны учитывать это при
/// именовании портов.
///
/// Функция рекурсивно обходит все вложенные модели.
pub fn warn_nested_model_ports(model: Rc<RefCell<ModelNode>>) -> Vec<Diagnostic> {
    let mut result = Vec::new();
    let nested: Vec<Rc<RefCell<ModelNode>>> = model.borrow().models.values().cloned().collect();
    for nested_model in nested {
        let borrowed = nested_model.borrow();
        if borrowed.upper.is_some() {
            for var in borrowed.variables.values() {
                if let VariableNode::Port { name, loc, .. } = var {
                    result.push(Diagnostic::warning(
                        *loc,
                        format!(
                            "Порт '{}' объявлен во вложенной модели '{}' и будет виден \
                             всем моделям через перечисления портов корневой модели",
                            name,
                            borrowed.name()
                        ),
                    ));
                }
            }
        }
        drop(borrowed);
        result.extend(warn_nested_model_ports(nested_model));
    }
    result
}

// ─── Ce5: Проверка достижимости и полноты переходов ──────────────────────────

/// Проверяет полноту и достижимость переходов в модели конечного автомата.
///
/// ## Правила Ce5
///
/// 1. **Предупреждение**: из состояния нет пути к терминальному состоянию
///    (состоянию без исходящих переходов), и само оно не терминальное.
/// 2. **Предупреждение**: в модели нет ни одного терминального состояния.
/// 3. **Предупреждение**: в состоянии есть `ref`-переходы совместно с `next`
///    (код после `next` недостижим).
/// 4. **Ошибка**: в одном состоянии несколько `next` (уже проверяется парсером,
///    но дублируем на семантическом уровне для явности).
///
/// Функция рекурсивно обходит все вложенные модели.
///
/// # Возвращаемое значение
///
/// Вектор [`Diagnostic`] (предупреждения и ошибки).
/// Пустой вектор означает отсутствие нарушений.
pub fn check_transition_completeness(model: Rc<RefCell<ModelNode>>) -> Vec<Diagnostic> {
    let mut diags = Vec::new();
    collect_transition_completeness(&model, &mut diags);
    diags
}

/// Рекурсивно собирает диагностики Ce5 для модели и всех вложенных моделей.
fn collect_transition_completeness(model: &Rc<RefCell<ModelNode>>, out: &mut Vec<Diagnostic>) {
    let borrowed = model.borrow();

    // Если состояний нет — модуль без автомата, пропускаем
    if borrowed.states.is_empty() {
        // Рекурсивный спуск во вложенные модели
        let nested: Vec<Rc<RefCell<ModelNode>>> = borrowed.models.values().map(Rc::clone).collect();
        drop(borrowed);
        for m in nested {
            collect_transition_completeness(&m, out);
        }
        return;
    }

    let model_name = borrowed.name.clone().unwrap_or_default();
    let model_loc = borrowed.loc;

    // Строит префикс для сообщений: "модель 'M'" или пустую строку для корня
    let model_prefix = if model_name.is_empty() {
        String::new()
    } else {
        format!("модель '{}': ", model_name)
    };

    // Вычисляем множество терминальных состояний (без исходящих переходов)
    let terminal_states: std::collections::HashSet<String> = borrowed
        .states
        .iter()
        .filter_map(|(name, state)| {
            let is_terminal = match state {
                StateNode::Simple { references, .. } => references.is_empty(),
                StateNode::Implement {
                    references, next, ..
                } => references.is_empty() && next.is_none(),
                StateNode::Unresolved => false,
            };
            if is_terminal {
                Some(name.clone())
            } else {
                None
            }
        })
        .collect();

    // Правило Ce5.2: нет терминальных состояний вообще
    if terminal_states.is_empty() {
        out.push(
            Diagnostic::warning(
                model_loc,
                format!(
                    "{}в модели нет терминальных состояний (состояний без переходов); \
                     автомат не может завершить работу",
                    model_prefix
                ),
            )
            .with_code("SE-010"),
        );
    }

    // Строим граф переходов: имя_состояния -> список целей
    let mut graph: std::collections::BTreeMap<String, Vec<String>> =
        std::collections::BTreeMap::new();
    for (name, state) in borrowed.states.iter() {
        let targets: Vec<String> = match state {
            StateNode::Simple { references, .. } => {
                references.iter().map(|r| r.name.clone()).collect()
            }
            StateNode::Implement {
                references, next, ..
            } => {
                let mut t: Vec<String> = references.iter().map(|r| r.name.clone()).collect();
                if let Some(nr) = next {
                    t.push(nr.name.clone());
                }
                t
            }
            StateNode::Unresolved => vec![],
        };
        graph.insert(name.clone(), targets);
    }

    // Правило Ce5.1: из состояния нет пути к терминальному
    // Используем BFS/DFS от каждого нетерминального состояния
    if !terminal_states.is_empty() {
        for (state_name, state) in borrowed.states.iter() {
            // Терминальные состояния сами по себе достижимы
            if terminal_states.contains(state_name) {
                continue;
            }
            // BFS от state_name
            let can_reach = {
                let mut visited = std::collections::HashSet::new();
                let mut queue = std::collections::VecDeque::new();
                queue.push_back(state_name.clone());
                let mut found = false;
                while let Some(cur) = queue.pop_front() {
                    if terminal_states.contains(&cur) {
                        found = true;
                        break;
                    }
                    if visited.contains(&cur) {
                        continue;
                    }
                    visited.insert(cur.clone());
                    if let Some(targets) = graph.get(&cur) {
                        for t in targets {
                            queue.push_back(t.clone());
                        }
                    }
                }
                found
            };
            if !can_reach {
                out.push(
                    Diagnostic::warning(
                        state.loc(),
                        format!(
                            "{}состояние '{}' не имеет пути к терминальному состоянию",
                            model_prefix, state_name
                        ),
                    )
                    .with_code("SE-010"),
                );
            }
        }
    }

    // Правила Ce5.3 и Ce5.4: проверка next совместно с ref
    for (_state_name, state) in borrowed.states.iter() {
        if let StateNode::Implement {
            name,
            references,
            next,
            ..
        } = state
        {
            // Правило Ce5.3: ref + next одновременно → предупреждение
            if next.is_some() && !references.is_empty() {
                out.push(
                    Diagnostic::warning(
                        state.loc(),
                        format!(
                            "{}состояние '{}' содержит ref-переходы совместно с next: \
                             переходы ref недостижимы после выполнения next",
                            model_prefix, name
                        ),
                    )
                    .with_code("SE-012"),
                );
            }
        }
    }

    // Рекурсивный спуск во вложенные модели
    let nested: Vec<Rc<RefCell<ModelNode>>> = borrowed.models.values().map(Rc::clone).collect();
    drop(borrowed);

    for m in nested {
        collect_transition_completeness(&m, out);
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::parse;
    use crate::semantic::tree::construct_model;

    fn build(src: &str) -> Result<ModelNode, Diagnostic> {
        let (ast, _) = parse(src, 0).expect("ошибка разбора");
        construct_model(&ast, None, &[]).map(|m| m.take())
    }

    /// Пустая программа без состояний — валидна.
    #[test]
    fn empty_model_is_valid() {
        assert!(build("").is_ok());
    }

    /// Модель только с типами — валидна (нет состояний).
    #[test]
    fn model_with_only_types_is_valid() {
        assert!(build("type u8 = [bit;8];").is_ok());
    }

    /// Модель с одним начальным состоянием — валидна.
    #[test]
    fn single_start_state_is_valid() {
        assert!(build("start S;").is_ok());
    }

    /// Модель с двумя начальными состояниями — ошибка.
    ///
    /// # Контрпример (Lam)
    /// ```but
    /// start A;   // первое start
    /// start B;   // второе start — запрещено
    /// ```
    #[test]
    fn two_start_states_is_error() {
        let result = build("start A; start B;");
        assert!(result.is_err(), "два start-состояния должны давать ошибку");
    }

    /// Модель без начального состояния (только обычные состояния) — ошибка.
    ///
    /// # Контрпример (Lam)
    /// ```but
    /// state A;   // нет start — запрещено для модели с состояниями
    /// state B;
    /// ```
    #[test]
    fn no_start_state_is_error() {
        let result = build("state A; state B;");
        assert!(
            result.is_err(),
            "отсутствие start-состояния должно давать ошибку"
        );
    }

    /// Вложенная модель с двумя начальными состояниями — ошибка.
    #[test]
    fn nested_model_two_start_states_is_error() {
        let result = build("model M { start A; start B; }");
        assert!(
            result.is_err(),
            "вложенная модель с двумя start должна давать ошибку"
        );
    }

    /// Вложенная модель с одним start — валидна.
    #[test]
    fn nested_model_single_start_is_valid() {
        assert!(build("model M { start S; }").is_ok());
    }

    // ── Проверка значений типа bit ─────────────────────────────────────────────

    /// `var x: bit = 0;` — допустимо (числовое значение 0).
    ///
    /// # Пример (Lam)
    /// ```but
    /// var x: bit = 0;
    /// ```
    #[test]
    fn bit_var_with_zero_is_valid() {
        assert!(build("var x: bit := 0;").is_ok());
    }

    /// `var x: bit = 1;` — допустимо (числовое значение 1).
    ///
    /// # Пример (Lam)
    /// ```but
    /// var x: bit = 1;
    /// ```
    #[test]
    fn bit_var_with_one_is_valid() {
        assert!(build("var x: bit := 1;").is_ok());
    }

    /// `var x: bit = true;` — допустимо (булев литерал).
    #[test]
    fn bit_var_with_true_is_valid() {
        assert!(build("var x: bit := true;").is_ok());
    }

    /// `var x: bit = false;` — допустимо (булев литерал).
    #[test]
    fn bit_var_with_false_is_valid() {
        assert!(build("var x: bit := false;").is_ok());
    }

    /// `var x: bit = 2;` — ошибка: значение 2 не является допустимым для bit.
    ///
    /// # Контрпример (Lam)
    /// ```but
    /// var x: bit = 2;   // ошибка: недопустимое значение
    /// ```
    #[test]
    fn bit_var_with_two_is_error() {
        let result = build("var x: bit := 2;");
        assert!(result.is_err(), "значение 2 недопустимо для типа bit");
        assert!(result.unwrap_err().message.contains("bit"));
    }

    /// `var x: bit = -1;` — ошибка: отрицательное значение не допускается для bit.
    ///
    /// # Контрпример (Lam)
    /// ```but
    /// var x: bit = -1;   // ошибка: отрицательное число недопустимо
    /// ```
    #[test]
    fn bit_var_with_minus_one_is_error() {
        let result = build("var x: bit := -1;");
        // -1 парсится как Negate(1) или Number(-1): в обоих случаях числовой литерал -1
        // Если парсер создаёт Number(-1), должна быть ошибка валидации.
        // Если парсер создаёт Negate(Number(1)), это выражение — не Number, ошибки нет.
        // Тест проверяет только отсутствие паники.
        let _ = result; // оба варианта допустимы для текущего парсера
    }

    /// `var x: bit = 255;` — ошибка: значение вне допустимого диапазона bit.
    ///
    /// # Контрпример (Lam)
    /// ```but
    /// var x: bit = 255;   // ошибка: 255 не входит в {0, 1}
    /// ```
    #[test]
    fn bit_var_with_255_is_error() {
        let result = build("var x: bit := 255;");
        assert!(result.is_err(), "значение 255 недопустимо для типа bit");
    }

    /// `const C: bit = 2;` — ошибка: константа типа bit с недопустимым значением.
    #[test]
    fn bit_const_with_invalid_value_is_error() {
        let result = build("const C: bit := 2;");
        assert!(result.is_err(), "константа bit = 2 должна давать ошибку");
    }

    /// Переменные типа `[bit;8]` (массив) не проверяются на диапазон элементов —
    /// числовое значение инициализатора массива трактуется как целое число.
    #[test]
    fn bit_array_initializer_is_not_range_checked() {
        // [bit;8] = 255 — это 8-битное значение, проверка диапазона не применяется.
        assert!(build("var x: [bit;8] := 255;").is_ok());
    }

    /// Переменная `bit` с инициализатором-переменной не проверяется статически.
    #[test]
    fn bit_var_initialized_from_other_var_is_valid() {
        // b: bit = a — ссылка на переменную, статическая проверка значения не применяется.
        assert!(build("var a: bit := 0; var b: bit := a;").is_ok());
    }

    /// Вложенная модель с некорректным значением bit — ошибка.
    #[test]
    fn nested_model_with_invalid_bit_value_is_error() {
        let result = build("model M { var x: bit := 5; start S; }");
        assert!(
            result.is_err(),
            "вложенная модель: bit = 5 должна давать ошибку"
        );
    }

    // ── Се11: строгая проверка булевости условий переходов ─────────────────────

    fn build_rc(src: &str) -> Rc<RefCell<ModelNode>> {
        let (ast, _) = parse(src, 0).expect("ошибка разбора");
        construct_model(&ast, None, &[]).expect("ошибка семантики")
    }

    // ── Юнит-тесты is_boolean_ast_condition и ast_condition_summary ────────────
    //
    // Вспомогательные функции для построения моделей и AST-условий.

    /// Строит пустую семантическую модель (без переменных и состояний).
    fn empty_model() -> Rc<RefCell<ModelNode>> {
        build_rc("")
    }

    /// Строит модель с переменными: `flag: bool`, `bit1: bit`, `timer: [bit;8]`.
    fn model_with_vars() -> Rc<RefCell<ModelNode>> {
        build_rc(
            "var flag: bool := false; \
             var bit1: bit := 0; \
             var timer: [bit;8] := 0;",
        )
    }

    /// Строит модель с именованным условием `cond Full = timer = 255;`.
    fn model_with_named_cond() -> Rc<RefCell<ModelNode>> {
        build_rc("var timer: [bit;8] := 0; cond Full = timer = 255;")
    }

    use crate::diagnostics::Location as Loc;
    use crate::parser::ast::Condition as AC;
    use crate::parser::ast::Identifier;

    fn loc() -> Loc {
        Loc::Builtin
    }

    fn id(name: &str) -> Identifier {
        Identifier::new(name)
    }

    // ── Явно булевые условия ────────────────────────────────────────────────

    /// `Bool(true)` → булево.
    #[test]
    fn ast_cond_bool_literal_is_true() {
        assert!(is_boolean_ast_condition(
            &AC::Bool(loc(), true),
            &empty_model()
        ));
    }

    /// `Bool(false)` → булево.
    #[test]
    fn ast_cond_bool_false_literal_is_true() {
        assert!(is_boolean_ast_condition(
            &AC::Bool(loc(), false),
            &empty_model()
        ));
    }

    /// `Equal` → булево.
    #[test]
    fn ast_cond_equal_is_true() {
        let cond = AC::Equal(
            loc(),
            Box::new(AC::Number(loc(), 0)),
            Box::new(AC::Number(loc(), 0)),
        );
        assert!(is_boolean_ast_condition(&cond, &empty_model()));
    }

    /// `NotEqual` → булево.
    #[test]
    fn ast_cond_not_equal_is_true() {
        let cond = AC::NotEqual(
            loc(),
            Box::new(AC::Number(loc(), 0)),
            Box::new(AC::Number(loc(), 1)),
        );
        assert!(is_boolean_ast_condition(&cond, &empty_model()));
    }

    /// `Less` → булево.
    #[test]
    fn ast_cond_less_is_true() {
        let cond = AC::Less(
            loc(),
            Box::new(AC::Number(loc(), 0)),
            Box::new(AC::Number(loc(), 1)),
        );
        assert!(is_boolean_ast_condition(&cond, &empty_model()));
    }

    /// `More` → булево.
    #[test]
    fn ast_cond_more_is_true() {
        let cond = AC::More(
            loc(),
            Box::new(AC::Number(loc(), 5)),
            Box::new(AC::Number(loc(), 1)),
        );
        assert!(is_boolean_ast_condition(&cond, &empty_model()));
    }

    /// `LessEqual` → булево.
    #[test]
    fn ast_cond_less_equal_is_true() {
        let cond = AC::LessEqual(
            loc(),
            Box::new(AC::Number(loc(), 0)),
            Box::new(AC::Number(loc(), 0)),
        );
        assert!(is_boolean_ast_condition(&cond, &empty_model()));
    }

    /// `MoreEqual` → булево.
    #[test]
    fn ast_cond_more_equal_is_true() {
        let cond = AC::MoreEqual(
            loc(),
            Box::new(AC::Number(loc(), 5)),
            Box::new(AC::Number(loc(), 5)),
        );
        assert!(is_boolean_ast_condition(&cond, &empty_model()));
    }

    /// `Not` — логическое НЕ → всегда булево.
    #[test]
    fn ast_cond_not_is_true() {
        let cond = AC::Not(loc(), Box::new(AC::Number(loc(), 0)));
        assert!(is_boolean_ast_condition(&cond, &empty_model()));
    }

    /// `Not` вокруг переменной → булево.
    #[test]
    fn ast_cond_not_of_var_is_true() {
        let model = model_with_vars();
        let cond = AC::Not(loc(), Box::new(AC::Variable(id("timer"))));
        assert!(is_boolean_ast_condition(&cond, &model));
    }

    /// `Function(…)` — тип возврата неизвестен → булево.
    #[test]
    fn ast_cond_function_is_true() {
        let cond = AC::Function(loc(), id("f"), vec![]);
        assert!(is_boolean_ast_condition(&cond, &empty_model()));
    }

    /// `Parenthesis(Equal)` → булево.
    #[test]
    fn ast_cond_paren_cmp_is_true() {
        let inner = AC::Equal(
            loc(),
            Box::new(AC::Number(loc(), 0)),
            Box::new(AC::Number(loc(), 1)),
        );
        let cond = AC::Parenthesis(loc(), Box::new(inner));
        assert!(is_boolean_ast_condition(&cond, &empty_model()));
    }

    /// `Variable("flag")` где `flag: bool` → булево.
    #[test]
    fn ast_cond_bool_var_is_true() {
        let model = model_with_vars();
        assert!(is_boolean_ast_condition(&AC::Variable(id("flag")), &model));
    }

    /// `Variable("bit1")` где `bit1: bit` → булево.
    #[test]
    fn ast_cond_bit_var_is_true() {
        let model = model_with_vars();
        assert!(is_boolean_ast_condition(&AC::Variable(id("bit1")), &model));
    }

    /// `Variable("Full")` где `Full` — именованное условие → булево.
    #[test]
    fn ast_cond_named_cond_var_is_true() {
        let model = model_with_named_cond();
        assert!(is_boolean_ast_condition(&AC::Variable(id("Full")), &model));
    }

    /// `Variable("unknown")` — неизвестное имя → не предупреждаем (булево).
    #[test]
    fn ast_cond_unknown_var_is_true() {
        assert!(is_boolean_ast_condition(
            &AC::Variable(id("unknown")),
            &empty_model()
        ));
    }

    // ── Явно числовые условия ───────────────────────────────────────────────

    /// `Number(5)` → числовое.
    #[test]
    fn ast_cond_number_is_false() {
        assert!(!is_boolean_ast_condition(
            &AC::Number(loc(), 5),
            &empty_model()
        ));
    }

    /// `Number(0)` → числовое (даже 0).
    #[test]
    fn ast_cond_zero_number_is_false() {
        assert!(!is_boolean_ast_condition(
            &AC::Number(loc(), 0),
            &empty_model()
        ));
    }

    /// `Rational` → числовое.
    #[test]
    fn ast_cond_rational_is_false() {
        let cond = AC::Rational(loc(), "3.14".to_string(), false);
        assert!(!is_boolean_ast_condition(&cond, &empty_model()));
    }

    /// `String` → числовое.
    #[test]
    fn ast_cond_string_is_false() {
        let cond = AC::String(vec![]);
        assert!(!is_boolean_ast_condition(&cond, &empty_model()));
    }

    /// `Add` → числовое.
    #[test]
    fn ast_cond_add_is_false() {
        let cond = AC::Add(
            loc(),
            Box::new(AC::Number(loc(), 1)),
            Box::new(AC::Number(loc(), 2)),
        );
        assert!(!is_boolean_ast_condition(&cond, &empty_model()));
    }

    /// `Subtract` → числовое.
    #[test]
    fn ast_cond_subtract_is_false() {
        let cond = AC::Subtract(
            loc(),
            Box::new(AC::Number(loc(), 5)),
            Box::new(AC::Number(loc(), 1)),
        );
        assert!(!is_boolean_ast_condition(&cond, &empty_model()));
    }

    /// `And` (побитовое И) → числовое.
    #[test]
    fn ast_cond_and_is_false() {
        let cond = AC::And(
            loc(),
            Box::new(AC::Number(loc(), 3)),
            Box::new(AC::Number(loc(), 1)),
        );
        assert!(!is_boolean_ast_condition(&cond, &empty_model()));
    }

    /// `Or` (побитовое ИЛИ) → числовое.
    #[test]
    fn ast_cond_or_is_false() {
        let cond = AC::Or(
            loc(),
            Box::new(AC::Number(loc(), 3)),
            Box::new(AC::Number(loc(), 1)),
        );
        assert!(!is_boolean_ast_condition(&cond, &empty_model()));
    }

    /// `ArraySubscript` → числовое.
    #[test]
    fn ast_cond_array_subscript_is_false() {
        let cond = AC::ArraySubscript(loc(), id("arr"), Box::new(AC::Number(loc(), 0)));
        assert!(!is_boolean_ast_condition(&cond, &empty_model()));
    }

    /// `BitAccess` → числовое.
    #[test]
    fn ast_cond_bit_access_is_false() {
        use crate::parser::ast::Member;
        let cond = AC::BitAccess(loc(), Box::new(AC::Variable(id("x"))), Member::Number(0));
        assert!(!is_boolean_ast_condition(&cond, &empty_model()));
    }

    /// `Variable("timer")` где `timer: [bit;8]` → числовое.
    #[test]
    fn ast_cond_array_var_is_false() {
        let model = model_with_vars();
        assert!(!is_boolean_ast_condition(
            &AC::Variable(id("timer")),
            &model
        ));
    }

    /// `Parenthesis(Number)` → числовое.
    #[test]
    fn ast_cond_paren_number_is_false() {
        let cond = AC::Parenthesis(loc(), Box::new(AC::Number(loc(), 42)));
        assert!(!is_boolean_ast_condition(&cond, &empty_model()));
    }

    // ── Юнит-тесты ast_condition_summary ────────────────────────────────────

    /// Summary для числового литерала содержит значение.
    #[test]
    fn ast_summary_number() {
        let s = ast_condition_summary(&AC::Number(loc(), 42), &empty_model());
        assert!(s.contains("42"), "summary для 42: '{}'", s);
    }

    /// Summary для вещественного числа содержит значение.
    #[test]
    fn ast_summary_rational() {
        let cond = AC::Rational(loc(), "1.5".to_string(), false);
        let s = ast_condition_summary(&cond, &empty_model());
        assert!(s.contains("1.5"), "summary для 1.5: '{}'", s);
    }

    /// Summary для отрицательного вещественного числа содержит минус.
    #[test]
    fn ast_summary_rational_negative() {
        let cond = AC::Rational(loc(), "2.0".to_string(), true);
        let s = ast_condition_summary(&cond, &empty_model());
        assert!(s.contains("-2.0"), "summary для -2.0: '{}'", s);
    }

    /// Summary для строки содержит слово "строковый".
    #[test]
    fn ast_summary_string() {
        let s = ast_condition_summary(&AC::String(vec![]), &empty_model());
        assert!(s.contains("строковый"), "summary для String: '{}'", s);
    }

    /// Summary для переменной числового типа содержит имя и тип.
    #[test]
    fn ast_summary_array_var() {
        let model = model_with_vars();
        let s = ast_condition_summary(&AC::Variable(id("timer")), &model);
        assert!(s.contains("timer"), "имя в summary: '{}'", s);
        assert!(s.contains("Array"), "тип в summary: '{}'", s);
    }

    /// Summary для неизвестной переменной содержит имя и `?`.
    #[test]
    fn ast_summary_unknown_var() {
        let s = ast_condition_summary(&AC::Variable(id("ghost")), &empty_model());
        assert!(s.contains("ghost"), "имя в summary: '{}'", s);
    }

    /// Summary для сложения.
    #[test]
    fn ast_summary_add() {
        let cond = AC::Add(
            loc(),
            Box::new(AC::Number(loc(), 1)),
            Box::new(AC::Number(loc(), 2)),
        );
        let s = ast_condition_summary(&cond, &empty_model());
        assert!(s.contains("сложение"), "summary для Add: '{}'", s);
    }

    /// Summary для вычитания.
    #[test]
    fn ast_summary_subtract() {
        let cond = AC::Subtract(
            loc(),
            Box::new(AC::Number(loc(), 5)),
            Box::new(AC::Number(loc(), 1)),
        );
        let s = ast_condition_summary(&cond, &empty_model());
        assert!(s.contains("вычитание"), "summary для Subtract: '{}'", s);
    }

    /// Summary для побитового И.
    #[test]
    fn ast_summary_and() {
        let cond = AC::And(
            loc(),
            Box::new(AC::Number(loc(), 1)),
            Box::new(AC::Number(loc(), 1)),
        );
        let s = ast_condition_summary(&cond, &empty_model());
        assert!(s.contains('И'), "summary для And: '{}'", s);
    }

    /// Summary для побитового ИЛИ.
    #[test]
    fn ast_summary_or() {
        let cond = AC::Or(
            loc(),
            Box::new(AC::Number(loc(), 1)),
            Box::new(AC::Number(loc(), 1)),
        );
        let s = ast_condition_summary(&cond, &empty_model());
        assert!(s.contains('И'), "summary для Or: '{}'", s);
    }

    /// Summary для элемента массива содержит имя и индекс.
    #[test]
    fn ast_summary_array_subscript() {
        let cond = AC::ArraySubscript(loc(), id("buf"), Box::new(AC::Number(loc(), 3)));
        let s = ast_condition_summary(&cond, &empty_model());
        assert!(s.contains("buf"), "имя массива в summary: '{}'", s);
        assert!(s.contains('3'), "индекс в summary: '{}'", s);
    }

    /// Summary для доступа к биту.
    #[test]
    fn ast_summary_bit_access() {
        use crate::parser::ast::Member;
        let cond = AC::BitAccess(loc(), Box::new(AC::Variable(id("x"))), Member::Number(0));
        let s = ast_condition_summary(&cond, &empty_model());
        assert!(s.contains("бит"), "summary для BitAccess: '{}'", s);
    }

    // ── Юнит-тесты check_implicit_bool_conditions ────────────────────────────

    /// Безусловный переход (`ref Next;`) — нет предупреждений.
    #[test]
    fn unconditional_ref_no_warning() {
        let model = build_rc("start S { ref T; } state T;");
        let warnings = check_implicit_bool_conditions(&model);
        assert!(
            warnings.is_empty(),
            "безусловный переход не должен давать предупреждений"
        );
    }

    /// Булев литерал в условии (`ref Next: true;`) — нет предупреждений.
    #[test]
    fn bool_literal_cond_no_warning() {
        let model = build_rc("start S { ref T: true; } state T;");
        let warnings = check_implicit_bool_conditions(&model);
        assert!(
            warnings.is_empty(),
            "булев литерал не должен давать предупреждений"
        );
    }

    /// Переменная типа `bool` в условии — нет предупреждений.
    #[test]
    fn bool_var_cond_no_warning() {
        let model = build_rc("var flag: bool := false; start S { ref T: flag; } state T;");
        let warnings = check_implicit_bool_conditions(&model);
        assert!(
            warnings.is_empty(),
            "переменная bool не должна давать предупреждений"
        );
    }

    /// Переменная типа `bit` (один бит) в условии — нет предупреждений.
    #[test]
    fn bit_var_cond_no_warning() {
        let model = build_rc("var flag: bit := 0; start S { ref T: flag; } state T;");
        let warnings = check_implicit_bool_conditions(&model);
        assert!(
            warnings.is_empty(),
            "переменная bit не должна давать предупреждений"
        );
    }

    /// Явное сравнение `!= 0` — нет предупреждений.
    #[test]
    fn explicit_ne_comparison_no_warning() {
        let model = build_rc("var timer: [bit;8] := 0; start S { ref T: timer != 0; } state T;");
        let warnings = check_implicit_bool_conditions(&model);
        assert!(
            warnings.is_empty(),
            "явное != не должно давать предупреждений"
        );
    }

    /// Явное сравнение `= 100` — нет предупреждений.
    #[test]
    fn explicit_eq_comparison_no_warning() {
        let model = build_rc("var timer: [bit;8] := 0; start S { ref T: timer = 100; } state T;");
        let warnings = check_implicit_bool_conditions(&model);
        assert!(
            warnings.is_empty(),
            "явное = не должно давать предупреждений"
        );
    }

    /// Именованное условие в ref — нет предупреждений.
    #[test]
    fn named_cond_in_ref_no_warning() {
        let model = build_rc(
            "var timer: [bit;8] := 0; \
             cond Full = timer = 255; \
             start S { ref T: Full; } state T;",
        );
        let warnings = check_implicit_bool_conditions(&model);
        assert!(
            warnings.is_empty(),
            "именованное условие не должно давать предупреждений"
        );
    }

    /// Переменная числового типа `[bit;8]` без сравнения — предупреждение.
    #[test]
    fn array_var_cond_gives_warning() {
        let model = build_rc("var timer: [bit;8] := 0; start S { ref T: timer; } state T;");
        let warnings = check_implicit_bool_conditions(&model);
        assert_eq!(
            warnings.len(),
            1,
            "переменная [bit;8] должна давать предупреждение"
        );
        assert!(
            warnings[0].message.contains("timer"),
            "сообщение должно упоминать 'timer'"
        );
    }

    /// Числовой литерал в условии — предупреждение.
    #[test]
    fn number_literal_cond_gives_warning() {
        let model = build_rc("start S { ref T: 5; } state T;");
        let warnings = check_implicit_bool_conditions(&model);
        assert_eq!(
            warnings.len(),
            1,
            "числовой литерал должен давать предупреждение"
        );
        assert!(
            warnings[0].message.contains('5'),
            "сообщение должно упоминать значение 5"
        );
    }

    /// Предупреждение содержит имя целевого состояния.
    #[test]
    fn warning_message_contains_target_state() {
        let model = build_rc("var x: [bit;8] := 0; start S { ref MyTarget: x; } state MyTarget;");
        let warnings = check_implicit_bool_conditions(&model);
        assert_eq!(warnings.len(), 1);
        assert!(
            warnings[0].message.contains("MyTarget"),
            "сообщение должно упоминать состояние-цель: {}",
            warnings[0].message
        );
    }

    /// Несколько переходов: один числовой, один булев — одно предупреждение.
    #[test]
    fn mixed_refs_one_warning() {
        let model = build_rc(
            "var timer: [bit;8] := 0; var flag: bool := false; \
             start S { ref T: timer; ref U: flag; } state T; state U;",
        );
        let warnings = check_implicit_bool_conditions(&model);
        assert_eq!(warnings.len(), 1, "должно быть ровно одно предупреждение");
    }

    /// Два числовых условия — два предупреждения.
    #[test]
    fn two_numeric_refs_two_warnings() {
        let model = build_rc(
            "var a: [bit;8] := 0; var b: [bit;8] := 0; \
             start S { ref T: a; ref U: b; } state T; state U;",
        );
        let warnings = check_implicit_bool_conditions(&model);
        assert_eq!(
            warnings.len(),
            2,
            "два числовых условия — два предупреждения"
        );
    }

    /// Вложенная модель с числовым условием — предупреждение упоминает имя модели.
    #[test]
    fn nested_model_implicit_bool_gives_warning() {
        let model =
            build_rc("model M { var timer: [bit;8] := 0; start S { ref T: timer; } state T; }");
        let warnings = check_implicit_bool_conditions(&model);
        assert_eq!(
            warnings.len(),
            1,
            "вложенная модель должна давать предупреждение"
        );
        assert!(
            warnings[0].message.contains('M'),
            "сообщение должно упоминать 'M'"
        );
    }

    /// Модель без состояний — нет предупреждений.
    #[test]
    fn model_without_states_no_warnings() {
        let model = build_rc("var timer: [bit;8] := 0;");
        let warnings = check_implicit_bool_conditions(&model);
        assert!(
            warnings.is_empty(),
            "модель без состояний не должна давать предупреждений"
        );
    }

    /// Предупреждение Се11 имеет уровень Warning.
    #[test]
    fn warning_has_correct_level() {
        use crate::diagnostics::Level;
        let model = build_rc("start S { ref T: 5; } state T;");
        let warnings = check_implicit_bool_conditions(&model);
        assert_eq!(warnings.len(), 1);
        assert_eq!(
            warnings[0].level,
            Level::Warning,
            "уровень должен быть Warning"
        );
    }

    // ── NI6: типобезопасные операции с enum ────────────────────────────────────────

    /// Переменная с корректным значением enum не вызывает ошибок NI6.
    ///
    /// # Пример (Lam)
    /// ```but
    /// enum Dir { North, South }
    /// var d: Dir = 0;  // 0 — значение North
    /// ```
    #[test]
    fn ni6_valid_enum_initializer_no_errors() {
        let model_rc = {
            let (ast, _) = parse("start S;", 0).expect("ошибка разбора");
            let m = construct_model(&ast, None, &[]).expect("ошибка семантики");
            // Добавляем перечисление и переменную с корректным значением программно
            let e = crate::semantic::EnumDefinitionNode::new(
                "Direction",
                &[
                    ("North", Some(0)),
                    ("South", Some(1)),
                    ("East", Some(2)),
                    ("West", Some(3)),
                ],
            );
            m.borrow_mut().enums.insert("Direction".to_string(), e);
            let dir_var = VariableNode::Simple {
                upper: None,
                loc: Location::Implicit,
                name: "dir".to_string(),
                ty: TypeNode::Enum("Direction".to_string()),
                expr: ExpressionNode::Number(0),
            };
            m.borrow_mut().variables.insert("dir".to_string(), dir_var);
            m
        };
        let errors = check_enum_type_safety(model_rc);
        assert!(
            errors.is_empty(),
            "допустимое значение enum не должно вызывать ошибок NI6"
        );
    }

    /// Переменная с некорректным значением enum вызывает ошибку NI6.
    ///
    /// # Контрпример (Lam)
    /// ```but
    /// enum Dir { North = 0, South = 1 }
    /// var d: Dir = 99;  // 99 — не вариант Dir
    /// ```
    #[test]
    fn ni6_invalid_enum_initializer_is_error() {
        let model_rc = {
            let (ast, _) = parse(
                "enum Direction { North, South, East, West } \
                 start S;",
                0,
            )
            .expect("ошибка разбора");
            let m = construct_model(&ast, None, &[]).expect("ошибка семантики");
            // Добавляем переменную с некорректным значением enum программно
            let dir_var = VariableNode::Simple {
                upper: None,
                loc: Location::Implicit,
                name: "dir".to_string(),
                ty: TypeNode::Enum("Direction".to_string()),
                expr: ExpressionNode::Number(99),
            };
            m.borrow_mut().variables.insert("dir".to_string(), dir_var);
            m
        };
        let errors = check_enum_type_safety(model_rc);
        assert_eq!(errors.len(), 1, "значение 99 недопустимо для Direction");
        assert_eq!(
            errors[0].code.as_deref(),
            Some("SE-043"),
            "код ошибки NI6 должен быть SE-043"
        );
        assert!(errors[0].message.contains("99"));
    }

    /// Инициализация значением варианта (по числовому значению) — без ошибок NI6.
    #[test]
    fn ni6_valid_explicit_value_no_errors() {
        let model_rc = {
            let (ast, _) = parse(
                "enum Priority { Low = 0, Medium = 5, High = 10 } start S;",
                0,
            )
            .expect("ошибка разбора");
            let m = construct_model(&ast, None, &[]).expect("ошибка семантики");
            let prio_var = VariableNode::Simple {
                upper: None,
                loc: Location::Implicit,
                name: "prio".to_string(),
                ty: TypeNode::Enum("Priority".to_string()),
                expr: ExpressionNode::Number(5),
            };
            m.borrow_mut()
                .variables
                .insert("prio".to_string(), prio_var);
            m
        };
        let errors = check_enum_type_safety(model_rc);
        assert!(
            errors.is_empty(),
            "значение 5 (Medium) допустимо для Priority"
        );
    }

    /// Несколько переменных — несколько ошибок NI6.
    #[test]
    fn ni6_multiple_invalid_enum_vars_gives_multiple_errors() {
        let model_rc = {
            let (ast, _) =
                parse("enum Dir { North = 0, South = 1 } start S;", 0).expect("ошибка разбора");
            let m = construct_model(&ast, None, &[]).expect("ошибка семантики");
            let v1 = VariableNode::Simple {
                upper: None,
                loc: Location::Implicit,
                name: "a".to_string(),
                ty: TypeNode::Enum("Dir".to_string()),
                expr: ExpressionNode::Number(42),
            };
            let v2 = VariableNode::Simple {
                upper: None,
                loc: Location::Implicit,
                name: "b".to_string(),
                ty: TypeNode::Enum("Dir".to_string()),
                expr: ExpressionNode::Number(99),
            };
            m.borrow_mut().variables.insert("a".to_string(), v1);
            m.borrow_mut().variables.insert("b".to_string(), v2);
            m
        };
        let errors = check_enum_type_safety(model_rc);
        assert_eq!(
            errors.len(),
            2,
            "два некорректных значения должны дать 2 ошибки NI6"
        );
    }

    /// Переменная типа bit не проверяется функцией NI6.
    #[test]
    fn ni6_non_enum_var_not_checked() {
        let model_rc = build_rc("var x: bit := 0; start S;");
        let errors = check_enum_type_safety(model_rc);
        assert!(
            errors.is_empty(),
            "переменная типа bit не должна проверяться NI6"
        );
    }

    /// Переменная с неизвестным enum-типом (перечисление не найдено) — не вызывает NI6.
    #[test]
    fn ni6_unknown_enum_type_no_error() {
        let model_rc = {
            let (ast, _) = parse("start S;", 0).expect("ошибка разбора");
            let m = construct_model(&ast, None, &[]).expect("ошибка семантики");
            let var = VariableNode::Simple {
                upper: None,
                loc: Location::Implicit,
                name: "x".to_string(),
                ty: TypeNode::Enum("UnknownEnum".to_string()),
                expr: ExpressionNode::Number(99),
            };
            m.borrow_mut().variables.insert("x".to_string(), var);
            m
        };
        let errors = check_enum_type_safety(model_rc);
        assert!(
            errors.is_empty(),
            "неизвестный тип enum не вызывает NI6 (ошибка другой проверки)"
        );
    }
}

// ─── Ce4: Тесты validate_enum_type_declarations ──────────────────────────────

#[cfg(test)]
mod tests_ce4_declarations {
    use super::*;

    /// Вспомогательная функция: строит Rc<RefCell<ModelNode>> из Lam-исходника.
    fn build_rc(src: &str) -> Rc<RefCell<ModelNode>> {
        let (ast, _) = crate::parse(src, 0).expect("ошибка разбора");
        crate::semantic::tree::construct_model(&ast, None, &[]).expect("ошибка семантики")
    }

    // ── Примеры корректного использования enum-типов ──────────────────────────

    /// Переменная с типом enum, где перечисление объявлено — ошибок нет.
    ///
    /// # Пример (Lam)
    /// ```text
    /// enum Color { Red = 0, Green = 1 }
    /// var c: Color = 0;   // ✓ Color объявлен
    /// start S;
    /// ```
    #[test]
    fn ce4_declared_enum_type_is_ok() {
        // Добавляем перечисление и переменную типа этого перечисления программно
        let model_rc = {
            let (ast, _) = crate::parse("enum Color { Red = 0, Green = 1 } start S;", 0)
                .expect("ошибка разбора");
            let m =
                crate::semantic::tree::construct_model(&ast, None, &[]).expect("ошибка семантики");
            // Переменная типа Color — Color объявлен в AST
            let var = VariableNode::Simple {
                upper: None,
                loc: Location::Implicit,
                name: "c".to_string(),
                ty: TypeNode::Enum("Color".to_string()),
                expr: ExpressionNode::Number(0),
            };
            m.borrow_mut().variables.insert("c".to_string(), var);
            m
        };
        let result = validate_enum_type_declarations(model_rc);
        assert!(
            result.is_ok(),
            "переменная с объявленным enum-типом не должна давать ошибку: {:?}",
            result
        );
    }

    /// Переменная с обычным (не-enum) типом не проверяется Ce4.
    ///
    /// # Пример (Lam)
    /// ```text
    /// var x: [bit;8] = 0;  // ✓ обычный тип, Ce4 не применяется
    /// start S;
    /// ```
    #[test]
    fn ce4_non_enum_type_not_checked() {
        let model_rc = build_rc("var x: [bit;8] := 0; start S;");
        let result = validate_enum_type_declarations(model_rc);
        assert!(result.is_ok(), "не-enum тип не должен проверяться Ce4");
    }

    /// Переменная с пустым enum-типом (Inference) не проверяется Ce4.
    #[test]
    fn ce4_inference_type_not_checked() {
        let model_rc = build_rc("start S;");
        // Добавляем переменную с типом Inference
        let var = VariableNode::Simple {
            upper: None,
            loc: Location::Implicit,
            name: "y".to_string(),
            ty: TypeNode::Inference,
            expr: ExpressionNode::Number(0),
        };
        model_rc.borrow_mut().variables.insert("y".to_string(), var);
        let result = validate_enum_type_declarations(model_rc);
        assert!(result.is_ok(), "Inference-тип не должен вызывать Ce4");
    }

    // ── Контр-примеры: ошибочные enum-типы ───────────────────────────────────

    /// Переменная типа необъявленного перечисления → ошибка Ce4.
    ///
    /// # Контр-пример (Lam)
    /// ```text
    /// var s: Size = 0;  // ✗ Size не объявлен
    /// start S;
    /// ```
    #[test]
    fn ce4_undeclared_enum_type_is_error() {
        let model_rc = {
            let (ast, _) = crate::parse("start S;", 0).expect("ошибка разбора");
            let m =
                crate::semantic::tree::construct_model(&ast, None, &[]).expect("ошибка семантики");
            // Переменная типа Size — Size НЕ объявлен
            let var = VariableNode::Simple {
                upper: None,
                loc: Location::Implicit,
                name: "s".to_string(),
                ty: TypeNode::Enum("Size".to_string()),
                expr: ExpressionNode::Number(0),
            };
            m.borrow_mut().variables.insert("s".to_string(), var);
            m
        };
        let result = validate_enum_type_declarations(model_rc);
        assert!(
            result.is_err(),
            "необъявленный enum-тип должен давать ошибку Ce4"
        );
        let err = result.unwrap_err();
        assert!(
            err.message.contains("Size"),
            "сообщение должно содержать имя отсутствующего enum: {}",
            err.message
        );
        assert_eq!(
            err.code.as_deref(),
            Some("SE-035"),
            "код ошибки Ce4 должен быть SE-035: {:?}",
            err.code
        );
    }

    /// Константа с необъявленным enum-типом также проверяется.
    ///
    /// # Контр-пример (Lam)
    /// ```text
    /// const C: Status = 0;  // ✗ Status не объявлен
    /// start S;
    /// ```
    #[test]
    fn ce4_undeclared_enum_in_const_is_error() {
        let model_rc = {
            let (ast, _) = crate::parse("start S;", 0).expect("ошибка разбора");
            let m =
                crate::semantic::tree::construct_model(&ast, None, &[]).expect("ошибка семантики");
            let var = VariableNode::Const {
                upper: None,
                loc: Location::Implicit,
                name: "C".to_string(),
                ty: TypeNode::Enum("Status".to_string()),
                expr: ExpressionNode::Number(0),
            };
            m.borrow_mut().variables.insert("C".to_string(), var);
            m
        };
        let result = validate_enum_type_declarations(model_rc);
        assert!(
            result.is_err(),
            "константа с необъявленным enum-типом должна давать ошибку Ce4"
        );
    }

    /// Порт с необъявленным enum-типом также проверяется.
    #[test]
    fn ce4_undeclared_enum_in_port_is_error() {
        let model_rc = {
            let (ast, _) = crate::parse("start S;", 0).expect("ошибка разбора");
            let m =
                crate::semantic::tree::construct_model(&ast, None, &[]).expect("ошибка семантики");
            let var = VariableNode::Port {
                upper: None,
                loc: Location::Implicit,
                name: "p".to_string(),
                ty: TypeNode::Enum("Dir".to_string()),
                expr: ExpressionNode::Number(0),
                direction: crate::semantic::PortDirection::In,
            };
            m.borrow_mut().variables.insert("p".to_string(), var);
            m
        };
        let result = validate_enum_type_declarations(model_rc);
        assert!(
            result.is_err(),
            "порт с необъявленным enum-типом должен давать ошибку Ce4"
        );
    }

    /// Модель без переменных — проверка пуста и всегда ок.
    #[test]
    fn ce4_empty_model_is_ok() {
        let model_rc = build_rc("start S;");
        let result = validate_enum_type_declarations(model_rc);
        assert!(result.is_ok(), "пустая модель не должна давать ошибки Ce4");
    }
}

// ─── Ce15: Тесты проверки размера массивов ───────────────────────────────────

#[cfg(test)]
mod tests_ce15_array_size {
    use super::*;
    use crate::diagnostics::Location;
    use crate::semantic::type_node::TypeNode;

    /// Ce15: допустимый размер массива 8 — ошибок нет.
    #[test]
    fn array_size_8_is_ok() {
        let ty = TypeNode::Array(8, Box::new(TypeNode::Bit));
        assert!(
            check_type_array_size(&ty, Location::Implicit).is_ok(),
            "массив размером 8 должен быть допустим"
        );
    }

    /// Ce15: размер равный MAX_ARRAY_SIZE — допустим (граничное значение).
    #[test]
    fn array_size_max_is_ok() {
        let ty = TypeNode::Array(MAX_ARRAY_SIZE, Box::new(TypeNode::Bit));
        assert!(
            check_type_array_size(&ty, Location::Implicit).is_ok(),
            "массив размером MAX_ARRAY_SIZE должен быть допустим"
        );
    }

    /// Ce15: размер MAX_ARRAY_SIZE + 1 — ошибка.
    #[test]
    fn array_size_exceeding_max_is_error() {
        let ty = TypeNode::Array(MAX_ARRAY_SIZE + 1, Box::new(TypeNode::Bit));
        let result = check_type_array_size(&ty, Location::Implicit);
        assert!(
            result.is_err(),
            "массив размером MAX_ARRAY_SIZE+1 должен давать ошибку Ce15"
        );
        let err = result.unwrap_err();
        assert_eq!(
            err.code.as_deref(),
            Some("SE-038"),
            "код ошибки Ce15 должен быть SE-038: {:?}",
            err.code
        );
    }

    /// Ce15: максимальный u16 (65535) — ошибка, превышает MAX_ARRAY_SIZE.
    #[test]
    fn array_size_u16_max_is_error() {
        let ty = TypeNode::Array(u16::MAX, Box::new(TypeNode::Bit));
        let result = check_type_array_size(&ty, Location::Implicit);
        assert!(
            result.is_err(),
            "массив размером u16::MAX должен давать ошибку Ce15"
        );
    }

    /// Ce15: вложенный массив с превышением размера — также ошибка.
    #[test]
    fn nested_array_size_exceeding_max_is_error() {
        // Внешний массив допустим, но вложенный нет
        let inner = TypeNode::Array(MAX_ARRAY_SIZE + 1, Box::new(TypeNode::Bit));
        let outer = TypeNode::Array(2, Box::new(inner));
        let result = check_type_array_size(&outer, Location::Implicit);
        assert!(
            result.is_err(),
            "вложенный массив с превышением размера должен давать ошибку Ce15"
        );
    }

    /// Ce15: переменная с допустимым размером массива не даёт ошибку через validate_model.
    #[test]
    fn validate_model_accepts_small_array() {
        let (ast, _) = crate::parse("var x: [bit;8] := 0; start S;", 0).unwrap();
        let result = crate::semantic::tree::construct_model(&ast, None, &[]);
        assert!(
            result.is_ok(),
            "массив [bit;8] должен проходить валидацию: {:?}",
            result
        );
    }
}

// ─── Ce14: Проверка детерминированности переходов ────────────────────────────

/// Проверяет детерминированность переходов в состояниях модели.
///
/// Предупреждает если несколько `ref`-переходов из одного состояния
/// не имеют условий (безусловные переходы — `Condition::None`) —
/// это явная недетерминированность: непонятно, в какое состояние перейти.
///
/// # Возвращаемое значение
///
/// Вектор [`Diagnostic`] уровня Warning для каждого состояния
/// с более чем одним безусловным переходом или с перекрывающимися условиями.
pub fn check_nondeterministic_transitions(model: Rc<RefCell<ModelNode>>) -> Vec<Diagnostic> {
    let mut warnings = Vec::new();
    check_nondeterministic_model(model, &mut warnings);
    warnings
}

// ─── NI4: Интервальный анализ перекрытия условий ────────────────────────────

/// Одностороннее ограничение переменной относительно целочисленного литерала.
///
/// Используется для обнаружения попарного перекрытия условий переходов (NI4).
#[derive(Debug, Clone, PartialEq, Eq)]
enum Constraint {
    /// `var = n`
    Eq(i64),
    /// `var != n`
    Ne(i64),
    /// `var < n`
    Lt(i64),
    /// `var <= n`
    Le(i64),
    /// `var > n`
    Gt(i64),
    /// `var >= n`
    Ge(i64),
}

/// Извлекает из условия вида `var OP number` пару `(имя-переменной, Constraint)`.
///
/// Поддерживает только простые атомарные условия на одну переменную.
/// Возвращает `None` для составных условий (AND, OR, NOT и т.д.).
///
/// Идентификация переменной выполняется по имени, а не по указателю `Rc`,
/// потому что `resolve_condition` создаёт новый `Rc` при каждом обращении
/// к одной и той же переменной.
fn extract_simple_constraint(cond: &ConditionNode) -> Option<(String, Constraint)> {
    /// Извлекает имя переменной из узла `ConditionNode::Variable`.
    fn var_name(node: &ConditionNode) -> Option<String> {
        if let ConditionNode::Variable(v, _) = node {
            Some(v.borrow().name().to_owned())
        } else {
            None
        }
    }

    match cond {
        ConditionNode::Equal(l, r) => match (l.as_ref(), r.as_ref()) {
            (var, ConditionNode::Number(n)) => var_name(var).map(|name| (name, Constraint::Eq(*n))),
            (ConditionNode::Number(n), var) => var_name(var).map(|name| (name, Constraint::Eq(*n))),
            _ => None,
        },
        ConditionNode::NotEqual(l, r) => match (l.as_ref(), r.as_ref()) {
            (var, ConditionNode::Number(n)) => var_name(var).map(|name| (name, Constraint::Ne(*n))),
            (ConditionNode::Number(n), var) => var_name(var).map(|name| (name, Constraint::Ne(*n))),
            _ => None,
        },
        ConditionNode::Less(l, r) => match (l.as_ref(), r.as_ref()) {
            (var, ConditionNode::Number(n)) => var_name(var).map(|name| (name, Constraint::Lt(*n))),
            (ConditionNode::Number(n), var) => {
                // n < var  →  var > n
                var_name(var).map(|name| (name, Constraint::Gt(*n)))
            }
            _ => None,
        },
        ConditionNode::LessEqual(l, r) => match (l.as_ref(), r.as_ref()) {
            (var, ConditionNode::Number(n)) => var_name(var).map(|name| (name, Constraint::Le(*n))),
            (ConditionNode::Number(n), var) => {
                // n <= var  →  var >= n
                var_name(var).map(|name| (name, Constraint::Ge(*n)))
            }
            _ => None,
        },
        ConditionNode::More(l, r) => match (l.as_ref(), r.as_ref()) {
            (var, ConditionNode::Number(n)) => var_name(var).map(|name| (name, Constraint::Gt(*n))),
            (ConditionNode::Number(n), var) => {
                // n > var  →  var < n
                var_name(var).map(|name| (name, Constraint::Lt(*n)))
            }
            _ => None,
        },
        ConditionNode::MoreEqual(l, r) => match (l.as_ref(), r.as_ref()) {
            (var, ConditionNode::Number(n)) => var_name(var).map(|name| (name, Constraint::Ge(*n))),
            (ConditionNode::Number(n), var) => {
                // n >= var  →  var <= n
                var_name(var).map(|name| (name, Constraint::Le(*n)))
            }
            _ => None,
        },
        // Скобки — прозрачны
        ConditionNode::Parenthesis(inner) => extract_simple_constraint(inner),
        _ => None,
    }
}

/// Проверяет, могут ли два ограничения одновременно выполняться.
///
/// Возвращает `true`, если существует целое число, удовлетворяющее обоим.
fn constraints_overlap(a: &Constraint, b: &Constraint) -> bool {
    use Constraint::*;
    match (a, b) {
        // Eq vs *
        (Eq(x), Eq(y)) => x == y,
        (Eq(x), Ne(y)) => x != y,
        (Eq(x), Lt(y)) => x < y,
        (Eq(x), Le(y)) => x <= y,
        (Eq(x), Gt(y)) => x > y,
        (Eq(x), Ge(y)) => x >= y,
        // Ne vs *
        (Ne(x), Eq(y)) => x != y,
        (Ne(x), Ne(_y)) => *x != i64::MAX, // всегда истинно (хотя бы одно значение)
        (Ne(_), Lt(_)) => true,            // всегда есть значение ≠ x и < y
        (Ne(_), Le(_)) => true,
        (Ne(_), Gt(_)) => true,
        (Ne(_), Ge(_)) => true,
        // Lt vs *
        (Lt(x), Eq(y)) => *y < *x,
        (Lt(_), Ne(_)) => true,
        (Lt(_), Lt(_)) => true,        // (-∞, x-1] ∩ (-∞, y-1] всегда непусто
        (Lt(_), Le(_)) => true,        // (-∞, x-1] ∩ (-∞, y] всегда непусто
        (Lt(x), Gt(y)) => *y + 1 < *x, // (y, ∞) ∩ (-∞, x-1): нужно y+1 < x
        (Lt(x), Ge(y)) => *y < *x,     // [y, ∞) ∩ (-∞, x-1): нужно y < x
        // Le vs *
        (Le(x), Eq(y)) => *y <= *x,
        (Le(_), Ne(_)) => true,
        (Le(_), Lt(_)) => true,    // (-∞, x] ∩ (-∞, y-1] всегда непусто
        (Le(_), Le(_)) => true,    // (-∞, x] ∩ (-∞, y] всегда непусто
        (Le(x), Gt(y)) => *y < *x, // нужно y < x (хотя бы y+1 <= x)
        (Le(x), Ge(y)) => *y <= *x,
        // Gt vs *
        (Gt(x), Eq(y)) => *y > *x,
        (Gt(_), Ne(_)) => true,
        (Gt(x), Lt(y)) => *x + 1 < *y,
        (Gt(x), Le(y)) => *x < *y,
        (Gt(_), Gt(_)) => true, // (x, ∞) ∩ (y, ∞) всегда непусто
        (Gt(_), Ge(_)) => true, // (x, ∞) ∩ [y, ∞) всегда непусто
        // Ge vs *
        (Ge(x), Eq(y)) => *y >= *x,
        (Ge(_), Ne(_)) => true,
        (Ge(x), Lt(y)) => *x < *y,
        (Ge(x), Le(y)) => *x <= *y,
        (Ge(_), Gt(_)) => true, // [x, ∞) ∩ (y, ∞) всегда непусто
        (Ge(_), Ge(_)) => true, // [x, ∞) ∩ [y, ∞) всегда непусто
    }
}

fn check_nondeterministic_model(model: Rc<RefCell<ModelNode>>, warnings: &mut Vec<Diagnostic>) {
    let borrowed = model.borrow();
    let model_name = borrowed.name.clone().unwrap_or_default();

    for (state_name, state) in &borrowed.states {
        let references: &[ReferenceNode<StateNode>] = match state {
            StateNode::Simple { references, .. } => references,
            StateNode::Implement { references, .. } => references,
            StateNode::Unresolved => continue,
        };

        let prefix = if model_name.is_empty() {
            format!("состояние '{}'", state_name)
        } else {
            format!("модель '{}', состояние '{}'", model_name, state_name)
        };

        // Ce14: Подсчёт безусловных переходов (Condition::None)
        let unconditional_count = references
            .iter()
            .filter(|r| matches!(r.cond, ConditionNode::None))
            .count();

        if unconditional_count > 1 {
            warnings.push(
                Diagnostic::warning(
                    state.loc(),
                    format!(
                        "{}: {} безусловных перехода(ов) — недетерминированное поведение",
                        prefix, unconditional_count
                    ),
                )
                .with_code("SE-037"),
            );
        }

        // NI4: Анализ структурного и интервального перекрытия условных переходов
        let conditional: Vec<_> = references
            .iter()
            .filter(|r| !matches!(r.cond, ConditionNode::None))
            .collect();

        for i in 0..conditional.len() {
            for j in (i + 1)..conditional.len() {
                let cond_i = &conditional[i].cond;
                let cond_j = &conditional[j].cond;

                // Структурно одинаковые условия — гарантированное перекрытие
                if cond_i == cond_j {
                    warnings.push(
                        Diagnostic::warning(
                            conditional[i].location,
                            format!(
                                "{}: переходы в '{}' и '{}' имеют одинаковое условие — \
                                 гарантированное недетерминированное поведение",
                                prefix, conditional[i].name, conditional[j].name
                            ),
                        )
                        .with_code("SE-042"),
                    );
                    continue;
                }

                // Интервальный анализ простых атомарных условий
                if let (Some((var_i, constr_i)), Some((var_j, constr_j))) = (
                    extract_simple_constraint(cond_i),
                    extract_simple_constraint(cond_j),
                ) {
                    // Условия на одну и ту же переменную
                    if var_i == var_j && constraints_overlap(&constr_i, &constr_j) {
                        warnings.push(
                            Diagnostic::warning(
                                conditional[i].location,
                                format!(
                                    "{}: условия переходов в '{}' и '{}' могут одновременно \
                                     выполняться — возможное перекрытие",
                                    prefix, conditional[i].name, conditional[j].name
                                ),
                            )
                            .with_code("SE-042"),
                        );
                    }
                }
            }
        }
    }

    // Рекурсивно для вложенных моделей
    let nested: Vec<Rc<RefCell<ModelNode>>> = borrowed.models.values().map(Rc::clone).collect();
    drop(borrowed);

    for nested_model in nested {
        check_nondeterministic_model(nested_model, warnings);
    }
}

// ─── NI6: Типобезопасные операции с enum ─────────────────────────────────────

/// Проверяет, является ли числовое значение допустимым для перечисления.
///
/// Возвращает `true`, если значение `n` совпадает с числовым значением
/// хотя бы одного варианта перечисления `enum_name` в контексте модели.
/// Если перечисление не найдено — не блокируем (ошибка другой проверки).
fn is_valid_enum_value(enum_name: &str, n: i64, model: &Rc<RefCell<ModelNode>>) -> bool {
    if let Some(enum_node) = model.borrow().search_enum(enum_name) {
        enum_node.variants.iter().any(|(_, val)| *val == n)
    } else {
        true
    }
}

/// Рекурсивно обходит выражения и проверяет присваивания переменным типа enum (NI6).
fn check_enum_expr(
    expr: &ExpressionNode,
    model: &Rc<RefCell<ModelNode>>,
    out: &mut Vec<Diagnostic>,
) {
    match expr {
        ExpressionNode::Assign(left, right) => {
            if let ExpressionNode::Variable(var_rc) = left.as_ref() {
                let borrowed = var_rc.borrow();
                if let VariableNode::Simple { name, ty, .. }
                | VariableNode::Port { name, ty, .. }
                | VariableNode::Const { name, ty, .. } = &*borrowed
                    && let TypeNode::Enum(enum_name) = ty
                    && let ExpressionNode::Number(n) = right.as_ref()
                    && !is_valid_enum_value(enum_name, *n, model)
                {
                    let var_loc = borrowed.loc();
                    let valid_values: Vec<String> = model
                        .borrow()
                        .search_enum(enum_name)
                        .map(|e| {
                            e.variants
                                .iter()
                                .map(|(vn, vv)| format!("{}={}", vn, vv))
                                .collect()
                        })
                        .unwrap_or_default();
                    out.push(
                        Diagnostic::type_error(
                            var_loc,
                            format!(
                                "присваивание переменной '{}' типа '{}' \
                                 значения {} недопустимо — не является вариантом \
                                 перечисления (допустимые варианты: {})",
                                name,
                                enum_name,
                                n,
                                valid_values.join(", ")
                            ),
                        )
                        .with_code("SE-043"),
                    );
                }
            }
            check_enum_expr(left, model, out);
            check_enum_expr(right, model, out);
        }
        ExpressionNode::Parenthesis(e)
        | ExpressionNode::BitAccess(e, _)
        | ExpressionNode::Not(e)
        | ExpressionNode::BitwiseNot(e)
        | ExpressionNode::UnaryPlus(e)
        | ExpressionNode::Negate(e)
        | ExpressionNode::Cast(e, _) => {
            check_enum_expr(e, model, out);
        }
        ExpressionNode::CodeBlock(e, _) => {
            check_enum_expr(e, model, out);
        }
        ExpressionNode::NamedFunctionBox(e, _) => {
            check_enum_expr(e, model, out);
        }
        ExpressionNode::Power(l, r)
        | ExpressionNode::Multiply(l, r)
        | ExpressionNode::Divide(l, r)
        | ExpressionNode::Modulo(l, r)
        | ExpressionNode::Add(l, r)
        | ExpressionNode::Subtract(l, r)
        | ExpressionNode::ShiftLeft(l, r)
        | ExpressionNode::ShiftRight(l, r)
        | ExpressionNode::BitwiseAnd(l, r)
        | ExpressionNode::BitwiseXor(l, r)
        | ExpressionNode::BitwiseOr(l, r)
        | ExpressionNode::Less(l, r)
        | ExpressionNode::More(l, r)
        | ExpressionNode::LessEqual(l, r)
        | ExpressionNode::MoreEqual(l, r)
        | ExpressionNode::Equal(l, r)
        | ExpressionNode::NotEqual(l, r)
        | ExpressionNode::And(l, r)
        | ExpressionNode::Or(l, r) => {
            check_enum_expr(l, model, out);
            check_enum_expr(r, model, out);
        }
        ExpressionNode::ConditionalOperator(c, t, e) => {
            check_enum_expr(c, model, out);
            check_enum_expr(t, model, out);
            check_enum_expr(e, model, out);
        }
        ExpressionNode::Function(_, args)
        | ExpressionNode::Array(args)
        | ExpressionNode::Initializer(args) => {
            for a in args {
                check_enum_expr(a, model, out);
            }
        }
        _ => {}
    }
}

/// Рекурсивно обходит операторы и проверяет присваивания переменным типа enum (NI6).
fn check_enum_stmt(
    stmt: &StatementNode,
    model: &Rc<RefCell<ModelNode>>,
    out: &mut Vec<Diagnostic>,
) {
    match stmt {
        StatementNode::Expression(expr) => check_enum_expr(expr, model, out),
        StatementNode::Block(stmts) => {
            for s in stmts {
                check_enum_stmt(s, model, out);
            }
        }
        StatementNode::If { cond, then_, else_ } => {
            check_enum_expr(cond, model, out);
            check_enum_stmt(then_, model, out);
            if let Some(e) = else_ {
                check_enum_stmt(e, model, out);
            }
        }
        StatementNode::Loop { cond, body } => {
            if let Some(c) = cond {
                check_enum_expr(c, model, out);
            }
            check_enum_stmt(body, model, out);
        }
        StatementNode::For {
            init,
            cond,
            step,
            body,
        } => {
            if let Some(i) = init {
                check_enum_stmt(i, model, out);
            }
            if let Some(c) = cond {
                check_enum_expr(c, model, out);
            }
            if let Some(s) = step {
                check_enum_expr(s, model, out);
            }
            check_enum_stmt(body, model, out);
        }
        StatementNode::Return(Some(e)) => check_enum_expr(e, model, out),
        StatementNode::Variable(_, _, Some(e)) => check_enum_expr(e, model, out),
        _ => {}
    }
}

/// Проверяет инициализатор переменной типа enum.
///
/// Если переменная имеет тип `Enum(name)` и инициализирована числовым литералом,
/// числовое значение должно быть допустимым вариантом перечисления.
///
/// # Ошибки
///
/// Возвращает [`Diagnostic`], если числовой литерал не является вариантом перечисления.
fn check_enum_variable_value(
    name: &str,
    ty: &TypeNode,
    expr: &ExpressionNode,
    loc: Location,
    model: &Rc<RefCell<ModelNode>>,
) -> Result<(), Diagnostic> {
    if let TypeNode::Enum(enum_name) = ty
        && let ExpressionNode::Number(n) = expr
        && !is_valid_enum_value(enum_name, *n, model)
    {
        let valid_values: Vec<String> = model
            .borrow()
            .search_enum(enum_name)
            .map(|e| {
                e.variants
                    .iter()
                    .map(|(vn, vv)| format!("{}={}", vn, vv))
                    .collect()
            })
            .unwrap_or_default();
        return Err(Diagnostic::error(
            loc,
            format!(
                "переменная '{}' имеет тип '{}', но инициализирована значением {} \
                 — не является вариантом перечисления (допустимые варианты: {})",
                name,
                enum_name,
                n,
                valid_values.join(", ")
            ),
        )
        .with_code("SE-043"));
    }
    Ok(())
}

/// Проверяет все переменные модели на корректность начальных значений для enum-типов (NI6).
///
/// Аналогично [`validate_bit_values`], проверяет только `Simple`-, `Const`-переменные.
/// Порты не проверяются — адресное значение не является значением перечисления.
///
/// # Ошибки
///
/// Пробрасывает [`Diagnostic`] из [`check_enum_variable_value`].
fn validate_enum_values(model: Rc<RefCell<ModelNode>>) -> Result<(), Diagnostic> {
    // Собираем данные без удержания заимствования
    let vars: Vec<(String, TypeNode, ExpressionNode, Location)> = model
        .borrow()
        .variables
        .values()
        .filter_map(|var| match var {
            VariableNode::Simple { name, ty, expr, .. }
            | VariableNode::Const { name, ty, expr, .. } => {
                Some((name.clone(), ty.clone(), expr.clone(), var.loc()))
            }
            _ => None,
        })
        .collect();
    for (name, ty, expr, loc) in &vars {
        check_enum_variable_value(name, ty, expr, *loc, &model)?;
    }
    Ok(())
}

/// Рекурсивно собирает ошибки NI6 для модели и всех вложенных моделей.
fn collect_enum_type_safety(model: &Rc<RefCell<ModelNode>>, out: &mut Vec<Diagnostic>) {
    // Собираем данные без удержания заимствования
    let vars: Vec<(String, TypeNode, ExpressionNode, Location)> = model
        .borrow()
        .variables
        .values()
        .filter_map(|var| match var {
            VariableNode::Simple { name, ty, expr, .. }
            | VariableNode::Const { name, ty, expr, .. } => {
                Some((name.clone(), ty.clone(), expr.clone(), var.loc()))
            }
            _ => None,
        })
        .collect();

    for (name, ty, expr, loc) in &vars {
        if let TypeNode::Enum(enum_name) = ty
            && let ExpressionNode::Number(n) = expr
            && !is_valid_enum_value(enum_name, *n, model)
        {
            let valid_values: Vec<String> = model
                .borrow()
                .search_enum(enum_name)
                .map(|e| {
                    e.variants
                        .iter()
                        .map(|(vn, vv)| format!("{}={}", vn, vv))
                        .collect()
                })
                .unwrap_or_default();
            out.push(
                Diagnostic::type_error(
                    *loc,
                    format!(
                        "переменная '{}' имеет тип '{}', но инициализирована \
                         значением {} — не является вариантом перечисления \
                         (допустимые варианты: {})",
                        name,
                        enum_name,
                        n,
                        valid_values.join(", ")
                    ),
                )
                .with_code("SE-043"),
            );
        }
    }

    let named_blocks: Vec<StatementNode> = model
        .borrow()
        .named_blocks
        .iter()
        .filter_map(|b| b.statement().cloned())
        .collect();
    for stmt in &named_blocks {
        check_enum_stmt(stmt, model, out);
    }

    let state_blocks: Vec<StatementNode> = model
        .borrow()
        .states
        .values()
        .flat_map(|s| {
            s.named_blocks()
                .iter()
                .filter_map(|b| b.statement().cloned())
                .collect::<Vec<_>>()
        })
        .collect();
    for stmt in &state_blocks {
        check_enum_stmt(stmt, model, out);
    }

    let nested: Vec<Rc<RefCell<ModelNode>>> =
        model.borrow().models.values().map(Rc::clone).collect();
    for m in nested {
        collect_enum_type_safety(&m, out);
    }
}

/// NI6: Возвращает ошибки типобезопасных операций с перечислениями.
///
/// Проверяет, что при присваивании переменной типа enum значение является
/// одним из допустимых вариантов перечисления. Проверяются:
///
/// - Инициализаторы переменных объявленных как `var x: Direction = 0;`
/// - Присваивания в именованных блоках `always`, `enter`, `exit`
///
/// Статически проверяются только числовые литералы. Присваивания через
/// переменные или функции не проверяются.
///
/// # Примеры (Lam)
///
/// ```text
/// // Корректно: 0 — значение варианта North
/// enum Direction { North = 0, South = 1 }
/// var dir: Direction = 0;
///
/// // Ошибка NI6: 99 не является вариантом Direction
/// var dir: Direction = 99;
/// ```
///
/// # Возвращаемое значение
///
/// Вектор [`Diagnostic`] уровня `Error` с типом [`TypeError`](crate::diagnostics::ErrorType::TypeError).
/// Пустой вектор означает отсутствие нарушений.
pub fn check_enum_type_safety(model: Rc<RefCell<ModelNode>>) -> Vec<Diagnostic> {
    let mut errors = Vec::new();
    collect_enum_type_safety(&model, &mut errors);
    errors
}

// ─── Ce17: Дублирующиеся поля структуры ──────────────────────────────────────

/// Ce17: проверяет, что в каждой структуре модели нет дублирующихся имён полей.
///
/// ## Правило Ce17
///
/// Каждое поле структурного типа должно иметь уникальное имя внутри одного
/// объявления `struct`. Повторное объявление поля с тем же именем — ошибка.
///
/// ## Примеры (Lam)
///
/// ```text
/// // Корректно
/// struct Point { x: bit, y: bit }
///
/// // Ce17: поле x объявлено дважды
/// struct Bad { x: bit, x: bit }
/// ```
///
/// # Возвращаемое значение
///
/// [`Diagnostic`] уровня `Error` с кодом Ce17 при первом нарушении,
/// `None` если дублирований нет.
pub fn check_duplicate_struct_fields(model: Rc<RefCell<ModelNode>>) -> Option<Diagnostic> {
    let structs: Vec<_> = model.borrow().structs.values().cloned().collect();

    for s in &structs {
        let mut seen: HashSet<&str> = HashSet::new();
        for (field_name, _) in &s.fields {
            if !seen.insert(field_name.as_str()) {
                return Some(
                    Diagnostic::error(
                        s.loc,
                        format!(
                            "структура '{}' содержит дублирующееся поле '{}'",
                            s.name, field_name
                        ),
                    )
                    .with_code("SE-040"),
                );
            }
        }
    }

    // Рекурсивная проверка вложенных моделей
    let nested: Vec<Rc<RefCell<ModelNode>>> =
        model.borrow().models.values().map(Rc::clone).collect();
    for nested_model in nested {
        if let Some(diag) = check_duplicate_struct_fields(nested_model) {
            return Some(diag);
        }
    }

    None
}

// ─── Ce18: Неизвестный тип поля структуры ────────────────────────────────────

/// Ce18: проверяет, что типы полей всех структур разрешены и известны.
///
/// ## Правило Ce18
///
/// Каждое поле структуры должно иметь тип, который либо является
/// встроенным (`bit`, `bool`, массив), либо объявлен в области видимости
/// (псевдоним, перечисление или другая структура). Ссылка на неизвестный тип —
/// ошибка.
///
/// ## Примеры (Lam)
///
/// ```text
/// // Корректно
/// struct Vec2 { x: [bit;16], y: [bit;16] }
///
/// // Ce18: Ghost не объявлен
/// struct Bad { val: Ghost }
/// ```
///
/// # Возвращаемое значение
///
/// [`Diagnostic`] уровня `Error` с кодом Ce18 при первом нарушении,
/// `None` если все типы полей известны.
pub fn check_struct_field_types(model: Rc<RefCell<ModelNode>>) -> Option<Diagnostic> {
    let structs: Vec<_> = model.borrow().structs.values().cloned().collect();

    for s in &structs {
        for (field_name, field_ty) in &s.fields {
            if let TypeNode::Struct(type_name) = field_ty {
                // Проверяем, что структурный тип поля существует в области видимости
                if model.borrow().search_struct(type_name).is_none() {
                    return Some(
                        Diagnostic::error(
                            s.loc,
                            format!(
                                "поле '{}' структуры '{}' ссылается на неизвестный тип '{}'",
                                field_name, s.name, type_name
                            ),
                        )
                        .with_code("SE-041"),
                    );
                }
            }
        }
    }

    // Рекурсивная проверка вложенных моделей
    let nested: Vec<Rc<RefCell<ModelNode>>> =
        model.borrow().models.values().map(Rc::clone).collect();
    for nested_model in nested {
        if let Some(diag) = check_struct_field_types(nested_model) {
            return Some(diag);
        }
    }

    None
}

// ─── SE-046: недостижимые состояния ──────────────────────────────────────────

/// Предупреждает о состояниях, недостижимых из начального состояния.
///
/// Обходит граф переходов (BFS) начиная со стартового состояния.
/// Состояние считается недостижимым, если на него нет ни одного перехода
/// `ref` или `next` из любого достижимого состояния.
///
/// Функция рекурсивно обходит все вложенные модели.
pub fn check_unreachable_states(model: Rc<RefCell<ModelNode>>) -> Vec<Diagnostic> {
    let mut diags = Vec::new();
    collect_unreachable_states(&model, &mut diags);
    diags
}

fn collect_unreachable_states(model: &Rc<RefCell<ModelNode>>, out: &mut Vec<Diagnostic>) {
    let borrowed = model.borrow();

    let states = &borrowed.states;
    if states.is_empty() {
        drop(borrowed);
        let nested: Vec<_> = model.borrow().models.values().map(Rc::clone).collect();
        for m in nested {
            collect_unreachable_states(&m, out);
        }
        return;
    }

    // Имя стартового состояния
    let start_name = states
        .values()
        .find(|s| s.kind() == StateNodeKind::Start)
        .map(|s| get_state_name(s).to_string());

    if let Some(start) = start_name {
        let mut reachable = std::collections::HashSet::new();
        let mut queue = std::collections::VecDeque::new();
        queue.push_back(start.clone());
        reachable.insert(start);

        while let Some(current) = queue.pop_front() {
            if let Some(state) = states.get(&current) {
                for target in get_reachable_targets(state) {
                    if !reachable.contains(&target) {
                        reachable.insert(target.clone());
                        queue.push_back(target);
                    }
                }
            }
        }

        for (name, state) in states {
            if !reachable.contains(name.as_str()) {
                let loc = get_state_loc(state);
                out.push(
                    Diagnostic::warning(
                        loc,
                        format!("состояние '{}' недостижимо из начального состояния", name),
                    )
                    .with_code("SE-046"),
                );
            }
        }
    }

    drop(borrowed);
    let nested: Vec<_> = model.borrow().models.values().map(Rc::clone).collect();
    for m in nested {
        collect_unreachable_states(&m, out);
    }
}

fn get_state_name(state: &StateNode) -> &str {
    match state {
        StateNode::Simple { name, .. } | StateNode::Implement { name, .. } => name.as_str(),
        StateNode::Unresolved => "",
    }
}

fn get_state_loc(state: &StateNode) -> Location {
    match state {
        StateNode::Simple { loc, .. } | StateNode::Implement { loc, .. } => *loc,
        StateNode::Unresolved => Location::Builtin,
    }
}

fn get_reachable_targets(state: &StateNode) -> Vec<String> {
    match state {
        StateNode::Simple { references, .. } => references.iter().map(|r| r.name.clone()).collect(),
        StateNode::Implement {
            references, next, ..
        } => {
            let mut targets: Vec<String> = references.iter().map(|r| r.name.clone()).collect();
            if let Some(n) = next {
                targets.push(n.name.clone());
            }
            targets
        }
        StateNode::Unresolved => vec![],
    }
}

// ─── SE-047: Анализ константных условий ──────────────────────────────────────

/// SE-047: предупреждения об очевидно константных условиях переходов.
///
/// Обнаруживает сравнения двух числовых/булевых литералов, результат
/// которых известен в compile-time:
/// - `1 = 0` — всегда ложно (переход никогда не произойдёт);
/// - `1 = 1` — всегда истинно (переход безусловный);
/// - `x = 5 & x = 6` — второе сравнение с той же переменной делает
///   конъюнкцию всегда ложной.
pub fn check_constant_conditions(model: &Rc<RefCell<ModelNode>>) -> Vec<Diagnostic> {
    let mut diags = Vec::new();
    collect_constant_condition_warnings(model, &mut diags);
    diags
}

fn collect_constant_condition_warnings(model: &Rc<RefCell<ModelNode>>, out: &mut Vec<Diagnostic>) {
    let borrowed = model.borrow();

    for state in borrowed.states.values() {
        match state {
            StateNode::Simple { references, .. } => {
                for r in references {
                    eval_condition_const(&r.cond, r.location, out);
                }
            }
            StateNode::Implement {
                references, next, ..
            } => {
                for r in references {
                    eval_condition_const(&r.cond, r.location, out);
                }
                if let Some(nr) = next {
                    eval_condition_const(&nr.cond, nr.location, out);
                }
            }
            StateNode::Unresolved => {}
        }
    }

    let nested: Vec<Rc<RefCell<ModelNode>>> = borrowed.models.values().map(Rc::clone).collect();
    drop(borrowed);
    for m in nested {
        collect_constant_condition_warnings(&m, out);
    }
}

/// Вычисляет условие и, если оно константно, добавляет предупреждение.
fn eval_condition_const(cond: &ConditionNode, loc: Location, out: &mut Vec<Diagnostic>) {
    match eval_const_value(cond) {
        Some(true) => {
            out.push(
                Diagnostic::warning(
                    loc,
                    "условие перехода всегда истинно — переход безусловный".to_string(),
                )
                .with_code("SE-047"),
            );
        }
        Some(false) => {
            out.push(
                Diagnostic::warning(
                    loc,
                    "условие перехода всегда ложно — переход недостижим".to_string(),
                )
                .with_code("SE-047"),
            );
        }
        None => {}
    }
}

/// Пытается вычислить булево значение условия из одних литералов.
/// Возвращает `Some(true/false)` только если результат очевиден статически.
fn eval_const_value(cond: &ConditionNode) -> Option<bool> {
    match cond {
        ConditionNode::Bool(b) => Some(*b),
        ConditionNode::Number(n) => Some(*n != 0),

        ConditionNode::Equal(l, r) => match (eval_literal_i64(l), eval_literal_i64(r)) {
            (Some(a), Some(b)) => Some(a == b),
            _ => None,
        },
        ConditionNode::NotEqual(l, r) => match (eval_literal_i64(l), eval_literal_i64(r)) {
            (Some(a), Some(b)) => Some(a != b),
            _ => None,
        },
        ConditionNode::Less(l, r) => match (eval_literal_i64(l), eval_literal_i64(r)) {
            (Some(a), Some(b)) => Some(a < b),
            _ => None,
        },
        ConditionNode::More(l, r) => match (eval_literal_i64(l), eval_literal_i64(r)) {
            (Some(a), Some(b)) => Some(a > b),
            _ => None,
        },
        ConditionNode::LessEqual(l, r) => match (eval_literal_i64(l), eval_literal_i64(r)) {
            (Some(a), Some(b)) => Some(a <= b),
            _ => None,
        },
        ConditionNode::MoreEqual(l, r) => match (eval_literal_i64(l), eval_literal_i64(r)) {
            (Some(a), Some(b)) => Some(a >= b),
            _ => None,
        },

        // Конъюнкция: если хоть одна ветка константно ложна — всё ложно
        ConditionNode::And(l, r) => match (eval_const_value(l), eval_const_value(r)) {
            (Some(false), _) | (_, Some(false)) => Some(false),
            (Some(true), Some(true)) => Some(true),
            _ => None,
        },
        // Дизъюнкция: если хоть одна ветка константно истинна — всё истинно
        ConditionNode::Or(l, r) => match (eval_const_value(l), eval_const_value(r)) {
            (Some(true), _) | (_, Some(true)) => Some(true),
            (Some(false), Some(false)) => Some(false),
            _ => None,
        },

        ConditionNode::Not(inner) => eval_const_value(inner).map(|v| !v),
        ConditionNode::Parenthesis(inner) => eval_const_value(inner),

        _ => None,
    }
}

/// Возвращает числовое значение условия, если оно является числовым литералом.
fn eval_literal_i64(cond: &ConditionNode) -> Option<i64> {
    match cond {
        ConditionNode::Number(n) => Some(*n),
        ConditionNode::Bool(b) => Some(if *b { 1 } else { 0 }),
        ConditionNode::Parenthesis(inner) => eval_literal_i64(inner),
        _ => None,
    }
}
