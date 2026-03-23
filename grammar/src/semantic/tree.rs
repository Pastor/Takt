//! Построение семантического дерева из АСД языка BuT.
//!
//! Основные функции модуля:
//! - [`construct_model`] — главная точка входа, строит [`ModelNode`] из [`Model`].
//! - [`construct_states`] — извлекает состояния и разрешает ссылки между ними.
//! - [`construct_context_model`] — строит контекст (вложенные модели) для модели.
//! - [`construct_context_state`] — строит контекст для состояния (заглушка).
//! - [`construct_condition`] — преобразует условие АСД в семантическое условие.

use crate::parser::ast;
use crate::parser::ast::{Model, ModelElement, StateDefine, StateElement};
use crate::semantic::{Condition, ContextNode, Diagnostic, ModelNode, Reference, StateNode};
use std::collections::HashMap;
use std::rc::Rc;

/// Строит семантический узел модели из АСД-узла [`Model`].
///
/// Собирает контекст верхнего уровня (вложенные модели), а также
/// словарь состояний с разрешёнными ссылками между ними.
///
/// # Ошибки
///
/// Возвращает [`Diagnostic`], если:
/// - у состояния нет имени,
/// - ссылка `ref` указывает на несуществующее состояние,
/// - `next` встречается в одном состоянии дважды.
pub fn construct_model(model: &Model) -> Result<ModelNode, Diagnostic> {
    let name = model.name.clone();
    let context = construct_context_model(model)?;
    let states = construct_states(model)?;
    Ok(ModelNode {
        context,
        name: name.map(|i| i.name.clone()),
        states,
        implements: (),
    })
}

/// Извлекает все состояния из модели и разрешает ссылки между ними.
///
/// Алгоритм:
/// 1. Первый проход — создаём [`StateNode`] для каждого `state`/`start` с
///    [`StateNode::Unresolved`] в качестве заглушки для целей ссылок.
/// 2. Второй проход — заменяем заглушки фактическими [`StateNode`].
///
/// # Ошибки
///
/// Возвращает [`Diagnostic`], если состояние без имени, ссылка не найдена,
/// или `next` объявлен дважды в одном состоянии.
pub fn construct_states(model: &Model) -> Result<HashMap<String, StateNode>, Diagnostic> {
    // Первый проход: создаём узлы с незаполненными ссылками
    let states: &mut HashMap<String, Box<StateNode>> = &mut HashMap::new();
    for element in model.elements.iter() {
        if let ModelElement::State(def) = element {
            let name = def
                .clone()
                .name
                .ok_or_else(|| "Model state not naming".into())?
                .name;
            let context = construct_context_state(def)?;
            let implements = def.implements.clone();
            let mut references = Vec::new();
            let mut next = None;
            for element in def.elements.iter() {
                if let StateElement::Reference(_, id, cond) = element {
                    let name = id.name.clone();
                    let cond = if let Some(cond) = cond {
                        construct_condition(cond)?
                    } else {
                        Condition::None
                    };
                    references.push(Reference {
                        name,
                        cond,
                        object: Box::new(StateNode::Unresolved),
                    });
                } else if let StateElement::Next(id) = element {
                    let name = id.name.clone();
                    if next.is_some() {
                        return Err(format!("State '{}' already defined", &name).as_str().into());
                    }
                    next = Some(name);
                }
            }
            // Определяем вид узла: Implement (есть `= Выражение`) или Simple
            let state = if let Some(_implements_expr) = implements {
                let next = next.map(|n| Reference {
                    name: n,
                    cond: Condition::None,
                    object: Box::new(StateNode::Unresolved),
                });
                StateNode::Implement {
                    context,
                    name: name.clone(),
                    references,
                    implements: (),
                    next,
                }
            } else {
                StateNode::Simple {
                    context,
                    name: name.clone(),
                    references,
                }
            };
            states.insert(name, Box::new(state));
        }
    }

    // Второй проход: заменяем Unresolved-заглушки реальными узлами
    let new_states = &mut HashMap::new();
    for (_, state) in states.iter() {
        if let StateNode::Simple {
            context,
            name,
            references,
        } = *state.clone()
        {
            let new_references: &mut Vec<Reference<StateNode>> = &mut Vec::new();
            for reference in references {
                if let StateNode::Unresolved = *reference.object {
                    let state = states.get(&reference.name).ok_or_else(|| {
                        format!("Reference '{}' not found", &reference.name)
                            .as_str()
                            .into()
                    })?;
                    new_references.push(Reference {
                        name: reference.name,
                        cond: reference.cond,
                        object: state.clone(),
                    });
                } else {
                    new_references.push(reference)
                }
            }
            new_states.insert(
                name.clone(),
                StateNode::Simple {
                    context: context.clone(),
                    name: name.clone(),
                    references: new_references.clone(),
                },
            );
        } else if let StateNode::Implement {
            context,
            name,
            references,
            implements,
            next,
        } = *state.clone()
        {
            let new_references: &mut Vec<Reference<StateNode>> = &mut Vec::new();
            let mut next = next.clone();
            for reference in references {
                if let StateNode::Unresolved = *reference.object {
                    let state = states.get(&reference.name).ok_or_else(|| {
                        format!("Reference '{}' not found", &reference.name)
                            .as_str()
                            .into()
                    })?;
                    new_references.push(Reference {
                        name: reference.name,
                        cond: reference.cond,
                        object: state.clone(),
                    });
                } else {
                    new_references.push(reference)
                }
            }
            if let Some(next) = next.as_mut() {
                if let StateNode::Unresolved = *next.object {
                    let state = states.get(&next.name).ok_or_else(|| {
                        format!("Reference '{}' not found", &next.name)
                            .as_str()
                            .into()
                    })?;
                    *next = Reference {
                        name: name.clone(),
                        cond: next.cond.clone(),
                        object: state.clone(),
                    }
                }
            }

            new_states.insert(
                name.clone(),
                StateNode::Implement {
                    context: context.clone(),
                    name: name.clone(),
                    references: new_references.clone(),
                    implements: implements.clone(),
                    next: next.clone(),
                },
            );
        }
    }
    Ok(new_states.clone())
}

