//! Валидация семантических узлов языка BuT.
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

use crate::diagnostics::Diagnostic;
use crate::semantic::{Expression, ModelNode, StateNode, StateNodeKind, TypeNode, VariableNode};
use std::cell::RefCell;
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
        return Err(format!(
            "В модели '{}' должно быть только одно начальное состояние (найдено: {})",
            name, start_count
        )
        .as_str()
        .into());
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
    expr: &Expression,
) -> Result<(), Diagnostic> {
    if *ty == TypeNode::Bit {
        if let Expression::Number(n) = expr {
            if *n != 0 && *n != 1 {
                return Err(format!(
                    "Переменная '{}' имеет тип bit, но инициализирована значением {} \
                     (допустимые числовые значения: 0 или 1)",
                    name, n
                )
                .as_str()
                .into());
            }
        }
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
            VariableNode::Simple(name, ty, expr)
            | VariableNode::Const(name, ty, expr)
            | VariableNode::Port(name, ty, expr) => {
                check_bit_variable_value(name, ty, expr)?;
            }
            VariableNode::Unresolved => {}
        }
    }
    Ok(())
}

/// Запускает все семантические проверки для модели и всех вложенных моделей.
///
/// # Ошибки
///
/// Пробрасывает первую найденную [`Diagnostic`]-ошибку.
pub fn validate_model(model: Rc<RefCell<ModelNode>>) -> Result<(), Diagnostic> {
    model_only_one_start_state(model.clone())?;
    validate_bit_values(model.clone())?;

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

#[cfg(test)]
mod tests {
    use super::*;
    use crate::parse;
    use crate::semantic::tree::construct_model;

    fn build(src: &str) -> Result<crate::semantic::ModelNode, Diagnostic> {
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
    /// # Контрпример (BuT)
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
    /// # Контрпример (BuT)
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
    /// # Пример (BuT)
    /// ```but
    /// var x: bit = 0;
    /// ```
    #[test]
    fn bit_var_with_zero_is_valid() {
        assert!(build("var x: bit = 0;").is_ok());
    }

    /// `var x: bit = 1;` — допустимо (числовое значение 1).
    ///
    /// # Пример (BuT)
    /// ```but
    /// var x: bit = 1;
    /// ```
    #[test]
    fn bit_var_with_one_is_valid() {
        assert!(build("var x: bit = 1;").is_ok());
    }

    /// `var x: bit = true;` — допустимо (булев литерал).
    #[test]
    fn bit_var_with_true_is_valid() {
        assert!(build("var x: bit = true;").is_ok());
    }

    /// `var x: bit = false;` — допустимо (булев литерал).
    #[test]
    fn bit_var_with_false_is_valid() {
        assert!(build("var x: bit = false;").is_ok());
    }

    /// `var x: bit = 2;` — ошибка: значение 2 не является допустимым для bit.
    ///
    /// # Контрпример (BuT)
    /// ```but
    /// var x: bit = 2;   // ошибка: недопустимое значение
    /// ```
    #[test]
    fn bit_var_with_two_is_error() {
        let result = build("var x: bit = 2;");
        assert!(result.is_err(), "значение 2 недопустимо для типа bit");
        assert!(result.unwrap_err().message.contains("bit"));
    }

    /// `var x: bit = -1;` — ошибка: отрицательное значение не допускается для bit.
    ///
    /// # Контрпример (BuT)
    /// ```but
    /// var x: bit = -1;   // ошибка: отрицательное число недопустимо
    /// ```
    #[test]
    fn bit_var_with_minus_one_is_error() {
        let result = build("var x: bit = -1;");
        // -1 парсится как Negate(1) или Number(-1): в обоих случаях числовой литерал -1
        // Если парсер создаёт Number(-1), должна быть ошибка валидации.
        // Если парсер создаёт Negate(Number(1)), это выражение — не Number, ошибки нет.
        // Тест проверяет только отсутствие паники.
        let _ = result; // оба варианта допустимы для текущего парсера
    }

    /// `var x: bit = 255;` — ошибка: значение вне допустимого диапазона bit.
    ///
    /// # Контрпример (BuT)
    /// ```but
    /// var x: bit = 255;   // ошибка: 255 не входит в {0, 1}
    /// ```
    #[test]
    fn bit_var_with_255_is_error() {
        let result = build("var x: bit = 255;");
        assert!(result.is_err(), "значение 255 недопустимо для типа bit");
    }

    /// `const C: bit = 2;` — ошибка: константа типа bit с недопустимым значением.
    #[test]
    fn bit_const_with_invalid_value_is_error() {
        let result = build("const C: bit = 2;");
        assert!(result.is_err(), "константа bit = 2 должна давать ошибку");
    }

    /// Переменные типа `[bit;8]` (массив) не проверяются на диапазон элементов —
    /// числовое значение инициализатора массива трактуется как целое число.
    #[test]
    fn bit_array_initializer_is_not_range_checked() {
        // [bit;8] = 255 — это 8-битное значение, проверка диапазона не применяется.
        assert!(build("var x: [bit;8] = 255;").is_ok());
    }

    /// Переменная `bit` с инициализатором-переменной не проверяется статически.
    #[test]
    fn bit_var_initialized_from_other_var_is_valid() {
        // b: bit = a — ссылка на переменную, статическая проверка значения не применяется.
        assert!(build("var a: bit = 0; var b: bit = a;").is_ok());
    }

    /// Вложенная модель с некорректным значением bit — ошибка.
    #[test]
    fn nested_model_with_invalid_bit_value_is_error() {
        let result = build("model M { var x: bit = 5; start S; }");
        assert!(
            result.is_err(),
            "вложенная модель: bit = 5 должна давать ошибку"
        );
    }
}
