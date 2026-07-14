//! Адаптер выражений: `ExpressionNode` → [`Value`] поверх ядра [`crate::eval`].
//!
//! Симметричен [`crate::predicate`] (адаптер условий) и, как и он, **не содержит
//! семантики** — только структурный разбор и делегирование в ядро (ADR 0025,
//! Option B).
//!
//! # Что здесь чинится
//!
//! До задачи 0025-02 тела блоков `enter`/`exit`/`always` вычислял
//! `unit/builder.rs::eval_expression_rt`, покрывавший **6 из ~45** вариантов
//! `ExpressionNode`; всё остальное уходило в `_ => None`, а `None` приводил к
//! **молчаливому пропуску присваивания** (Д1, Д2). Здесь разбор исчерпывающий:
//! новый вариант ломает сборку, а невычислимое выражение даёт `Err`, который
//! вызывающий обязан обработать.
//!
//! # Позиции в диагностиках
//!
//! `ExpressionNode::Variable` — в отличие от `ConditionNode::Variable` — позиции
//! использования **не несёт**, поэтому диагностика привязывается к позиции
//! *объявления* переменной (`VariableNode::loc()`). Это менее точно, чем в
//! условиях, но существенно лучше `Location::Builtin`.

use crate::context::Context;
use crate::eval::error::EvalError;
use crate::eval::ops::{self, BinOp, UnOp};
use crate::eval::value::Value;
use crate::eval::{self as eval_core};
use grammar::diagnostics::{Diagnostic, Location};
use grammar::parser::ast::Member;
use grammar::semantic::ExpressionNode;

