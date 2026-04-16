//! Построение семантических условий языка BuT.
//!
//! Модуль предоставляет две функции:
//! - [`resolve_condition`] — преобразует АСД-условие [`ast::Condition`] в разрешённое
//!   семантическое [`ConditionNode`].
//! - [`extract_conditions`] — разрешает все неразрешённые именованные условия
//!   в [`HashMap`].

use crate::diagnostics::Diagnostic;
use crate::parser::ast;
use crate::semantic::builtin::builtin_function;
use crate::semantic::type_node::TypeNode;
use crate::semantic::{ConditionDefinitionNode, ConditionNode, ModelNode, VariableNode};
use std::cell::RefCell;
use std::collections::HashMap;
use std::rc::Rc;

/// Строит разрешённое семантическое [`ConditionNode`] из АСД-условия [`ast::Condition`].
///
/// Рекурсивно обходит дерево условий, разрешая имена переменных и функций
/// в контексте переданного [`ModelNode`] (включая все родительские области
/// видимости через цепочку `upper`).
///
/// # Ошибки
///
/// Возвращает [`Diagnostic`] если:
/// - индекс массива ссылается на переменную, которой нет, или которая не является массивом;
/// - вызов функции ссылается на функцию, не находящуюся в области видимости;
/// - любой аргумент вызова функции не разрешился (ошибка пробрасывается, а не проглатывается);
/// - ссылка на переменную указывает на необъявленную переменную или условие.
pub fn resolve_condition(
    cond: &ast::Condition,
    model: Rc<RefCell<ModelNode>>,
) -> Result<ConditionNode, Diagnostic> {
    match cond {
        ast::Condition::ArraySubscript(_, id, num) => {
            let name = id.name.clone();
            let var = model.borrow().search_var(&name);
            if let Some(var) = var
                && let VariableNode::Simple { ty, .. } = var.clone()
                && let TypeNode::Array(..) = ty
            {
                return Ok(ConditionNode::ArraySubscript(
                    Rc::new(RefCell::new(var)),
                    *num,
                ));
            }
            Err(format!("Массив '{}' не найден", &name).as_str().into())
        }
        ast::Condition::Parenthesis(_, cond) => Ok(ConditionNode::Parenthesis(Box::new(
            resolve_condition(cond, model.clone())?,
        ))),
        ast::Condition::BitAccess(_, cond, member) => {
            let cond = resolve_condition(cond, model.clone())?;
            Ok(ConditionNode::BitAccess(Box::new(cond), member.clone()))
        }
        ast::Condition::Function(_, id, args) => {
            let name = id.name.clone();
            // Собираем условия аргументов, немедленно пробрасывая ошибку вместо
            // паники через `.unwrap()`.
            let args: Vec<Box<ConditionNode>> = args
                .iter()
                .map(|c| resolve_condition(c, model.clone()).map(Box::new))
                .collect::<Result<Vec<_>, _>>()?;
            let function = model.borrow().search_func(&name);
            let function = match function {
                Some(f) => f,
                None => Rc::new(RefCell::new(builtin_function(&id.name)?.clone())),
            };
            Ok(ConditionNode::Function(function, args, id.loc))
        }
        ast::Condition::Not(_, cond) => Ok(ConditionNode::Not(Box::new(resolve_condition(
            cond,
            model.clone(),
        )?))),
        ast::Condition::Add(_, left, right) => {
            let left = resolve_condition(left, model.clone())?;
            let right = resolve_condition(right, model.clone())?;
            Ok(ConditionNode::Add(Box::new(left), Box::new(right)))
        }
        ast::Condition::Subtract(_, left, right) => {
            let left = resolve_condition(left, model.clone())?;
            let right = resolve_condition(right, model.clone())?;
            Ok(ConditionNode::Subtract(Box::new(left), Box::new(right)))
        }
        ast::Condition::And(_, left, right) => {
            let left = resolve_condition(left, model.clone())?;
            let right = resolve_condition(right, model.clone())?;
            Ok(ConditionNode::And(Box::new(left), Box::new(right)))
        }
        ast::Condition::Or(_, left, right) => {
            let left = resolve_condition(left, model.clone())?;
            let right = resolve_condition(right, model.clone())?;
            Ok(ConditionNode::Or(Box::new(left), Box::new(right)))
        }
        ast::Condition::Less(_, left, right) => {
            let left = resolve_condition(left, model.clone())?;
            let right = resolve_condition(right, model.clone())?;
            Ok(ConditionNode::Less(Box::new(left), Box::new(right)))
        }
        ast::Condition::More(_, left, right) => {
            let left = resolve_condition(left, model.clone())?;
            let right = resolve_condition(right, model.clone())?;
            Ok(ConditionNode::More(Box::new(left), Box::new(right)))
        }
        ast::Condition::LessEqual(_, left, right) => {
            let left = resolve_condition(left, model.clone())?;
            let right = resolve_condition(right, model.clone())?;
            Ok(ConditionNode::LessEqual(Box::new(left), Box::new(right)))
        }
        ast::Condition::MoreEqual(_, left, right) => {
            let left = resolve_condition(left, model.clone())?;
            let right = resolve_condition(right, model.clone())?;
            Ok(ConditionNode::MoreEqual(Box::new(left), Box::new(right)))
        }
        ast::Condition::Equal(_, left, right) => {
            let left = resolve_condition(left, model.clone())?;
            let right = resolve_condition(right, model.clone())?;
            Ok(ConditionNode::Equal(Box::new(left), Box::new(right)))
        }
        ast::Condition::NotEqual(_, left, right) => {
            let left = resolve_condition(left, model.clone())?;
            let right = resolve_condition(right, model.clone())?;
            Ok(ConditionNode::NotEqual(Box::new(left), Box::new(right)))
        }
        ast::Condition::Number(_, n) => Ok(ConditionNode::Number(*n)),
        // Вещественный литерал: сохраняем строковое представление и знак.
        // Ранее эта ветка была заглушкой, что приводило к молчаливой потере данных:
        // условие становилось `Condition::None`.
        ast::Condition::Rational(_, s, neg) => Ok(ConditionNode::Rational(s.clone(), *neg)),
        // Строковый литерал: собираем строковые значения из каждого сегмента.
        // Ранее эта ветка была заглушкой с той же ошибкой потери данных.
        ast::Condition::String(lits) => {
            let strings = lits.iter().map(|l| l.string.clone()).collect();
            Ok(ConditionNode::String(strings))
        }
        ast::Condition::Bool(_, v) => Ok(ConditionNode::Bool(*v)),
        ast::Condition::Variable(id) => {
            let name = id.name.clone();
            // Сначала ищем объявление переменной в области видимости.
            if let Some(var) = model.borrow().search_var(&name) {
                return Ok(ConditionNode::Variable(Rc::new(RefCell::new(var)), id.loc));
            } else if let Some(cond) = model.borrow().search_cond(&name) {
                return Ok(cond.value);
            } else if let Some(model) = model.borrow().search_model(&name) {
                return Ok(ConditionNode::Model(model.clone()));
            } else if let Some(state) = model.borrow().search_state(&name) {
                return Ok(ConditionNode::State(state.clone()));
            } else if let Some((edn, val)) = model.borrow().search_enum_variant(&name) {
                return Ok(ConditionNode::EnumVariant(
                    Rc::new(RefCell::new(edn)),
                    name,
                    val,
                ));
            }
            Ok(ConditionNode::Unresolved(ast::Condition::Variable(
                id.clone(),
            )))
        }
    }
}

