//! Вывод типов для переменных языка BuT.
//!
//! Основная функция [`type_inference`] обходит таблицу переменных и для
//! каждой переменной с [`TypeNode::Inference`] (тип не задан явно) вызывает
//! [`extract_type`], чтобы определить тип из инициализирующего выражения.
//!
//! ## Алгоритм вывода
//!
//! 1. Числовой литерал (`Number`) → `Bit`
//! 2. Булевый литерал (`Bool`) → `Bit`
//! 3. Вещественный литерал (`Rational`) → `Rational`
//! 4. Переменная → тип referenced переменной
//! 5. Условие → `Bit`
//! 6. Арифметика → наиболее «широкий» тип из операндов (`Bit` < `Rational`)
//! 7. Логика/сравнение → `Bit`
//! 8. Скобки → тип внутреннего выражения
//! 9. Приведение типа (`as T`) → `T`
//! 10. Прочее → `Unsupported`

use crate::diagnostics::Diagnostic;
use crate::parser::ast::Type;
use crate::semantic::{Expression, ModelNode, TypeNode, VariableNode};
use std::cell::RefCell;
use std::collections::HashMap;
use std::rc::Rc;

/// Запускает вывод типов для всех переменных с незаданным типом.
///
/// Для каждой переменной [`VariableNode::Simple`] или [`VariableNode::Const`]
/// с типом [`TypeNode::Inference`] вызывает [`extract_type`] и заменяет
/// `Inference` на выведенный тип.
///
/// # Ошибки
///
/// Пробрасывает [`Diagnostic`] из [`extract_type`].
pub fn type_inference(
    variables: &mut HashMap<String, VariableNode>,
    model: Rc<RefCell<ModelNode>>,
) -> Result<HashMap<String, VariableNode>, Diagnostic> {
    for (name, var) in variables.clone() {
        match var {
            VariableNode::Simple(_, TypeNode::Inference, ref expr) => {
                let typ = extract_type(expr, model.clone())?;
                variables.insert(name.clone(), VariableNode::Simple(name.clone(), typ, expr.clone()));
            }
            VariableNode::Const(_, TypeNode::Inference, ref expr) => {
                let typ = extract_type(expr, model.clone())?;
                variables.insert(name.clone(), VariableNode::Const(name.clone(), typ, expr.clone()));
            }
            _ => {}
        }
    }
    Ok(variables.clone())
}

// ── Вспомогательные функции ────────────────────────────────────────────────────

/// Возвращает тип переменной из её узла.
#[inline]
fn type_of_var(var: &VariableNode) -> TypeNode {
    match var {
        VariableNode::Simple(_, ty, _) => ty.clone(),
        VariableNode::Port(_, ty, _) => ty.clone(),
        VariableNode::Const(_, ty, _) => ty.clone(),
        VariableNode::Unresolved => TypeNode::Inference,
    }
}

/// Возвращает наиболее «широкий» тип из двух.
///
/// Порядок расширения: `Bit` < `Array` < `Rational`.
/// Если один из типов — `Rational`, результат — `Rational`.
/// Если оба `Bit` — результат `Bit`. Иначе — `Unsupported`.
#[inline]
fn wider_type(a: TypeNode, b: TypeNode) -> TypeNode {
    match (&a, &b) {
        (TypeNode::Rational, _) | (_, TypeNode::Rational) => TypeNode::Rational,
        (TypeNode::Bit, TypeNode::Bit) => TypeNode::Bit,
        (TypeNode::Array(n, t), _) => TypeNode::Array(*n, t.clone()),
        (_, TypeNode::Array(n, t)) => TypeNode::Array(*n, t.clone()),
        _ => TypeNode::Unsupported,
    }
}

/// Преобразует АСД-тип [`Type`] в семантический [`TypeNode`].
///
/// Используется при выводе типа для выражений `as T` (приведение типа).
/// Псевдонимы типов (`Alias`) и функциональные типы возвращают `Unsupported`.
fn ast_type_to_node(ty: &Type) -> TypeNode {
    match ty {
        Type::Bit | Type::Bool => TypeNode::Bit,
        Type::Rational => TypeNode::Rational,
        Type::Array {
            element_count,
            element_type,
            ..
        } => TypeNode::Array(*element_count, Box::new(ast_type_to_node(element_type))),
        _ => TypeNode::Unsupported,
    }
}

