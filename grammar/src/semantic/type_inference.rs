//! Вывод типов для переменных языка BuT.
//!
//! Основная функция [`type_inference`] обходит таблицу переменных и для
//! каждой переменной с [`TypeNode::Inference`] (тип не задан явно) вызывает
//! [`extract_type`], чтобы определить тип из инициализирующего выражения.
//!
//! ## Алгоритм вывода
//!
//! 1. Числовой литерал (Number(n)) → минимальный целочисленный тип [bit;8/16/32/64]
//! 2. Булевый литерал (Bool) → Bool
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
            VariableNode::Simple { upper, ty: TypeNode::Inference, ref expr, .. } => {
                let typ = extract_type(expr, model.clone())?;
                variables.insert(
                    name.clone(),
                    VariableNode::Simple {
                        upper,
                        name: name.clone(),
                        ty: typ,
                        expr: expr.clone(),
                    },
                );
            }
            VariableNode::Const { upper, ty: TypeNode::Inference, ref expr, .. } => {
                let typ = extract_type(expr, model.clone())?;
                variables.insert(
                    name.clone(),
                    VariableNode::Const {
                        upper,
                        name: name.clone(),
                        ty: typ,
                        expr: expr.clone(),
                    },
                );
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
        VariableNode::Simple { ty, .. }
        | VariableNode::Port { ty, .. }
        | VariableNode::Const { ty, .. } => ty.clone(),
        VariableNode::Unresolved => TypeNode::Inference,
    }
}

/// Возвращает наиболее «широкий» тип из двух.
///
/// Порядок расширения: `Bit`/`Bool` < `Array` < `Rational`.
/// Если один из типов — `Rational`, результат — `Rational`.
/// Из двух массивов выбирается наибольший по размеру.
/// `Bool` + `Bit` → `Bit`. Иначе — `Unsupported`.
#[inline]
fn wider_type(a: TypeNode, b: TypeNode) -> TypeNode {
    match (&a, &b) {
        (TypeNode::Rational, _) | (_, TypeNode::Rational) => TypeNode::Rational,
        // Из двух массивов выбирается наибольший по размеру
        (TypeNode::Array(n, t), TypeNode::Array(m, s)) => {
            if n >= m {
                TypeNode::Array(*n, t.clone())
            } else {
                TypeNode::Array(*m, s.clone())
            }
        }
        (TypeNode::Array(n, t), _) => TypeNode::Array(*n, t.clone()),
        (_, TypeNode::Array(n, t)) => TypeNode::Array(*n, t.clone()),
        (TypeNode::Bit, TypeNode::Bit) => TypeNode::Bit,
        (TypeNode::Bool, TypeNode::Bool) => TypeNode::Bool,
        (TypeNode::Bool, TypeNode::Bit) | (TypeNode::Bit, TypeNode::Bool) => TypeNode::Bit,
        _ => TypeNode::Unsupported,
    }
}

/// Определяет минимальный целочисленный тип для числового литерала.
///
/// Начинает с 8 бит и удваивает размер, пока значение не вмещается:
/// - `0..=255`               → `[bit;8]`
/// - `256..=65535`           → `[bit;16]`
/// - `65536..=4294967295`    → `[bit;32]`
/// - иначе (или отрицательное) → `[bit;64]`
fn infer_int_type(n: i64) -> TypeNode {
    let arr = |size| TypeNode::Array(size, Box::new(TypeNode::Bit));
    if n >= 0 && n <= u8::MAX as i64 {
        arr(8)
    } else if n >= 0 && n <= u16::MAX as i64 {
        arr(16)
    } else if n >= 0 && n <= u32::MAX as i64 {
        arr(32)
    } else {
        arr(64)
    }
}

