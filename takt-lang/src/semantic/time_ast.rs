//! Проход по АСД для конструкций времени (фича 0134): частота и `every`.
//!
//! Отдельная стадия, а не ветка в `tree.rs`, по двум причинам. Первая
//! формальная: `tree.rs` пришпилен реестром размеров и расти не имеет права.
//! Вторая содержательная: частота — **свойство единицы компиляции**, а не
//! элемента модели, и собирать её обходом всего дерева честнее, чем
//! накапливать в разборе элементов.
//!
//! ## Что делает
//!
//! 1. Собирает объявления `clock` (включая вложенные модели). Все они обязаны
//!    называть **одну** частоту: разные значения — ошибка автора (`SE-067`), а
//!    не «побеждает последнее». Итог кладётся в
//!    [`ModelNode::clock_hz`](super::ModelNode::clock_hz) корня.
//! 2. Отвергает `every 100ms { … }` (`SE-066`) — до его реализации.
//! 3. Следит, что `after` стоит **только** в условии перехода `ref` (`SE-068`).
//!
//! ## Почему `after` только на ребре (решение заказчика)
//!
//! Отсчёт `after` начинается с **перехода в состояние-источник**, то есть
//! выдержка — свойство пары «состояние + ребро». Именованное условие
//! (`cond Timeout = after 6s;`) видно из нескольких состояний, и одно скрытое
//! начало отсчёта на всех дало бы выдержку, зависящую от того, кто спросил
//! первым; в LTL/Guard-формуле у выдержки нет состояния-источника вовсе.
//! Прежде такая запись принималась **молча**.
//!
//! ⚠️ **Второй пункт закрывает молчаливую потерю.** Проба при разработке
//! 0134-03: модель с `every` компилировалась **успешно**, тело блока пропадало
//! бесследно, и вдобавок компилятор сообщал «переменная 'n' нигде не
//! используется» — то есть говорил автору неправду о его собственном коде.
//! Отказ обязан быть громким, пока конструкция не исполняется.
//!
//! Профиль выбирается позже и **не здесь**:
//! [`duration::resolve_profile`](super::duration::resolve_profile) сводит
//! объявление модели с флагом `--tick-hz` (флаг переопределяет).

use crate::diagnostics::Diagnostic;
use crate::parser::ast::{Condition, InlineFormulaDefine, Model, ModelElement, StateElement};
use crate::semantic::ModelNode;
use std::cell::RefCell;
use std::rc::Rc;

/// Обходит АСД: собирает частоту тактирования и отвергает нереализованное.
///
/// # Ошибки
///
/// - `SE-067` — две несовпадающие частоты в одной единице компиляции;
/// - `SE-066` — `every` (пока не исполняется ни одной целью).
pub(crate) fn collect_clock(ast: &Model, model: &Rc<RefCell<ModelNode>>) -> Result<(), Diagnostic> {
    let mut found: Option<u64> = None;
    walk(ast, &mut found)?;
    if let Some(hertz) = found {
        model.borrow_mut().clock_hz = Some(hertz);
    }
    Ok(())
}

/// Использует ли модель выдержку `after` хотя бы на одном ребре (фича 0134).
///
/// Нужно генераторам: поле-счётчик времени эмитится **только** при
/// использовании (правило 1 ADR — модель без времени даёт прежний вывод
/// байт-в-байт). Условия рёбер живут как `Unresolved(ast)` (инвариант проекта),
/// поэтому проверяются оба представления: и сырое АСД, и разрешённый узел.
pub fn model_uses_after(model: &ModelNode) -> bool {
    model.states.values().any(|state| {
        state
            .references()
            .iter()
            .any(|reference| cond_uses_after(&reference.cond))
    })
}

/// Есть ли `after` в условии перехода (в любом из двух представлений).
fn cond_uses_after(cond: &crate::semantic::ConditionNode) -> bool {
    match cond {
        crate::semantic::ConditionNode::After(_)
        | crate::semantic::ConditionNode::AfterTicks(_) => true,
        crate::semantic::ConditionNode::Unresolved(raw) => find_after(raw).is_some(),
        _ => false,
    }
}