/// Выводит тип семантического выражения.
///
/// Рекурсивно обходит выражение и возвращает [`TypeNode`] в соответствии
/// с описанными в модуле правилами вывода.
///
/// # Ошибки
///
/// В текущей реализации всегда успешен. Возвращает `Unsupported`
/// для выражений, тип которых не поддаётся автоматическому выводу
/// (например, вызовы функций с неизвестной сигнатурой).
fn extract_type(expr: &Expression, model: Rc<RefCell<ModelNode>>) -> Result<TypeNode, Diagnostic> {
    match expr {
        // ── Литералы ──────────────────────────────────────────────────────────
        Expression::Bool(_) => Ok(TypeNode::Bit),
        Expression::Number(_) => Ok(TypeNode::Bit),
        Expression::Rational(_, _) => Ok(TypeNode::Rational),
        Expression::String(_) | Expression::Address(_, _) => Ok(TypeNode::Unsupported),

        // ── Идентификаторы ────────────────────────────────────────────────────
        Expression::Variable(var_rc) => Ok(type_of_var(&var_rc.borrow())),
        // Условия всегда вычисляются в булев (1-битный) результат.
        Expression::Condition(_) => Ok(TypeNode::Bit),
        Expression::Model(_) => Ok(TypeNode::Unsupported),

        // ── Скобки ────────────────────────────────────────────────────────────
        Expression::Parenthesis(inner) => extract_type(inner, model),

        // ── Логические операции и сравнения → Bit ─────────────────────────────
        Expression::Not(_)
        | Expression::And(_, _)
        | Expression::Or(_, _)
        | Expression::Equal(_, _)
        | Expression::NotEqual(_, _)
        | Expression::Less(_, _)
        | Expression::More(_, _)
        | Expression::LessEqual(_, _)
        | Expression::MoreEqual(_, _) => Ok(TypeNode::Bit),

        // ── Унарные операции → тип операнда ──────────────────────────────────
        Expression::BitwiseNot(e) | Expression::UnaryPlus(e) | Expression::Negate(e) => {
            extract_type(e, model)
        }

        // ── Арифметические бинарные операции → наиболее широкий тип ──────────
        Expression::Add(l, r)
        | Expression::Subtract(l, r)
        | Expression::Multiply(l, r)
        | Expression::Divide(l, r)
        | Expression::Modulo(l, r)
        | Expression::Power(l, r) => {
            let lt = extract_type(l, model.clone())?;
            let rt = extract_type(r, model)?;
            Ok(wider_type(lt, rt))
        }

        // ── Побитовые бинарные операции → наиболее широкий тип ───────────────
        Expression::BitwiseAnd(l, r)
        | Expression::BitwiseXor(l, r)
        | Expression::BitwiseOr(l, r)
        | Expression::ShiftLeft(l, r)
        | Expression::ShiftRight(l, r) => {
            let lt = extract_type(l, model.clone())?;
            let rt = extract_type(r, model)?;
            Ok(wider_type(lt, rt))
        }

        // ── Тернарный оператор → тип наиболее широкой ветви ──────────────────
        Expression::ConditionalOperator(_, then_e, else_e) => {
            let tt = extract_type(then_e, model.clone())?;
            let et = extract_type(else_e, model)?;
            Ok(wider_type(tt, et))
        }

        // ── Присваивание → тип правой части ──────────────────────────────────
        Expression::Assign(_, r) => extract_type(r, model),

        // ── Обращение к массиву → тип элемента ───────────────────────────────
        Expression::ArraySubscript(var_rc, _) => {
            match type_of_var(&var_rc.borrow()) {
                TypeNode::Array(_, elem_type) => Ok(*elem_type),
                other => Ok(other),
            }
        }

        // ── Срез массива → тип элемента ──────────────────────────────────────
        Expression::ArraySlice(var_rc, _, _) => {
            match type_of_var(&var_rc.borrow()) {
                TypeNode::Array(_, elem_type) => Ok(*elem_type),
                other => Ok(other),
            }
        }

        // ── Массивный литерал → Array(N, тип_элемента) ───────────────────────
        Expression::Array(items) => {
            let n = items.len() as u16;
            if items.is_empty() {
                return Ok(TypeNode::Array(0, Box::new(TypeNode::Bit)));
            }
            let elem_type = extract_type(&items[0], model)?;
            Ok(TypeNode::Array(n, Box::new(elem_type)))
        }

        // ── Инициализатор структуры → тип первого элемента ───────────────────
        Expression::Initializer(items) => {
            if items.is_empty() {
                return Ok(TypeNode::Unsupported);
            }
            let n = items.len() as u16;
            let elem_type = extract_type(&items[0], model)?;
            Ok(TypeNode::Array(n, Box::new(elem_type)))
        }

        // ── Приведение типа → результирующий тип ─────────────────────────────
        Expression::Cast(_, ty) => Ok(ast_type_to_node(ty)),
        Expression::Type(ty) => Ok(ast_type_to_node(ty)),

        // ── Выражения без выводимого типа ────────────────────────────────────
        Expression::BitAccess(_, _)
        | Expression::Function(_, _)
        | Expression::CodeBlock(_, _)
        | Expression::NamedFunctionBox(_, _)
        | Expression::List(_)
        | Expression::None
        | Expression::Unresolved(_) => Ok(TypeNode::Unsupported),
    }
}