/// Вычисляет выражение в значение.
///
/// Паники недостижимы: любой неподдержанный случай — `Err` (R4).
pub(crate) fn eval_expression(
    expr: &ExpressionNode,
    ctx: &dyn Context,
) -> Result<Value, Diagnostic> {
    match expr {
        // ── Литералы ─────────────────────────────────────────────────────────
        ExpressionNode::Number(n) => Ok(Value::Number(*n)),
        ExpressionNode::Bool(b) => Ok(Value::Boolean(*b)),
        ExpressionNode::Rational(text, negative) => parse_rational(text, *negative),
        // Адресный литерал `адрес:бит` — значение самого адреса.
        ExpressionNode::Address(addr, _bit) => Ok(Value::Number(*addr)),

        // ── Переменные и доступ ──────────────────────────────────────────────
        ExpressionNode::Variable(var) => {
            let borrowed = var.borrow();
            ctx.get_value(borrowed.name()).ok_or_else(|| {
                Diagnostic::error(
                    borrowed.loc(),
                    format!("переменная '{}' не найдена", borrowed.name()),
                )
                .with_code("SIM-009")
            })
        }
        ExpressionNode::Parenthesis(inner) => eval_expression(inner, ctx),
        ExpressionNode::ArraySubscript(var, index) => {
            let (name, loc) = {
                let b = var.borrow();
                (b.name().to_string(), b.loc())
            };
            let array = ctx.get_value(&name).ok_or_else(|| {
                Diagnostic::error(loc, format!("переменная '{name}' не найдена"))
                    .with_code("SIM-009")
            })?;
            let Value::Array(items) = array else {
                return Err(Diagnostic::error(
                    loc,
                    format!("переменная '{name}' не является массивом"),
                )
                .with_code("SIM-010"));
            };
            let Value::Number(idx) = eval_expression(index, ctx)? else {
                return Err(
                    Diagnostic::error(loc, "индекс массива должен быть целым".to_string())
                        .with_code("SIM-010"),
                );
            };
            usize::try_from(idx)
                .ok()
                .and_then(|i| items.get(i).cloned())
                .ok_or_else(|| {
                    Diagnostic::error(
                        loc,
                        format!(
                            "индекс {idx} вне границ массива '{name}' (длина {})",
                            items.len()
                        ),
                    )
                    .with_code("SIM-010")
                })
        }
        ExpressionNode::ArraySlice(var, from, to) => {
            let (name, loc) = {
                let b = var.borrow();
                (b.name().to_string(), b.loc())
            };
            let array = ctx.get_value(&name).ok_or_else(|| {
                Diagnostic::error(loc, format!("переменная '{name}' не найдена"))
                    .with_code("SIM-009")
            })?;
            let Value::Array(items) = array else {
                return Err(Diagnostic::error(
                    loc,
                    format!("переменная '{name}' не является массивом"),
                )
                .with_code("SIM-010"));
            };
            let start = usize::try_from(from.unwrap_or(0)).unwrap_or(0);
            let end = to
                .map(|t| usize::try_from(t).unwrap_or(0))
                .unwrap_or(items.len());
            items
                .get(start..end)
                .map(|slice| Value::Array(slice.to_vec()))
                .ok_or_else(|| {
                    Diagnostic::error(
                        loc,
                        format!(
                            "срез [{start}:{end}] вне границ массива '{name}' (длина {})",
                            items.len()
                        ),
                    )
                    .with_code("SIM-010")
                })
        }
        ExpressionNode::BitAccess(inner, member) => {
            let value = eval_expression(inner, ctx)?;
            let loc = loc_of(expr);
            let Member::Number(bit) = member else {
                return Err(Diagnostic::error(
                    loc,
                    "доступ к биту возможен только по номеру".to_string(),
                )
                .with_code("SIM-011"));
            };
            let bits = match value {
                Value::Number(n) => n,
                Value::Boolean(b) => i64::from(b),
                Value::Real(_) | Value::Array(_) => {
                    return Err(Diagnostic::error(
                        loc,
                        "доступ к биту возможен только у целого значения".to_string(),
                    )
                    .with_code("SIM-011"));
                }
            };
            if !(0..64).contains(bit) {
                return Err(Diagnostic::error(
                    loc,
                    format!("номер бита {bit} вне диапазона 0..64"),
                )
                .with_code("SIM-011"));
            }
            Ok(Value::Boolean(bits & (1 << bit) != 0))
        }

        // ── Унарные ──────────────────────────────────────────────────────────
        ExpressionNode::Not(inner) => unary(UnOp::Not, inner, ctx),
        ExpressionNode::BitwiseNot(inner) => unary(UnOp::BitwiseNot, inner, ctx),
        ExpressionNode::UnaryPlus(inner) => unary(UnOp::UnaryPlus, inner, ctx),
        ExpressionNode::Negate(inner) => unary(UnOp::Negate, inner, ctx),

        // ── Арифметика — то, чего не умел eval_expression_rt (Д1) ────────────
        ExpressionNode::Add(l, r) => binary(BinOp::Add, l, r, ctx),
        ExpressionNode::Subtract(l, r) => binary(BinOp::Subtract, l, r, ctx),
        ExpressionNode::Multiply(l, r) => binary(BinOp::Multiply, l, r, ctx),
        ExpressionNode::Divide(l, r) => binary(BinOp::Divide, l, r, ctx),
        ExpressionNode::Modulo(l, r) => binary(BinOp::Modulo, l, r, ctx),
        ExpressionNode::Power(l, r) => binary(BinOp::Power, l, r, ctx),
        ExpressionNode::ShiftLeft(l, r) => binary(BinOp::ShiftLeft, l, r, ctx),
        ExpressionNode::ShiftRight(l, r) => binary(BinOp::ShiftRight, l, r, ctx),
        ExpressionNode::BitwiseAnd(l, r) => binary(BinOp::BitwiseAnd, l, r, ctx),
        ExpressionNode::BitwiseXor(l, r) => binary(BinOp::BitwiseXor, l, r, ctx),
        ExpressionNode::BitwiseOr(l, r) => binary(BinOp::BitwiseOr, l, r, ctx),

        // ── Сравнения и логика ───────────────────────────────────────────────
        ExpressionNode::Less(l, r) => binary(BinOp::Less, l, r, ctx),
        ExpressionNode::More(l, r) => binary(BinOp::More, l, r, ctx),
        ExpressionNode::LessEqual(l, r) => binary(BinOp::LessEqual, l, r, ctx),
        ExpressionNode::MoreEqual(l, r) => binary(BinOp::MoreEqual, l, r, ctx),
        ExpressionNode::Equal(l, r) => binary(BinOp::Equal, l, r, ctx),
        ExpressionNode::NotEqual(l, r) => binary(BinOp::NotEqual, l, r, ctx),
        // В `ExpressionNode` побитовые операции — отдельные варианты, поэтому
        // `And`/`Or` здесь именно логические (в отличие от `ConditionNode`).
        ExpressionNode::And(l, r) => binary(BinOp::LogicalAnd, l, r, ctx),
        ExpressionNode::Or(l, r) => binary(BinOp::LogicalOr, l, r, ctx),

        // ── Составные ────────────────────────────────────────────────────────
        ExpressionNode::ConditionalOperator(cond, then_, else_) => {
            let cond_value = eval_expression(cond, ctx)?;
            let taken = ops::to_bool(&cond_value).map_err(|e| e.to_diagnostic(loc_of(cond)))?;
            if taken {
                eval_expression(then_, ctx)
            } else {
                eval_expression(else_, ctx)
            }
        }
        // Приведение типа опирается на то же ядро, что и запись в переменную.
        ExpressionNode::Cast(inner, ty) => {
            let value = eval_expression(inner, ctx)?;
            eval_core::coerce_to_type(value, ty).map_err(|e| e.to_diagnostic(loc_of(inner)))
        }
        ExpressionNode::Array(items) | ExpressionNode::Initializer(items) => {
            let values = items
                .iter()
                .map(|item| eval_expression(item, ctx))
                .collect::<Result<Vec<Value>, Diagnostic>>()?;
            Ok(Value::Array(values))
        }

        // ── Пока не поддержано — но с диагностикой, а не тихим пропуском ─────
        //
        // Вызовы функций требуют интерпретатора тела `fn` — задача `0025-02b`.
        ExpressionNode::Function(func, _) => Err(Diagnostic::error(
            func.borrow().loc(),
            format!(
                "вызов функции '{}' пока не поддерживается симулятором",
                func.borrow().name()
            ),
        )
        .with_code("SIM-012")),
        ExpressionNode::NamedFunctionBox(_, _) => Err(unsupported(
            "вызов с именованными аргументами",
            loc_of(expr),
        )),
        ExpressionNode::CodeBlock(_, _) => {
            Err(unsupported("блок кода как выражение", loc_of(expr)))
        }
        // Присваивание внутри выражения требует доступа на запись, которого у
        // вычислителя нет. Присваивание-оператор обрабатывает `builder.rs`.
        ExpressionNode::Assign(_, _) => {
            Err(unsupported("присваивание внутри выражения", loc_of(expr)))
        }
        // `Value` не представляет строки — пробел зафиксирован анализом.
        ExpressionNode::String(_) => Err(unsupported("строки", loc_of(expr))),
        ExpressionNode::Type(_) => Err(unsupported("тип как выражение", loc_of(expr))),
        ExpressionNode::Model(_) => Err(unsupported("модель как выражение", loc_of(expr))),
        ExpressionNode::Condition(_) => Err(unsupported(
            "именованное условие как выражение",
            loc_of(expr),
        )),
        ExpressionNode::List(_) => Err(unsupported("список параметров", loc_of(expr))),

        // ── Невычислимые по определению ──────────────────────────────────────
        ExpressionNode::None => Err(Diagnostic::error(
            loc_of(expr),
            "пустое выражение не может быть вычислено".to_string(),
        )
        .with_code("SIM-015")),
        ExpressionNode::Unresolved(_) => Err(Diagnostic::error(
            loc_of(expr),
            "неразрешённое выражение не может быть вычислено".to_string(),
        )
        .with_code("SIM-016")),
    }
}

