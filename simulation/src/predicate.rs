use crate::context::Context;
use crate::unit::Predicate;
use crate::value::Value;
use grammar::diagnostics::{Diagnostic, Location};
use grammar::parser::ast::Member;
use grammar::semantic::ConditionNode;

pub(crate) fn create_predicate(cond: &ConditionNode) -> Predicate {
    let name = condition_label(cond);
    let cond = cond.clone();
    Predicate::new(name, move |c: &dyn Context| match flat(&cond, c) {
        Ok(ConditionNode::Bool(b)) => b,
        Ok(ConditionNode::Number(n)) => n != 0,
        Ok(_) | Err(_) => false,
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

fn flat(cond: &ConditionNode, context: &dyn Context) -> Result<ConditionNode, Diagnostic> {
    match cond {
        ConditionNode::None => Ok(ConditionNode::None),
        ConditionNode::Unresolved(_) => Err(Diagnostic::error(
            Location::Builtin,
            "Unresolved condition not supported".to_string(),
        )),
        ConditionNode::ArraySubscript(_, _) => Err(Diagnostic::error(
            Location::Builtin,
            "Array subscript not implemented".to_string(),
        )),
        ConditionNode::Parenthesis(cond) => Ok(*cond.clone()),
        ConditionNode::BitAccess(cond, member) => {
            let cond = flat(cond, context)?;
            let ConditionNode::Variable(var, _) = cond else {
                return Err(Diagnostic::error(
                    Location::Builtin,
                    "Invalid operand".to_string(),
                ));
            };
            let var = &*var.borrow();
            let Some(value) = context.get_value(var.name()) else {
                return Err(Diagnostic::error(
                    Location::Builtin,
                    format!("Variable '{}' not found", var.name()),
                ));
            };
            let Member::Number(bit) = member else {
                return Err(Diagnostic::error(
                    Location::Builtin,
                    "Invalid member access".to_string(),
                ));
            };
            match value {
                Value::Number(n) => Ok(ConditionNode::Bool(n & (1 << bit) != 0)),
                Value::Real(_) => Ok(ConditionNode::Bool(false)),
                Value::Boolean(n) => Ok(ConditionNode::Bool(n)),
                Value::Array(_) => Err(Diagnostic::error(
                    Location::Builtin,
                    format!("Cannot access bit of array variable '{}'", var.name()),
                )),
            }
        }
        ConditionNode::Function(_fun, _params, _) => {
            unimplemented!()
        }
        ConditionNode::Not(cond) => {
            let inner = flat(cond, context)?;
            match inner {
                ConditionNode::Bool(b) => Ok(ConditionNode::Bool(!b)),
                ConditionNode::Number(n) => Ok(ConditionNode::Bool(n == 0)),
                _ => Ok(ConditionNode::Not(Box::new(inner))),
            }
        }
        ConditionNode::Add(left, right) => {
            let left = flat(left, context)?;
            let left = extract_number(left)?;
            let right = flat(right, context)?;
            let right = extract_number(right)?;
            if left.0.is_none() && right.0.is_none() {
                let result = left.1.unwrap() + right.1.unwrap();
                return Ok(ConditionNode::Rational(result.to_string(), result < 0.0));
            }
            let result = left.0.unwrap() + right.0.unwrap();
            Ok(ConditionNode::Number(result))
        }
        ConditionNode::Subtract(left, right) => {
            let left = flat(left, context)?;
            let left = extract_number(left)?;
            let right = flat(right, context)?;
            let right = extract_number(right)?;
            if left.0.is_none() && right.0.is_none() {
                let result = left.1.unwrap() - right.1.unwrap();
                return Ok(ConditionNode::Rational(result.to_string(), result < 0.0));
            }
            let result = left.0.unwrap() - right.0.unwrap();
            Ok(ConditionNode::Number(result))
        }
        ConditionNode::And(left, right) => {
            let left = flat(left, context)?;
            let right = flat(right, context)?;
            match (to_bool(&left), to_bool(&right)) {
                (Some(l), Some(r)) => Ok(ConditionNode::Bool(l && r)),
                _ => Err(Diagnostic::error(
                    Location::Builtin,
                    "Invalid operand".to_string(),
                )),
            }
        }
        ConditionNode::Or(left, right) => {
            let left = flat(left, context)?;
            let right = flat(right, context)?;
            match (to_bool(&left), to_bool(&right)) {
                (Some(l), Some(r)) => Ok(ConditionNode::Bool(l || r)),
                _ => Err(Diagnostic::error(
                    Location::Builtin,
                    "Invalid operand".to_string(),
                )),
            }
        }
        ConditionNode::Less(left, right) => {
            let left = flat(left, context)?;
            let left = extract_number(left)?;
            let right = flat(right, context)?;
            let right = extract_number(right)?;
            if left.0.is_none() && right.0.is_none() {
                return Ok(ConditionNode::Bool(left.1.unwrap() < right.1.unwrap()));
            }
            Ok(ConditionNode::Bool(left.0.unwrap() < right.0.unwrap()))
        }
        ConditionNode::More(left, right) => {
            let left = flat(left, context)?;
            let left = extract_number(left)?;
            let right = flat(right, context)?;
            let right = extract_number(right)?;
            if left.0.is_none() && right.0.is_none() {
                return Ok(ConditionNode::Bool(left.1.unwrap() > right.1.unwrap()));
            }
            Ok(ConditionNode::Bool(left.0.unwrap() > right.0.unwrap()))
        }
        ConditionNode::LessEqual(left, right) => {
            let left = flat(left, context)?;
            let left = extract_number(left)?;
            let right = flat(right, context)?;
            let right = extract_number(right)?;
            if left.0.is_none() && right.0.is_none() {
                return Ok(ConditionNode::Bool(left.1.unwrap() <= right.1.unwrap()));
            }
            Ok(ConditionNode::Bool(left.0.unwrap() <= right.0.unwrap()))
        }
        ConditionNode::MoreEqual(left, right) => {
            let left = flat(left, context)?;
            let left = extract_number(left)?;
            let right = flat(right, context)?;
            let right = extract_number(right)?;
            if left.0.is_none() && right.0.is_none() {
                return Ok(ConditionNode::Bool(left.1.unwrap() >= right.1.unwrap()));
            }
            Ok(ConditionNode::Bool(left.0.unwrap() >= right.0.unwrap()))
        }
        ConditionNode::Equal(left, right) => {
            let left = flat(left, context)?;
            let right = flat(right, context)?;
            Ok(ConditionNode::Bool(condition_nodes_equal(&left, &right)))
        }
        ConditionNode::NotEqual(left, right) => {
            let left = flat(left, context)?;
            let right = flat(right, context)?;
            Ok(ConditionNode::Bool(!condition_nodes_equal(&left, &right)))
        }
        ConditionNode::Number(n) => Ok(ConditionNode::Number(*n)),
        ConditionNode::Rational(n, neg) => Ok(ConditionNode::Rational(n.clone(), *neg)),
        ConditionNode::String(_) => Err(Diagnostic::error(
            Location::Builtin,
            "String comparison not supported".to_string(),
        )),
        ConditionNode::Bool(b) => Ok(ConditionNode::Bool(*b)),
        ConditionNode::Variable(var, _) => {
            let var = &*var.borrow();
            let Some(value) = context.get_value(var.name()) else {
                return Err(Diagnostic::error(
                    Location::Builtin,
                    format!("Variable '{}' not found", var.name()),
                ));
            };
            match value {
                Value::Number(n) => Ok(ConditionNode::Number(n)),
                Value::Real(n) => Ok(ConditionNode::Rational(n.to_string(), false)),
                Value::Boolean(n) => Ok(ConditionNode::Bool(n)),
                Value::Array(_) => Err(Diagnostic::error(
                    Location::Builtin,
                    format!("Cannot access bit of array variable '{}'", var.name()),
                )),
            }
        }
        ConditionNode::Model(_) => Err(Diagnostic::error(
            Location::Builtin,
            "Model comparison not supported".to_string(),
        )),
        ConditionNode::State(_) => Err(Diagnostic::error(
            Location::Builtin,
            "State comparison not supported".to_string(),
        )),
        ConditionNode::EnumVariant(_, _, _) => Err(Diagnostic::error(
            Location::Builtin,
            "Enum variant comparison not supported".to_string(),
        )),
    }
}

fn condition_nodes_equal(a: &ConditionNode, b: &ConditionNode) -> bool {
    match (a, b) {
        (ConditionNode::Bool(x), ConditionNode::Bool(y)) => x == y,
        (ConditionNode::Number(x), ConditionNode::Number(y)) => x == y,
        (ConditionNode::Number(x), ConditionNode::Rational(y, neg)) => {
            let y: f64 = if *neg {
                -y.parse().unwrap_or(0.0)
            } else {
                y.parse().unwrap_or(0.0)
            };
            (*x as f64) == y
        }
        (ConditionNode::Rational(x, neg_x), ConditionNode::Rational(y, neg_y)) => {
            let xv: f64 = if *neg_x {
                -x.parse().unwrap_or(0.0)
            } else {
                x.parse().unwrap_or(0.0)
            };
            let yv: f64 = if *neg_y {
                -y.parse().unwrap_or(0.0)
            } else {
                y.parse().unwrap_or(0.0)
            };
            xv == yv
        }
        (ConditionNode::Rational(x, neg), ConditionNode::Number(y)) => {
            let xv: f64 = if *neg {
                -x.parse().unwrap_or(0.0)
            } else {
                x.parse().unwrap_or(0.0)
            };
            xv == (*y as f64)
        }
        _ => false,
    }
}

fn to_bool(node: &ConditionNode) -> Option<bool> {
    match node {
        ConditionNode::Bool(b) => Some(*b),
        ConditionNode::Number(n) => Some(*n != 0),
        _ => None,
    }
}

fn extract_number(left: ConditionNode) -> Result<(Option<i64>, Option<f64>), Diagnostic> {
    Ok(if let ConditionNode::Number(n) = left {
        (Some(n), None)
    } else if let ConditionNode::Rational(n, neg) = left {
        (
            None,
            Some(if neg {
                -n.parse::<f64>().unwrap()
            } else {
                n.parse::<f64>().unwrap()
            }),
        )
    } else {
        return Err(Diagnostic::error(
            Location::Builtin,
            "Invalid operand".to_string(),
        ));
    })
}
