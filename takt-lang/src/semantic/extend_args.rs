//! Аргументы инстанцирования модели: `M(ИМЯ := ВЫРАЖЕНИЕ, …)` — фича 0185.
//!
//! Разбирает список аргументов при имени модели в выражении реализации и
//! проверяет его **структурно**: форма аргумента, существование параметра,
//! отсутствие повторов. Значение аргумента остаётся сырым выражением — его
//! вычисляет константный вычислитель (задача 0185-03), а применяет потребитель
//! дерева (0185-04/05).
//!
//! ⚠️ Каждая ошибка употребления получает **свой** код: `M(unknown := 1)` и
//! `M(acc := 1)` — разные ошибки автора (опечатка против попытки задать
//! переменную), и общий текст «плохой аргумент» заставил бы гадать.

use crate::diagnostics::{Diagnostic, Location};
use crate::parser::ast;
use crate::semantic::ModelNode;
use crate::semantic::extend::ParameterArgument;
use std::cell::RefCell;
use std::collections::BTreeMap;
use std::rc::Rc;

/// Разбирает аргументы инстанцирования и сверяет их с параметрами модели.
///
/// `target` — модель, которую инстанцируют; `call_loc` — позиция вызова целиком
/// (для диагностик, у которых своей позиции нет).
pub(super) fn parse_arguments(
    target: &Rc<RefCell<ModelNode>>,
    model_name: &str,
    args: &[ast::Expression],
    call_loc: Location,
) -> Result<Vec<ParameterArgument>, Diagnostic> {
    let mut parsed: Vec<ParameterArgument> = Vec::with_capacity(args.len());
    // Позиция первого вхождения имени — чтобы повтор указал на оба места.
    let mut seen: BTreeMap<String, Location> = BTreeMap::new();

    for arg in args {
        let (loc, name, value) = destructure(arg, call_loc)?;
        check_declared(target, model_name, &name, loc)?;
        if let Some(first) = seen.get(&name) {
            return Err(Diagnostic::error(
                loc,
                format!("Параметр '{name}' задан в этом вызове дважды"),
            )
            .with_code("SE-080")
            .with_note(*first, format!("первое задание '{name}'")));
        }
        seen.insert(name.clone(), loc);
        parsed.push(ParameterArgument { name, loc, value });
    }
    Ok(parsed)
}

/// Разбирает один аргумент в тройку «позиция имени, имя, значение».
///
/// Единственная допустимая форма — `ИМЯ := ВЫРАЖЕНИЕ`. Позиционных аргументов
/// нет намеренно: у параметров есть значения по умолчанию, поэтому позиция
/// ничего не значит, а порядок объявления автор менять вправе.
fn destructure(
    arg: &ast::Expression,
    call_loc: Location,
) -> Result<(Location, String, ast::Expression), Diagnostic> {
    let ast::Expression::Assign(assign_loc, target, value) = arg else {
        return Err(Diagnostic::error(
            arg_loc(arg).unwrap_or(call_loc),
            "Аргумент инстанцирования задаётся формой 'имя := значение'".to_string(),
        )
        .with_code("SE-076"));
    };
    match target.as_ref() {
        ast::Expression::Variable(id) => Ok((id.loc, id.name.clone(), (**value).clone())),
        other => Err(Diagnostic::error(
            arg_loc(other).unwrap_or(*assign_loc),
            "Слева от ':=' в аргументе инстанцирования обязано стоять имя параметра".to_string(),
        )
        .with_code("SE-076")),
    }
}

/// Проверяет, что имя обозначает **параметр** целевой модели.
fn check_declared(
    target: &Rc<RefCell<ModelNode>>,
    model_name: &str,
    name: &str,
    loc: Location,
) -> Result<(), Diagnostic> {
    let target = target.borrow();
    if target.parameters.iter().any(|p| p.name == name) {
        return Ok(());
    }
    // Модель параметров не объявляет вовсе — автор, скорее всего, перепутал
    // модель, а не имя.
    if target.parameters.is_empty() {
        return Err(Diagnostic::error(
            loc,
            format!("Модель '{model_name}' не объявляет параметров, задать '{name}' нечем"),
        )
        .with_code("SE-077"));
    }
    // Имя в модели есть, но это переменная, константа или порт: задавать их при
    // инстанцировании нельзя — это величины такта, а не настройка сборки.
    if target.variables.contains_key(name) {
        return Err(Diagnostic::error(
            loc,
            format!(
                "'{name}' в модели '{model_name}' объявлен не как parameter — \
                 при инстанцировании задаются только параметры"
            ),
        )
        .with_code("SE-079"));
    }
    let known = target
        .parameters
        .iter()
        .map(|p| p.name.as_str())
        .collect::<Vec<_>>()
        .join(", ");
    Err(Diagnostic::error(
        loc,
        format!("Модель '{model_name}' не имеет параметра '{name}' (объявлены: {known})"),
    )
    .with_code("SE-078"))
}

/// Позиция выражения, если она у варианта есть.
///
/// Нужна только для сообщений: у аргумента неверной формы своей позиции может и
/// не быть, тогда указываем на вызов целиком.
fn arg_loc(expr: &ast::Expression) -> Option<Location> {
    match expr {
        ast::Expression::Variable(id) => Some(id.loc),
        ast::Expression::Assign(loc, _, _)
        | ast::Expression::Number(loc, _)
        | ast::Expression::Function(loc, _, _)
        | ast::Expression::Parenthesis(loc, _) => Some(*loc),
        _ => None,
    }
}