fn unsupported(what: &str, loc: Location) -> Diagnostic {
    Diagnostic::error(loc, format!("{what} не поддерживается симулятором")).with_code("SIM-014")
}

fn parse_rational(text: &str, negative: bool) -> Result<Value, Diagnostic> {
    let parsed: f64 = text.parse().map_err(|_| {
        Diagnostic::error(
            Location::Builtin,
            format!("не удалось разобрать вещественный литерал '{text}'"),
        )
        .with_code("SIM-008")
    })?;
    Ok(Value::Real(if negative { -parsed } else { parsed }))
}

/// Ищет позицию для диагностики.
///
/// `ExpressionNode::Variable` несёт позицию **объявления** (не использования) —
/// см. заголовок модуля.
fn loc_of(expr: &ExpressionNode) -> Location {
    match expr {
        ExpressionNode::Variable(var) => var.borrow().loc(),
        ExpressionNode::ArraySubscript(var, _) | ExpressionNode::ArraySlice(var, _, _) => {
            var.borrow().loc()
        }
        ExpressionNode::Function(func, _) => func.borrow().loc(),
        ExpressionNode::Parenthesis(inner)
        | ExpressionNode::BitAccess(inner, _)
        | ExpressionNode::Not(inner)
        | ExpressionNode::BitwiseNot(inner)
        | ExpressionNode::UnaryPlus(inner)
        | ExpressionNode::Negate(inner)
        | ExpressionNode::Cast(inner, _)
        | ExpressionNode::CodeBlock(inner, _)
        | ExpressionNode::NamedFunctionBox(inner, _) => loc_of(inner),
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
        | ExpressionNode::Or(l, r)
        | ExpressionNode::Assign(l, r) => match loc_of(l) {
            Location::Builtin => loc_of(r),
            found => found,
        },
        ExpressionNode::ConditionalOperator(c, t, e) => match loc_of(c) {
            Location::Builtin => match loc_of(t) {
                Location::Builtin => loc_of(e),
                found => found,
            },
            found => found,
        },
        ExpressionNode::Array(items) | ExpressionNode::Initializer(items) => items
            .iter()
            .map(loc_of)
            .find(|l| !matches!(l, Location::Builtin))
            .unwrap_or(Location::Builtin),
        ExpressionNode::None
        | ExpressionNode::Unresolved(_)
        | ExpressionNode::Number(_)
        | ExpressionNode::Rational(_, _)
        | ExpressionNode::String(_)
        | ExpressionNode::Type(_)
        | ExpressionNode::Address(_, _)
        | ExpressionNode::Bool(_)
        | ExpressionNode::Model(_)
        | ExpressionNode::Condition(_)
        | ExpressionNode::List(_) => Location::Builtin,
    }
}

