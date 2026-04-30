use crate::context::Context;
use crate::state::Predicate;
use crate::value::Value;
use grammar::diagnostics::{Diagnostic, Location};
use grammar::parser::ast::Member;
use grammar::semantic::ConditionNode;

pub(crate) fn create_predicate(cond: &ConditionNode) -> Predicate {
    let cond = cond.clone();
    Box::new(move |c| match flat(&cond, c) {
        Ok(ConditionNode::Bool(b)) => b,
        _ => panic!(),
    })
}

pub fn test(cond: &ConditionNode, context: &dyn Context) -> Result<bool, Diagnostic> {
    match flat(cond, context)? {
        ConditionNode::Bool(b) => Ok(b),
        _ => Err(Diagnostic::error(
            Location::Builtin,
            "Invalid condition".to_string(),
        )),
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
                Value::Real(n) => Ok(ConditionNode::Bool(false)),
                Value::Boolean(n) => Ok(ConditionNode::Bool(n)),
                Value::Array(n) => Err(Diagnostic::error(
                    Location::Builtin,
                    format!("Cannot access bit of array variable '{}'", var.name()),
                )),
            }
        }
        ConditionNode::Function(fun, params, _) => {
            unimplemented!()
        }
        ConditionNode::Not(cond) => Ok(ConditionNode::Not(Box::new(flat(cond, context)?))),
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
            if let ConditionNode::Bool(left) = left
                && let ConditionNode::Bool(right) = right
            {
                return Ok(ConditionNode::Bool(left && right));
            }
            Err(Diagnostic::error(
                Location::Builtin,
                "Invalid operand".to_string(),
            ))
        }
        ConditionNode::Or(left, right) => {
            let left = flat(left, context)?;
            let right = flat(right, context)?;
            if let ConditionNode::Bool(left) = left
                && let ConditionNode::Bool(right) = right
            {
                return Ok(ConditionNode::Bool(left || right));
            }
            Err(Diagnostic::error(
                Location::Builtin,
                "Invalid operand".to_string(),
            ))
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
        ConditionNode::Equal(left, right) => unimplemented!(),
        ConditionNode::NotEqual(left, right) => unimplemented!(),
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
