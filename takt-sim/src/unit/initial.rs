//! Вычислитель **начальных значений** переменных и портов (фича 0163).
//!
//! # Зачем отдельный модуль
//!
//! Это **второй** вычислитель проекта над `ExpressionNode`. Первый — полный
//! интерпретатор `takt-sim/src/eval/`, работающий в такте; этот считает только
//! то, что известно **до** первого такта: литералы, их отрицание, скобки и
//! списочные инициализаторы. Слить их нельзя — у них разные входы (здесь нет ни
//! контекста, ни портов) и разная обязанность, а попытка слияния уже стоила бы
//! порт-регресса (урок 0080).
//!
//! # Почему модуль объявляет `deny`
//!
//! Пока вычислитель жил в `builder.rs` под веткой `_ => None`, добавление
//! варианта `ExpressionNode` **не ломало сборку**: новый узел молча получал
//! «значения нет». Ровно этот класс дал восемь дефектов фичи 0025 и был
//! закреплён правилом ADR 0093 — но `deny` стоял только в `eval/`, и второй
//! вычислитель оставался вне его действия.
//!
//! `#![deny(clippy::wildcard_enum_match_arm)]` переносит проверку на
//! компилятор: `_ =>` по узлу языка здесь просто не соберётся, а новый вариант
//! обязан быть разобран явно — пусть даже отнесён к «не константа». Сторож
//! самого атрибута — `scripts/check-exhaustive-nodes.sh`: снять его молча тоже
//! не выйдет.
//!
//! ⚠️ Поведение при этом **не менялось**: варианты, прежде попадавшие в
//! `_ => None`, перечислены явно и дают тот же `None`. Смысл правки — не в
//! результате, а в том, что следующий вариант языка потребует решения, а не
//! получит умолчание.

#![deny(clippy::wildcard_enum_match_arm)]

use crate::eval::value::Value;
use takt_lang::semantic::ExpressionNode;

/// Вычисляет простое константное выражение в [`Value`].
///
/// `None` — «константой не является»: значение будет взято из умолчания по типу
/// (`default_field` в `builder.rs`) либо вычислено в такте.
pub(super) fn eval_expr(expr: &ExpressionNode) -> Option<Value> {
    match expr {
        ExpressionNode::Number(n) => Some(Value::Number(*n)),
        // ⚠️ Длительность (фича 0134) обязана быть здесь: её отсутствие молча
        // превращало `var left: duration := 1m30s;` в «значения нет» (тест
        // поймал ровно это).
        ExpressionNode::Duration(ns) => Some(Value::Duration(*ns)),
        ExpressionNode::Bool(b) => Some(Value::Boolean(*b)),
        ExpressionNode::Rational(s, neg) => {
            let v: f64 = s.parse().ok()?;
            Some(Value::Real(if *neg { -v } else { v }))
        }
        ExpressionNode::Negate(inner) => match eval_expr(inner)? {
            Value::Number(n) => Some(Value::Number(-n)),
            Value::Real(f) => Some(Value::Real(-f)),
            // Отрицание прочих значений смысла не имеет. Перечислено явно, хотя
            // `Value` — тип крейта, а не узел языка: модульный `deny` не
            // различает их, и это к лучшему. Появится новый род значения
            // (например, ещё одно числовое представление) — компилятор
            // потребует решить, отрицаемо ли оно, вместо молчаливого `None`.
            Value::Boolean(_)
            | Value::Array(_)
            | Value::Fixed { .. }
            | Value::Struct { .. }
            | Value::Duration(_) => None,
        },
        ExpressionNode::Parenthesis(inner) => eval_expr(inner),
        // Инициализатор `{…}` структуры (фича 0034) и массивный литерал `[…]` —
        // оба дают список значений; тип цели (структура/массив) различит
        // `coerce_initial` по объявленному типу переменной.
        ExpressionNode::Array(items) | ExpressionNode::Initializer(items) => {
            let values: Option<Vec<Value>> = items.iter().map(eval_expr).collect();
            Some(Value::Array(values?))
        }
        // Адресные порты (`bit := 0x600:0`) инициализируются нулём по умолчанию.
        ExpressionNode::Address(_, _) => Some(Value::Number(0)),

        // ── Не константа ────────────────────────────────────────────────────
        //
        // Перечислено ЯВНО, а не сведено в `_ =>`, и в этом весь смысл модуля:
        // добавив вариант языка, автор обязан вписать его сюда — то есть
        // ответить, константа он или нет. Умолчания у нового узла больше нет.
        //
        // Здесь собраны узлы, значение которых до первого такта неизвестно:
        // они читают переменные и порты, зовут функции, ветвятся или вовсе не
        // являются выражением-значением.
        ExpressionNode::None
        | ExpressionNode::Unresolved(..)
        | ExpressionNode::Variable(..)
        | ExpressionNode::Model(..)
        | ExpressionNode::Condition(..)
        | ExpressionNode::Type(..)
        | ExpressionNode::String(..)
        | ExpressionNode::List(..)
        | ExpressionNode::CodeBlock(..)
        | ExpressionNode::NamedFunctionBox(..)
        | ExpressionNode::Function(..)
        | ExpressionNode::ArraySubscript(..)
        | ExpressionNode::ArraySlice(..)
        | ExpressionNode::BitAccess(..)
        | ExpressionNode::Cast(..)
        | ExpressionNode::Assign(..)
        | ExpressionNode::ConditionalOperator(..)
        // Унарные и бинарные операции: свёртки констант этот вычислитель не
        // делает намеренно — она принадлежит семантике, а не построению юнита.
        | ExpressionNode::Not(..)
        | ExpressionNode::BitwiseNot(..)
        | ExpressionNode::UnaryPlus(..)
        | ExpressionNode::Power(..)
        | ExpressionNode::Multiply(..)
        | ExpressionNode::Divide(..)
        | ExpressionNode::Modulo(..)
        | ExpressionNode::Add(..)
        | ExpressionNode::Subtract(..)
        | ExpressionNode::ShiftLeft(..)
        | ExpressionNode::ShiftRight(..)
        | ExpressionNode::BitwiseAnd(..)
        | ExpressionNode::BitwiseXor(..)
        | ExpressionNode::BitwiseOr(..)
        | ExpressionNode::Less(..)
        | ExpressionNode::More(..)
        | ExpressionNode::LessEqual(..)
        | ExpressionNode::MoreEqual(..)
        | ExpressionNode::Equal(..)
        | ExpressionNode::NotEqual(..)
        | ExpressionNode::And(..)
        | ExpressionNode::Or(..) => None,
    }
}
