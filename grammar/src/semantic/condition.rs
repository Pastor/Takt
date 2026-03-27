//! Построение семантических условий языка BuT.
//!
//! Модуль предоставляет две функции:
//! - [`resolve_condition`] — преобразует АСД-условие [`ast::Condition`] в разрешённое
//!   семантическое [`Condition`].
//! - [`extract_conditions`] — разрешает все неразрешённые именованные условия
//!   в [`HashMap`].

use crate::diagnostics::Diagnostic;
use crate::parser::ast;
use crate::semantic::builtin::builtin_function;
use crate::semantic::{Condition, ConditionNode, ModelNode, TypeNode, VariableNode};
use std::cell::RefCell;
use std::collections::HashMap;
use std::rc::Rc;

/// Строит разрешённое семантическое [`Condition`] из АСД-условия [`ast::Condition`].
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
) -> Result<Condition, Diagnostic> {
    match cond {
        ast::Condition::ArraySubscript(_, id, num) => {
            let name = id.name.clone();
            let var = model.borrow().search_var(&name);
            if let Some(var) = var
                && let VariableNode::Simple { ty, .. } = var.clone()
                && let TypeNode::Array(..) = ty
            {
                return Ok(Condition::ArraySubscript(Rc::new(RefCell::new(var)), *num));
            }
            Err(format!("Массив '{}' не найден", &name).as_str().into())
        }
        ast::Condition::Parenthesis(_, cond) => Ok(Condition::Parenthesis(Box::new(
            resolve_condition(cond, model.clone())?,
        ))),
        ast::Condition::BitAccess(_, cond, member) => {
            let cond = resolve_condition(cond, model.clone())?;
            Ok(Condition::BitAccess(Box::new(cond), member.clone()))
        }
        ast::Condition::Function(_, id, args) => {
            let name = id.name.clone();
            // Собираем условия аргументов, немедленно пробрасывая ошибку вместо
            // паники через `.unwrap()`.
            let args: Vec<Box<Condition>> = args
                .iter()
                .map(|c| resolve_condition(c, model.clone()).map(Box::new))
                .collect::<Result<Vec<_>, _>>()?;
            let function = model.borrow().search_func(&name);
            let function = if function.clone().is_none() {
                Rc::new(RefCell::new(builtin_function(&id.name)?.clone()))
            } else {
                function.unwrap()
            };
            Ok(Condition::Function(function, args))
        }
        ast::Condition::Not(_, cond) => Ok(Condition::Not(Box::new(resolve_condition(
            cond,
            model.clone(),
        )?))),
        ast::Condition::Add(_, left, right) => {
            let left = resolve_condition(left, model.clone())?;
            let right = resolve_condition(right, model.clone())?;
            Ok(Condition::Add(Box::new(left), Box::new(right)))
        }
        ast::Condition::Subtract(_, left, right) => {
            let left = resolve_condition(left, model.clone())?;
            let right = resolve_condition(right, model.clone())?;
            Ok(Condition::Subtract(Box::new(left), Box::new(right)))
        }
        ast::Condition::And(_, left, right) => {
            let left = resolve_condition(left, model.clone())?;
            let right = resolve_condition(right, model.clone())?;
            Ok(Condition::And(Box::new(left), Box::new(right)))
        }
        ast::Condition::Or(_, left, right) => {
            let left = resolve_condition(left, model.clone())?;
            let right = resolve_condition(right, model.clone())?;
            Ok(Condition::Or(Box::new(left), Box::new(right)))
        }
        ast::Condition::Less(_, left, right) => {
            let left = resolve_condition(left, model.clone())?;
            let right = resolve_condition(right, model.clone())?;
            Ok(Condition::Less(Box::new(left), Box::new(right)))
        }
        ast::Condition::More(_, left, right) => {
            let left = resolve_condition(left, model.clone())?;
            let right = resolve_condition(right, model.clone())?;
            Ok(Condition::More(Box::new(left), Box::new(right)))
        }
        ast::Condition::LessEqual(_, left, right) => {
            let left = resolve_condition(left, model.clone())?;
            let right = resolve_condition(right, model.clone())?;
            Ok(Condition::LessEqual(Box::new(left), Box::new(right)))
        }
        ast::Condition::MoreEqual(_, left, right) => {
            let left = resolve_condition(left, model.clone())?;
            let right = resolve_condition(right, model.clone())?;
            Ok(Condition::MoreEqual(Box::new(left), Box::new(right)))
        }
        ast::Condition::Equal(_, left, right) => {
            let left = resolve_condition(left, model.clone())?;
            let right = resolve_condition(right, model.clone())?;
            Ok(Condition::Equal(Box::new(left), Box::new(right)))
        }
        ast::Condition::NotEqual(_, left, right) => {
            let left = resolve_condition(left, model.clone())?;
            let right = resolve_condition(right, model.clone())?;
            Ok(Condition::NotEqual(Box::new(left), Box::new(right)))
        }
        ast::Condition::Number(_, n) => Ok(Condition::Number(*n)),
        // Вещественный литерал: сохраняем строковое представление и знак.
        // Ранее эта ветка была заглушкой, что приводило к молчаливой потере данных:
        // условие становилось `Condition::None`.
        ast::Condition::Rational(_, s, neg) => Ok(Condition::Rational(s.clone(), *neg)),
        // Строковый литерал: собираем строковые значения из каждого сегмента.
        // Ранее эта ветка была заглушкой с той же ошибкой потери данных.
        ast::Condition::String(lits) => {
            let strings = lits.iter().map(|l| l.string.clone()).collect();
            Ok(Condition::String(strings))
        }
        ast::Condition::Bool(_, v) => Ok(Condition::Bool(*v)),
        ast::Condition::Variable(id) => {
            let name = id.name.clone();
            // Сначала ищем объявление переменной в области видимости.
            if let Some(var) = model.borrow().search_var(&name) {
                return Ok(Condition::Variable(Rc::new(RefCell::new(var))));
            } else if let Some(cond) = model.borrow().search_cond(&name) {
                return Ok(cond.value);
            } else if let Some(model) = model.borrow().search_model(&name) {
                return Ok(Condition::Model(model.clone()));
            } else if let Some(state) = model.borrow().search_state(&name) {
                return Ok(Condition::State(state.clone()));
            }
            Ok(Condition::Unresolved(ast::Condition::Variable(id.clone())))
        }
    }
}