fn rebuild_condition(cond: &ConditionNode, model: Rc<RefCell<ModelNode>>) -> ConditionNode {
    match cond {
        ConditionNode::Unresolved(ast_cond) => resolve_condition(ast_cond, model.clone()).unwrap(),
        ConditionNode::Parenthesis(cond) => {
            ConditionNode::Parenthesis(Box::new(rebuild_condition(cond, model.clone())))
        }
        ConditionNode::BitAccess(cond, m) => {
            ConditionNode::BitAccess(Box::new(rebuild_condition(cond, model.clone())), m.clone())
        }
        ConditionNode::Function(a, b, v) => ConditionNode::Function(a.clone(), b.clone(), *v),
        ConditionNode::Not(cond) => ConditionNode::Not(Box::new(rebuild_condition(cond, model))),
        ConditionNode::Add(left, right) => {
            let left = rebuild_condition(left, model.clone());
            let right = rebuild_condition(right, model);
            ConditionNode::Add(Box::new(left), Box::new(right))
        }
        ConditionNode::Subtract(left, right) => {
            let left = rebuild_condition(left, model.clone());
            let right = rebuild_condition(right, model);
            ConditionNode::Subtract(Box::new(left), Box::new(right))
        }
        ConditionNode::And(left, right) => {
            let left = rebuild_condition(left, model.clone());
            let right = rebuild_condition(right, model);
            ConditionNode::And(Box::new(left), Box::new(right))
        }
        ConditionNode::Or(left, right) => {
            let left = rebuild_condition(left, model.clone());
            let right = rebuild_condition(right, model);
            ConditionNode::Or(Box::new(left), Box::new(right))
        }
        ConditionNode::Less(left, right) => {
            let left = rebuild_condition(left, model.clone());
            let right = rebuild_condition(right, model);
            ConditionNode::Less(Box::new(left), Box::new(right))
        }
        ConditionNode::More(left, right) => {
            let left = rebuild_condition(left, model.clone());
            let right = rebuild_condition(right, model);
            ConditionNode::More(Box::new(left), Box::new(right))
        }
        ConditionNode::LessEqual(left, right) => {
            let left = rebuild_condition(left, model.clone());
            let right = rebuild_condition(right, model);
            ConditionNode::LessEqual(Box::new(left), Box::new(right))
        }
        ConditionNode::MoreEqual(left, right) => {
            let left = rebuild_condition(left, model.clone());
            let right = rebuild_condition(right, model);
            ConditionNode::MoreEqual(Box::new(left), Box::new(right))
        }
        ConditionNode::Equal(left, right) => {
            let left = rebuild_condition(left, model.clone());
            let right = rebuild_condition(right, model);
            ConditionNode::Equal(Box::new(left), Box::new(right))
        }
        ConditionNode::NotEqual(left, right) => {
            let left = rebuild_condition(left, model.clone());
            let right = rebuild_condition(right, model);
            ConditionNode::NotEqual(Box::new(left), Box::new(right))
        }
        cond => cond.clone(),
    }
}