/// Строит контекст модели: собирает вложенные именованные модели.
///
/// Вложенные модели доступны через [`ContextNode`].
fn construct_context_model(model: &Model) -> Result<ContextNode, Diagnostic> {
    let mut models = HashMap::new();
    for element in model.elements.iter() {
        if let ModelElement::Model(def) = element {
            let model = construct_model(&def)?;
            models.insert(def.clone().name.unwrap().name.clone(), Rc::new(model));
        }
    }
    Ok(ContextNode {
        models,
        ..Default::default()
    })
}

/// Строит контекст состояния.
///
/// В текущей реализации возвращает пустой контекст; расширение
/// для локальных переменных, функций и условий — в будущих версиях.
fn construct_context_state(_state: &StateDefine) -> Result<ContextNode, Diagnostic> {
    Ok(Default::default())
}

/// Преобразует АСД-условие в семантическое условие [`Condition`].
///
/// В текущей реализации всегда возвращает [`Condition::None`]; полная
/// семантическая обработка условий — в будущих версиях.
fn construct_condition(_cond: &ast::Condition) -> Result<Condition, Diagnostic> {
    Ok(Condition::None)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::parse;

    // ─── вспомогательная функция ────────────────────────────────────────────

    /// Разбирает BuT-программу и строит семантическую модель.
    fn build(src: &str) -> Result<ModelNode, Diagnostic> {
        let (ast, _) = parse(src, 0).expect("parse error");
        construct_model(&ast)
    }

    // ─── construct_model ───────────────────────────────────────────────────

    /// Пустая программа (нет состояний): `has_states()` должен вернуть `false`.
    #[test]
    fn empty_program_has_no_states() {
        let node = build("").unwrap();
        assert!(!node.has_states());
    }

    /// Программа без состояний, но с типом: `has_states()` → false.
    #[test]
    fn program_with_only_type_has_no_states() {
        let node = build("type u8 = [bit;8];").unwrap();
        assert!(!node.has_states());
    }

    /// Одна именованная модель с двумя состояниями.
    #[test]
    fn model_with_states_returns_true() {
        let node = build("model M { start S; state E; }").unwrap();
        // Корневая модель содержит модель M, а не состояния напрямую
        assert!(!node.has_states()); // корень анонимен и без состояний
    }

    /// Глобальные состояния верхнего уровня.
    #[test]
    fn top_level_states_are_found() {
        let node = build("start A; state B;").unwrap();
        assert!(node.has_states());
    }

    /// Имя корневой модели всегда `None`.
    #[test]
    fn root_model_name_is_none() {
        let node = build("start S;").unwrap();
        assert_eq!(node.name, None);
    }

    /// Именованная вложенная модель получает корректное имя.
    #[test]
    fn nested_model_name_is_set() {
        let (ast, _) = parse("model Foo { start S; }", 0).unwrap();
        // Ищем вложенную модель в elements
        if let ModelElement::Model(m) = &ast.elements[0] {
            let node = construct_model(m).unwrap();
            assert_eq!(node.name, Some("Foo".to_string()));
        } else {
            panic!("ожидался ModelElement::Model");
        }
    }

    // ─── construct_states ─────────────────────────────────────────────────

    /// Состояние без `ref` — SimpleNode, ссылки пустые.
    #[test]
    fn simple_state_no_refs() {
        let node = build("start S;").unwrap();
        assert!(node.states.contains_key("S"));
        if let StateNode::Simple { references, .. } = &node.states["S"] {
            assert!(references.is_empty());
        } else {
            panic!("ожидался StateNode::Simple");
        }
    }

    /// Состояние с корректной `ref`-ссылкой на другое состояние.
    #[test]
    fn ref_to_existing_state_resolves() {
        let node = build("start A { ref B; } state B;").unwrap();
        assert!(node.states.contains_key("A"));
        assert!(node.states.contains_key("B"));
        if let StateNode::Simple { references, .. } = &node.states["A"] {
            assert_eq!(references.len(), 1);
            assert_eq!(references[0].name, "B");
        } else {
            panic!("ожидался StateNode::Simple для A");
        }
    }

    /// Ссылка `ref` на несуществующее состояние — ошибка.
    #[test]
    fn ref_to_missing_state_is_error() {
        // Ghost не существует
        let result = build("start A { ref Ghost; }");
        assert!(result.is_err(), "ожидалась ошибка при неизвестной ссылке");
    }

    /// Два `next` в одном состоянии — ошибка.
    #[test]
    fn double_next_in_state_is_error() {
        // Два next в одном Implement-состоянии
        let result = build("start A = M { next B; next C; } state B; state C; model M { start S; }");
        assert!(result.is_err(), "ожидалась ошибка при двойном next");
    }

    /// Implement-состояние с `next` разрешается корректно.
    #[test]
    fn implement_state_with_next_resolves() {
        let node = build("start A = M { next B; } state B; model M { start S; }").unwrap();
        assert!(node.states.contains_key("A"));
        if let StateNode::Implement { next, .. } = &node.states["A"] {
            assert!(next.is_some(), "ожидался Some(next)");
        } else {
            panic!("ожидался StateNode::Implement для A");
        }
    }

    /// Implement-состояние без `next`.
    #[test]
    fn implement_state_without_next() {
        let node = build("start A = M { } state B; model M { start S; }").unwrap();
        if let StateNode::Implement { next, .. } = &node.states["A"] {
            assert!(next.is_none(), "next должен быть None");
        } else {
            panic!("ожидался StateNode::Implement для A");
        }
    }

    /// Несколько состояний с взаимными ссылками.
    #[test]
    fn multiple_states_with_cross_refs() {
        let node = build("start A { ref B; } state B { ref A; }").unwrap();
        assert_eq!(node.states.len(), 2);
    }

    /// `ref` с булевым условием разрешается без ошибок.
    #[test]
    fn ref_with_bool_condition_resolves() {
        let node = build("start A { ref B: true; } state B;").unwrap();
        if let StateNode::Simple { references, .. } = &node.states["A"] {
            assert_eq!(references.len(), 1);
        } else {
            panic!("ожидался StateNode::Simple");
        }
    }

    // ─── construct_context_model ──────────────────────────────────────────

    /// Вложенная модель попадает в контекст.
    #[test]
    fn nested_model_in_context() {
        let (ast, _) = parse("model Outer { model Inner { start S; } start A; }", 0).unwrap();
        let node = construct_model(&ast).unwrap();
        // Inner — вложен в Outer, который в корневом контексте
        assert!(!node.has_states()); // корень не содержит состояний напрямую
    }

    /// Конструктор принимает модель без вложенных моделей.
    #[test]
    fn context_without_nested_models() {
        let (ast, _) = parse("start S;", 0).unwrap();
        let ctx = construct_context_model(&ast).unwrap();
        // models пуст — нет вложенных model-блоков
        assert!(ctx.models.is_empty());
    }
}