// ── Тесты ─────────────────────────────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use super::*;
    use crate::parse;
    use crate::semantic::tree::construct_model;

    /// Строит модель из BuT-кода и возвращает корневой ModelNode.
    fn build(src: &str) -> Result<ModelNode, Diagnostic> {
        let (ast, _) = parse(src, 0).expect("ошибка разбора");
        construct_model(&ast, None, &[]).map(|m| m.take())
    }

    // ── Тесты extract_type ────────────────────────────────────────────────────

    /// `extract_type(Bool(true))` → `Bit`.
    #[test]
    fn bool_literal_type_is_bit() {
        let ty = extract_type(&Expression::Bool(true), Rc::new(RefCell::new(ModelNode::default()))).unwrap();
        assert_eq!(ty, TypeNode::Bit);
    }

    /// `extract_type(Number(42))` → `Bit`.
    #[test]
    fn number_literal_type_is_bit() {
        let ty = extract_type(&Expression::Number(42), Rc::new(RefCell::new(ModelNode::default()))).unwrap();
        assert_eq!(ty, TypeNode::Bit);
    }

    /// `extract_type(Rational("3.14", false))` → `Rational`.
    #[test]
    fn rational_literal_type_is_rational() {
        let ty = extract_type(&Expression::Rational("3.14".to_string(), false), Rc::new(RefCell::new(ModelNode::default()))).unwrap();
        assert_eq!(ty, TypeNode::Rational);
    }

    /// `extract_type(Parenthesis(Bool(_)))` → `Bit` (тип из внутреннего).
    #[test]
    fn parenthesis_propagates_inner_type() {
        let inner = Box::new(Expression::Bool(false));
        let ty = extract_type(&Expression::Parenthesis(inner), Rc::new(RefCell::new(ModelNode::default()))).unwrap();
        assert_eq!(ty, TypeNode::Bit);
    }

    /// `Not(Bool(_))` → `Bit`.
    #[test]
    fn not_expression_type_is_bit() {
        let ty = extract_type(&Expression::Not(Box::new(Expression::Bool(true))), Rc::new(RefCell::new(ModelNode::default()))).unwrap();
        assert_eq!(ty, TypeNode::Bit);
    }

    /// `Equal(Number, Number)` → `Bit`.
    #[test]
    fn comparison_type_is_bit() {
        let ty = extract_type(
            &Expression::Equal(Box::new(Expression::Number(1)), Box::new(Expression::Number(2))),
            Rc::new(RefCell::new(ModelNode::default())),
        ).unwrap();
        assert_eq!(ty, TypeNode::Bit);
    }

    /// `Add(Number, Number)` → `Bit` (оба Bit, результат Bit).
    #[test]
    fn add_bit_bit_is_bit() {
        let ty = extract_type(
            &Expression::Add(Box::new(Expression::Number(1)), Box::new(Expression::Number(2))),
            Rc::new(RefCell::new(ModelNode::default())),
        ).unwrap();
        assert_eq!(ty, TypeNode::Bit);
    }

    /// `Add(Rational, Number)` → `Rational` (расширение типа).
    #[test]
    fn add_rational_bit_is_rational() {
        let ty = extract_type(
            &Expression::Add(
                Box::new(Expression::Rational("1.0".into(), false)),
                Box::new(Expression::Number(2)),
            ),
            Rc::new(RefCell::new(ModelNode::default())),
        ).unwrap();
        assert_eq!(ty, TypeNode::Rational);
    }

    /// `Negate(Rational)` → `Rational`.
    #[test]
    fn negate_rational_is_rational() {
        let ty = extract_type(
            &Expression::Negate(Box::new(Expression::Rational("1.0".into(), false))),
            Rc::new(RefCell::new(ModelNode::default())),
        ).unwrap();
        assert_eq!(ty, TypeNode::Rational);
    }

    /// `wider_type(Bit, Bit)` → `Bit`.
    #[test]
    fn wider_type_bit_bit() {
        assert_eq!(wider_type(TypeNode::Bit, TypeNode::Bit), TypeNode::Bit);
    }

    /// `wider_type(Rational, Bit)` → `Rational`.
    #[test]
    fn wider_type_rational_bit() {
        assert_eq!(wider_type(TypeNode::Rational, TypeNode::Bit), TypeNode::Rational);
    }

    /// `wider_type(Bit, Rational)` → `Rational`.
    #[test]
    fn wider_type_bit_rational() {
        assert_eq!(wider_type(TypeNode::Bit, TypeNode::Rational), TypeNode::Rational);
    }

    /// `ast_type_to_node(Type::Bit)` → `Bit`.
    #[test]
    fn ast_type_bit_to_node() {
        assert_eq!(ast_type_to_node(&Type::Bit), TypeNode::Bit);
    }

    /// `ast_type_to_node(Type::Bool)` → `Bit`.
    #[test]
    fn ast_type_bool_to_node() {
        assert_eq!(ast_type_to_node(&Type::Bool), TypeNode::Bit);
    }

    /// `ast_type_to_node(Type::Rational)` → `Rational`.
    #[test]
    fn ast_type_rational_to_node() {
        assert_eq!(ast_type_to_node(&Type::Rational), TypeNode::Rational);
    }

    // ── Интеграционные тесты через type_inference ─────────────────────────────

    /// `var x = false;` → тип `Bit`.
    #[test]
    fn infer_bool_initializer() {
        let node = build("var x = false;").unwrap();
        if let Some(VariableNode::Simple(_, ty, _)) = node.search_var("x") {
            assert_eq!(ty, TypeNode::Bit);
        } else {
            panic!("переменная x не найдена");
        }
    }

    /// `var x = 3.14;` → тип `Rational`.
    #[test]
    fn infer_rational_initializer() {
        let node = build("var x = 3.14;").unwrap();
        if let Some(VariableNode::Simple(_, ty, _)) = node.search_var("x") {
            assert_eq!(ty, TypeNode::Rational);
        } else {
            panic!("переменная x не найдена");
        }
    }

    /// `const C = false;` → тип `Bit`.
    #[test]
    fn infer_const_bool() {
        let node = build("const C = false;").unwrap();
        if let Some(VariableNode::Const(_, ty, _)) = node.search_var("C") {
            assert_eq!(ty, TypeNode::Bit);
        } else {
            panic!("константа C не найдена");
        }
    }

    /// Переменная с явным типом не перезаписывается выводом типа.
    #[test]
    fn explicit_type_not_overwritten() {
        let node = build("var x: [bit;8] = false;").unwrap();
        if let Some(VariableNode::Simple(_, ty, _)) = node.search_var("x") {
            assert_eq!(ty, TypeNode::Array(8, Box::new(TypeNode::Bit)));
        } else {
            panic!("переменная x не найдена");
        }
    }

    /// Вывод типа из другой переменной: `var b: bit; var a = b;` → `a: Bit`.
    #[test]
    fn infer_type_from_variable() {
        let node = build("var b: bit = false; var a = b;").unwrap();
        if let Some(VariableNode::Simple(_, ty, _)) = node.search_var("a") {
            assert_eq!(ty, TypeNode::Bit);
        } else {
            panic!("переменная a не найдена");
        }
    }

    /// Вывод типа: `var x = 1 + 2;` → `Bit` (оба операнда числовые литералы).
    #[test]
    fn infer_type_from_add_numbers() {
        let node = build("var x = 1 + 2;").unwrap();
        if let Some(VariableNode::Simple(_, ty, _)) = node.search_var("x") {
            assert_eq!(ty, TypeNode::Bit);
        } else {
            panic!("переменная x не найдена");
        }
    }
}