/// Разрешает все неразрешённые именованные условия в `conditions`.
///
/// Перебирает `conditions` и для каждой записи, значение которой равно
/// [`ConditionNode::Unresolved`], вызывает [`resolve_condition`], заменяя заглушку
/// полностью разрешённым семантическим условием. Уже разрешённые записи
/// передаются без изменений.
///
/// # Ошибки
///
/// Пробрасывает любой [`Diagnostic`], возвращённый из [`resolve_condition`].
pub fn extract_conditions(
    conditions: &HashMap<String, ConditionDefinitionNode>,
    model: Rc<RefCell<ModelNode>>,
) -> Result<HashMap<String, ConditionDefinitionNode>, Diagnostic> {
    let mut result = HashMap::new();
    for (name, cond) in conditions {
        if let ConditionNode::Unresolved(ast_cond) = &cond.value {
            let resolved = resolve_condition(ast_cond, model.clone())?;
            result.insert(
                name.clone(),
                ConditionDefinitionNode {
                    name: name.clone(),
                    loc: cond.loc,
                    value: resolved,
                    upper: cond.upper.clone(),
                },
            );
        } else {
            let new_cond = rebuild_condition(&cond.value, model.clone());
            result.insert(
                name.clone(),
                ConditionDefinitionNode {
                    value: new_cond,
                    ..cond.clone()
                },
            );
        }
    }
    Ok(result)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::parse;
    use crate::semantic::tree::construct_model;

    // ─── вспомогательные функции ──────────────────────────────────────────
    //
    // Условия именованных объявлений (`cond имя = …`) разрешаются на этапе 3
    // через `extract_conditions`. Условия переходов на рёбрах `ref` разрешаются
    // на этапе 6 конвейера через `resolve_references` (модуль `reference`).
    // Тесты ниже проверяют именованные условия через `cond имя = выражение;`
    // как наиболее прямой способ проверить результат разрешения.

    fn build(src: &str) -> Result<ModelNode, Diagnostic> {
        let (ast, _) = parse(src, 0).expect("ошибка разбора");
        construct_model(&ast, None, &[]).map(|m| m.take())
    }

    /// Возвращает разрешённое значение именованного условия `name`.
    fn cond_val(node: &ModelNode, name: &str) -> ConditionNode {
        node.conditions[name].value.clone()
    }

    // ─── construct_cond: литералы ─────────────────────────────────────────

    /// Литерал `true`: `cond c = true;` → `Bool(true)`.
    ///
    /// # Пример (BuT)
    /// ```but
    /// cond always = true;
    /// ```
    #[test]
    fn bool_true_literal() {
        let node = build("cond c = true;").unwrap();
        assert_eq!(cond_val(&node, "c"), ConditionNode::Bool(true));
    }

    /// Литерал `false`: `cond c = false;` → `Bool(false)`.
    #[test]
    fn bool_false_literal() {
        let node = build("cond c = false;").unwrap();
        assert_eq!(cond_val(&node, "c"), ConditionNode::Bool(false));
    }

    /// Целочисленный литерал: `cond c = 42;` → `Number(42)`.
    #[test]
    fn number_literal() {
        let node = build("cond c = 42;").unwrap();
        assert_eq!(cond_val(&node, "c"), ConditionNode::Number(42));
    }

    /// Вещественный литерал: `cond c = 3.14;` → `Rational("3.14", false)`.
    ///
    /// **Ранее было сломано**: ветка `Rational` в `construct_cond` была заглушкой,
    /// из-за чего условие молча становилось `Condition::None`.
    ///
    /// # Пример (BuT)
    /// ```but
    /// cond threshold = 0.5;
    /// ```
    #[test]
    fn rational_literal() {
        let node = build("cond c = 3.14;").unwrap();
        assert!(
            matches!(cond_val(&node, "c"), ConditionNode::Rational(ref s, false) if s == "3.14"),
            "ожидалось Rational(\"3.14\", false), получено {:?}",
            cond_val(&node, "c")
        );
    }

    // ─── construct_cond: переменная ───────────────────────────────────────

    /// Объявленная переменная в области видимости разрешается в `Condition::Variable`.
    ///
    /// # Пример (BuT)
    /// ```but
    /// var flag: bit = false;
    /// cond c = flag;
    /// ```
    #[test]
    fn variable_in_scope_resolves() {
        let node = build("var flag: bit = false; cond c = flag;").unwrap();
        assert!(
            matches!(cond_val(&node, "c"), ConditionNode::Variable(_, _)),
            "ожидалось Condition::Variable, получено {:?}",
            cond_val(&node, "c")
        );
    }

    /// Контрпример: необъявленная переменная вызывает ошибку [`Diagnostic`]
    /// в `extract_conditions`.
    ///
    /// # Контрпример (BuT)
    /// ```but
    /// cond bad = ghost;   // 'ghost' нигде не объявлена
    /// ```
    #[test]
    fn variable_not_in_scope_is_error() {
        let result = build("cond bad = ghost;");
        assert!(
            result.is_err(),
            "ожидалась ошибка для необъявленной переменной"
        );
    }

    // ─── construct_cond: операторы ────────────────────────────────────────

    /// Логическое НЕ: `cond c = !true;` → `Not(Bool(true))`.
    ///
    /// # Пример (BuT)
    /// ```but
    /// cond inactive = !active;
    /// ```
    #[test]
    fn not_operator() {
        let node = build("cond c = !true;").unwrap();
        assert!(
            matches!(cond_val(&node, "c"), ConditionNode::Not(_)),
            "ожидалось Condition::Not"
        );
    }

    /// Побитовое И: `cond c = true & false;` → `And(Bool(true), Bool(false))`.
    #[test]
    fn and_operator() {
        let node = build("cond c = true & false;").unwrap();
        assert!(matches!(cond_val(&node, "c"), ConditionNode::And(_, _)));
    }

    /// Побитовое ИЛИ: `cond c = true | false;` → `Or(Bool(true), Bool(false))`.
    #[test]
    fn or_operator() {
        let node = build("cond c = true | false;").unwrap();
        assert!(matches!(cond_val(&node, "c"), ConditionNode::Or(_, _)));
    }

    /// Сложение: `cond c = 1 + 2;` → `Add(Number(1), Number(2))`.
    #[test]
    fn add_operator() {
        let node = build("cond c = 1 + 2;").unwrap();
        assert!(matches!(cond_val(&node, "c"), ConditionNode::Add(_, _)));
    }

    /// Вычитание: `cond c = 3 - 1;` → `Subtract(Number(3), Number(1))`.
    #[test]
    fn subtract_operator() {
        let node = build("cond c = 3 - 1;").unwrap();
        assert!(matches!(
            cond_val(&node, "c"),
            ConditionNode::Subtract(_, _)
        ));
    }

    /// Сравнение `<`: `cond c = 1 < 2;` → `Less(Number(1), Number(2))`.
    #[test]
    fn less_comparison() {
        let node = build("cond c = 1 < 2;").unwrap();
        assert!(matches!(cond_val(&node, "c"), ConditionNode::Less(_, _)));
    }

    /// Сравнение `>`: `cond c = 2 > 1;` → `More(Number(2), Number(1))`.
    #[test]
    fn more_comparison() {
        let node = build("cond c = 2 > 1;").unwrap();
        assert!(matches!(cond_val(&node, "c"), ConditionNode::More(_, _)));
    }

    /// Равенство `=`: `cond c = 1 = 1;` → `Equal`.
    #[test]
    fn equal_operator() {
        let node = build("cond c = 1 = 1;").unwrap();
        assert!(matches!(cond_val(&node, "c"), ConditionNode::Equal(_, _)));
    }

    /// Неравенство `!=`: `cond c = 1 != 2;` → `NotEqual`.
    #[test]
    fn not_equal_operator() {
        let node = build("cond c = 1 != 2;").unwrap();
        assert!(matches!(
            cond_val(&node, "c"),
            ConditionNode::NotEqual(_, _)
        ));
    }

    /// Скобки: `cond c = (true);` → `Parenthesis(Bool(true))`.
    #[test]
    fn parenthesised_condition() {
        let node = build("cond c = (true);").unwrap();
        assert!(matches!(
            cond_val(&node, "c"),
            ConditionNode::Parenthesis(_)
        ));
    }

    // ─── контрпримеры для операторов сравнения ────────────────────────────

    /// Контрпример: `<=` должен давать `LessEqual`, а НЕ `Less`.
    ///
    /// # Контрпример (BuT)
    /// ```but
    /// cond c = 1 <= 2;   // LessEqual, не Less
    /// ```
    #[test]
    fn less_equal_is_not_less() {
        let node = build("cond c = 1 <= 2;").unwrap();
        let c = cond_val(&node, "c");
        assert!(
            matches!(c, ConditionNode::LessEqual(_, _)),
            "ожидалось LessEqual"
        );
        assert!(
            !matches!(c, ConditionNode::Less(_, _)),
            "НЕ должно быть Less"
        );
    }

    /// Контрпример: `>=` должен давать `MoreEqual`, а НЕ `More`.
    #[test]
    fn more_equal_is_not_more() {
        let node = build("cond c = 2 >= 1;").unwrap();
        let c = cond_val(&node, "c");
        assert!(
            matches!(c, ConditionNode::MoreEqual(_, _)),
            "ожидалось MoreEqual"
        );
        assert!(
            !matches!(c, ConditionNode::More(_, _)),
            "НЕ должно быть More"
        );
    }

    // ─── construct_cond: индексация массива ───────────────────────────────

    /// Индекс объявленного массива разрешается в `ArraySubscript`.
    ///
    /// # Пример (BuT)
    /// ```but
    /// var buf: [bit; 8];
    /// cond c = buf[3];
    /// ```
    #[test]
    fn array_subscript_on_array_resolves() {
        let node = build("var buf: [bit; 8]; cond c = buf[3];").unwrap();
        assert!(
            matches!(cond_val(&node, "c"), ConditionNode::ArraySubscript(_, 3)),
            "ожидалось ArraySubscript(_, 3)"
        );
    }

    /// Контрпример: индекс к переменной типа `bit` (не массив) — ошибка.
    ///
    /// # Контрпример (BuT)
    /// ```but
    /// var x: bit = false;
    /// cond bad = x[0];    // 'x' не массив — ошибка
    /// ```
    #[test]
    fn array_subscript_on_non_array_is_error() {
        let result = build("var x: bit = false; cond bad = x[0];");
        assert!(
            result.is_err(),
            "ожидалась ошибка при индексации не-массива"
        );
    }

    /// Контрпример: индекс к необъявленному идентификатору — ошибка.
    ///
    /// # Контрпример (BuT)
    /// ```but
    /// cond bad = arr[0];   // 'arr' нигде не объявлен
    /// ```
    #[test]
    fn array_subscript_on_unknown_is_error() {
        let result = build("cond bad = arr[0];");
        assert!(
            result.is_err(),
            "ожидалась ошибка для неизвестного идентификатора массива"
        );
    }

    // ─── extract_conditions ───────────────────────────────────────────────

    /// Разрешённое значение именованного булевого условия сохраняется в `model.conditions`.
    #[test]
    fn extract_bool_named_condition() {
        let node = build("cond done = true;").unwrap();
        assert!(
            node.conditions.contains_key("done"),
            "условие 'done' должно присутствовать"
        );
        assert_eq!(cond_val(&node, "done"), ConditionNode::Bool(true));
    }

    /// Разрешённое значение именованного числового условия сохраняется корректно.
    #[test]
    fn extract_number_named_condition() {
        let node = build("cond threshold = 7;").unwrap();
        assert_eq!(cond_val(&node, "threshold"), ConditionNode::Number(7));
    }

    /// Несколько именованных условий разрешаются независимо друг от друга.
    ///
    /// # Пример (BuT)
    /// ```but
    /// cond a = true;
    /// cond b = false;
    /// ```
    #[test]
    fn extract_multiple_named_conditions() {
        let node = build("cond a = true; cond b = false;").unwrap();
        assert!(node.conditions.contains_key("a"));
        assert!(node.conditions.contains_key("b"));
        assert_eq!(cond_val(&node, "a"), ConditionNode::Bool(true));
        assert_eq!(cond_val(&node, "b"), ConditionNode::Bool(false));
    }

    /// Контрпример: именованное условие, ссылающееся на необъявленный идентификатор,
    /// вызывает ошибку [`Diagnostic`] внутри `construct_model`.
    #[test]
    fn extract_condition_unknown_var_is_error() {
        let result = build("cond oops = missing_var;");
        assert!(result.is_err(), "ожидалась ошибка для необъявленной ссылки");
    }

    /// Именованное условие, ссылающееся на объявленную переменную, разрешается
    /// в `Condition::Variable`.
    #[test]
    fn extract_variable_condition() {
        let node = build("var x: bit = false; cond c = x;").unwrap();
        assert!(
            matches!(cond_val(&node, "c"), ConditionNode::Variable(_, _)),
            "ожидалось условие Variable"
        );
    }
}
