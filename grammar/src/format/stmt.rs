//! Печать операторов и блоков.

use super::{FormatError, Out, expr};
use crate::parser::ast;

/// Печатает блок с заголовком: `<head>{ … }` с телом на отдельных строках.
///
/// Используется и для именованных блоков (`enter`/`exit`/`always`), и для тел
/// функций — раскладка у них одна.
pub(crate) fn block_with_head(
    out: &mut Out,
    head: &str,
    statement: &ast::Statement,
) -> Result<(), FormatError> {
    let ast::Statement::Block { statements, .. } = statement else {
        // Тело не-блок: печатаем как одиночный оператор в фигурных скобках,
        // чтобы канон был единообразным.
        out.line(&format!("{head}{{"));
        out.up();
        print(out, statement)?;
        out.down();
        out.line("}");
        return Ok(());
    };
    if statements.is_empty() {
        out.line(&format!("{head}{{}}"));
        return Ok(());
    }
    out.line(&format!("{head}{{"));
    out.up();
    for s in statements {
        print(out, s)?;
    }
    out.down();
    out.line("}");
    Ok(())
}

/// Печатает оператор.
///
/// `ast::Statement` помечен `#[non_exhaustive]`, поэтому ветка `_` **вынужденная**
/// (как `TypeNode` в фиче 0025). Она возвращает **ошибку**: неизвестный оператор
/// нельзя молча выбросить из исходника пользователя.
#[allow(clippy::wildcard_enum_match_arm)]
pub(crate) fn print(out: &mut Out, statement: &ast::Statement) -> Result<(), FormatError> {
    use ast::Statement as S;
    match statement {
        S::Block { statements, .. } => {
            out.line("{");
            out.up();
            for s in statements {
                print(out, s)?;
            }
            out.down();
            out.line("}");
            Ok(())
        }
        S::Expression(_, e) => {
            out.line(&format!("{};", expr::expression(e)?));
            Ok(())
        }
        S::Variable(_, define, init) => {
            let head = expr::variable_define(define)?;
            match init {
                Some(init) => out.line(&format!("{head} := {};", expr::expression(init)?)),
                None => out.line(&format!("{head};")),
            }
            Ok(())
        }
        S::If(_, cond, then_, else_) => {
            block_with_head(out, &format!("if {} ", expr::expression(cond)?), then_)?;
            if let Some(else_) = else_ {
                // `else if` печатается цепочкой, а не вложенным блоком.
                if matches!(**else_, S::If(..)) {
                    // Заголовок `else ` + рекурсивный `if` дал бы разрыв строки,
                    // поэтому печатаем `else` отдельной строкой перед `if`.
                    out.line("else");
                    print(out, else_)?;
                } else {
                    block_with_head(out, "else ", else_)?;
                }
            }
            Ok(())
        }
        S::Loop(_, cond, body) => {
            let head = match cond {
                Some(cond) => format!("loop {} ", expr::expression(cond)?),
                None => "loop ".to_string(),
            };
            block_with_head(out, &head, body)
        }
        S::For(_, init, cond, step, body) => {
            let init = match init {
                Some(init) => single_line(init)?,
                None => ";".to_string(),
            };
            let cond = match cond {
                Some(cond) => expr::expression(cond)?,
                None => String::new(),
            };
            let step = match step {
                Some(step) => expr::expression(step)?,
                None => String::new(),
            };
            let head = format!("for {init} {cond}; {step} ");
            match body {
                Some(body) => block_with_head(out, &head, body),
                None => {
                    out.line(&format!("{head}{{}}"));
                    Ok(())
                }
            }
        }
        S::Continue(_) => {
            out.line("continue;");
            Ok(())
        }
        S::Break(_) => {
            out.line("break;");
            Ok(())
        }
        S::Return(_, value) => {
            match value {
                Some(value) => out.line(&format!("return {};", expr::expression(value)?)),
                None => out.line("return;"),
            }
            Ok(())
        }
        S::Match(_, subject, arms) => {
            out.line(&format!("match {} {{", expr::expression(subject)?));
            out.up();
            for arm in arms {
                let patterns = arm
                    .patterns
                    .iter()
                    .map(|p| match p {
                        ast::MatchPattern::Wildcard(_) => Ok("_".to_string()),
                        ast::MatchPattern::Value(e) => expr::expression(e),
                    })
                    .collect::<Result<Vec<_>, FormatError>>()?
                    .join(" | ");
                block_with_head(out, &format!("{patterns} => "), &arm.body)?;
            }
            out.down();
            out.line("}");
            Ok(())
        }
        // Вынужденная ветка: перечисление `#[non_exhaustive]`. Отказ, а не
        // молчаливая потеря оператора.
        other => Err(FormatError::Unsupported(format!("Statement::{other:?}"))),
    }
}

/// Печатает оператор в одну строку (для заголовка `for`).
fn single_line(statement: &ast::Statement) -> Result<String, FormatError> {
    use ast::Statement as S;
    match statement {
        S::Variable(_, define, init) => {
            let head = expr::variable_define(define)?;
            Ok(match init {
                Some(init) => format!("{head} := {};", expr::expression(init)?),
                None => format!("{head};"),
            })
        }
        S::Expression(_, e) => Ok(format!("{};", expr::expression(e)?)),
        other => Err(FormatError::Unsupported(format!(
            "инициализатор for: {other:?}"
        ))),
    }
}
