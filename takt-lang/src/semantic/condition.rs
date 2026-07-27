//! Построение семантических условий языка Takt.
//!
//! Модуль предоставляет две функции:
//! - [`resolve_condition`] — преобразует АСД-условие [`ast::Condition`] в разрешённое
//!   семантическое [`ConditionNode`].
//! - [`extract_conditions`] — разрешает все неразрешённые именованные условия
//!   в [`BTreeMap`].

use crate::diagnostics::Diagnostic;
use crate::parser::ast;
use crate::semantic::builtin::builtin_function;
use crate::semantic::type_node::TypeNode;
use crate::semantic::{
    ConditionDefinitionNode, ConditionNode, FunctionDefinitionNode, ModelNode, VariableNode,
};
use std::cell::RefCell;
use std::collections::BTreeMap;
use std::rc::Rc;

/// Снимает прозрачные обёртки [`ConditionNode::Parenthesis`], возвращая
/// внутреннее условие.
///
/// Скобки в языке — грамматическая группировка; для *значения* и для
/// *распознавания паттерна* `S(Модель) = Состояние` они прозрачны. Снимать их
/// **вообще везде** нельзя: генераторы печатают `Parenthesis` как `(…)`, и это
/// сохраняет группировку операндов (в цели `rust` приоритеты иначе расходятся с
/// C — см. `CLAUDE.md`). Поэтому хелпер применяется **только** к операндам
/// паттерна `S(…)` (фича 0074), где скобки не несут ни группировки, ни вывода.
fn strip_cond_parens(mut cond: ConditionNode) -> ConditionNode {
    while let ConditionNode::Parenthesis(inner) = cond {
        cond = *inner;
    }
    cond
}

/// Является ли условие паттерном «текущее состояние модели»: встроенная
/// `S(Модель)` либо её краткая форма `Модель`.
///
/// Общий предикат для канонизации скобочных форм в [`resolve_condition`]
/// (фича 0074) — распознаётся так же, как в генераторе C
/// (`c_expr::condition::state_of_model`).
fn is_state_of(cond: &ConditionNode) -> bool {
    match cond {
        ConditionNode::Model(..) => true,
        ConditionNode::Function(fun, ..) => {
            matches!(&*fun.borrow(), FunctionDefinitionNode::Builtin(name, ..) if *name == "S")
        }
        _ => false,
    }
}

/// Канонизирует скобочные формы паттерна `S(Модель) = Состояние` (фича 0074).
///
/// Если левый операнд `=`/`!=` (со снятыми скобками) — «текущее состояние
/// модели» ([`is_state_of`]), скобки вокруг всей левой части (`(S(Ping)) = End`)
/// и вокруг имени состояния (`S(Ping) = (End)`) семантически прозрачны, но их
/// обёртка `Parenthesis` ломает распознавание паттерна во **всех** потребителях
/// (спецслучай `validate.rs` → `SE-025`, генератор C → `CC-003`). Снимаем их
/// здесь, в единственной воронке разбора, чтобы паттерн дошёл каноничным до
/// семантики, всех генераторов и симулятора.
///
/// Возвращает `Some((левый_без_скобок, правый_без_скобок))`, если паттерн
/// распознан; `None` — если это обычное сравнение (тогда вызывающий сохраняет
/// прежнюю логику, включая разрешение варианта перечисления справа).
///
/// Имя состояния справа **намеренно не разрешается вариантом перечисления**:
/// левая часть — модель, а не переменная-перечисление, поэтому ветка
/// `resolve_rval_as_enum_variant` здесь неприменима (она включается лишь при
/// `ConditionNode::Variable` слева). Правая часть остаётся `Unresolved`, как и
/// у бесскобочной формы — имя состояния ищется в области видимости
/// модели-аргумента потребителем (инвариант `resolve_state_references`, `CLAUDE.md`).
#[allow(clippy::type_complexity)]
fn canonicalize_state_of(
    left: ConditionNode,
    right: &ast::Condition,
    model: Rc<RefCell<ModelNode>>,
) -> Result<Option<(ConditionNode, ConditionNode)>, Diagnostic> {
    let left = strip_cond_parens(left);
    if !is_state_of(&left) {
        return Ok(None);
    }
    let right = strip_cond_parens(resolve_condition(right, model)?);
    Ok(Some((left, right)))
}