fn unary(op: UnOp, inner: &ExpressionNode, ctx: &dyn Context) -> Result<Value, Diagnostic> {
    let value = eval_expression(inner, ctx)?;
    ops::apply_unary(op, &value).map_err(|e: EvalError| e.to_diagnostic(loc_of(inner)))
}

fn binary(
    op: BinOp,
    left: &ExpressionNode,
    right: &ExpressionNode,
    ctx: &dyn Context,
) -> Result<Value, Diagnostic> {
    let lhs = eval_expression(left, ctx)?;
    let rhs = eval_expression(right, ctx)?;
    ops::apply_binary(op, &lhs, &rhs).map_err(|e: EvalError| {
        let loc = match loc_of(left) {
            Location::Builtin => loc_of(right),
            found => found,
        };
        e.to_diagnostic(loc)
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use grammar::semantic::type_node::TypeNode;
    use std::collections::HashMap;

    struct MockContext {
        vars: HashMap<String, Value>,
    }

    impl MockContext {
        fn new(pairs: &[(&str, Value)]) -> Self {
            Self {
                vars: pairs
                    .iter()
                    .map(|(k, v)| ((*k).to_string(), v.clone()))
                    .collect(),
            }
        }
    }

    impl Context for MockContext {
        fn get_value(&self, name: &str) -> Option<Value> {
            self.vars.get(name).cloned()
        }

        fn set_value(&mut self, name: &str, value: Value) {
            self.vars.insert(name.to_string(), value);
        }
    }

    fn num(n: i64) -> Box<ExpressionNode> {
        Box::new(ExpressionNode::Number(n))
    }

    // ── Д1: арифметика, которой не было ───────────────────────────────────────

    #[test]
    fn d1_add_is_evaluated() {
        // Ядро дефекта Д1: `a + 1` уходило в `_ => None`.
        let ctx = MockContext::new(&[]);
        let expr = ExpressionNode::Add(num(5), num(1));
        assert_eq!(eval_expression(&expr, &ctx), Ok(Value::Number(6)));
    }

    #[test]
    fn d1_all_arithmetic_operators_work() {
        let ctx = MockContext::new(&[]);
        for (expr, expected) in [
            (ExpressionNode::Subtract(num(5), num(3)), 2),
            (ExpressionNode::Multiply(num(5), num(3)), 15),
            (ExpressionNode::Divide(num(7), num(2)), 3),
            (ExpressionNode::Modulo(num(7), num(2)), 1),
            (ExpressionNode::Power(num(2), num(10)), 1024),
            (ExpressionNode::ShiftLeft(num(1), num(8)), 256),
            (ExpressionNode::ShiftRight(num(256), num(8)), 1),
            (ExpressionNode::BitwiseAnd(num(6), num(3)), 2),
            (ExpressionNode::BitwiseOr(num(6), num(3)), 7),
            (ExpressionNode::BitwiseXor(num(6), num(3)), 5),
        ] {
            assert_eq!(
                eval_expression(&expr, &ctx),
                Ok(Value::Number(expected)),
                "провал на {expr:?}"
            );
        }
    }

    #[test]
    fn comparisons_and_logic() {
        let ctx = MockContext::new(&[]);
        assert_eq!(
            eval_expression(&ExpressionNode::Less(num(1), num(2)), &ctx),
            Ok(Value::Boolean(true))
        );
        assert_eq!(
            eval_expression(
                &ExpressionNode::And(
                    Box::new(ExpressionNode::Bool(true)),
                    Box::new(ExpressionNode::Bool(false))
                ),
                &ctx
            ),
            Ok(Value::Boolean(false))
        );
    }

    #[test]
    fn parenthesis_recurses() {
        let ctx = MockContext::new(&[]);
        let inner = ExpressionNode::Add(num(5), num(1));
        let expr = ExpressionNode::More(
            Box::new(ExpressionNode::Parenthesis(Box::new(inner))),
            num(2),
        );
        assert_eq!(eval_expression(&expr, &ctx), Ok(Value::Boolean(true)));
    }

    #[test]
    fn conditional_operator_picks_branch() {
        let ctx = MockContext::new(&[]);
        let expr = ExpressionNode::ConditionalOperator(
            Box::new(ExpressionNode::Bool(true)),
            num(1),
            num(2),
        );
        assert_eq!(eval_expression(&expr, &ctx), Ok(Value::Number(1)));
    }

    #[test]
    fn cast_uses_core_coercion() {
        // Cast опирается на то же coerce_to_type, что и запись в переменную.
        let ctx = MockContext::new(&[]);
        let expr = ExpressionNode::Cast(
            num(300),
            TypeNode::Integer {
                bits: 8,
                signed: false,
            },
        );
        assert_eq!(eval_expression(&expr, &ctx), Ok(Value::Number(44)));
    }

    #[test]
    fn array_literal_builds_array() {
        let ctx = MockContext::new(&[]);
        let expr =
            ExpressionNode::Array(vec![ExpressionNode::Number(1), ExpressionNode::Number(2)]);
        assert_eq!(
            eval_expression(&expr, &ctx),
            Ok(Value::Array(vec![Value::Number(1), Value::Number(2)]))
        );
    }

    // ── Ошибки: отказ вместо тихого пропуска (Д2) ─────────────────────────────

    #[test]
    fn d2_division_by_zero_is_diagnostic_not_silence() {
        let ctx = MockContext::new(&[]);
        let expr = ExpressionNode::Divide(num(10), num(0));
        let err = eval_expression(&expr, &ctx).unwrap_err();
        assert_eq!(err.code.as_deref(), Some("SIM-001"));
    }

    #[test]
    fn function_call_is_diagnostic_not_panic() {
        // Полная поддержка — задача 0025-02b.
        let ctx = MockContext::new(&[]);
        let expr = ExpressionNode::Function(
            std::rc::Rc::new(std::cell::RefCell::new(Default::default())),
            vec![],
        );
        let err = eval_expression(&expr, &ctx).unwrap_err();
        assert_eq!(err.code.as_deref(), Some("SIM-012"));
    }

    #[test]
    fn string_expression_is_diagnostic() {
        let ctx = MockContext::new(&[]);
        let expr = ExpressionNode::String(vec!["a".to_string()]);
        let err = eval_expression(&expr, &ctx).unwrap_err();
        assert_eq!(err.code.as_deref(), Some("SIM-014"));
    }

    #[test]
    fn none_expression_is_diagnostic() {
        let ctx = MockContext::new(&[]);
        let err = eval_expression(&ExpressionNode::None, &ctx).unwrap_err();
        assert_eq!(err.code.as_deref(), Some("SIM-015"));
    }
}