/// Разрешает все неразрешённые именованные условия в `conditions`.
///
/// Перебирает `conditions` и для каждой записи, значение которой равно
/// [`Condition::Unresolved`], вызывает [`resolve_condition`], заменяя заглушку
/// полностью разрешённым семантическим условием. Уже разрешённые записи
/// передаются без изменений.
///
/// # Ошибки
///
/// Пробрасывает любой [`Diagnostic`], возвращённый из [`resolve_condition`].
pub fn extract_conditions(
    conditions: &HashMap<String, ConditionNode>,
    model: Rc<RefCell<ModelNode>>,
) -> Result<HashMap<String, ConditionNode>, Diagnostic> {
    let mut result = HashMap::new();
    for (name, cond) in conditions {
        if let Condition::Unresolved(ast_cond) = &cond.value {
            let resolved = resolve_condition(ast_cond, model.clone())?;
            result.insert(
                name.clone(),
                ConditionNode {
                    name: name.clone(),
                    value: resolved,
                    upper: cond.upper.clone(),
                },
            );
        } else {
            result.insert(name.clone(), cond.clone());
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
    fn cond_val(node: &ModelNode, name: &str) -> Condition {
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
        assert_eq!(cond_val(&node, "c"), Condition::Bool(true));
    }

    /// Литерал `false`: `cond c = false;` → `Bool(false)`.
    #[test]
    fn bool_false_literal() {
        let node = build("cond c = false;").unwrap();
        assert_eq!(cond_val(&node, "c"), Condition::Bool(false));
    }

    /// Целочисленный литерал: `cond c = 42;` → `Number(42)`.
    #[test]
    fn number_literal() {
        let node = build("cond c = 42;").unwrap();
        assert_eq!(cond_val(&node, "c"), Condition::Number(42));
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
            matches!(cond_val(&node, "c"), Condition::Rational(ref s, false) if s == "3.14"),
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
            matches!(cond_val(&node, "c"), Condition::Variable(_)),
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
            matches!(cond_val(&node, "c"), Condition::Not(_)),
            "ожидалось Condition::Not"
        );
    }

    /// Побитовое И: `cond c = true & false;` → `And(Bool(true), Bool(false))`.
    #[test]
    fn and_operator() {
        let node = build("cond c = true & false;").unwrap();
        assert!(matches!(cond_val(&node, "c"), Condition::And(_, _)));
    }

    /// Побитовое ИЛИ: `cond c = true | false;` → `Or(Bool(true), Bool(false))`.
    #[test]
    fn or_operator() {
        let node = build("cond c = true | false;").unwrap();
        assert!(matches!(cond_val(&node, "c"), Condition::Or(_, _)));
    }

    /// Сложение: `cond c = 1 + 2;` → `Add(Number(1), Number(2))`.
    #[test]
    fn add_operator() {
        let node = build("cond c = 1 + 2;").unwrap();
        assert!(matches!(cond_val(&node, "c"), Condition::Add(_, _)));
    }

    /// Вычитание: `cond c = 3 - 1;` → `Subtract(Number(3), Number(1))`.
    #[test]
    fn subtract_operator() {
        let node = build("cond c = 3 - 1;").unwrap();
        assert!(matches!(cond_val(&node, "c"), Condition::Subtract(_, _)));
    }

    /// Сравнение `<`: `cond c = 1 < 2;` → `Less(Number(1), Number(2))`.
    #[test]
    fn less_comparison() {
        let node = build("cond c = 1 < 2;").unwrap();
        assert!(matches!(cond_val(&node, "c"), Condition::Less(_, _)));
    }

    /// Сравнение `>`: `cond c = 2 > 1;` → `More(Number(2), Number(1))`.
    #[test]
    fn more_comparison() {
        let node = build("cond c = 2 > 1;").unwrap();
        assert!(matches!(cond_val(&node, "c"), Condition::More(_, _)));
    }

    /// Равенство `=`: `cond c = 1 = 1;` → `Equal`.
    #[test]
    fn equal_operator() {
        let node = build("cond c = 1 = 1;").unwrap();
        assert!(matches!(cond_val(&node, "c"), Condition::Equal(_, _)));
    }

    /// Неравенство `!=`: `cond c = 1 != 2;` → `NotEqual`.
    #[test]
    fn not_equal_operator() {
        let node = build("cond c = 1 != 2;").unwrap();
        assert!(matches!(cond_val(&node, "c"), Condition::NotEqual(_, _)));
    }

    /// Скобки: `cond c = (true);` → `Parenthesis(Bool(true))`.
    #[test]
    fn parenthesised_condition() {
        let node = build("cond c = (true);").unwrap();
        assert!(matches!(cond_val(&node, "c"), Condition::Parenthesis(_)));
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
            matches!(c, Condition::LessEqual(_, _)),
            "ожидалось LessEqual"
        );
        assert!(!matches!(c, Condition::Less(_, _)), "НЕ должно быть Less");
    }

    /// Контрпример: `>=` должен давать `MoreEqual`, а НЕ `More`.
    #[test]
    fn more_equal_is_not_more() {
        let node = build("cond c = 2 >= 1;").unwrap();
        let c = cond_val(&node, "c");
        assert!(
            matches!(c, Condition::MoreEqual(_, _)),
            "ожидалось MoreEqual"
        );
        assert!(!matches!(c, Condition::More(_, _)), "НЕ должно быть More");
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
            matches!(cond_val(&node, "c"), Condition::ArraySubscript(_, 3)),
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
        assert_eq!(cond_val(&node, "done"), Condition::Bool(true));
    }

    /// Разрешённое значение именованного числового условия сохраняется корректно.
    #[test]
    fn extract_number_named_condition() {
        let node = build("cond threshold = 7;").unwrap();
        assert_eq!(cond_val(&node, "threshold"), Condition::Number(7));
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
        assert_eq!(cond_val(&node, "a"), Condition::Bool(true));
        assert_eq!(cond_val(&node, "b"), Condition::Bool(false));
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
            matches!(cond_val(&node, "c"), Condition::Variable(_)),
            "ожидалось условие Variable"
        );
    }
}