/// Преобразует АСД-тип [`Type`] в семантический [`TypeNode`].
///
/// Используется при выводе типа для выражений `as T` (приведение типа).
/// Псевдонимы типов (`Alias`) и функциональные типы возвращают `Unsupported`.
fn ast_type_to_node(ty: &Type) -> TypeNode {
    match ty {
        Type::Bit => TypeNode::Bit,
        Type::Bool => TypeNode::Bool,
        Type::Rational => TypeNode::Rational,
        Type::Unit => TypeNode::Unit,
        Type::Array {
            element_count,
            element_type,
            ..
        } => TypeNode::Array(*element_count, Box::new(ast_type_to_node(element_type))),
        // Type::Alias, Type::Function, Type::Address — не поддерживаются при выводе типа
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
        Expression::Bool(_) => Ok(TypeNode::Bool),
        Expression::Number(n) => Ok(infer_int_type(*n)),
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
        Expression::ArraySubscript(var_rc, _) => match type_of_var(&var_rc.borrow()) {
            TypeNode::Array(_, elem_type) => Ok(*elem_type),
            other => Ok(other),
        },

        // ── Срез массива → тип элемента ──────────────────────────────────────
        Expression::ArraySlice(var_rc, _, _) => match type_of_var(&var_rc.borrow()) {
            TypeNode::Array(_, elem_type) => Ok(*elem_type),
            other => Ok(other),
        },

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

    /// `extract_type(Bool(true))` → `Bool`.
    #[test]
    fn bool_literal_type_is_bool() {
        let ty = extract_type(
            &Expression::Bool(true),
            Rc::new(RefCell::new(ModelNode::default())),
        )
        .unwrap();
        assert_eq!(ty, TypeNode::Bool);
    }

    /// `extract_type(Number(42))` → `Array(8, Bit)`.
    #[test]
    fn number_literal_type_is_array8() {
        let ty = extract_type(
            &Expression::Number(42),
            Rc::new(RefCell::new(ModelNode::default())),
        )
        .unwrap();
        assert_eq!(ty, TypeNode::Array(8, Box::new(TypeNode::Bit)));
    }

    /// `extract_type(Rational("3.14", false))` → `Rational`.
    #[test]
    fn rational_literal_type_is_rational() {
        let ty = extract_type(
            &Expression::Rational("3.14".to_string(), false),
            Rc::new(RefCell::new(ModelNode::default())),
        )
        .unwrap();
        assert_eq!(ty, TypeNode::Rational);
    }

    /// `extract_type(Parenthesis(Bool(_)))` → `Bool` (тип из внутреннего).
    #[test]
    fn parenthesis_propagates_inner_type() {
        let inner = Box::new(Expression::Bool(false));
        let ty = extract_type(
            &Expression::Parenthesis(inner),
            Rc::new(RefCell::new(ModelNode::default())),
        )
        .unwrap();
        assert_eq!(ty, TypeNode::Bool);
    }

    /// `Not(Bool(_))` → `Bit`.
    #[test]
    fn not_expression_type_is_bit() {
        let ty = extract_type(
            &Expression::Not(Box::new(Expression::Bool(true))),
            Rc::new(RefCell::new(ModelNode::default())),
        )
        .unwrap();
        assert_eq!(ty, TypeNode::Bit);
    }

    /// `Equal(Number, Number)` → `Bit`.
    #[test]
    fn comparison_type_is_bit() {
        let ty = extract_type(
            &Expression::Equal(
                Box::new(Expression::Number(1)),
                Box::new(Expression::Number(2)),
            ),
            Rc::new(RefCell::new(ModelNode::default())),
        )
        .unwrap();
        assert_eq!(ty, TypeNode::Bit);
    }

    /// `Add(Number(1), Number(2))` → `Array(8, Bit)` (оба маленьких числа, результат [bit;8]).
    #[test]
    fn add_two_small_numbers_is_array8() {
        let ty = extract_type(
            &Expression::Add(
                Box::new(Expression::Number(1)),
                Box::new(Expression::Number(2)),
            ),
            Rc::new(RefCell::new(ModelNode::default())),
        )
        .unwrap();
        assert_eq!(ty, TypeNode::Array(8, Box::new(TypeNode::Bit)));
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
        )
        .unwrap();
        assert_eq!(ty, TypeNode::Rational);
    }

    /// `Negate(Rational)` → `Rational`.
    #[test]
    fn negate_rational_is_rational() {
        let ty = extract_type(
            &Expression::Negate(Box::new(Expression::Rational("1.0".into(), false))),
            Rc::new(RefCell::new(ModelNode::default())),
        )
        .unwrap();
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
        assert_eq!(
            wider_type(TypeNode::Rational, TypeNode::Bit),
            TypeNode::Rational
        );
    }

    /// `wider_type(Bit, Rational)` → `Rational`.
    #[test]
    fn wider_type_bit_rational() {
        assert_eq!(
            wider_type(TypeNode::Bit, TypeNode::Rational),
            TypeNode::Rational
        );
    }

    /// `ast_type_to_node(Type::Bit)` → `Bit`.
    #[test]
    fn ast_type_bit_to_node() {
        assert_eq!(ast_type_to_node(&Type::Bit), TypeNode::Bit);
    }

    /// `ast_type_to_node(Type::Bool)` → `Bool`.
    #[test]
    fn ast_type_bool_to_node() {
        assert_eq!(ast_type_to_node(&Type::Bool), TypeNode::Bool);
    }

    /// `ast_type_to_node(Type::Rational)` → `Rational`.
    #[test]
    fn ast_type_rational_to_node() {
        assert_eq!(ast_type_to_node(&Type::Rational), TypeNode::Rational);
    }

    // ── Интеграционные тесты через type_inference ─────────────────────────────

    /// `var x = false;` → тип `Bool`.
    #[test]
    fn infer_bool_initializer() {
        let node = build("var x = false;").unwrap();
        if let Some(VariableNode::Simple { ty, .. }) = node.search_var("x") {
            assert_eq!(ty, TypeNode::Bool);
        } else {
            panic!("переменная x не найдена");
        }
    }

    /// `var x = 3.14;` → тип `Rational`.
    #[test]
    fn infer_rational_initializer() {
        let node = build("var x = 3.14;").unwrap();
        if let Some(VariableNode::Simple { ty, .. }) = node.search_var("x") {
            assert_eq!(ty, TypeNode::Rational);
        } else {
            panic!("переменная x не найдена");
        }
    }

    /// `const C = false;` → тип `Bool`.
    #[test]
    fn infer_const_bool() {
        let node = build("const C = false;").unwrap();
        if let Some(VariableNode::Const { ty, .. }) = node.search_var("C") {
            assert_eq!(ty, TypeNode::Bool);
        } else {
            panic!("константа C не найдена");
        }
    }

    /// Переменная с явным типом не перезаписывается выводом типа.
    #[test]
    fn explicit_type_not_overwritten() {
        let node = build("var x: [bit;8] = false;").unwrap();
        if let Some(VariableNode::Simple { ty, .. }) = node.search_var("x") {
            assert_eq!(ty, TypeNode::Array(8, Box::new(TypeNode::Bit)));
        } else {
            panic!("переменная x не найдена");
        }
    }

    /// Вывод типа из другой переменной: `var b: bit; var a = b;` → `a: Bit`.
    #[test]
    fn infer_type_from_variable() {
        let node = build("var b: bit = false; var a = b;").unwrap();
        if let Some(VariableNode::Simple { ty, .. }) = node.search_var("a") {
            assert_eq!(ty, TypeNode::Bit);
        } else {
            panic!("переменная a не найдена");
        }
    }

    /// Вывод типа: `var x = 1 + 2;` → `Array(8, Bit)` (оба операнда числовые литералы ≤255).
    #[test]
    fn infer_type_from_add_numbers() {
        let node = build("var x = 1 + 2;").unwrap();
        if let Some(VariableNode::Simple { ty, .. }) = node.search_var("x") {
            assert_eq!(ty, TypeNode::Array(8, Box::new(TypeNode::Bit)));
        } else {
            panic!("переменная x не найдена");
        }
    }

    // ── Тесты extract_type: унарные и бинарные операции ──────────────────────

    /// `UnaryPlus(Number(5))` → `Array(8, Bit)`.
    #[test]
    fn unary_plus_number_type_is_bit() {
        let ty = extract_type(
            &Expression::UnaryPlus(Box::new(Expression::Number(5))),
            Rc::new(RefCell::new(ModelNode::default())),
        )
        .unwrap();
        assert_eq!(ty, TypeNode::Array(8, Box::new(TypeNode::Bit)));
    }

    /// `BitwiseNot(Rational)` → `Rational`.
    #[test]
    fn bitwise_not_rational_is_rational() {
        let ty = extract_type(
            &Expression::BitwiseNot(Box::new(Expression::Rational("1.0".into(), false))),
            Rc::new(RefCell::new(ModelNode::default())),
        )
        .unwrap();
        assert_eq!(ty, TypeNode::Rational);
    }

    /// `Multiply(Rational, Number)` → `Rational`.
    #[test]
    fn multiply_rational_bit_is_rational() {
        let ty = extract_type(
            &Expression::Multiply(
                Box::new(Expression::Rational("2.0".into(), false)),
                Box::new(Expression::Number(3)),
            ),
            Rc::new(RefCell::new(ModelNode::default())),
        )
        .unwrap();
        assert_eq!(ty, TypeNode::Rational);
    }

    /// `Subtract(Number(5), Number(3))` → `Array(8, Bit)`.
    #[test]
    fn subtract_two_numbers_is_array8() {
        let ty = extract_type(
            &Expression::Subtract(
                Box::new(Expression::Number(5)),
                Box::new(Expression::Number(3)),
            ),
            Rc::new(RefCell::new(ModelNode::default())),
        )
        .unwrap();
        assert_eq!(ty, TypeNode::Array(8, Box::new(TypeNode::Bit)));
    }

    /// `ShiftLeft(Number(1), Number(2))` → `Array(8, Bit)`.
    #[test]
    fn shift_left_numbers_is_array8() {
        let ty = extract_type(
            &Expression::ShiftLeft(
                Box::new(Expression::Number(1)),
                Box::new(Expression::Number(2)),
            ),
            Rc::new(RefCell::new(ModelNode::default())),
        )
        .unwrap();
        assert_eq!(ty, TypeNode::Array(8, Box::new(TypeNode::Bit)));
    }

    // ── Тесты extract_type: условие и тернарный оператор ─────────────────────

    /// `ConditionalOperator(_, Bit, Rational)` → `Rational`.
    #[test]
    fn conditional_operator_widens_type() {
        let ty = extract_type(
            &Expression::ConditionalOperator(
                Box::new(Expression::Bool(true)),
                Box::new(Expression::Number(0)),
                Box::new(Expression::Rational("1.0".into(), false)),
            ),
            Rc::new(RefCell::new(ModelNode::default())),
        )
        .unwrap();
        assert_eq!(ty, TypeNode::Rational);
    }

    /// `ConditionalOperator(_, Number(1), Number(0))` → `Array(8, Bit)`.
    #[test]
    fn conditional_operator_both_bit_is_bit() {
        let ty = extract_type(
            &Expression::ConditionalOperator(
                Box::new(Expression::Bool(true)),
                Box::new(Expression::Number(1)),
                Box::new(Expression::Number(0)),
            ),
            Rc::new(RefCell::new(ModelNode::default())),
        )
        .unwrap();
        assert_eq!(ty, TypeNode::Array(8, Box::new(TypeNode::Bit)));
    }

    // ── Тесты extract_type: массивы ───────────────────────────────────────────

    /// `Array([Number(1), Number(2)])` → `Array(2, Array(8, Bit))`.
    #[test]
    fn array_literal_infers_element_type() {
        let ty = extract_type(
            &Expression::Array(vec![Expression::Number(1), Expression::Number(2)]),
            Rc::new(RefCell::new(ModelNode::default())),
        )
        .unwrap();
        assert_eq!(
            ty,
            TypeNode::Array(2, Box::new(TypeNode::Array(8, Box::new(TypeNode::Bit))))
        );
    }

    /// `Array([])` → `Array(0, Bit)`.
    #[test]
    fn empty_array_literal_type() {
        let ty = extract_type(
            &Expression::Array(vec![]),
            Rc::new(RefCell::new(ModelNode::default())),
        )
        .unwrap();
        assert_eq!(ty, TypeNode::Array(0, Box::new(TypeNode::Bit)));
    }

    /// `Assign(_, Rational)` → `Rational`.
    #[test]
    fn assign_infers_rhs_type() {
        let ty = extract_type(
            &Expression::Assign(
                Box::new(Expression::None),
                Box::new(Expression::Rational("3.14".into(), false)),
            ),
            Rc::new(RefCell::new(ModelNode::default())),
        )
        .unwrap();
        assert_eq!(ty, TypeNode::Rational);
    }

    // ── Тесты extract_type: приведение типа ──────────────────────────────────

    /// `Cast(_, Type::Rational)` → `Rational`.
    #[test]
    fn cast_to_rational_type() {
        use crate::parser::ast::Type;
        let ty = extract_type(
            &Expression::Cast(Box::new(Expression::Number(0)), Type::Rational),
            Rc::new(RefCell::new(ModelNode::default())),
        )
        .unwrap();
        assert_eq!(ty, TypeNode::Rational);
    }

    /// `Cast(_, Type::Array{...})` → `Array(N, Bit)`.
    #[test]
    fn cast_to_array_type() {
        use crate::parser::ast::Type;
        let ty = extract_type(
            &Expression::Cast(
                Box::new(Expression::Number(0)),
                Type::Array {
                    loc: crate::diagnostics::Location::default(),
                    element_type: Box::new(Type::Bit),
                    element_count: 4,
                },
            ),
            Rc::new(RefCell::new(ModelNode::default())),
        )
        .unwrap();
        assert_eq!(ty, TypeNode::Array(4, Box::new(TypeNode::Bit)));
    }

    // ── Тесты extract_type: спец. выражения ─────────────────────────────────

    /// `String([...])` → `Unsupported`.
    #[test]
    fn string_literal_type_is_unsupported() {
        let ty = extract_type(
            &Expression::String(vec!["hello".into()]),
            Rc::new(RefCell::new(ModelNode::default())),
        )
        .unwrap();
        assert_eq!(ty, TypeNode::Unsupported);
    }

    /// `Expression::None` → `Unsupported`.
    #[test]
    fn none_expression_type_is_unsupported() {
        let ty = extract_type(
            &Expression::None,
            Rc::new(RefCell::new(ModelNode::default())),
        )
        .unwrap();
        assert_eq!(ty, TypeNode::Unsupported);
    }

    /// `wider_type(Unsupported, Bit)` → `Unsupported`.
    #[test]
    fn wider_type_unsupported_bit() {
        assert_eq!(
            wider_type(TypeNode::Unsupported, TypeNode::Bit),
            TypeNode::Unsupported
        );
    }

    /// `wider_type(Array, Bit)` → `Array` (пока сохраняет первый тип массива).
    #[test]
    fn wider_type_array_bit_returns_array() {
        let arr = TypeNode::Array(4, Box::new(TypeNode::Bit));
        let result = wider_type(arr.clone(), TypeNode::Bit);
        assert!(matches!(result, TypeNode::Array(4, _)));
    }

    /// `ast_type_to_node(Type::Unit)` → `Unit`.
    #[test]
    fn ast_type_unit_to_node() {
        use crate::parser::ast::Type;
        assert_eq!(ast_type_to_node(&Type::Unit), TypeNode::Unit);
    }

    /// `ast_type_to_node(Type::Array{...})` → `Array`.
    #[test]
    fn ast_type_array_to_node() {
        use crate::parser::ast::Type;
        let arr_type = Type::Array {
            loc: crate::diagnostics::Location::default(),
            element_type: Box::new(Type::Bit),
            element_count: 8,
        };
        assert_eq!(
            ast_type_to_node(&arr_type),
            TypeNode::Array(8, Box::new(TypeNode::Bit))
        );
    }

    // ── Интеграционные тесты через type_inference ─────────────────────────────

    /// `var x = 1 + 3.14;` → тип `Rational` (расширение через сложение).
    #[test]
    fn infer_type_from_add_rational() {
        let node = build("var x = 1 + 3.14;").unwrap();
        if let Some(VariableNode::Simple { ty, .. }) = node.search_var("x") {
            assert_eq!(ty, TypeNode::Rational);
        } else {
            panic!("переменная x не найдена");
        }
    }

    /// `const C = 1;` → тип `Array(8, Bit)` (числовой литерал ≤255 → [bit;8]).
    #[test]
    fn infer_const_number() {
        let node = build("const C = 1;").unwrap();
        if let Some(VariableNode::Const { ty, .. }) = node.search_var("C") {
            assert_eq!(ty, TypeNode::Array(8, Box::new(TypeNode::Bit)));
        } else {
            panic!("константа C не найдена");
        }
    }

    /// `const C = 42;` → тип `Array(8, Bit)` (42 ≤ 255 → [bit;8]).
    #[test]
    fn infer_const_number_42_is_array8() {
        let node = build("const C = 42;").unwrap();
        if let Some(VariableNode::Const { ty, .. }) = node.search_var("C") {
            assert_eq!(ty, TypeNode::Array(8, Box::new(TypeNode::Bit)));
        } else {
            panic!("константа C не найдена");
        }
    }

    // ── Тесты infer_int_type ──────────────────────────────────────────────────

    /// `infer_int_type(0)` → `Array(8, Bit)`.
    #[test]
    fn infer_int_type_zero() {
        assert_eq!(
            infer_int_type(0),
            TypeNode::Array(8, Box::new(TypeNode::Bit))
        );
    }

    /// `infer_int_type(255)` → `Array(8, Bit)`.
    #[test]
    fn infer_int_type_255() {
        assert_eq!(
            infer_int_type(255),
            TypeNode::Array(8, Box::new(TypeNode::Bit))
        );
    }

    /// `infer_int_type(256)` → `Array(16, Bit)`.
    #[test]
    fn infer_int_type_256() {
        assert_eq!(
            infer_int_type(256),
            TypeNode::Array(16, Box::new(TypeNode::Bit))
        );
    }

    /// `infer_int_type(65535)` → `Array(16, Bit)`.
    #[test]
    fn infer_int_type_65535() {
        assert_eq!(
            infer_int_type(65535),
            TypeNode::Array(16, Box::new(TypeNode::Bit))
        );
    }

    /// `infer_int_type(65536)` → `Array(32, Bit)`.
    #[test]
    fn infer_int_type_65536() {
        assert_eq!(
            infer_int_type(65536),
            TypeNode::Array(32, Box::new(TypeNode::Bit))
        );
    }

    /// `infer_int_type(4294967296)` → `Array(64, Bit)`.
    #[test]
    fn infer_int_type_large() {
        assert_eq!(
            infer_int_type(4294967296),
            TypeNode::Array(64, Box::new(TypeNode::Bit))
        );
    }

    /// `infer_int_type(-1)` → `Array(64, Bit)` (отрицательное → 64-бит).
    #[test]
    fn infer_int_type_negative() {
        assert_eq!(
            infer_int_type(-1),
            TypeNode::Array(64, Box::new(TypeNode::Bit))
        );
    }

    // ── Тесты wider_type: Bool-варианты и массивы ─────────────────────────────

    /// `wider_type(Bool, Bool)` → `Bool`.
    #[test]
    fn wider_type_bool_bool() {
        assert_eq!(wider_type(TypeNode::Bool, TypeNode::Bool), TypeNode::Bool);
    }

    /// `wider_type(Bool, Bit)` → `Bit`.
    #[test]
    fn wider_type_bool_bit() {
        assert_eq!(wider_type(TypeNode::Bool, TypeNode::Bit), TypeNode::Bit);
    }

    /// `wider_type(Bit, Bool)` → `Bit`.
    #[test]
    fn wider_type_bit_bool() {
        assert_eq!(wider_type(TypeNode::Bit, TypeNode::Bool), TypeNode::Bit);
    }

    /// `wider_type(Array(8, Bit), Array(16, Bit))` → `Array(16, Bit)`.
    #[test]
    fn wider_type_array8_array16_returns_array16() {
        let a = TypeNode::Array(8, Box::new(TypeNode::Bit));
        let b = TypeNode::Array(16, Box::new(TypeNode::Bit));
        assert_eq!(
            wider_type(a, b),
            TypeNode::Array(16, Box::new(TypeNode::Bit))
        );
    }

    /// `wider_type(Array(16, Bit), Array(8, Bit))` → `Array(16, Bit)`.
    #[test]
    fn wider_type_array16_array8_returns_array16() {
        let a = TypeNode::Array(16, Box::new(TypeNode::Bit));
        let b = TypeNode::Array(8, Box::new(TypeNode::Bit));
        assert_eq!(
            wider_type(a, b),
            TypeNode::Array(16, Box::new(TypeNode::Bit))
        );
    }

    // ── Интеграционные тесты: вывод типа для больших числовых литералов ───────

    /// `var x = 256;` → тип `Array(16, Bit)`.
    #[test]
    fn infer_number_256_is_array16() {
        let node = build("var x = 256;").unwrap();
        if let Some(VariableNode::Simple { ty, .. }) = node.search_var("x") {
            assert_eq!(ty, TypeNode::Array(16, Box::new(TypeNode::Bit)));
        } else {
            panic!("переменная x не найдена");
        }
    }

    /// `var x = 65536;` → тип `Array(32, Bit)`.
    #[test]
    fn infer_number_65536_is_array32() {
        let node = build("var x = 65536;").unwrap();
        if let Some(VariableNode::Simple { ty, .. }) = node.search_var("x") {
            assert_eq!(ty, TypeNode::Array(32, Box::new(TypeNode::Bit)));
        } else {
            panic!("переменная x не найдена");
        }
    }
}
