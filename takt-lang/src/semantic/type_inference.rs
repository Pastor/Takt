//! Вывод типов для переменных языка Takt.
//!
//! Основная функция [`type_inference`] обходит таблицу переменных и для
//! каждой переменной с [`TypeNode::Inference`] (тип не задан явно) вызывает
//! [`extract_type`], чтобы определить тип из инициализирующего выражения.
//!
//! ## Алгоритм вывода
//!
//! 1. Числовой литерал (Number(n)) → минимальный целочисленный тип [bit;8/16/32/64]
//! 2. Литерал `Bool` → `Bool`; `Rational` → `Rational`; `Duration` → `Duration`
//! 3. Переменная → тип referenced переменной
//! 4. Условие → `Bit`
//! 5. Арифметика → наиболее «широкий» тип из операндов (`Bit` < `Rational`)
//! 6. Логика/сравнение → `Bit`
//! 7. Скобки → тип внутреннего выражения
//! 8. Приведение типа (`as T`) → `T`
//! 9. Перечисление (`Type::Enum(name)`) → `TypeNode::Enum(name)`
//! 10. Прочее → `Unsupported`
//!
//! ## Расширение типов (`wider_type`)
//!
//! При выводе типа бинарных выражений выбирается «более широкий» тип:
//!
//! | Операнды                        | Результат              |
//! |---------------------------------|------------------------|
//! | `Enum("X")` + `Enum("X")`       | `Enum("X")`            |
//! | `Enum("X")` + `Enum("Y")`       | `Unsupported`          |
//! | `Enum(_)` + любой не-Enum       | `Unsupported`          |
//! | `Rational` + любой              | `Rational`             |
//! | `Array(N)` + `Array(M)`         | `Array(max(N,M), ...)`  |
//! | `Bool` + `Bit`                  | `Bit`                  |

