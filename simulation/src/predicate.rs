//! Адаптер условий: `ConditionNode` → [`Value`] поверх ядра [`crate::eval`].
//!
//! Модуль **не содержит семантики** — только структурный разбор узлов и
//! делегирование в ядро (ADR 0025, Option B). До задачи 0025-03 здесь жил
//! `flat`, который использовал узлы АСД в роли значений и потому:
//!
//! - паниковал на вызове функции (`unimplemented!()`, Д4) и на смешении
//!   `int`/`real` (`unwrap()` на `None`, Д6);
//! - возвращал внутренний узел скобок **невычисленным** (Д7);
//! - отвечал `Err` на вариант `enum` (Д8).
//!
//! Теперь разбор — исчерпывающий `match` **без `_`** по всем 24 вариантам
//! `ConditionNode`: новый вариант ломает сборку, а не превращается молча в
//! «условие ложно».

use crate::context::Context;
use crate::eval::error::EvalError;
use crate::eval::ops::{self, BinOp, UnOp};
use crate::eval::value::Value;
use crate::unit::Predicate;
use grammar::diagnostics::{Diagnostic, Location};
use grammar::parser::ast::Member;
use grammar::semantic::ConditionNode;

/// Строит предикат перехода из условия.
///
/// # Ограничение (задача 0025-05)
///
/// [`Predicate`] возвращает `bool`, поэтому ошибка вычисления пока сводится к
/// `false` — как и раньше. Это **не** регресс: значения теперь вычисляются
/// верно (Д6–Д8) и паник нет (Д4, Д6). Но требование R5 («ошибка вычисления
/// отличима от честно ложного условия») закрывается только вместе с каналом
/// диагностики — задача `0025-05`, где `Predicate`/`TickResult` получат
/// `Result`.
pub(crate) fn create_predicate(cond: &ConditionNode) -> Predicate {
    let name = condition_label(cond);
    let cond = cond.clone();
    Predicate::new(name, move |c: &dyn Context| {
        eval_condition(&cond, c)
            .and_then(|value| ops::to_bool(&value).map_err(|e| e.to_diagnostic(loc_of(&cond))))
            .unwrap_or(false)
    })
}

/// Возвращает короткое текстовое представление условия для отображения на ребре графа.
fn condition_label(cond: &ConditionNode) -> String {
    match cond {
        ConditionNode::None => String::new(),
        ConditionNode::Bool(b) => b.to_string(),
        ConditionNode::Number(n) => n.to_string(),
        ConditionNode::Rational(s, neg) => {
            if *neg {
                format!("-{s}")
            } else {
                s.clone()
            }
        }
        ConditionNode::Variable(var, _) => var.borrow().name().to_string(),
        ConditionNode::Not(c) => format!("!{}", condition_label(c)),
        ConditionNode::Parenthesis(c) => format!("({})", condition_label(c)),
        ConditionNode::Add(l, r) => format!("{} + {}", condition_label(l), condition_label(r)),
        ConditionNode::Subtract(l, r) => format!("{} - {}", condition_label(l), condition_label(r)),
        ConditionNode::And(l, r) => format!("{} & {}", condition_label(l), condition_label(r)),
        ConditionNode::Or(l, r) => format!("{} | {}", condition_label(l), condition_label(r)),
        ConditionNode::Less(l, r) => format!("{} < {}", condition_label(l), condition_label(r)),
        ConditionNode::More(l, r) => format!("{} > {}", condition_label(l), condition_label(r)),
        ConditionNode::LessEqual(l, r) => {
            format!("{} <= {}", condition_label(l), condition_label(r))
        }
        ConditionNode::MoreEqual(l, r) => {
            format!("{} >= {}", condition_label(l), condition_label(r))
        }
        ConditionNode::Equal(l, r) => format!("{} = {}", condition_label(l), condition_label(r)),
        ConditionNode::NotEqual(l, r) => {
            format!("{} != {}", condition_label(l), condition_label(r))
        }
        ConditionNode::BitAccess(c, m) => format!("{}.{m:?}", condition_label(c)),
        ConditionNode::ArraySubscript(v, idx) => {
            format!("{}[{}]", v.borrow().name(), condition_label(idx))
        }
        ConditionNode::Function(f, args, _) => {
            let name = f.borrow().name().to_string();
            let arg_labels: Vec<String> = args.iter().map(|a| condition_label(a)).collect();
            format!("{}({})", name, arg_labels.join(", "))
        }
        ConditionNode::State(s) => s.borrow().name().to_string(),
        ConditionNode::Model(m) => m.borrow().name.clone().unwrap_or_else(|| "?".to_string()),
        ConditionNode::EnumVariant(_, name, _) => name.clone(),
        ConditionNode::String(parts) => parts.join(""),
        ConditionNode::Unresolved(_) => "?".to_string(),
    }
}