/// Отвергает `after` в условии, стоящем не на ребре перехода (`SE-068`).
fn reject_after(cond: &Condition, place: &str) -> Result<(), Diagnostic> {
    if let Some(loc) = find_after(cond) {
        return Err(Diagnostic::error(
            loc,
            format!(
                "выдержка 'after' допустима только в условии перехода 'ref'; в {place} \
у неё нет состояния-источника, от входа в которое ведётся отсчёт"
            ),
        )
        .with_code("SE-068"));
    }
    Ok(())
}

/// Отвергает `after` в Guard-формуле (`: условие;`).
///
/// В LTL-формуле условие не хранится обычным `Condition` (там свой узел
/// `LtlExpr`), а лексема `after` в атом LTL не входит — поэтому проверять там
/// нечего: до семантики такая запись не доходит.
fn reject_after_in_formula(def: &InlineFormulaDefine) -> Result<(), Diagnostic> {
    match def {
        InlineFormulaDefine::Guard { conditions, .. } => {
            for cond in conditions {
                reject_after(cond, "Guard-формуле")?;
            }
            Ok(())
        }
        InlineFormulaDefine::Ltl { .. } => Ok(()),
    }
}

/// Ищет `after` в дереве условия; возвращает его позицию.
///
/// Обход исчерпывающий по построению: новый узел условия сюда не попадёт молча —
/// компилятор потребует ветку (в отличие от `_ =>`, который проглотил бы её).
fn find_after(cond: &Condition) -> Option<crate::diagnostics::Location> {
    match cond {
        Condition::After(loc, _, _) | Condition::AfterTicks(loc, _, _) => Some(*loc),
        Condition::Parenthesis(_, inner)
        | Condition::Not(_, inner)
        | Condition::BitAccess(_, inner, _)
        | Condition::ArraySubscript(_, _, inner) => find_after(inner),
        Condition::Add(_, l, r)
        | Condition::Subtract(_, l, r)
        | Condition::And(_, l, r)
        | Condition::Or(_, l, r)
        | Condition::Less(_, l, r)
        | Condition::More(_, l, r)
        | Condition::LessEqual(_, l, r)
        | Condition::MoreEqual(_, l, r)
        | Condition::Equal(_, l, r)
        | Condition::NotEqual(_, l, r) => find_after(l).or_else(|| find_after(r)),
        Condition::Function(_, _, args) => args.iter().find_map(find_after),
        Condition::Duration(_, _, _)
        | Condition::Number(_, _)
        | Condition::Rational(_, _, _)
        | Condition::String(_)
        | Condition::Bool(_, _)
        | Condition::Variable(_) => None,
    }
}

/// Использует ли модель длительностную выдержку (`after 3s`, `after 500ms`).
///
/// В профиле «часы» такая выдержка меряется меткой времени `now_ms`, а не
/// счётчиком тактов (фича 0134-04b) — отсюда нужен отдельный от `after Nt`
/// предикат: поля структуры у профилей разные.
pub fn model_uses_duration_after(model: &ModelNode) -> bool {
    model_uses_after_kind(model, false)
}

/// Использует ли модель тактовую выдержку (`after 3t`) — считается `takt_dwell`
/// в любом профиле (такт — шаг логики, частота не нужна).
pub fn model_uses_tick_after(model: &ModelNode) -> bool {
    model_uses_after_kind(model, true)
}

/// Использует ли **дерево** модели (сама + вложенные) длительностную выдержку.
///
/// Колбэк `now_ms` профиля «часы» (фича 0134-04b) живёт на КОРНЕВОЙ структуре, а
/// длительностная выдержка бывает во вложенной под-модели композиции — поэтому
/// решение «нужен ли `now_ms` корню» принимается по всему дереву, а не по корню.
pub fn model_tree_uses_duration_after(model: &ModelNode) -> bool {
    model_uses_duration_after(model)
        || model
            .models
            .values()
            .any(|nested| model_tree_uses_duration_after(&nested.borrow()))
}