use crate::diagnostics::Diagnostic;
use crate::parser::ast::Type;
use crate::semantic::type_node::TypeNode;
use crate::semantic::{ExpressionNode, ModelNode, VariableNode};
use std::cell::RefCell;
use std::collections::BTreeMap;
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
    variables: &mut BTreeMap<String, VariableNode>,
    model: Rc<RefCell<ModelNode>>,
) -> Result<BTreeMap<String, VariableNode>, Diagnostic> {
    for (name, var) in variables.clone() {
        match var {
            VariableNode::Simple {
                upper,
                loc,
                ty: TypeNode::Inference,
                ref expr,
                ..
            } => {
                let typ = extract_type(expr, model.clone())?;
                variables.insert(
                    name.clone(),
                    VariableNode::Simple {
                        upper,
                        loc,
                        name: name.clone(),
                        ty: typ,
                        expr: expr.clone(),
                    },
                );
            }
            VariableNode::Const {
                upper,
                loc,
                ty: TypeNode::Inference,
                ref expr,
                ..
            } => {
                let typ = extract_type(expr, model.clone())?;
                variables.insert(
                    name.clone(),
                    VariableNode::Const {
                        upper,
                        loc,
                        name: name.clone(),
                        ty: typ,
                        expr: expr.clone(),
                    },
                );
            }
            // q(m,n) (0061): понижение литерала в представление v — см. type_node.
            ref other => {
                if let Some(nv) = crate::semantic::type_node::lower_fixed_var(other)? {
                    variables.insert(name.clone(), nv);
                }
            }
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
/// ## Правила расширения
///
/// | Пара типов                          | Результат                  |
/// |-------------------------------------|----------------------------|
/// | `Enum("X")` + `Enum("X")`           | `Enum("X")` (одинаковые)   |
/// | `Enum("X")` + `Enum("Y")`           | `Unsupported` (разные)     |
/// | `Enum(_)` + любой не-Enum           | `Unsupported`              |
/// | `Rational` + любой                  | `Rational`                 |
/// | `Array(N, T)` + `Array(M, T)`       | `Array(max(N,M), T)`       |
/// | `Array(N, T)` + скаляр              | `Array(N, T)`              |
/// | `Bool` + `Bit`                      | `Bit`                      |
/// | `Bool` + `Bool`                     | `Bool`                     |
/// | `Bit` + `Bit`                       | `Bit`                      |
/// | иначе                               | `Unsupported`              |
///
/// `wider_type` — `pub(crate)`; тесты — в модуле `tests` ниже.
#[inline]
pub(crate) fn wider_type(a: TypeNode, b: TypeNode) -> TypeNode {
    match (&a, &b) {
        // Ce4: два одинаковых перечисления → сохраняем тип перечисления
        (TypeNode::Enum(na), TypeNode::Enum(nb)) if na == nb => TypeNode::Enum(na.clone()),
        // Ce4: разные перечисления или перечисление с не-перечислением → несовместимо
        (TypeNode::Enum(_), _) | (_, TypeNode::Enum(_)) => TypeNode::Unsupported,
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
        // q(m,n) (0061): два одинаковых fixed-point → тот же тип; смешение с любым
        // другим типом (иное q, целое, float, …) даёт `Unsupported` и ловится
        // стражем T6 (`validate::fixed`) — см. правило 6 ADR 0061.
        (TypeNode::Fixed { m: ma, n: na }, TypeNode::Fixed { m: mb, n: nb })
            if ma == mb && na == nb =>
        {
            TypeNode::Fixed { m: *ma, n: *na }
        }
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

/// Преобразует АСД-тип [`Type`] в семантический [`TypeNode`] без контекста модели.
///
/// Используется при выводе типа для выражений `as T` (приведение типа)
/// и при Ce6-выводе из возвращаемого типа функции.
///
/// Псевдонимы встроенных типов (`bool`, `bit`, `float`, `unit`) разрешаются.
/// Пользовательские псевдонимы (`Type::Alias`) возвращают `Unsupported` — для их
/// разрешения используйте [`ast_type_to_node_ctx`].
/// Перечисления (`Type::Enum`) возвращают `TypeNode::Enum(name)` — факт того,
/// что перечисление объявлено, проверяется в `validate_model`.
///
/// | АСД-тип              | Результат                     |
/// |----------------------|-------------------------------|
/// | `Type::Enum("X")`    | `TypeNode::Enum("X")`         |
/// | `Type::Alias(local)` | `TypeNode::Unsupported`       |
/// | `Type::Function`     | `TypeNode::Unsupported`       |
/// | `Type::Address`      | `TypeNode::Unsupported`       |
/// Строит [`TypeNode::Fixed`] из конструктора `q(m, n)` без диагностики (для
/// цели приведения `as`, где путь инфраллибелен). Конструктор не `q` или
/// границы вне правила 1 ADR 0061 (`m ≥ 1`, `n ≥ 1`, `m + n ≤ 64`) →
/// [`TypeNode::Unsupported`]; объявление типа тот же случай ловит `SE-057`
/// (`construct_fixed`).
fn fixed_node_or_unsupported(ctor: &str, m: i64, n: i64) -> TypeNode {
    if ctor == "q" && m >= 1 && n >= 1 && m + n <= 64 {
        TypeNode::Fixed {
            m: m as u8,
            n: n as u8,
        }
    } else {
        TypeNode::Unsupported
    }
}

pub(crate) fn ast_type_to_node(ty: &Type) -> TypeNode {
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
        // q(m, n) (0061): цель приведения `x as q(m, n)`. Границы — те же, что у
        // объявления (правило 1 ADR); нарушение даёт `Unsupported` (страж T6/T7).
        Type::Fixed(_, ctor, m, n) => fixed_node_or_unsupported(ctor, *m, *n),
        // Ce4: перечисление по имени — без проверки существования (нет контекста)
        Type::Enum(name) => TypeNode::Enum(name.clone()),
        // Ce6: разрешаем встроенные псевдонимы типов без контекста модели
        Type::Alias(id) => match id.name.as_str() {
            "bit" => TypeNode::Bit,
            "bool" => TypeNode::Bool,
            "float" => TypeNode::Rational,
            "unit" => TypeNode::Unit,
            // Пользовательский псевдоним — не поддерживается без контекста
            _ => TypeNode::Unsupported,
        },
        // Type::Function, Type::Address — не поддерживаются при выводе типа
        _ => TypeNode::Unsupported,
    }
}

/// Преобразует АСД-тип в семантический с разрешением пользовательских псевдонимов
/// и перечислений через контекст модели.
///
/// FE2/Ce6/Ce4: В отличие от [`ast_type_to_node`], эта функция ищет:
/// - псевдонимы типов в `ModelNode::types` (`type u8 = [bit;8]` → `Array(8, Bit)`)
/// - перечисления в `ModelNode::enums` (`Color` → `Enum("Color")` если объявлено)
///
/// Если перечисление не найдено — возвращает `TypeNode::Unsupported`; ошибка
/// будет диагностирована при `validate_model`.
///
/// | АСД-тип              | Результат                                         |
/// |----------------------|---------------------------------------------------|
/// | `Type::Enum("X")`    | `TypeNode::Enum("X")` если X объявлен             |
/// | `Type::Enum("X")`    | `TypeNode::Unsupported` если X не найден          |
/// | `Type::Alias(local)` | из `types` или `TypeNode::Unsupported`            |
pub(crate) fn ast_type_to_node_ctx(ty: &Type, model: Rc<RefCell<ModelNode>>) -> TypeNode {
    match ty {
        Type::Bit => TypeNode::Bit,
        Type::Bool => TypeNode::Bool,
        Type::Rational => TypeNode::Rational,
        Type::Unit => TypeNode::Unit,
        Type::Array {
            element_count,
            element_type,
            ..
        } => TypeNode::Array(
            *element_count,
            Box::new(ast_type_to_node_ctx(element_type, model)),
        ),
        // q(m, n) (0061): цель приведения `x as q(m, n)` (см. `ast_type_to_node`).
        Type::Fixed(_, ctor, m, n) => fixed_node_or_unsupported(ctor, *m, *n),
        // Ce4: перечисление по имени — проверяем наличие в контексте модели
        Type::Enum(name) => {
            let borrowed = model.borrow();
            if borrowed.search_enum(name).is_some() {
                TypeNode::Enum(name.clone())
            } else {
                // Перечисление не найдено; validate_model сообщит об ошибке
                TypeNode::Unsupported
            }
        }
        Type::Alias(id) => match id.name.as_str() {
            "bit" => TypeNode::Bit,
            "bool" => TypeNode::Bool,
            "float" => TypeNode::Rational,
            "unit" => TypeNode::Unit,
            name => {
                // FE2: ищем пользовательский псевдоним типа в модели
                let borrowed = model.borrow();
                if let Some(alias_type) = borrowed.types.get(name) {
                    alias_type.clone()
                } else {
                    TypeNode::Unsupported
                }
            }
        },
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
pub(crate) fn extract_type(
    expr: &ExpressionNode,
    model: Rc<RefCell<ModelNode>>,
) -> Result<TypeNode, Diagnostic> {
    match expr {
        // ── Литералы ──────────────────────────────────────────────────────────
        ExpressionNode::Bool(_) => Ok(TypeNode::Bool),
        ExpressionNode::Number(n) => Ok(infer_int_type(*n)),
        ExpressionNode::Duration(_) => Ok(TypeNode::Duration), // фича 0134
        ExpressionNode::Rational(_, _) => Ok(TypeNode::Rational),
        ExpressionNode::String(_) | ExpressionNode::Address(_, _) => Ok(TypeNode::Unsupported),

        // ── Идентификаторы ────────────────────────────────────────────────────
        ExpressionNode::Variable(var_rc) => Ok(type_of_var(&var_rc.borrow())),
        // Условия всегда вычисляются в булев (1-битный) результат.
        ExpressionNode::Condition(_) => Ok(TypeNode::Bit),
        ExpressionNode::Model(_) => Ok(TypeNode::Unsupported),

        // ── Скобки ────────────────────────────────────────────────────────────
        ExpressionNode::Parenthesis(inner) => extract_type(inner, model),

        // ── Логические операции и сравнения → Bit ─────────────────────────────
        ExpressionNode::Not(_)
        | ExpressionNode::And(_, _)
        | ExpressionNode::Or(_, _)
        | ExpressionNode::Equal(_, _)
        | ExpressionNode::NotEqual(_, _)
        | ExpressionNode::Less(_, _)
        | ExpressionNode::More(_, _)
        | ExpressionNode::LessEqual(_, _)
        | ExpressionNode::MoreEqual(_, _) => Ok(TypeNode::Bit),

        // ── Унарные операции → тип операнда ──────────────────────────────────
        ExpressionNode::BitwiseNot(e)
        | ExpressionNode::UnaryPlus(e)
        | ExpressionNode::Negate(e) => extract_type(e, model),

        // ── Арифметические бинарные операции → наиболее широкий тип ──────────
        ExpressionNode::Add(l, r)
        | ExpressionNode::Subtract(l, r)
        | ExpressionNode::Multiply(l, r)
        | ExpressionNode::Divide(l, r)
        | ExpressionNode::Modulo(l, r)
        | ExpressionNode::Power(l, r) => {
            let lt = extract_type(l, model.clone())?;
            let rt = extract_type(r, model)?;
            Ok(wider_type(lt, rt))
        }

        // ── Побитовые бинарные операции → наиболее широкий тип ───────────────
        ExpressionNode::BitwiseAnd(l, r)
        | ExpressionNode::BitwiseXor(l, r)
        | ExpressionNode::BitwiseOr(l, r)
        | ExpressionNode::ShiftLeft(l, r)
        | ExpressionNode::ShiftRight(l, r) => {
            let lt = extract_type(l, model.clone())?;
            let rt = extract_type(r, model)?;
            Ok(wider_type(lt, rt))
        }

        // ── Тернарный оператор → тип наиболее широкой ветви ──────────────────
        ExpressionNode::ConditionalOperator(_, then_e, else_e) => {
            let tt = extract_type(then_e, model.clone())?;
            let et = extract_type(else_e, model)?;
            Ok(wider_type(tt, et))
        }

        // ── Присваивание → тип правой части ──────────────────────────────────
        ExpressionNode::Assign(_, r) => extract_type(r, model),

        // ── Обращение к массиву → тип элемента ───────────────────────────────
        ExpressionNode::ArraySubscript(var_rc, _) => match type_of_var(&var_rc.borrow()) {
            TypeNode::Array(_, elem_type) => Ok(*elem_type),
            other => Ok(other),
        },

        // ── Срез массива → тип элемента ──────────────────────────────────────
        ExpressionNode::ArraySlice(var_rc, _, _) => match type_of_var(&var_rc.borrow()) {
            TypeNode::Array(_, elem_type) => Ok(*elem_type),
            other => Ok(other),
        },

        // ── Массивный литерал → Array(N, тип_элемента) ───────────────────────
        ExpressionNode::Array(items) => {
            let n = items.len() as u16;
            if items.is_empty() {
                return Ok(TypeNode::Array(0, Box::new(TypeNode::Bit)));
            }
            let elem_type = extract_type(&items[0], model)?;
            Ok(TypeNode::Array(n, Box::new(elem_type)))
        }

        // ── Инициализатор структуры → тип первого элемента ───────────────────
        ExpressionNode::Initializer(items) => {
            if items.is_empty() {
                return Ok(TypeNode::Unsupported);
            }
            let n = items.len() as u16;
            let elem_type = extract_type(&items[0], model)?;
            Ok(TypeNode::Array(n, Box::new(elem_type)))
        }

        // ── Приведение типа → результирующий тип (FE2: с разрешением псевдонимов) ──
        ExpressionNode::Cast(_, ty) => Ok(ty.clone()),
        ExpressionNode::Type(ty) => Ok(ast_type_to_node(ty)),

        // ── Ce6: Вывод типа из возвращаемого типа функции ────────────────────
        //
        // Если инициализирующее выражение — вызов известной функции,
        // тип переменной выводится из возвращаемого типа функции.
        // Это реализует двунаправленный вывод: `var result = add(1, 2);` →
        // тип `result` = возвращаемый тип `add`.
        //
        // Поддерживаются разрешённые функции (Local, External, Builtin)
        // и неразрешённые (Unresolved), для которых тип берётся из AST-определения.
        ExpressionNode::Function(func_rc, _args) => {
            let func = func_rc.borrow();
            let ret_type = match &*func {
                crate::semantic::FunctionDefinitionNode::Local { ret, .. } => ret.clone(),
                crate::semantic::FunctionDefinitionNode::External { ret, .. } => ret.clone(),
                crate::semantic::FunctionDefinitionNode::Builtin(_, _, ret) => ret.clone(),
                // Ce6+FE2: функция ещё не разрешена — читаем return_type из AST,
                // с разрешением пользовательских псевдонимов через контекст модели
                crate::semantic::FunctionDefinitionNode::Unresolved(def) => {
                    if let Some(ret_ast) = &def.return_type {
                        ast_type_to_node_ctx(ret_ast, model.clone())
                    } else {
                        TypeNode::Unit // функция без return_type → void/Unit
                    }
                }
                crate::semantic::FunctionDefinitionNode::None => TypeNode::Unsupported,
            };
            Ok(ret_type)
        }

        // ── Выражения без выводимого типа ────────────────────────────────────
        ExpressionNode::BitAccess(_, _)
        | ExpressionNode::CodeBlock(_, _)
        | ExpressionNode::NamedFunctionBox(_, _)
        | ExpressionNode::List(_)
        | ExpressionNode::None
        | ExpressionNode::Unresolved(_) => Ok(TypeNode::Unsupported),
    }
}

// ── Тесты ─────────────────────────────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use super::*;
    use crate::parse;
    use crate::semantic::tree::construct_model;

    /// Строит модель из Takt-кода и возвращает корневой ModelNode.
    fn build(src: &str) -> Result<ModelNode, Diagnostic> {
        let (ast, _) = parse(src, 0).expect("ошибка разбора");
        construct_model(&ast, None, &[]).map(|m| m.take())
    }

    // ── Тесты extract_type ────────────────────────────────────────────────────

    /// `extract_type(Bool(true))` → `Bool`.
    #[test]
    fn bool_literal_type_is_bool() {
        let ty = extract_type(
            &ExpressionNode::Bool(true),
            Rc::new(RefCell::new(ModelNode::default())),
        )
        .unwrap();
        assert_eq!(ty, TypeNode::Bool);
    }

    /// `extract_type(Number(42))` → `Array(8, Bit)`.
    #[test]
    fn number_literal_type_is_array8() {
        let ty = extract_type(
            &ExpressionNode::Number(42),
            Rc::new(RefCell::new(ModelNode::default())),
        )
        .unwrap();
        assert_eq!(ty, TypeNode::Array(8, Box::new(TypeNode::Bit)));
    }

    /// `extract_type(Rational("3.14", false))` → `Rational`.
    #[test]
    fn rational_literal_type_is_rational() {
        let ty = extract_type(
            &ExpressionNode::Rational("3.14".to_string(), false),
            Rc::new(RefCell::new(ModelNode::default())),
        )
        .unwrap();
        assert_eq!(ty, TypeNode::Rational);
    }

    /// `extract_type(Parenthesis(Bool(_)))` → `Bool` (тип из внутреннего).
    #[test]
    fn parenthesis_propagates_inner_type() {
        let inner = Box::new(ExpressionNode::Bool(false));
        let ty = extract_type(
            &ExpressionNode::Parenthesis(inner),
            Rc::new(RefCell::new(ModelNode::default())),
        )
        .unwrap();
        assert_eq!(ty, TypeNode::Bool);
    }

    /// `Not(Bool(_))` → `Bit`.
    #[test]
    fn not_expression_type_is_bit() {
        let ty = extract_type(
            &ExpressionNode::Not(Box::new(ExpressionNode::Bool(true))),
            Rc::new(RefCell::new(ModelNode::default())),
        )
        .unwrap();
        assert_eq!(ty, TypeNode::Bit);
    }

    /// `Equal(Number, Number)` → `Bit`.
    #[test]
    fn comparison_type_is_bit() {
        let ty = extract_type(
            &ExpressionNode::Equal(
                Box::new(ExpressionNode::Number(1)),
                Box::new(ExpressionNode::Number(2)),
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
            &ExpressionNode::Add(
                Box::new(ExpressionNode::Number(1)),
                Box::new(ExpressionNode::Number(2)),
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
            &ExpressionNode::Add(
                Box::new(ExpressionNode::Rational("1.0".into(), false)),
                Box::new(ExpressionNode::Number(2)),
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
            &ExpressionNode::Negate(Box::new(ExpressionNode::Rational("1.0".into(), false))),
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
        let node = build("var x := false;").unwrap();
        if let Some(VariableNode::Simple { ty, .. }) = node.search_var("x") {
            assert_eq!(ty, TypeNode::Bool);
        } else {
            panic!("переменная x не найдена");
        }
    }

    /// `var x = 3.14;` → тип `Rational`.
    #[test]
    fn infer_rational_initializer() {
        let node = build("var x := 3.14;").unwrap();
        if let Some(VariableNode::Simple { ty, .. }) = node.search_var("x") {
            assert_eq!(ty, TypeNode::Rational);
        } else {
            panic!("переменная x не найдена");
        }
    }

    /// `const C = false;` → тип `Bool`.
    #[test]
    fn infer_const_bool() {
        let node = build("const C := false;").unwrap();
        if let Some(VariableNode::Const { ty, .. }) = node.search_var("C") {
            assert_eq!(ty, TypeNode::Bool);
        } else {
            panic!("константа C не найдена");
        }
    }

    /// Переменная с явным типом не перезаписывается выводом типа.
    #[test]
    fn explicit_type_not_overwritten() {
        let node = build("var x: [bit;8] := false;").unwrap();
        if let Some(VariableNode::Simple { ty, .. }) = node.search_var("x") {
            assert_eq!(ty, TypeNode::Array(8, Box::new(TypeNode::Bit)));
        } else {
            panic!("переменная x не найдена");
        }
    }

    /// Вывод типа из другой переменной: `var b: bit; var a = b;` → `a: Bit`.
    #[test]
    fn infer_type_from_variable() {
        let node = build("var b: bit := false; var a := b;").unwrap();
        if let Some(VariableNode::Simple { ty, .. }) = node.search_var("a") {
            assert_eq!(ty, TypeNode::Bit);
        } else {
            panic!("переменная a не найдена");
        }
    }

    /// Вывод типа: `var x = 1 + 2;` → `Array(8, Bit)` (оба операнда числовые литералы ≤255).
    #[test]
    fn infer_type_from_add_numbers() {
        let node = build("var x := 1 + 2;").unwrap();
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
            &ExpressionNode::UnaryPlus(Box::new(ExpressionNode::Number(5))),
            Rc::new(RefCell::new(ModelNode::default())),
        )
        .unwrap();
        assert_eq!(ty, TypeNode::Array(8, Box::new(TypeNode::Bit)));
    }

    /// `BitwiseNot(Rational)` → `Rational`.
    #[test]
    fn bitwise_not_rational_is_rational() {
        let ty = extract_type(
            &ExpressionNode::BitwiseNot(Box::new(ExpressionNode::Rational("1.0".into(), false))),
            Rc::new(RefCell::new(ModelNode::default())),
        )
        .unwrap();
        assert_eq!(ty, TypeNode::Rational);
    }

    /// `Multiply(Rational, Number)` → `Rational`.
    #[test]
    fn multiply_rational_bit_is_rational() {
        let ty = extract_type(
            &ExpressionNode::Multiply(
                Box::new(ExpressionNode::Rational("2.0".into(), false)),
                Box::new(ExpressionNode::Number(3)),
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
            &ExpressionNode::Subtract(
                Box::new(ExpressionNode::Number(5)),
                Box::new(ExpressionNode::Number(3)),
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
            &ExpressionNode::ShiftLeft(
                Box::new(ExpressionNode::Number(1)),
                Box::new(ExpressionNode::Number(2)),
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
            &ExpressionNode::ConditionalOperator(
                Box::new(ExpressionNode::Bool(true)),
                Box::new(ExpressionNode::Number(0)),
                Box::new(ExpressionNode::Rational("1.0".into(), false)),
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
            &ExpressionNode::ConditionalOperator(
                Box::new(ExpressionNode::Bool(true)),
                Box::new(ExpressionNode::Number(1)),
                Box::new(ExpressionNode::Number(0)),
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
            &ExpressionNode::Array(vec![ExpressionNode::Number(1), ExpressionNode::Number(2)]),
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
            &ExpressionNode::Array(vec![]),
            Rc::new(RefCell::new(ModelNode::default())),
        )
        .unwrap();
        assert_eq!(ty, TypeNode::Array(0, Box::new(TypeNode::Bit)));
    }

    /// `Assign(_, Rational)` → `Rational`.
    #[test]
    fn assign_infers_rhs_type() {
        let ty = extract_type(
            &ExpressionNode::Assign(
                Box::new(ExpressionNode::None),
                Box::new(ExpressionNode::Rational("3.14".into(), false)),
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
        let ty = extract_type(
            &ExpressionNode::Cast(Box::new(ExpressionNode::Number(0)), TypeNode::Rational),
            Rc::new(RefCell::new(ModelNode::default())),
        )
        .unwrap();
        assert_eq!(ty, TypeNode::Rational);
    }

    /// `Cast(_, Type::Array{...})` → `Array(N, Bit)`.
    #[test]
    fn cast_to_array_type() {
        let ty = extract_type(
            &ExpressionNode::Cast(
                Box::new(ExpressionNode::Number(0)),
                TypeNode::Array(4, Box::new(TypeNode::Bit)),
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
            &ExpressionNode::String(vec!["hello".into()]),
            Rc::new(RefCell::new(ModelNode::default())),
        )
        .unwrap();
        assert_eq!(ty, TypeNode::Unsupported);
    }

    /// `Expression::None` → `Unsupported`.
    #[test]
    fn none_expression_type_is_unsupported() {
        let ty = extract_type(
            &ExpressionNode::None,
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
        let node = build("var x := 1 + 3.14;").unwrap();
        if let Some(VariableNode::Simple { ty, .. }) = node.search_var("x") {
            assert_eq!(ty, TypeNode::Rational);
        } else {
            panic!("переменная x не найдена");
        }
    }

    /// `const C = 1;` → тип `Array(8, Bit)` (числовой литерал ≤255 → [bit;8]).
    #[test]
    fn infer_const_number() {
        let node = build("const C := 1;").unwrap();
        if let Some(VariableNode::Const { ty, .. }) = node.search_var("C") {
            assert_eq!(ty, TypeNode::Array(8, Box::new(TypeNode::Bit)));
        } else {
            panic!("константа C не найдена");
        }
    }

    /// `const C = 42;` → тип `Array(8, Bit)` (42 ≤ 255 → [bit;8]).
    #[test]
    fn infer_const_number_42_is_array8() {
        let node = build("const C := 42;").unwrap();
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

    // ── Ce4: Тесты wider_type для перечислений ────────────────────────────────

    /// Ce4: два одинаковых перечисления → сохраняют тип.
    ///
    /// # Пример
    /// Выражение `color1 + color2`, оба типа `Color` → тип результата `Color`.
    #[test]
    fn wider_type_same_enum_returns_enum() {
        let a = TypeNode::Enum("Color".to_string());
        let b = TypeNode::Enum("Color".to_string());
        assert_eq!(wider_type(a, b), TypeNode::Enum("Color".to_string()));
    }

    /// Ce4: два разных перечисления несовместимы → `Unsupported`.
    ///
    /// # Контр-пример
    /// `Color` и `Size` — разные типы, смешение недопустимо.
    #[test]
    fn wider_type_different_enums_is_unsupported() {
        let a = TypeNode::Enum("Color".to_string());
        let b = TypeNode::Enum("Size".to_string());
        assert_eq!(wider_type(a, b), TypeNode::Unsupported);
    }

    /// Ce4: перечисление с числовым типом несовместимо → `Unsupported`.
    ///
    /// # Контр-пример
    /// `Color` + `Bit` — нельзя расширить enum до Bit.
    #[test]
    fn wider_type_enum_and_bit_is_unsupported() {
        let a = TypeNode::Enum("Color".to_string());
        assert_eq!(wider_type(a.clone(), TypeNode::Bit), TypeNode::Unsupported);
        assert_eq!(wider_type(TypeNode::Bit, a), TypeNode::Unsupported);
    }

    /// Ce4: перечисление с Rational несовместимо → `Unsupported`.
    ///
    /// # Контр-пример
    /// Enum не расширяется до Rational — это нарушение типовой безопасности.
    #[test]
    fn wider_type_enum_and_rational_is_unsupported() {
        let a = TypeNode::Enum("Dir".to_string());
        assert_eq!(
            wider_type(a.clone(), TypeNode::Rational),
            TypeNode::Unsupported
        );
        assert_eq!(wider_type(TypeNode::Rational, a), TypeNode::Unsupported);
    }

    // ── Ce4: Тесты ast_type_to_node для перечислений ──────────────────────────

    /// Ce4: `ast_type_to_node(Type::Enum("Color"))` → `TypeNode::Enum("Color")`.
    #[test]
    fn ast_type_enum_to_node() {
        use crate::parser::ast::Type;
        assert_eq!(
            ast_type_to_node(&Type::Enum("Color".to_string())),
            TypeNode::Enum("Color".to_string())
        );
    }

    /// Ce4: `ast_type_to_node_ctx` при наличии enum в модели → `TypeNode::Enum`.
    ///
    /// # Пример
    /// Объявлен `enum Dir { ... }`, тип `Dir` разрешается в `TypeNode::Enum("Dir")`.
    #[test]
    fn ast_type_to_node_ctx_with_enum_in_model() {
        use crate::parser::ast::Type;
        use crate::semantic::EnumDefinitionNode;

        let model = Rc::new(RefCell::new(ModelNode::default()));
        let e = EnumDefinitionNode::new("Dir", &[("North", None), ("South", None)]);
        model.borrow_mut().enums.insert("Dir".to_string(), e);

        assert_eq!(
            ast_type_to_node_ctx(&Type::Enum("Dir".to_string()), model),
            TypeNode::Enum("Dir".to_string())
        );
    }

    /// Ce4: `ast_type_to_node_ctx` при отсутствии enum в модели → `TypeNode::Unsupported`.
    ///
    /// # Контр-пример
    /// Тип `UnknownEnum` не объявлен → `Unsupported` (ошибка диагностируется в validate_model).
    #[test]
    fn ast_type_to_node_ctx_unknown_enum_is_unsupported() {
        use crate::parser::ast::Type;

        let model = Rc::new(RefCell::new(ModelNode::default()));
        assert_eq!(
            ast_type_to_node_ctx(&Type::Enum("UnknownEnum".to_string()), model),
            TypeNode::Unsupported
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
        let node = build("var x := 256;").unwrap();
        if let Some(VariableNode::Simple { ty, .. }) = node.search_var("x") {
            assert_eq!(ty, TypeNode::Array(16, Box::new(TypeNode::Bit)));
        } else {
            panic!("переменная x не найдена");
        }
    }

    /// `var x = 65536;` → тип `Array(32, Bit)`.
    #[test]
    fn infer_number_65536_is_array32() {
        let node = build("var x := 65536;").unwrap();
        if let Some(VariableNode::Simple { ty, .. }) = node.search_var("x") {
            assert_eq!(ty, TypeNode::Array(32, Box::new(TypeNode::Bit)));
        } else {
            panic!("переменная x не найдена");
        }
    }
}