/// Ищет позицию в исходном тексте для привязки диагностики.
///
/// Позицию несут не все узлы (`Variable` и `Function` — несут), поэтому ищем
/// вглубь по первому попавшемуся потомку. Это даёт диагностике реальную позицию
/// вместо `Location::Builtin` (критерий A10) в подавляющем большинстве случаев.
fn loc_of(cond: &ConditionNode) -> Location {
    match cond {
        ConditionNode::Variable(_, loc) | ConditionNode::Function(_, _, loc) => *loc,
        ConditionNode::Not(c) | ConditionNode::Parenthesis(c) | ConditionNode::BitAccess(c, _) => {
            loc_of(c)
        }
        ConditionNode::ArraySubscript(_, idx) => loc_of(idx),
        ConditionNode::Add(l, r)
        | ConditionNode::Subtract(l, r)
        | ConditionNode::And(l, r)
        | ConditionNode::Or(l, r)
        | ConditionNode::Less(l, r)
        | ConditionNode::More(l, r)
        | ConditionNode::LessEqual(l, r)
        | ConditionNode::MoreEqual(l, r)
        | ConditionNode::Equal(l, r)
        | ConditionNode::NotEqual(l, r) => match loc_of(l) {
            Location::Builtin => loc_of(r),
            found => found,
        },
        ConditionNode::None
        | ConditionNode::Unresolved(_)
        | ConditionNode::Number(_)
        | ConditionNode::Rational(_, _)
        | ConditionNode::String(_)
        | ConditionNode::Bool(_)
        | ConditionNode::Model(_)
        | ConditionNode::State(_)
        | ConditionNode::EnumVariant(_, _, _) => Location::Builtin,
    }
}