fn model_uses_after_kind(model: &ModelNode, ticks: bool) -> bool {
    model.states.values().any(|state| {
        state
            .references()
            .iter()
            .any(|reference| cond_has_after_kind(&reference.cond, ticks))
    })
}

fn cond_has_after_kind(cond: &crate::semantic::ConditionNode, ticks: bool) -> bool {
    match cond {
        crate::semantic::ConditionNode::After(_) => !ticks,
        crate::semantic::ConditionNode::AfterTicks(_) => ticks,
        crate::semantic::ConditionNode::Unresolved(raw) => raw_has_after_kind(raw, ticks),
        _ => false,
    }
}

/// Ищет выдержку нужного вида в сыром дереве условия (для `Unresolved`-рёбер,
/// напр. составное `(after 10s) & x`). Обход исчерпывающий, как `find_after`.
fn raw_has_after_kind(cond: &Condition, ticks: bool) -> bool {
    match cond {
        Condition::After(..) => !ticks,
        Condition::AfterTicks(..) => ticks,
        Condition::Parenthesis(_, inner)
        | Condition::Not(_, inner)
        | Condition::BitAccess(_, inner, _)
        | Condition::ArraySubscript(_, _, inner) => raw_has_after_kind(inner, ticks),
        Condition::Add(_, l, r)
        | Condition::Subtract(_, l, r)
        | Condition::And(_, l, r)
        | Condition::Or(_, l, r)
        | Condition::Less(_, l, r)
        | Condition::More(_, l, r)
        | Condition::LessEqual(_, l, r)
        | Condition::MoreEqual(_, l, r)
        | Condition::Equal(_, l, r)
        | Condition::NotEqual(_, l, r) => {
            raw_has_after_kind(l, ticks) || raw_has_after_kind(r, ticks)
        }
        Condition::Function(_, _, args) => args.iter().any(|a| raw_has_after_kind(a, ticks)),
        Condition::Duration(_, _, _)
        | Condition::Number(_, _)
        | Condition::Rational(_, _, _)
        | Condition::String(_)
        | Condition::Bool(_, _)
        | Condition::Variable(_) => false,
    }
}

/// Рекурсивный обход АСД: объявления `clock` текущей модели и вложенных.
fn walk(ast: &Model, found: &mut Option<u64>) -> Result<(), Diagnostic> {
    for element in &ast.elements {
        match element {
            ModelElement::Clock(def) => match *found {
                Some(previous) if previous != def.hertz => {
                    return Err(Diagnostic::error(
                        def.loc,
                        format!(
                            "частота тактирования объявлена дважды и по-разному: \
                             {previous} Гц и {} Гц — какая из них настоящая, \
                             знает только автор",
                            def.hertz
                        ),
                    )
                    .with_code("SE-067"));
                }
                _ => *found = Some(def.hertz),
            },
            // `after` вне ребра — ошибка (см. шапку модуля).
            ModelElement::Condition(def) => reject_after(&def.value, "именованном условии")?,
            ModelElement::Invariant(def) => reject_after(&def.value, "инварианте")?,
            ModelElement::InlineFormula(def) => reject_after_in_formula(def)?,
            ModelElement::Model(nested) => walk(nested, found)?,
            // `every` пока не исполняется: отказ вместо тихой потери тела.
            ModelElement::State(state) => {
                for element in &state.elements {
                    match element {
                        StateElement::Every(def) => {
                            return Err(Diagnostic::error(
                                def.loc,
                                format!(
                                    "периодическое действие 'every {}' пока не поддерживается",
                                    def.text
                                ),
                            )
                            .with_code("SE-066"));
                        }
                        // Условие ребра — **единственное** законное место `after`.
                        StateElement::Reference(_, _, _) => {}
                        StateElement::Invariant(def) => {
                            reject_after(&def.value, "инварианте состояния")?;
                        }
                        StateElement::InlineFormula(def) => reject_after_in_formula(def)?,
                        StateElement::Next(_)
                        | StateElement::NamedBlockCode(_)
                        | StateElement::StraySemicolon(_) => {}
                    }
                }
            }
            _ => {}
        }
    }
    Ok(())
}