/// Разрешает правую часть оператора `=`/`!=` с учётом типа левой части.
///
/// Если левая часть — переменная типа перечисления `enum_name`, а правая —
/// идентификатор, совпадающий с вариантом этого перечисления, возвращает
/// `EnumVariant`, а не `State`. В остальных случаях делегирует в `resolve_condition`.
fn resolve_rval_as_enum_variant(
    cond: &ast::Condition,
    model: Rc<RefCell<ModelNode>>,
    enum_name: &str,
) -> Result<ConditionNode, Diagnostic> {
    if let ast::Condition::Variable(id) = cond {
        let name = &id.name;
        if let Some((edn, val)) = model.borrow().search_enum_variant(name)
            && edn.name == enum_name
        {
            return Ok(ConditionNode::EnumVariant(
                Rc::new(RefCell::new(edn)),
                name.clone(),
                val,
            ));
        }
        // Идентификатор не является вариантом enum_name — разрешаем как обычно
        // (например, это переменная того же типа перечисления).
        return resolve_condition(cond, model);
    }
    resolve_condition(cond, model)
}

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
    // Сторож глубины — см. `validate::depth` (фича 0129).
    let _depth = crate::semantic::validate::depth::enter(Some(cond.loc()))?;
    match cond {
        // ── Время (фича 0134) ────────────────────────────────────────────────
        // Литерал несёт наносекунды; `after` — сахар, его развернут задачи
        // симулятора и целей. До них выдержка даёт ЯВНЫЙ отказ (SE-066), а не
        // молчаливое «условие всегда ложно».
        ast::Condition::Duration(_, nanos, _) => Ok(ConditionNode::Duration(*nanos)),
        ast::Condition::After(loc, _, text) => Err(Diagnostic::error(
            *loc,
            format!("выдержка 'after {text}' пока не поддерживается семантикой"),
        )
        .with_code("SE-066")),
        ast::Condition::ArraySubscript(_, id, idx_cond) => {
            let name = id.name.clone();
            let var = model.borrow().search_var(&name);
            if let Some(var) = var
                && let VariableNode::Simple { ty, .. } = var.clone()
                && let TypeNode::Array(..) = ty
            {
                let resolved_idx = resolve_condition(idx_cond, model.clone())?;
                return Ok(ConditionNode::ArraySubscript(
                    Rc::new(RefCell::new(var)),
                    Box::new(resolved_idx),
                ));
            }
            Err(format!("Массив '{}' не найден", name).as_str().into())
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
            let mut args: Vec<Box<ConditionNode>> = args
                .iter()
                .map(|c| resolve_condition(c, model.clone()).map(Box::new))
                .collect::<Result<Vec<_>, _>>()?;
            let function = model.borrow().search_func(&name);
            let function = match function {
                Some(f) => f,
                None => Rc::new(RefCell::new(builtin_function(&id.name)?.clone())),
            };
            // Канонизация `S((Модель))` (фича 0074): скобки вокруг модели
            // прозрачны, но обёртка `Parenthesis` ломает распознавание паттерна
            // `S(Модель)` во всех потребителях (семантика `SE-025`, генераторы).
            // Снимаем их здесь — в единственной воронке разбора условий.
            if matches!(&*function.borrow(), FunctionDefinitionNode::Builtin(name, ..) if *name == "S")
            {
                for a in &mut args {
                    **a = strip_cond_parens((**a).clone());
                }
            }
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
            if let Some((left, right)) = canonicalize_state_of(left.clone(), right, model.clone())?
            {
                return Ok(ConditionNode::Equal(Box::new(left), Box::new(right)));
            }
            let right = if let ConditionNode::Variable(var_rc, _) = &left {
                let ty = var_rc.borrow().ty().clone();
                if let TypeNode::Enum(enum_name) = ty {
                    resolve_rval_as_enum_variant(right, model.clone(), &enum_name)?
                } else {
                    resolve_condition(right, model.clone())?
                }
            } else {
                resolve_condition(right, model.clone())?
            };
            Ok(ConditionNode::Equal(Box::new(left), Box::new(right)))
        }
        ast::Condition::NotEqual(_, left, right) => {
            let left = resolve_condition(left, model.clone())?;
            if let Some((left, right)) = canonicalize_state_of(left.clone(), right, model.clone())?
            {
                return Ok(ConditionNode::NotEqual(Box::new(left), Box::new(right)));
            }
            let right = if let ConditionNode::Variable(var_rc, _) = &left {
                let ty = var_rc.borrow().ty().clone();
                if let TypeNode::Enum(enum_name) = ty {
                    resolve_rval_as_enum_variant(right, model.clone(), &enum_name)?
                } else {
                    resolve_condition(right, model.clone())?
                }
            } else {
                resolve_condition(right, model.clone())?
            };
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
                return Ok(ConditionNode::Model(model.clone(), id.loc));
            } else if let Some(state) = model.borrow().search_state(&name) {
                // Второе поле — use-site позиция имени состояния (фича 0071),
                // как у `Model`/`Variable` рядом. Позволяет LSP-переходу найти
                // узел под курсором.
                return Ok(ConditionNode::State(state.clone(), id.loc));
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
    conditions: &BTreeMap<String, ConditionDefinitionNode>,
    model: Rc<RefCell<ModelNode>>,
) -> Result<BTreeMap<String, ConditionDefinitionNode>, Diagnostic> {
    let mut result = BTreeMap::new();
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
    /// # Пример (Takt)
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
    /// # Пример (Takt)
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
    /// # Пример (Takt)
    /// ```but
    /// var flag: bit = false;
    /// cond c = flag;
    /// ```
    #[test]
    fn variable_in_scope_resolves() {
        let node = build("var flag: bit := false; cond c = flag;").unwrap();
        assert!(
            matches!(cond_val(&node, "c"), ConditionNode::Variable(_, _)),
            "ожидалось Condition::Variable, получено {:?}",
            cond_val(&node, "c")
        );
    }

    /// Контрпример: необъявленная переменная вызывает ошибку [`Diagnostic`]
    /// в `extract_conditions`.
    ///
    /// # Контрпример (Takt)
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
    /// # Пример (Takt)
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
    /// # Контрпример (Takt)
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
    /// # Пример (Takt)
    /// ```but
    /// var buf: [bit; 8];
    /// cond c = buf[3];
    /// ```
    #[test]
    fn array_subscript_on_array_resolves() {
        let node = build("var buf: [bit; 8]; cond c = buf[3];").unwrap();
        assert!(
            matches!(cond_val(&node, "c"), ConditionNode::ArraySubscript(_, ref idx) if matches!(**idx, ConditionNode::Number(3))),
            "ожидалось ArraySubscript(_, 3)"
        );
    }

    /// Контрпример: индекс к переменной типа `bit` (не массив) — ошибка.
    ///
    /// # Контрпример (Takt)
    /// ```but
    /// var x: bit = false;
    /// cond bad = x[0];    // 'x' не массив — ошибка
    /// ```
    #[test]
    fn array_subscript_on_non_array_is_error() {
        let result = build("var x: bit := false; cond bad = x[0];");
        assert!(
            result.is_err(),
            "ожидалась ошибка при индексации не-массива"
        );
    }

    /// Контрпример: индекс к необъявленному идентификатору — ошибка.
    ///
    /// # Контрпример (Takt)
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
    /// # Пример (Takt)
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
        let node = build("var x: bit := false; cond c = x;").unwrap();
        assert!(
            matches!(cond_val(&node, "c"), ConditionNode::Variable(_, _)),
            "ожидалось условие Variable"
        );
    }

    // ─── тип-зависимое разрешение оператора = ────────────────────────────

    /// Позитивный тест: когда левая часть `=` — переменная типа перечисления,
    /// а одноимённое состояние тоже существует, правая часть должна разрешаться
    /// как `EnumVariant`, а не как `State`.
    ///
    /// # Пример (Takt)
    /// ```but
    /// enum Command { Up, Down, Stop }
    /// var command: Command = Stop;
    /// model Motor {
    ///     cond c = command = Stop;
    ///     start Up {}
    ///     state Stop {}
    /// }
    /// ```
    #[test]
    fn enum_variant_resolves_over_state_in_equal() {
        let src = r#"
enum Command { Up, Down, Stop }
var command: Command := Stop;
model Motor {
    cond c = command = Stop;
    start Up {}
    state Stop {}
}
start Main = Motor;
"#;
        let node = build(src).unwrap();
        let motor = node
            .search_model("Motor")
            .expect("модель Motor должна быть найдена");
        let motor_borrowed = motor.borrow();
        let c = motor_borrowed.conditions["c"].value.clone();
        if let ConditionNode::Equal(_, right) = c {
            assert!(
                matches!(*right, ConditionNode::EnumVariant(_, _, _)),
                "правая часть = должна быть EnumVariant, не State: {:?}",
                right
            );
        } else {
            panic!("ожидалось условие Equal, получено {:?}", c);
        }
    }

    /// Позитивный тест: аналогично для `!=`.
    #[test]
    fn enum_variant_resolves_over_state_in_not_equal() {
        let src = r#"
enum Command { Up, Down, Stop }
var command: Command := Stop;
model Motor {
    cond c = command != Stop;
    start Up {}
    state Stop {}
}
start Main = Motor;
"#;
        let node = build(src).unwrap();
        let motor = node
            .search_model("Motor")
            .expect("модель Motor должна быть найдена");
        let motor_borrowed = motor.borrow();
        let c = motor_borrowed.conditions["c"].value.clone();
        if let ConditionNode::NotEqual(_, right) = c {
            assert!(
                matches!(*right, ConditionNode::EnumVariant(_, _, _)),
                "правая часть != должна быть EnumVariant, не State: {:?}",
                right
            );
        } else {
            panic!("ожидалось условие NotEqual, получено {:?}", c);
        }
    }

    // ─── канонизация скобок паттерна S(Модель) (фича 0074) ────────────────

    /// Возвращает разрешённое условие первого `ref`-ребра стартового состояния
    /// под-модели `sub` корневой модели `node`.
    fn ref_cond(node: &ModelNode, sub: &str) -> ConditionNode {
        let submodel = node.search_model(sub).expect("под-модель");
        let submodel = submodel.borrow();
        let start = submodel.states.values().find_map(|s| match s {
            crate::semantic::StateNode::Simple { references, .. }
            | crate::semantic::StateNode::Implement { references, .. }
                if !references.is_empty() =>
            {
                Some(references[0].cond.clone())
            }
            _ => None,
        });
        start.expect("ref-ребро со стартового состояния")
    }

    /// Общая база: `Pong` со ссылкой на состояние `Ping` через `S(…)`.
    /// `%COND%` подставляется тестом.
    fn state_of_src(cond: &str) -> String {
        format!(
            r#"
model Ping {{ start A {{ ref End: true; }} state End; }}
model Pong {{ start Go {{ ref Stop: {cond}; }} state Stop; }}
start Entry = Ping | Pong;
"#
        )
    }

    /// Каноничная ли форма `Equal(Function(S, [Model]), Unresolved(Variable
    /// "End"))` — без обёрток `Parenthesis` ни слева, ни у аргумента, ни справа.
    ///
    /// Сравнение по **структуре**, а не `==`: полное равенство включает
    /// `Location`, а скобки сдвигают смещения — при этом C байт-в-байт совпадает
    /// (позиции на вывод не влияют), что и проверяет `c_state_ref_tests`.
    fn is_canonical_state_of(cond: &ConditionNode) -> bool {
        let ConditionNode::Equal(l, r) = cond else {
            return false;
        };
        let left_ok = matches!(&**l, ConditionNode::Function(fun, args, _)
            if matches!(&*fun.borrow(), FunctionDefinitionNode::Builtin(n, ..) if *n == "S")
            && args.len() == 1
            && matches!(&*args[0], ConditionNode::Model(..)));
        let right_ok = matches!(&**r,
            ConditionNode::Unresolved(ast::Condition::Variable(id)) if id.name == "End");
        left_ok && right_ok
    }

    /// Скобочные формы `S(Модель) = Состояние` канонизируются в **одну** форму
    /// `Equal(Function(S, …), Unresolved(Variable))` — без обёрток `Parenthesis`.
    #[test]
    fn parenthesised_state_of_canonicalizes() {
        for form in [
            "S(Ping) = End", // эталон — тоже обязан быть каноничным
            "(S(Ping)) = End",
            "S((Ping)) = End",
            "S(Ping) = (End)",
            "((S((Ping)))) = (End)",
        ] {
            let node = build(&state_of_src(form)).unwrap();
            let cond = ref_cond(&node, "Pong");
            assert!(
                is_canonical_state_of(&cond),
                "форма `{form}` обязана дать каноничную Equal(Function(S,[Model]), \
                 Unresolved(Variable \"End\")) без Parenthesis, получено {cond:?}"
            );
        }
    }

    /// Сторож против пере-снятия скобок: скобки вокруг **обычного** операнда
    /// (не паттерн `S(…)`) сохраняются — иначе изменился бы вывод генераторов.
    #[test]
    fn ordinary_parentheses_are_preserved() {
        // `(flag) = true`: слева переменная, не состояние модели → скобка цела.
        let node = build("var flag: bit := false; cond c = (flag) = true;").unwrap();
        let c = cond_val(&node, "c");
        assert!(
            matches!(&c, ConditionNode::Equal(l, _) if matches!(**l, ConditionNode::Parenthesis(_))),
            "скобка вокруг обычного операнда обязана сохраниться, получено {c:?}"
        );
    }

    /// Отрицательный тест: `command = Foo` где `Foo` не вариант `Command` — ошибка.
    #[test]
    fn enum_equal_wrong_variant_is_error() {
        let src = r#"
enum Command { Up, Down, Stop }
var command: Command := Stop;
cond c = command = Foo;
"#;
        let result = build(src);
        assert!(
            result.is_err(),
            "ожидалась ошибка: Foo не является вариантом Command"
        );
    }
}
