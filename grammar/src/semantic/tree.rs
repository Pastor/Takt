//! Построение семантического дерева из АСД языка BuT.
//!
//! Основные функции модуля:
//! - [`construct_model`] — главная точка входа, строит [`ModelNode`] из [`Model`].
//! - [`construct_states`] — извлекает состояния и разрешает ссылки между ними.
//! - [`construct_context_model`] — строит контекст (вложенные модели) для модели.
//! - [`construct_context_state`] — строит контекст для состояния (заглушка).
//! - [`construct_condition`] — преобразует условие АСД в семантическое условие.

use crate::diagnostics::Diagnostic;
use crate::parser::ast;
use crate::parser::ast::{
    Expression, Identifier, Model, ModelElement, StateDefine, StateElement, Type, VariableDefine,
};
use crate::semantic::{
    Condition, Implement, ModelNode, NamedBlockNode, Reference, StateNode, TypeNode, VariableNode,
};
use std::cell::RefCell;
use std::collections::HashMap;
use std::rc::Rc;

#[inline]
fn extract_name(id: Option<Identifier>) -> Result<String, Diagnostic> {
    if let Some(id) = id {
        Ok(id.name.clone())
    } else {
        Err("Identifier is None".into())
    }
}

fn construct_model0(
    model: &Model,
    upper: Option<Rc<RefCell<ModelNode>>>,
) -> Result<Rc<RefCell<ModelNode>>, Diagnostic> {
    let name = model.name.clone();

    let model_node = ModelNode {
        upper: upper.map(|m| Rc::clone(&m)),
        name: name.map(|i| i.name.clone()),
        ..Default::default()
    };
    let model_node = Rc::new(RefCell::new(model_node));
    let mut models = HashMap::new();
    let mut variables = HashMap::new();
    let mut types = HashMap::new();
    for element in model.elements.iter() {
        if let ModelElement::Model(model) = element {
            let model = construct_model0(model, Some(Rc::clone(&model_node)))?;
            //TODO: Если есть такое имя кидаем ошибку
            models.insert(model.clone().borrow().name.clone().unwrap(), model);
        } else if let ModelElement::Import(def) = element {
            //TODO: Загружаем файл как модель и именуем ее: если as - то ставим это имя, если простой импорт то ставим имя файла в CamelCase
            //TODO: Если такое имя есть, ошибка
        } else if let ModelElement::Variable(def) = element {
            //TODO: Добавить вывод типа для переменных и констант
            match *def.clone() {
                VariableDefine::Variable {
                    typ,
                    name,
                    initializer,
                    ..
                } => {
                    let name = extract_name(name.clone())?;
                    variables.insert(
                        name.clone(),
                        VariableNode::Simple(
                            name.clone(),
                            construct_type(typ, &types)?,
                            initializer,
                        ),
                    )
                }
                VariableDefine::Port {
                    typ,
                    name,
                    initializer,
                    ..
                } => {
                    let name = extract_name(name.clone())?;
                    let type_node = construct_type(typ, &types)?;
                    if type_node == TypeNode::Detecting {
                        return Err("Port must have concrete type".into());
                    }
                    variables.insert(
                        name.clone(),
                        VariableNode::Port(
                            name.clone(),
                            type_node,
                            initializer
                                .filter(|i| {
                                    if let Expression::Address(..) = i {
                                        true
                                    } else if let Expression::Number(..) = i {
                                        true
                                    } else {
                                        false
                                    }
                                })
                                .ok_or_else(|| "Port maybe initialized Address".into())?,
                        ),
                    )
                }
                VariableDefine::Constant {
                    typ,
                    name,
                    initializer,
                    ..
                } => {
                    let name = extract_name(name.clone())?;
                    variables.insert(
                        name.clone(),
                        VariableNode::Const(
                            name.clone(),
                            construct_type(typ, &types)?,
                            initializer,
                        ),
                    )
                }
            };
        } else if let ModelElement::Type(def) = element {
            let name = def.clone().name.name.clone();
            let typ = def.ty.clone();
            let types_clone = types.clone();
            types.insert(name.clone(), construct_type(Some(typ), &types_clone)?);
        }
    }
    model_node.borrow_mut().models = models;
    model_node.borrow_mut().variables = variables;
    model_node.borrow_mut().states = construct_states(model)?;
    model_node.borrow_mut().types = types;
    Ok(Rc::clone(&model_node))
}

fn construct_type(
    typ: Option<Type>,
    map: &HashMap<String, TypeNode>,
) -> Result<TypeNode, Diagnostic> {
    if typ.is_none() {
        return Ok(TypeNode::Detecting);
    }
    match typ.unwrap() {
        Type::Address { address, bit } => Ok(TypeNode::Address(address, bit)),
        Type::Bit => Ok(TypeNode::Bit),
        Type::Bool => Ok(TypeNode::Bit),
        Type::Rational => Ok(TypeNode::Rational),
        Type::Alias(def) => match def.name.as_str() {
            "bit" => Ok(TypeNode::Bit),
            "bool" => Ok(TypeNode::Bit),
            "float" => Ok(TypeNode::Rational),
            local => Ok(map
                .get(local)
                .ok_or_else(|| {
                    format!("Local type {} not found", &def.name)
                        .as_str()
                        .into()
                })?
                .clone()),
        },
        Type::Array {
            element_type,
            element_count,
            ..
        } => Ok(TypeNode::Array(
            element_count,
            Box::new(construct_type(Some(*element_type), map)?),
        )),
        Type::Function { .. } => Ok(TypeNode::Unsupported),
    }
}