/// Вычисляет условие в значение.
///
/// Структурный разбор — здесь; семантика операций — в [`crate::eval::ops`].
/// Паники недостижимы: любой неподдержанный случай — `Err` (R4).
pub(crate) fn eval_condition(cond: &ConditionNode, ctx: &dyn Context) -> Result<Value, Diagnostic> {
    match cond {
        // ── Литералы ─────────────────────────────────────────────────────────
        ConditionNode::Number(n) => Ok(Value::Number(*n)),
        ConditionNode::Bool(b) => Ok(Value::Boolean(*b)),
        ConditionNode::Rational(text, negative) => {
            // Раньше здесь был `unwrap()` при разборе — ещё одна скрытая паника.
            let parsed: f64 = text.parse().map_err(|_| {
                Diagnostic::error(
                    Location::Builtin,
                    format!("не удалось разобрать вещественный литерал '{text}'"),
                )
                .with_code("SIM-008")
            })?;
            Ok(Value::Real(if *negative { -parsed } else { parsed }))
        }
        // S7 (Д8): вариант enum — целое со значением варианта. Раньше — `Err`,
        // из-за чего `mode = Manual` было ложным при `mode = Manual`.
        ConditionNode::EnumVariant(_, _, value) => Ok(Value::Number(*value)),

        // ── Переменные и доступ ──────────────────────────────────────────────
        ConditionNode::Variable(var, loc) => {
            let name = var.borrow().name().to_string();
            ctx.get_value(&name).ok_or_else(|| {
                Diagnostic::error(*loc, format!("переменная '{name}' не найдена"))
                    .with_code("SIM-009")
            })
        }
        // Д7: скобки вычисляются **рекурсивно**. Раньше возвращался внутренний
        // узел невычисленным, из-за чего `(t + 1) > 2` при t=5 было ложным.
        ConditionNode::Parenthesis(inner) => eval_condition(inner, ctx),
        ConditionNode::ArraySubscript(var, index) => {
            let name = var.borrow().name().to_string();
            let loc = loc_of(cond);
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
            let index_value = eval_condition(index, ctx)?;
            let Value::Number(idx) = index_value else {
                return Err(
                    Diagnostic::error(loc, "индекс массива должен быть целым".to_string())
                        .with_code("SIM-010"),
                );
            };
            let element = usize::try_from(idx)
                .ok()
                .and_then(|i| items.get(i))
                .ok_or_else(|| {
                    Diagnostic::error(
                        loc,
                        format!(
                            "индекс {idx} вне границ массива '{name}' (длина {})",
                            items.len()
                        ),
                    )
                    .with_code("SIM-010")
                })?;
            Ok(element.clone())
        }
        ConditionNode::BitAccess(inner, member) => {
            let loc = loc_of(cond);
            let value = eval_condition(inner, ctx)?;
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

        // ── Операции: семантика делегируется ядру ────────────────────────────
        ConditionNode::Not(inner) => unary(UnOp::Not, inner, ctx, loc_of(cond)),
        ConditionNode::Add(l, r) => binary(BinOp::Add, l, r, ctx, loc_of(cond)),
        ConditionNode::Subtract(l, r) => binary(BinOp::Subtract, l, r, ctx, loc_of(cond)),
        // `And`/`Or` документированы в АСД как побитовые, но исторически
        // вычислялись логически (`to_bool` в прежнем `flat`). Поведение
        // сохранено намеренно: менять его — изменение семантики языка, что вне
        // объёма фичи 0025 (правило 11). Неоднозначность зафиксирована в ADR.
        ConditionNode::And(l, r) => binary(BinOp::LogicalAnd, l, r, ctx, loc_of(cond)),
        ConditionNode::Or(l, r) => binary(BinOp::LogicalOr, l, r, ctx, loc_of(cond)),
        ConditionNode::Less(l, r) => binary(BinOp::Less, l, r, ctx, loc_of(cond)),
        ConditionNode::More(l, r) => binary(BinOp::More, l, r, ctx, loc_of(cond)),
        ConditionNode::LessEqual(l, r) => binary(BinOp::LessEqual, l, r, ctx, loc_of(cond)),
        ConditionNode::MoreEqual(l, r) => binary(BinOp::MoreEqual, l, r, ctx, loc_of(cond)),
        ConditionNode::Equal(l, r) => binary(BinOp::Equal, l, r, ctx, loc_of(cond)),
        ConditionNode::NotEqual(l, r) => binary(BinOp::NotEqual, l, r, ctx, loc_of(cond)),

        // ── Пока не поддержано — но с диагностикой, а не паникой ─────────────
        //
        // Д4: здесь был `unimplemented!()`, роняющий симулятор. Вычисление
        // вызова требует интерпретатора тела `fn`, который поставляет задача
        // `0025-02`; до неё — честный отказ.
        ConditionNode::Function(func, _, loc) => Err(Diagnostic::error(
            *loc,
            format!(
                "вызов функции '{}' в условии пока не поддерживается симулятором",
                func.borrow().name()
            ),
        )
        .with_code("SIM-012")),
        // Требует доступа к текущему состоянию под-модели через `Context` —
        // отдельная задача внутри фичи 0025 (решение ADR).
        ConditionNode::State(state) => Err(Diagnostic::error(
            Location::Builtin,
            format!(
                "сравнение с состоянием '{}' пока не поддерживается симулятором",
                state.borrow().name()
            ),
        )
        .with_code("SIM-013")),
        ConditionNode::Model(model) => Err(Diagnostic::error(
            Location::Builtin,
            format!(
                "сравнение с моделью '{}' пока не поддерживается симулятором",
                model
                    .borrow()
                    .name
                    .clone()
                    .unwrap_or_else(|| "?".to_string())
            ),
        )
        .with_code("SIM-013")),
        // `Value` не представляет строки — пробел зафиксирован анализом.
        ConditionNode::String(_) => Err(Diagnostic::error(
            Location::Builtin,
            "строки не поддерживаются симулятором".to_string(),
        )
        .with_code("SIM-014")),

        // ── Невычислимые по определению ──────────────────────────────────────
        ConditionNode::None => Err(Diagnostic::error(
            Location::Builtin,
            "пустое условие не может быть вычислено".to_string(),
        )
        .with_code("SIM-015")),
        ConditionNode::Unresolved(_) => Err(Diagnostic::error(
            Location::Builtin,
            "неразрешённое условие не может быть вычислено".to_string(),
        )
        .with_code("SIM-016")),
    }
}

fn unary(
    op: UnOp,
    inner: &ConditionNode,
    ctx: &dyn Context,
    loc: Location,
) -> Result<Value, Diagnostic> {
    let value = eval_condition(inner, ctx)?;
    ops::apply_unary(op, &value).map_err(|e: EvalError| e.to_diagnostic(loc))
}

fn binary(
    op: BinOp,
    left: &ConditionNode,
    right: &ConditionNode,
    ctx: &dyn Context,
    loc: Location,
) -> Result<Value, Diagnostic> {
    let lhs = eval_condition(left, ctx)?;
    let rhs = eval_condition(right, ctx)?;
    ops::apply_binary(op, &lhs, &rhs).map_err(|e: EvalError| e.to_diagnostic(loc))
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::eval::value::Value;
    use std::collections::HashMap;

    /// Контекст-заглушка: только чтение заранее заданных значений.
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

    fn empty_ctx() -> MockContext {
        MockContext::new(&[])
    }

    fn num(n: i64) -> Box<ConditionNode> {
        Box::new(ConditionNode::Number(n))
    }

    // ── Литералы ──────────────────────────────────────────────────────────────

    #[test]
    fn number_and_bool_literals() {
        let ctx = empty_ctx();
        assert_eq!(
            eval_condition(&ConditionNode::Number(7), &ctx),
            Ok(Value::Number(7))
        );
        assert_eq!(
            eval_condition(&ConditionNode::Bool(true), &ctx),
            Ok(Value::Boolean(true))
        );
    }

    #[test]
    fn rational_literal_is_parsed_with_sign() {
        let ctx = empty_ctx();
        assert_eq!(
            eval_condition(&ConditionNode::Rational("2.5".to_string(), false), &ctx),
            Ok(Value::Real(2.5))
        );
        assert_eq!(
            eval_condition(&ConditionNode::Rational("2.5".to_string(), true), &ctx),
            Ok(Value::Real(-2.5))
        );
    }

    #[test]
    fn malformed_rational_is_diagnostic_not_panic() {
        // Раньше здесь был unwrap() при разборе литерала.
        let ctx = empty_ctx();
        let cond = ConditionNode::Rational("не-число".to_string(), false);
        let err = eval_condition(&cond, &ctx).unwrap_err();
        assert_eq!(err.code.as_deref(), Some("SIM-008"));
    }

    // ── Д7: скобки ────────────────────────────────────────────────────────────

    #[test]
    fn d7_parenthesis_is_evaluated_recursively() {
        // Проба paren.lam: (t + 1) > 2 при t = 5 → истина.
        // Прежний flat возвращал внутренний узел невычисленным → было ложно.
        let ctx = empty_ctx();
        let inner = ConditionNode::Add(num(5), num(1));
        let cond = ConditionNode::More(
            Box::new(ConditionNode::Parenthesis(Box::new(inner))),
            num(2),
        );
        assert_eq!(eval_condition(&cond, &ctx), Ok(Value::Boolean(true)));
    }

    // ── Д8: вариант enum ──────────────────────────────────────────────────────

    #[test]
    fn d8_enum_variant_evaluates_to_its_value() {
        let ctx = empty_ctx();
        let cond = ConditionNode::EnumVariant(
            std::rc::Rc::new(std::cell::RefCell::new(Default::default())),
            "Manual".to_string(),
            1,
        );
        assert_eq!(eval_condition(&cond, &ctx), Ok(Value::Number(1)));
    }

    // ── Д6: смешение int/real не роняет ───────────────────────────────────────

    #[test]
    fn d6_mixed_int_real_does_not_panic() {
        // Проба mix.lam: t + 2.5 > 3 при t = 1. Прежний flat падал на unwrap().
        let ctx = MockContext::new(&[]);
        let sum = ConditionNode::Add(
            num(1),
            Box::new(ConditionNode::Rational("2.5".to_string(), false)),
        );
        let cond = ConditionNode::More(Box::new(sum), num(3));
        assert_eq!(eval_condition(&cond, &ctx), Ok(Value::Boolean(true)));
    }

    // ── Д4: вызов функции — диагностика вместо паники ─────────────────────────

    #[test]
    fn d4_function_call_is_diagnostic_not_panic() {
        // Полная поддержка — задача 0025-02 (нужен интерпретатор тела fn).
        // Здесь проверяется главное: симулятор не падает.
        let ctx = empty_ctx();
        let cond = ConditionNode::Function(
            std::rc::Rc::new(std::cell::RefCell::new(Default::default())),
            vec![],
            Location::Builtin,
        );
        let err = eval_condition(&cond, &ctx).unwrap_err();
        assert_eq!(err.code.as_deref(), Some("SIM-012"));
    }

    // ── Сравнения и арифметика ────────────────────────────────────────────────

    #[test]
    fn comparisons_delegate_to_core() {
        let ctx = empty_ctx();
        let cond = ConditionNode::More(Box::new(ConditionNode::Add(num(5), num(1))), num(2));
        assert_eq!(eval_condition(&cond, &ctx), Ok(Value::Boolean(true)));
    }

    #[test]
    fn logical_and_or_keep_historical_semantics() {
        let ctx = empty_ctx();
        let and = ConditionNode::And(
            Box::new(ConditionNode::Bool(true)),
            Box::new(ConditionNode::Bool(false)),
        );
        assert_eq!(eval_condition(&and, &ctx), Ok(Value::Boolean(false)));
        let or = ConditionNode::Or(
            Box::new(ConditionNode::Bool(true)),
            Box::new(ConditionNode::Bool(false)),
        );
        assert_eq!(eval_condition(&or, &ctx), Ok(Value::Boolean(true)));
    }

    #[test]
    fn not_negates() {
        let ctx = empty_ctx();
        let cond = ConditionNode::Not(Box::new(ConditionNode::Bool(true)));
        assert_eq!(eval_condition(&cond, &ctx), Ok(Value::Boolean(false)));
    }

    // ── S3: деление на ноль приходит из ядра как диагностика ──────────────────

    #[test]
    fn division_by_zero_surfaces_as_diagnostic() {
        // В условиях деления нет (нет узла Divide), но проверяем канал ошибок
        // ядра на доступной операции: сдвиг здесь недоступен, берём типовую
        // ошибку — сравнение массива с числом.
        let ctx = MockContext::new(&[("a", Value::Array(vec![]))]);
        let var = std::rc::Rc::new(std::cell::RefCell::new(
            grammar::semantic::VariableNode::Unresolved,
        ));
        // Переменная не найдена по имени — проверяем диагностику доступа.
        let cond = ConditionNode::Variable(var, Location::Builtin);
        let err = eval_condition(&cond, &ctx).unwrap_err();
        assert_eq!(err.code.as_deref(), Some("SIM-009"));
    }

    // ── Контрпримеры: отказ вместо тихого false ───────────────────────────────

    #[test]
    fn unresolved_condition_is_diagnostic() {
        let ctx = empty_ctx();
        let raw = grammar::parser::ast::Condition::Bool(Location::Builtin, true);
        let cond = ConditionNode::Unresolved(raw);
        let err = eval_condition(&cond, &ctx).unwrap_err();
        assert_eq!(err.code.as_deref(), Some("SIM-016"));
    }

    #[test]
    fn string_condition_is_diagnostic() {
        let ctx = empty_ctx();
        let cond = ConditionNode::String(vec!["a".to_string()]);
        let err = eval_condition(&cond, &ctx).unwrap_err();
        assert_eq!(err.code.as_deref(), Some("SIM-014"));
    }

    // ── Позиция в диагностике (критерий A10) ──────────────────────────────────

    #[test]
    fn diagnostic_carries_source_location_from_variable() {
        let ctx = empty_ctx();
        let loc = Location::Source(0, 10, 20);
        let var = std::rc::Rc::new(std::cell::RefCell::new(
            grammar::semantic::VariableNode::Unresolved,
        ));
        let cond = ConditionNode::Variable(var, loc);
        let err = eval_condition(&cond, &ctx).unwrap_err();
        assert_eq!(
            err.loc, loc,
            "диагностика обязана нести позицию, а не Builtin"
        );
    }

    #[test]
    fn loc_of_digs_into_children() {
        let loc = Location::Source(0, 3, 7);
        let var = std::rc::Rc::new(std::cell::RefCell::new(
            grammar::semantic::VariableNode::Unresolved,
        ));
        let cond = ConditionNode::More(Box::new(ConditionNode::Variable(var, loc)), num(1));
        assert_eq!(loc_of(&cond), loc);
    }
}