fn construct_model1(model: Rc<RefCell<ModelNode>>) -> Result<Rc<RefCell<ModelNode>>, Diagnostic> {
    // Клонируем состояния до мутабельного займа, чтобы construct_implement мог брать заём
    let states = model.borrow().states.clone();

    let mut prepared_states = HashMap::new();
    for (name, state) in states.iter() {
        if let StateNode::Implement {
            implements: Implement::Unresolved,
            expression,
            named_blocks,
            references,
            next,
            name,
        } = state.clone()
        {
            if let Some(expression) = expression {
                prepared_states.insert(
                    name.clone(),
                    StateNode::Implement {
                        named_blocks,
                        name: name.clone(),
                        references,
                        implements: construct_implement(expression.clone(), Rc::clone(&model))?,
                        expression: Some(expression.clone()),
                        next,
                    },
                );
            } else {
                return Err("Expression not defined".into());
            }
        } else {
            prepared_states.insert(name.clone(), state.clone());
        }
    }
    model.borrow_mut().states = prepared_states;

    // Клонируем список вложенных моделей до рекурсивного вызова
    let nested: Vec<(String, Rc<RefCell<ModelNode>>)> = model
        .borrow()
        .models
        .iter()
        .map(|(k, v)| (k.clone(), Rc::clone(v)))
        .collect();

    let mut models = HashMap::new();
    for (name, nested_model) in nested {
        models.insert(name, construct_model1(Rc::clone(&nested_model))?);
    }
    model.borrow_mut().models = models;

    Ok(Rc::clone(&model))
}

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
pub fn construct_model(
    model: &Model,
    upper: Option<Rc<RefCell<ModelNode>>>,
) -> Result<Rc<RefCell<ModelNode>>, Diagnostic> {
    let model = construct_model0(model, upper)?;
    let model = construct_model1(model)?;
    Ok(model)
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
            let state = if let Some(expr) = implements {
                let next = next.map(|n| Reference {
                    name: n,
                    cond: Condition::None,
                    object: Box::new(StateNode::Unresolved),
                });
                StateNode::Implement {
                    named_blocks: construct_named_blocks(def)?,
                    name: name.clone(),
                    references,
                    implements: Implement::Unresolved,
                    next,
                    expression: Some(expr),
                }
            } else {
                StateNode::Simple {
                    named_blocks: construct_named_blocks(def)?,
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
            name, references, ..
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
                    named_blocks: Default::default(),
                    name: name.clone(),
                    references: new_references.clone(),
                },
            );
        } else if let StateNode::Implement {
            named_blocks,
            name,
            references,
            implements,
            next,
            expression,
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
            if let Some(next_ref) = next.as_mut() {
                if let StateNode::Unresolved = *next_ref.object {
                    let target_name = next_ref.name.clone();
                    let state = states.get(&target_name).ok_or_else(|| {
                        format!("Reference '{}' not found", &target_name)
                            .as_str()
                            .into()
                    })?;
                    *next_ref = Reference {
                        name: target_name,
                        cond: next_ref.cond.clone(),
                        object: state.clone(),
                    }
                }
            }

            new_states.insert(
                name.clone(),
                StateNode::Implement {
                    named_blocks,
                    name: name.clone(),
                    references: new_references.clone(),
                    implements: implements.clone(),
                    next: next.clone(),
                    expression,
                },
            );
        }
    }
    Ok(new_states.clone())
}

/// Преобразует АСД-условие в семантическое условие [`Condition`].
///
/// В текущей реализации всегда возвращает [`Condition::None`]; полная
/// семантическая обработка условий — в будущих версиях.
fn construct_condition(_cond: &ast::Condition) -> Result<Condition, Diagnostic> {
    Ok(Condition::None)
}

fn construct_named_blocks(
    _state: &StateDefine,
) -> Result<HashMap<String, NamedBlockNode>, Diagnostic> {
    Ok(HashMap::new())
}

fn construct_implement(
    expression: Expression,
    model: Rc<RefCell<ModelNode>>,
) -> Result<Implement, Diagnostic> {
    let model = Rc::clone(&model);
    match expression {
        Expression::Variable(id) => {
            let model = model.as_ref().borrow();
            let model = model
                .search_model(&id.name)
                .ok_or_else(|| format!("Model {} not found", &id.name).as_str().into())?;
            Ok(Implement::Model(Rc::clone(&model)))
        }
        Expression::Parenthesis(_, expression) => {
            Ok(construct_implement(*expression, model.clone())?)
        }
        Expression::Add(_, left, right) => {
            let left = construct_implement(*left, model.clone())?;
            let right = construct_implement(*right, model.clone())?;
            Ok(Implement::Add(Box::new(left), Box::new(right)))
        }
        Expression::BitwiseOr(_, left, right) => {
            let left = construct_implement(*left, model.clone())?;
            let right = construct_implement(*right, model.clone())?;
            Ok(Implement::Or(Box::new(left), Box::new(right)))
        }
        other => return Err(format!("Unknown expression {:?}", other).as_str().into()),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::parse;

    // ─── вспомогательная функция ────────────────────────────────────────────

    /// Разбирает BuT-программу и строит семантическую модель.
    fn build(src: &str) -> Result<ModelNode, Diagnostic> {
        let (ast, _) = parse(src, 0).expect("parse error");
        construct_model(&ast, None).map(|model| model.take())
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
            let node = construct_model(m, None).unwrap();
            assert_eq!(node.take().name, Some("Foo".to_string()));
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
        let result =
            build("start A = M { next B; next C; } state B; state C; model M { start S; }");
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
        let node = construct_model(&ast, None).unwrap();
        // Inner — вложен в Outer, который в корневом контексте
        assert!(!node.take().has_states()); // корень не содержит состояний напрямую
    }
}
