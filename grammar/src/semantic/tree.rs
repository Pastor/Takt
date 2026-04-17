//! Построение семантического дерева из АСД языка BuT.
//!
//! Основные функции модуля:
//! - [`construct_model`] — главная точка входа, строит [`ModelNode`] из [`Model`].
//! - [`construct_states`] — извлекает состояния и разрешает ссылки между ними.
//! - [`construct_context_model`] — строит контекст (вложенные модели) для модели.
//! - [`construct_context_state`] — строит контекст для состояния (заглушка).
//! - [`construct_condition`] — преобразует условие АСД в семантическое условие.

use crate::diagnostics::{Diagnostic, Location};
use crate::parse;
use crate::parser::ast;
use crate::parser::ast::{
    Identifier, ImportDefine, Model, ModelElement, StateDefine, StateElement, StateKind,
    VariableDefine,
};
use crate::semantic::condition::{extract_conditions, resolve_condition};
use crate::semantic::expression::construct_expression;
use crate::semantic::extend::Extend;
use crate::semantic::formula;
use crate::semantic::function::construct_function;
use crate::semantic::import::read_import_file;
use crate::semantic::named_block::resolve_named_blocks;
use crate::semantic::naming::normalize_camelcase_name;
use crate::semantic::reference::resolve_state_references;
use crate::semantic::type_inference::type_inference;
use crate::semantic::type_node::{TypeNode, construct_type};
use crate::semantic::validate::{
    check_implicit_bool_conditions, check_transition_completeness, check_type_alias_cycles_ast,
    validate_model,
};
use crate::semantic::{
    ConditionDefinitionNode, ConditionNode, ExpressionNode, Formula, FunctionDefinitionNode,
    ModelNode, NamedCodeBlockDefinitionNode, ReferenceNode, StateNode, StateNodeKind,
    StatementNode, VariableNode, extend,
};
use std::cell::RefCell;
use std::collections::HashMap;
use std::rc::Rc;
use std::rc::Weak;

/// Извлекает имя из опционального [`Identifier`].
///
/// Возвращает [`Diagnostic`]-ошибку с указанной позицией, если идентификатор отсутствует.
#[inline]
fn extract_name(id: Option<Identifier>, loc: Location) -> Result<String, Diagnostic> {
    if let Some(id) = id {
        Ok(id.name.clone())
    } else {
        Err(Diagnostic::error(loc, "Идентификатор не задан".to_string()).with_code("SE-021"))
    }
}

/// Проверяет, не создаёт ли импорт файла `new_file` цикл в текущем стеке обработки.
///
/// Если `new_file` уже присутствует в `import_stack`, значит мы столкнулись
/// с циклической зависимостью. В этом случае возвращается [`Diagnostic`]-ошибка
/// с цепочкой вида `a.but → b.but → a.but`.
///
/// # Примеры цикла
///
/// ```text
/// Циклический импорт: /src/a.but → /src/b.but → /src/a.but
/// ```
fn check_import_cycle(
    import_stack: &[String],
    new_file: &str,
    loc: Location,
) -> Result<(), Diagnostic> {
    if let Some(pos) = import_stack.iter().position(|f| f == new_file) {
        // Строим цепочку начиная с точки входа цикла
        let mut chain: Vec<&str> = import_stack[pos..].iter().map(|s| s.as_str()).collect();
        chain.push(new_file);
        return Err(
            Diagnostic::error(loc, format!("Циклический импорт: {}", chain.join(" → ")))
                .with_code("SE-013"),
        );
    }
    Ok(())
}

fn construct_model_stage0(
    model: &Model,
    upper: Option<Rc<RefCell<ModelNode>>>,
    search_paths: &[String],
    import_stack: &mut Vec<String>,
) -> Result<Rc<RefCell<ModelNode>>, Diagnostic> {
    let name = model.name.clone();

    let model_node = ModelNode {
        upper: upper.as_ref().map(Rc::downgrade),
        loc: model.loc,
        name: name.map(|i| i.name.clone()),
        implements: model
            .implements
            .clone()
            .map(Extend::Unresolved)
            .unwrap_or(Extend::None),
        ..Default::default()
    };
    let model_node = Rc::new(RefCell::new(model_node));
    let mut models = HashMap::new();
    let mut variables = HashMap::new();
    let mut conditions = HashMap::new();
    let mut named_blocks = Vec::new();
    let mut functions = HashMap::new();
    let mut formula_conditions: Vec<ast::Condition> = Vec::new();

    // Ce16: предварительная проверка циклических псевдонимов до вызова construct_type.
    // Собираем все AST-определения типов из текущего уровня модели.
    {
        let mut raw_defs: HashMap<String, ast::Type> = HashMap::new();
        let mut type_locs_pre: HashMap<String, Location> = HashMap::new();
        for element in model.elements.iter() {
            if let ModelElement::Type(def) = element {
                raw_defs.insert(def.name.name.clone(), def.ty.clone());
                type_locs_pre.insert(def.name.name.clone(), def.name.loc);
            }
        }
        if !raw_defs.is_empty() {
            let cycle_diags = check_type_alias_cycles_ast(&raw_defs, &type_locs_pre);
            if let Some(first) = cycle_diags.into_iter().next() {
                return Err(first);
            }
        }
    }

    for element in model.elements.iter() {
        if let ModelElement::Model(model) = element {
            let model = construct_model_stage0(
                model,
                Some(Rc::clone(&model_node)),
                search_paths,
                import_stack,
            )?;
            let model_name = model.borrow().name.clone().unwrap();
            if models.contains_key(&model_name) {
                return Err(Diagnostic::declaration_error(
                    model.borrow().loc,
                    format!("Модель с именем '{}' уже объявлена", &model_name),
                )
                .with_code("SE-006"));
            }
            models.insert(model_name, model);
        } else if let ModelElement::Import(def) = element {
            match def {
                ImportDefine::Plain(path, import_loc) => {
                    let (content, filename) = read_import_file(search_paths, path)?;
                    // Проверяем цикл ДО рекурсивной обработки файла
                    check_import_cycle(import_stack, &filename, *import_loc)?;
                    // Извлекаем только имя файла (без директории и расширения),
                    // затем нормализуем в CamelCase: "my_model.but" → "MyModel".
                    // Прежде использовался срез filename[..len-4], что давало полный путь
                    // и, как следствие, некорректное имя (например, "TmpMyModel").
                    let stem = std::path::Path::new(&filename)
                        .file_stem()
                        .ok_or_else(|| {
                            Diagnostic::error(
                                *import_loc,
                                format!("Неверный путь к файлу импорта: «{}»", filename),
                            )
                            .with_code("SE-013")
                        })?
                        .to_string_lossy();
                    let model_name = normalize_camelcase_name(&stem);
                    if models.contains_key(&model_name) {
                        return Err(Diagnostic::declaration_error(
                            *import_loc,
                            format!("Модель с именем '{}' уже объявлена", &model_name),
                        )
                        .with_code("SE-006"));
                    }
                    match parse(&content, 0) {
                        Ok((model, _)) => {
                            // Добавляем файл в стек, обрабатываем, убираем
                            import_stack.push(filename.clone());
                            let result =
                                construct_model_impl(&model, None, search_paths, import_stack);
                            import_stack.pop();
                            models.insert(model_name, result?);
                        }
                        Err(d) => return Err(d.first().unwrap().clone()),
                    }
                }
                ImportDefine::GlobalSymbol(path, id, import_loc) => {
                    let (content, filename) = read_import_file(search_paths, path)?;
                    // Проверяем цикл ДО рекурсивной обработки файла
                    check_import_cycle(import_stack, &filename, *import_loc)?;
                    let model_name = id.name.clone();
                    if models.contains_key(&model_name) {
                        return Err(Diagnostic::declaration_error(
                            id.loc,
                            format!("Модель с именем '{}' уже объявлена", &model_name),
                        )
                        .with_code("SE-006"));
                    }
                    match parse(&content, 0) {
                        Ok((model, _)) => {
                            // Добавляем файл в стек, обрабатываем, убираем
                            import_stack.push(filename.clone());
                            let result =
                                construct_model_impl(&model, None, search_paths, import_stack);
                            import_stack.pop();
                            models.insert(model_name, result?);
                        }
                        Err(d) => return Err(d.first().unwrap().clone()),
                    }
                }
                // `import { A, B as C } from "file.but";`
                //
                // Загружает файл, строит его семантическую модель, затем
                // выборочно экспортирует указанные имена в текущий контекст.
                //
                // Поддерживаемые категории: модели, псевдонимы типов, переменные, условия.
                // Приоритет поиска: модель → тип → переменная → условие.
                ImportDefine::Rename(path, symbols, import_loc) => {
                    let (content, filename) = read_import_file(search_paths, path)?;
                    // Проверяем цикл ДО рекурсивной обработки файла
                    check_import_cycle(import_stack, &filename, *import_loc)?;
                    import_stack.push(filename.clone());
                    let result = match parse(&content, 0) {
                        Ok((ast_model, _)) => {
                            construct_model_impl(&ast_model, None, search_paths, import_stack)
                        }
                        Err(d) => {
                            import_stack.pop();
                            return Err(d.first().unwrap().clone());
                        }
                    };
                    import_stack.pop();
                    let imported = result?;
                    let src = imported.borrow();
                    for (orig_id, alias_id) in symbols {
                        let orig = &orig_id.name;
                        // Целевое имя: alias если задан, иначе оригинальное
                        let alias = alias_id
                            .as_ref()
                            .map_or_else(|| orig.clone(), |a| a.name.clone());
                        let sym_loc = alias_id.as_ref().map(|a| a.loc).unwrap_or(orig_id.loc);
                        // Поиск в категориях: модель → тип → переменная → условие
                        if let Some(m) = src.models.get(orig) {
                            if models.contains_key(&alias) {
                                return Err(Diagnostic::declaration_error(
                                    sym_loc,
                                    format!("Модель с именем '{}' уже объявлена", alias),
                                )
                                .with_code("SE-006"));
                            }
                            models.insert(alias, Rc::clone(m));
                        } else if let Some(t) = src.types.get(orig) {
                            if model_node.borrow().types.contains_key(&alias) {
                                return Err(Diagnostic::declaration_error(
                                    sym_loc,
                                    format!("Тип '{}' уже объявлен", alias),
                                )
                                .with_code("SE-007"));
                            }
                            model_node.borrow_mut().types.insert(alias, t.clone());
                        } else if let Some(v) = src.variables.get(orig) {
                            if variables.contains_key(&alias) {
                                return Err(Diagnostic::declaration_error(
                                    sym_loc,
                                    format!("Переменная '{}' уже объявлена", alias),
                                )
                                .with_code("SE-005"));
                            }
                            variables.insert(alias, v.clone());
                        } else if let Some(c) = src.conditions.get(orig) {
                            if conditions.contains_key(&alias) {
                                return Err(Diagnostic::declaration_error(
                                    sym_loc,
                                    format!("Условие '{}' уже объявлено", alias),
                                )
                                .with_code("SE-008"));
                            }
                            conditions.insert(alias, c.clone());
                        } else {
                            return Err(Diagnostic::declaration_error(
                                orig_id.loc,
                                format!("Идентификатор '{}' не найден в импортируемом файле", orig),
                            )
                            .with_code("SE-017"));
                        }
                    }
                }
            }
        } else if let ModelElement::Variable(def) = element {
            // Пока тип определяется только из явной аннотации.
            match *def.clone() {
                VariableDefine::Variable {
                    loc,
                    typ,
                    name,
                    initializer,
                } => {
                    let name = extract_name(name.clone(), loc)?;
                    variables.insert(
                        name.clone(),
                        VariableNode::Simple {
                            upper: Some(Rc::downgrade(&model_node)),
                            loc,
                            name: name.clone(),
                            ty: construct_type(typ, model_node.clone())?,
                            expr: initializer
                                .map(ExpressionNode::Unresolved)
                                .unwrap_or(ExpressionNode::None),
                        },
                    )
                }
                VariableDefine::Port {
                    loc,
                    typ,
                    name,
                    initializer,
                } => {
                    let name = extract_name(name.clone(), loc)?;
                    let type_node = construct_type(typ, model_node.clone())?;
                    if type_node == TypeNode::Inference {
                        return Err(Diagnostic::error(
                            loc,
                            "Порт должен иметь конкретный тип".to_string(),
                        )
                        .with_code("SE-023"));
                    }
                    variables.insert(
                        name.clone(),
                        VariableNode::Port {
                            upper: Some(Rc::downgrade(&model_node)),
                            loc,
                            name: name.clone(),
                            ty: type_node,
                            expr: ExpressionNode::Unresolved(
                                initializer
                                    .filter(|i| {
                                        matches!(
                                            i,
                                            ast::Expression::Address(..)
                                                | ast::Expression::Number(..)
                                        )
                                    })
                                    .ok_or_else(|| {
                                        Diagnostic::error(
                                            loc,
                                            "Порт должен быть инициализирован адресом".to_string(),
                                        )
                                        .with_code("SE-024")
                                    })?,
                            ),
                        },
                    )
                }
                VariableDefine::Constant {
                    loc,
                    typ,
                    name,
                    initializer,
                } => {
                    let name = extract_name(name.clone(), loc)?;
                    variables.insert(
                        name.clone(),
                        VariableNode::Const {
                            upper: Some(Rc::downgrade(&model_node)),
                            loc,
                            name: name.clone(),
                            ty: construct_type(typ, model_node.clone())?,
                            expr: ExpressionNode::Unresolved(initializer),
                        },
                    )
                }
            };
        } else if let ModelElement::Type(def) = element {
            let name = def.clone().name.name.clone();
            let typ = def.ty.clone();
            // Вычисляем тип ДО borrow_mut, чтобы construct_type мог безопасно вызвать
            // model_node.borrow() (search_type) без конфликта с borrow_mut.
            let resolved_type = construct_type(Some(typ.clone()), model_node.clone())?;
            // Сохраняем сырой АСД-тип для последующей проверки циклических псевдонимов (Ce16)
            {
                let mut bm = model_node.borrow_mut();
                bm.raw_type_defs.insert(name.clone(), typ.clone());
                bm.types.insert(name.clone(), resolved_type);
                bm.type_locs.insert(name.clone(), def.name.loc);
            }
        } else if let ModelElement::Condition(def) = element {
            let def_loc = def
                .as_ref()
                .name
                .as_ref()
                .map(|id| id.loc)
                .unwrap_or(Location::Implicit);
            let name = def
                .clone()
                .name
                .ok_or_else(|| {
                    Diagnostic::error(
                        def_loc,
                        "Условие при определении должно иметь имя".to_string(),
                    )
                    .with_code("SE-019")
                })?
                .name
                .clone();
            conditions.insert(
                name.clone(),
                ConditionDefinitionNode {
                    name: name.clone(),
                    loc: def_loc,
                    value: ConditionNode::Unresolved(def.value.clone()),
                    upper: Some(Rc::downgrade(&model_node)),
                },
            );
        } else if let ModelElement::NamedBlockCode(def) = element {
            let name = def
                .clone()
                .name
                .ok_or_else(|| {
                    Diagnostic::error(
                        def.loc,
                        "Именованный блок кода при определении должен иметь имя".to_string(),
                    )
                    .with_code("SE-018")
                })?
                .name
                .clone();
            // I8: сохраняем сырой АСД-оператор для последующей индексации локальных переменных
            model_node
                .borrow_mut()
                .named_block_raw
                .push((name.clone(), def.statement.clone()));
            let block = match name.as_str() {
                "enter" => NamedCodeBlockDefinitionNode::Enter {
                    upper: Some(Rc::downgrade(&model_node)),
                    body: StatementNode::Unresolved(def.statement.clone()),
                },
                "exit" => NamedCodeBlockDefinitionNode::Exit {
                    upper: Some(Rc::downgrade(&model_node)),
                    body: StatementNode::Unresolved(def.statement.clone()),
                },
                "always" => NamedCodeBlockDefinitionNode::Always {
                    upper: Some(Rc::downgrade(&model_node)),
                    body: StatementNode::Unresolved(def.statement.clone()),
                },
                name => NamedCodeBlockDefinitionNode::Unknown {
                    upper: Some(Rc::downgrade(&model_node)),
                    name: name.to_string(),
                    body: StatementNode::Unresolved(def.statement.clone()),
                },
            };
            named_blocks.push(block);
        } else if let ModelElement::Function(def) = element {
            let name = def
                .clone()
                .name
                .ok_or_else(|| {
                    Diagnostic::error(
                        def.loc,
                        "При определении функция должна иметь имя".to_string(),
                    )
                    .with_code("SE-022")
                })?
                .name
                .clone();
            functions.insert(
                name.clone(),
                FunctionDefinitionNode::Unresolved(*def.clone()),
            );
        } else if let ModelElement::Enum(e) = element {
            // FE1: Обработка перечислений. Присваиваем последовательные значения
            // вариантам без явных значений (автоинкремент от 0).
            let enum_name = e
                .name
                .as_ref()
                .map(|id| id.name.clone())
                .unwrap_or_default();
            let mut next_val: i64 = 0;
            let mut variant_pairs = Vec::new();
            for variant in &e.variants {
                let val = variant.value.unwrap_or(next_val);
                next_val = val + 1;
                variant_pairs.push((variant.name.name.clone(), val));
            }
            let enum_loc = e.name.as_ref().map(|id| id.loc).unwrap_or(e.loc);
            let mut enum_node = crate::semantic::EnumDefinitionNode::new(
                &enum_name,
                &variant_pairs
                    .iter()
                    .map(|(n, v)| (n.as_str(), Some(*v)))
                    .collect::<Vec<_>>(),
            );
            enum_node.loc = enum_loc;
            // Ce4: Регистрируем перечисление в двух местах:
            //
            // 1. `model_node.enums` — для поиска через `search_enum` / `search_enum_variant`.
            //
            // 2. `types` — для разрешения аннотаций типа `var x: Color = 0;`.
            //    Парсер создаёт `Type::Alias("Color")` для таких аннотаций; `construct_type`
            //    ищет псевдоним в таблице `types`. Добавляем `TypeNode::Enum("Color")`,
            //    чтобы переменная получила корректный тип.
            //
            //    Ограничение: enum должен быть объявлен ДО переменных, использующих его как
            //    тип (аналогично псевдонимам `type`). Если enum объявлен после — тип будет
            //    `TypeNode::Unsupported`; это считается ошибкой пользователя.
            model_node
                .borrow_mut()
                .enums
                .insert(enum_name.clone(), enum_node);
            if !enum_name.is_empty() {
                model_node
                    .borrow_mut()
                    .types
                    .insert(enum_name.clone(), TypeNode::Enum(enum_name.clone()));
                model_node
                    .borrow_mut()
                    .type_locs
                    .insert(enum_name.clone(), enum_loc);
            }
        } else if let ModelElement::Struct(s) = element {
            // NI3: Обработка структурных типов.
            let struct_name = s
                .name
                .as_ref()
                .map(|id| id.name.clone())
                .unwrap_or_default();
            let struct_loc = s.name.as_ref().map(|id| id.loc).unwrap_or(s.loc);

            // Разрешаем типы полей в контексте текущей модели.
            let mut field_pairs: Vec<(String, TypeNode)> = Vec::new();
            for field in &s.fields {
                let field_ty = construct_type(Some(field.ty.clone()), model_node.clone())
                    .unwrap_or(TypeNode::Unsupported);
                field_pairs.push((field.name.name.clone(), field_ty));
            }

            let mut struct_node = crate::semantic::StructDefinitionNode {
                name: struct_name.clone(),
                fields: field_pairs,
                loc: struct_loc,
            };
            struct_node.loc = struct_loc;

            model_node
                .borrow_mut()
                .structs
                .insert(struct_name.clone(), struct_node);
            // Регистрируем структуру в таблице типов для разрешения `var p: Point = ...;`
            if !struct_name.is_empty() {
                model_node
                    .borrow_mut()
                    .types
                    .insert(struct_name.clone(), TypeNode::Struct(struct_name.clone()));
                model_node
                    .borrow_mut()
                    .type_locs
                    .insert(struct_name.clone(), struct_loc);
            }
        } else if let ModelElement::InlineFormula(inline) = element {
            match &**inline {
                ast::InlineFormulaDefine::Guard { conditions, .. } => {
                    for cond in conditions {
                        formula_conditions.push(cond.clone());
                    }
                }
                ast::InlineFormulaDefine::Ltl { .. } => {
                    // TODO: Реализовать поддержку LTL в моделях
                }
            }
        }
    }
    model_node.borrow_mut().models = models;
    model_node.borrow_mut().states = construct_states(model, model_node.clone())?;
    model_node.borrow_mut().variables = variables;
    model_node.borrow_mut().conditions = conditions;
    model_node.borrow_mut().named_blocks = named_blocks;
    model_node.borrow_mut().functions = functions;
    {
        let mut formulas = Vec::new();
        for cond in &formula_conditions {
            if let Ok(cn) = resolve_condition(cond, Rc::clone(&model_node)) {
                formulas.push(formula::condition_to_formula(&cn));
            }
        }
        model_node.borrow_mut().formulas = formulas;
    }
    Ok(Rc::clone(&model_node))
}

fn construct_model_stage1(
    model: Rc<RefCell<ModelNode>>,
) -> Result<Rc<RefCell<ModelNode>>, Diagnostic> {
    // Клонируем состояния до мутабельного займа, чтобы construct_implement мог брать заём
    let states = model.borrow().states.clone();

    let mut prepared_states = HashMap::new();
    for (name, state) in states.iter() {
        if let StateNode::Implement {
            upper,
            loc,
            implements: Extend::Unresolved(implement_expression),
            named_blocks,
            references,
            next,
            name,
            kind,
            formulas,
        } = state.clone()
        {
            let implements = extend::unroll_extend_expression(
                ExpressionNode::Unresolved(implement_expression),
                Rc::clone(&model),
            )?;
            // let implements = extend::compact_implement(implements, Rc::clone(&model), &name);
            prepared_states.insert(
                name.clone(),
                StateNode::Implement {
                    upper: upper.clone(),
                    loc,
                    named_blocks,
                    name: name.clone(),
                    references,
                    implements,
                    next,
                    kind,
                    formulas,
                },
            );
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
        models.insert(name, construct_model_stage1(Rc::clone(&nested_model))?);
    }
    model.borrow_mut().models = models;

    Ok(Rc::clone(&model))
}

/// Разрешает инициализаторы переменных, заменяя [`ExpressionNode::Unresolved`]
/// полностью разрешёнными семантическими выражениями.
///
/// Вызывается до [`type_inference`], чтобы вывод типа работал с разрешёнными,
/// а не «сырыми» АСД-выражениями.
///
/// # Ошибки
///
/// Пробрасывает [`Diagnostic`] из [`construct_expression`], если идентификатор
/// в инициализаторе не найден в области видимости.
fn resolve_variable_expressions(
    variables: &HashMap<String, VariableNode>,
    model: Rc<RefCell<ModelNode>>,
) -> Result<HashMap<String, VariableNode>, Diagnostic> {
    let mut result = HashMap::new();
    for (name, var) in variables {
        let resolved = match var.clone() {
            VariableNode::Simple {
                upper,
                loc,
                name: n,
                ty,
                expr: ExpressionNode::Unresolved(expr),
            } => VariableNode::Simple {
                upper,
                loc,
                name: n,
                ty,
                expr: construct_expression(expr, vec![], model.clone())?,
            },
            VariableNode::Const {
                upper,
                loc,
                name: n,
                ty,
                expr: ExpressionNode::Unresolved(expr),
            } => VariableNode::Const {
                upper,
                loc,
                name: n,
                ty,
                expr: construct_expression(expr, vec![], model.clone())?,
            },
            VariableNode::Port {
                upper,
                loc,
                name: n,
                ty,
                expr: ExpressionNode::Unresolved(expr),
            } => VariableNode::Port {
                upper,
                loc,
                name: n,
                ty,
                expr: construct_expression(expr, vec![], model.clone())?,
            },
            other => other,
        };
        result.insert(name.clone(), resolved);
    }
    Ok(result)
}

fn construct_model_stage2(
    model: Rc<RefCell<ModelNode>>,
) -> Result<Rc<RefCell<ModelNode>>, Diagnostic> {
    // Шаг 2а: разрешаем инициализаторы переменных (Unresolved → полноценное Expression)
    let variables = model.borrow().variables.clone();
    let variables = resolve_variable_expressions(&variables, model.clone())?;
    model.borrow_mut().variables = variables;

    // Шаг 2б: выводим типы переменных, у которых тип не задан явно
    let mut variables = model.borrow().variables.clone();
    variables = type_inference(&mut variables, model.clone())?;
    model.borrow_mut().variables = variables;
    // Рекурсивно обрабатываем вложенные модели с их собственным контекстом
    let nested: Vec<(String, Rc<RefCell<ModelNode>>)> = model
        .borrow()
        .models
        .iter()
        .map(|(k, v)| (k.clone(), Rc::clone(v)))
        .collect();
    let mut models = HashMap::new();
    for (name, nested_model) in nested {
        models.insert(name, construct_model_stage2(nested_model)?);
    }
    model.borrow_mut().models = models;
    Ok(Rc::clone(&model))
}

fn construct_model_stage3(
    model: Rc<RefCell<ModelNode>>,
) -> Result<Rc<RefCell<ModelNode>>, Diagnostic> {
    let mut conditions = model.borrow().conditions.clone();
    conditions = extract_conditions(&conditions, model.clone())?;
    conditions = extract_conditions(&conditions, model.clone())?;
    model.borrow_mut().conditions = conditions;
    // Рекурсивно обрабатываем вложенные модели с их собственным контекстом
    let nested: Vec<(String, Rc<RefCell<ModelNode>>)> = model
        .borrow()
        .models
        .iter()
        .map(|(k, v)| (k.clone(), Rc::clone(v)))
        .collect();
    let mut models = HashMap::new();
    for (name, nested_model) in nested {
        models.insert(name, construct_model_stage3(nested_model)?);
    }
    model.borrow_mut().models = models;
    Ok(Rc::clone(&model))
}

/// Этап 4: разрешение операторов в именованных блоках кода.
///
/// Выполняет три задачи:
/// 1. Разрешает блоки на уровне модели (`model.named_blocks`).
/// 2. Разрешает блоки в состояниях модели (`state.named_blocks`).
/// 3. Рекурсивно применяет этот же процесс ко всем вложенным моделям,
///    передавая контекст вложенной модели (для корректного разрешения
///    переменных во вложенных областях видимости).
///
/// При ошибке разрешения оператор сохраняется в виде [`StatementNode::Unresolved`]
/// (ошибка не пробрасывается), что позволяет корректно обрабатывать
/// встроенные функции (`debug`, `S`, …) без их явной регистрации.
fn construct_model_stage4(
    model: Rc<RefCell<ModelNode>>,
) -> Result<Rc<RefCell<ModelNode>>, Diagnostic> {
    // Разрешаем блоки на уровне текущей модели
    let named_blocks = std::mem::take(&mut model.borrow_mut().named_blocks);
    model.borrow_mut().named_blocks = resolve_named_blocks(named_blocks, model.clone())?;

    // Разрешаем блоки в состояниях текущей модели
    let states = std::mem::take(&mut model.borrow_mut().states);
    let mut resolved_states = HashMap::with_capacity(states.len());
    for (state_name, state) in states {
        let resolved = resolve_state_named_blocks(state, model.clone())?;
        resolved_states.insert(state_name, resolved);
    }
    model.borrow_mut().states = resolved_states;

    // Рекурсивно обрабатываем вложенные модели с их собственным контекстом
    let nested: Vec<(String, Rc<RefCell<ModelNode>>)> = model
        .borrow()
        .models
        .iter()
        .map(|(k, v)| (k.clone(), Rc::clone(v)))
        .collect();
    let mut models = HashMap::new();
    for (name, nested_model) in nested {
        models.insert(name, construct_model_stage4(nested_model)?);
    }
    model.borrow_mut().models = models;

    Ok(Rc::clone(&model))
}

fn construct_model_stage5(
    model: Rc<RefCell<ModelNode>>,
) -> Result<Rc<RefCell<ModelNode>>, Diagnostic> {
    // Разрешаем функции на уровне текущей модели
    let functions = std::mem::take(&mut model.borrow_mut().functions);
    model.borrow_mut().functions = resolve_functions(functions, model.clone())?;

    // Рекурсивно обрабатываем вложенные модели с их собственным контекстом
    let nested: Vec<(String, Rc<RefCell<ModelNode>>)> = model
        .borrow()
        .models
        .iter()
        .map(|(k, v)| (k.clone(), Rc::clone(v)))
        .collect();
    let mut models = HashMap::new();
    for (name, nested_model) in nested {
        models.insert(name, construct_model_stage5(nested_model)?);
    }
    model.borrow_mut().models = models;

    Ok(Rc::clone(&model))
}

fn construct_model_stage6(
    model: Rc<RefCell<ModelNode>>,
) -> Result<Rc<RefCell<ModelNode>>, Diagnostic> {
    let states = model.borrow().states.clone();

    let mut prepared_states = HashMap::new();
    for (name, state) in states.iter() {
        prepared_states.insert(name.clone(), resolve_state_references(state)?);
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
        models.insert(name, construct_model_stage6(Rc::clone(&nested_model))?);
    }
    model.borrow_mut().models = models;

    Ok(Rc::clone(&model))
}

fn resolve_functions(
    functions: HashMap<String, FunctionDefinitionNode>,
    model: Rc<RefCell<ModelNode>>,
) -> Result<HashMap<String, FunctionDefinitionNode>, Diagnostic> {
    let mut resolved_functions = HashMap::with_capacity(functions.len());

    for (name, function) in functions {
        resolved_functions.insert(name, construct_function(function, model.clone())?);
    }
    Ok(resolved_functions)
}

/// Разрешает именованные блоки кода внутри одного состояния.
///
/// Ошибки разрешения подавляются — оператор сохраняется как `Unresolved`.
fn resolve_state_named_blocks(
    state: StateNode,
    model: Rc<RefCell<ModelNode>>,
) -> Result<StateNode, Diagnostic> {
    match state {
        StateNode::Simple {
            upper,
            loc,
            name,
            references,
            named_blocks,
            kind,
            formulas,
        } => Ok(StateNode::Simple {
            upper: upper.clone(),
            loc,
            name,
            references,
            kind,
            formulas,
            named_blocks: resolve_named_blocks(named_blocks, model)?,
        }),
        StateNode::Implement {
            upper,
            loc,
            name,
            references,
            implements,
            next,
            named_blocks,
            kind,
            formulas,
        } => Ok(StateNode::Implement {
            upper: upper.clone(),
            loc,
            name,
            references,
            implements,
            next,
            kind,
            formulas,
            named_blocks: resolve_named_blocks(named_blocks, model)?,
        }),
        other => Ok(other),
    }
}

/// Внутренняя реализация построения семантического дерева.
///
/// Принимает `import_stack` — стек путей файлов, чьи импорты сейчас обрабатываются.
/// Используется для обнаружения циклических зависимостей между файлами.
fn construct_model_impl(
    model: &Model,
    upper: Option<Rc<RefCell<ModelNode>>>,
    search_paths: &[String],
    import_stack: &mut Vec<String>,
) -> Result<Rc<RefCell<ModelNode>>, Diagnostic> {
    let model = construct_model_stage0(model, upper, search_paths, import_stack)?;
    let model = construct_model_stage1(model)?;
    let model = construct_model_stage2(model)?;
    let model = construct_model_stage3(model)?;
    // Функции (этап 5) должны разрешаться перед именованными блоками (этап 4),
    // чтобы блоки always/enter/exit могли находить уже разрешённые функции через search_func.
    let model = construct_model_stage5(model)?;
    let model = construct_model_stage4(model)?;
    let model = construct_model_stage6(model)?;
    validate_model(model.clone())?;
    Ok(model)
}

/// Строит семантический узел модели из АСД-узла [`Model`].
///
/// Собирает контекст верхнего уровня (вложенные модели), а также
/// словарь состояний с разрешёнными ссылками между ними.
///
/// Обнаруживает циклические зависимости между файлами импорта:
/// при наличии цикла `a.but → b.but → a.but` возвращает [`Diagnostic`]-ошибку
/// с полным описанием цепочки.
///
/// # Ошибки
///
/// Возвращает [`Diagnostic`], если:
/// - у состояния нет имени,
/// - ссылка `ref` указывает на несуществующее состояние,
/// - `next` встречается в одном состоянии дважды,
/// - обнаружен циклический импорт.
pub fn construct_model(
    model: &Model,
    upper: Option<Rc<RefCell<ModelNode>>>,
    search_paths: &[String],
) -> Result<Rc<RefCell<ModelNode>>, Diagnostic> {
    // Стек путей файлов, чьи импорты сейчас обрабатываются.
    // Пустой на входе: текущая (корневая) единица компиляции не имеет пути.
    let mut import_stack: Vec<String> = Vec::new();
    let model = construct_model_stage0(model, upper, search_paths, &mut import_stack)?;
    let model = construct_model_stage1(model)?;
    let model = construct_model_stage2(model)?;
    let model = construct_model_stage3(model)?;
    // Функции (этап 5) должны разрешаться перед именованными блоками (этап 4),
    // чтобы блоки always/enter/exit могли находить уже разрешённые функции через search_func.
    let model = construct_model_stage5(model)?;
    let model = construct_model_stage4(model)?;
    let model = construct_model_stage6(model)?;
    validate_model(model.clone())?;
    Ok(model)
}

/// Строит семантическое дерево модели и привязывает `///`-комментарии.
///
/// Расширенный вариант [`construct_model`]: после построения дерева заполняет
/// поля [`ModelNode::doc`](crate::semantic::ModelNode::doc) и
/// [`ModelNode::docs`](crate::semantic::ModelNode::docs) на основе `///`-комментариев
/// из исходного текста.
///
/// # Параметры
///
/// - `model` — корневой узел АСД, результат [`parse`](parse).
/// - `upper` — родительская модель (`None` для корня).
/// - `search_paths` — пути поиска для файлов импорта.
/// - `comments` — комментарии из [`parse`](parse) (второй элемент кортежа).
///
/// # Алгоритм привязки
///
/// Для каждого именованного объявления (состояния, переменной, функции и т.д.)
/// ищутся `///`-комментарии, ближайшим следующим элементом которых является
/// данное объявление. Подробнее — в [`crate::semantic::docs`].
///
/// # Примеры
///
/// ```
/// use grammar::{parse, semantic::tree::construct_model_with_docs};
///
/// let src = "/// Документация состояния.\nstart S;";
/// let (ast, comments) = parse(src, 0).unwrap();
/// let root = construct_model_with_docs(&ast, None, &[], &comments).unwrap();
/// assert_eq!(root.borrow().element_doc("S"), ["Документация состояния."]);
/// ```
pub fn construct_model_with_docs(
    model: &Model,
    upper: Option<Rc<RefCell<ModelNode>>>,
    search_paths: &[String],
    comments: &[ast::Comment],
) -> Result<Rc<RefCell<ModelNode>>, Diagnostic> {
    // Строим семантическое дерево (без документации)
    let root = construct_model(model, upper, search_paths)?;
    // Привязываем doc-комментарии к узлам дерева
    crate::semantic::docs::attach_docs(&root, model, comments);
    Ok(root)
}

/// Проверяет условия переходов в семантическом дереве и возвращает
/// предупреждения о неявном приведении числового типа к булевому.
///
/// Функция обходит все состояния модели (рекурсивно) и проверяет, содержат
/// ли условия переходов (`ref`/`next`) выражения числового типа (например,
/// переменная типа `[bit;8]`, числовой литерал, арифметика), используемые
/// как булевые без явного сравнения.
///
/// # Примеры
///
/// ```rust,ignore
/// // BuT-код с числовым условием → предупреждение
/// let src = "var timer: [bit;8] = 0; start S { ref T: timer; } state T;";
/// let (ast, _) = parse(src, 0)?;
/// let root = construct_model(&ast, None, &[])?;
/// let warnings = implicit_bool_warnings(&root);
/// assert!(!warnings.is_empty());
///
/// // BuT-код с явным сравнением → без предупреждений
/// let src = "var timer: [bit;8] = 0; start S { ref T: timer != 0; } state T;";
/// let (ast, _) = parse(src, 0)?;
/// let root = construct_model(&ast, None, &[])?;
/// let warnings = implicit_bool_warnings(&root);
/// assert!(warnings.is_empty());
/// ```
pub fn implicit_bool_warnings(model: &Rc<RefCell<ModelNode>>) -> Vec<Diagnostic> {
    check_implicit_bool_conditions(model)
}

/// Проверяет полноту и достижимость переходов в семантическом дереве.
///
/// Возвращает предупреждения и ошибки Ce5:
/// - отсутствие терминальных состояний в модели;
/// - состояния без пути к терминальному;
/// - совместное использование `ref` и `next` в одном состоянии.
///
/// Функция обходит всё дерево моделей рекурсивно.
///
/// # Примеры
///
/// ```rust,ignore
/// use grammar::{parse, semantic::tree::{construct_model, transition_completeness_warnings}};
///
/// let src = "start A { ref B: true; } state B { ref A: true; }";
/// let (ast, _) = parse(src, 0).unwrap();
/// let root = construct_model(&ast, None, &[]).unwrap();
/// let warnings = transition_completeness_warnings(&root);
/// // Предупреждение: нет терминальных состояний
/// assert!(!warnings.is_empty());
/// ```
pub fn transition_completeness_warnings(model: &Rc<RefCell<ModelNode>>) -> Vec<Diagnostic> {
    check_transition_completeness(Rc::clone(model))
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
pub fn construct_states(
    model: &Model,
    upper: Rc<RefCell<ModelNode>>,
) -> Result<HashMap<String, StateNode>, Diagnostic> {
    // Первый проход: создаём узлы с незаполненными ссылками (заглушки Unresolved).
    let mut states: HashMap<String, Box<StateNode>> = HashMap::new();
    for element in model.elements.iter() {
        if let ModelElement::State(def) = element {
            let name = def
                .clone()
                .name
                .ok_or_else(|| {
                    Diagnostic::error(def.loc, "Имя состояния не задано".to_string())
                        .with_code("SE-020")
                })?
                .name;
            let implements = def.implements.clone();
            let kind = def.kind.clone();
            let mut references = Vec::new();
            let mut next: Option<String> = None;
            let mut state_formula_conditions: Vec<ast::Condition> = Vec::new();
            for element in def.elements.iter() {
                if let StateElement::Reference(_, id, cond) = element {
                    let cond = if let Some(cond) = cond {
                        ConditionNode::Unresolved(cond.clone())
                    } else {
                        ConditionNode::None
                    };
                    references.push(ReferenceNode {
                        location: id.loc,
                        name: id.name.clone(),
                        cond,
                        object: Box::new(StateNode::Unresolved),
                    });
                } else if let StateElement::Next(id) = element {
                    if next.is_some() {
                        return Err(Diagnostic::error(
                            id.loc,
                            format!("Состояние '{}' уже содержит оператор next", &id.name),
                        )
                        .with_code("SE-012"));
                    }
                    next = Some(id.name.clone());
                } else if let StateElement::InlineFormula(inline) = element {
                    match &**inline {
                        ast::InlineFormulaDefine::Guard { conditions, .. } => {
                            for cond in conditions {
                                state_formula_conditions.push(cond.clone());
                            }
                        }
                        ast::InlineFormulaDefine::Ltl { .. } => {
                            // TODO: Реализовать поддержку LTL в состояниях
                        }
                    }
                }
            }
            let state_formulas: Vec<Formula> = state_formula_conditions
                .iter()
                .filter_map(|c| resolve_condition(c, Rc::clone(&upper)).ok())
                .map(|cn| formula::condition_to_formula(&cn))
                .collect();
            let kind = match kind {
                None => {
                    if references.is_empty() {
                        StateNodeKind::End
                    } else {
                        StateNodeKind::Simple
                    }
                }
                Some(kind) => match kind {
                    StateKind::Start => StateNodeKind::Start,
                    StateKind::End => StateNodeKind::End,
                    StateKind::Next => {
                        return Err(Diagnostic::error(
                            def.loc,
                            "Состояние с типом next не поддерживается в качестве определения"
                                .to_string(),
                        )
                        .with_code("SE-021"));
                    }
                },
            };
            // Определяем вид узла: Implement (есть `= Выражение`) или Simple.
            let state_loc = def.loc;
            let state = if let Some(expr) = implements {
                let next = next.map(|n| ReferenceNode {
                    location: state_loc,
                    name: n,
                    cond: ConditionNode::None,
                    object: Box::new(StateNode::Unresolved),
                });
                StateNode::Implement {
                    upper: Some(Rc::downgrade(&upper)),
                    loc: state_loc,
                    named_blocks: construct_named_blocks(def, Some(Rc::downgrade(&upper)))?,
                    name: name.clone(),
                    references,
                    implements: Extend::Unresolved(expr),
                    next,
                    kind,
                    formulas: state_formulas,
                }
            } else {
                StateNode::Simple {
                    upper: Some(Rc::downgrade(&upper)),
                    loc: state_loc,
                    named_blocks: construct_named_blocks(def, Some(Rc::downgrade(&upper)))?,
                    name: name.clone(),
                    references,
                    kind,
                    formulas: state_formulas,
                }
            };
            states.insert(name, Box::new(state));
        }
    }

    // Второй проход: заменяем Unresolved-заглушки реальными узлами.
    let mut new_states: HashMap<String, StateNode> = HashMap::new();
    for (_, state) in states.iter() {
        match *state.clone() {
            StateNode::Simple {
                upper,
                loc,
                name,
                references,
                named_blocks,
                kind,
                formulas,
            } => {
                let resolved = resolve_references(references, &states)?;
                new_states.insert(
                    name.clone(),
                    StateNode::Simple {
                        upper: upper.clone(),
                        loc,
                        named_blocks,
                        name,
                        references: resolved,
                        kind,
                        formulas,
                    },
                );
            }
            StateNode::Implement {
                upper,
                loc,
                named_blocks,
                name,
                references,
                implements,
                next,
                kind,
                formulas,
            } => {
                let resolved = resolve_references(references, &states)?;
                // Разрешаем next-ссылку отдельно (это одиночный Reference, не список).
                let next = next
                    .map(|r| -> Result<ReferenceNode<StateNode>, Diagnostic> {
                        if let StateNode::Unresolved = *r.object {
                            let target = states.get(&r.name).ok_or_else(|| {
                                Diagnostic::error(
                                    r.location,
                                    format!("Ссылка '{}' не найдена", &r.name),
                                )
                                .with_code("SE-002")
                            })?;
                            Ok(ReferenceNode {
                                location: r.location,
                                name: r.name,
                                cond: r.cond,
                                object: target.clone(),
                            })
                        } else {
                            Ok(r)
                        }
                    })
                    .transpose()?;
                new_states.insert(
                    name.clone(),
                    StateNode::Implement {
                        upper: upper.clone(),
                        loc,
                        named_blocks,
                        name,
                        references: resolved,
                        implements,
                        next,
                        kind,
                        formulas,
                    },
                );
            }
            _ => {} // StateNode::Unresolved пропускаем
        }
    }
    Ok(new_states)
}

/// Разрешает список `ref`-ссылок, заменяя [`StateNode::Unresolved`]-заглушки
/// реальными узлами из таблицы первого прохода `states`.
///
/// # Ошибки
///
/// Возвращает [`Diagnostic`], если ссылка указывает на несуществующее состояние.
fn resolve_references(
    references: Vec<ReferenceNode<StateNode>>,
    states: &HashMap<String, Box<StateNode>>,
) -> Result<Vec<ReferenceNode<StateNode>>, Diagnostic> {
    references
        .into_iter()
        .map(|r| {
            if let StateNode::Unresolved = *r.object {
                let target = states.get(&r.name).ok_or_else(|| {
                    Diagnostic::error(r.location, format!("Ссылка '{}' не найдена", &r.name))
                        .with_code("SE-002")
                })?;
                Ok(ReferenceNode {
                    location: r.location,
                    name: r.name,
                    cond: r.cond,
                    object: target.clone(),
                })
            } else {
                Ok(r)
            }
        })
        .collect()
}

/// Извлекает именованные блоки кода из определения состояния.
///
/// Обходит `StateElement::NamedBlockCode` в `state.elements` и создаёт
/// `NamedBlockNode` с `Statement::Unresolved` для каждого блока.
/// Разрешение операторов происходит позднее в [`construct_model_stage4`].
///
/// Если несколько блоков имеют одинаковое имя (например, два `always`),
/// они все сохраняются в списке и могут быть получены через `get_named_blocks`.
fn construct_named_blocks(
    state: &StateDefine,
    upper: Option<Weak<RefCell<ModelNode>>>,
) -> Result<Vec<NamedCodeBlockDefinitionNode>, Diagnostic> {
    let mut named_blocks = Vec::new();
    for element in state.elements.iter() {
        if let StateElement::NamedBlockCode(def) = element {
            let name = def
                .name
                .as_ref()
                .ok_or_else(|| {
                    Diagnostic::error(
                        def.loc,
                        "Именованный блок кода при определении должен иметь имя".to_string(),
                    )
                    .with_code("SE-018")
                })?
                .name
                .clone();
            let block = match name.as_str() {
                "enter" => NamedCodeBlockDefinitionNode::Enter {
                    upper: upper.clone(),
                    body: StatementNode::Unresolved(def.statement.clone()),
                },
                "exit" => NamedCodeBlockDefinitionNode::Exit {
                    upper: upper.clone(),
                    body: StatementNode::Unresolved(def.statement.clone()),
                },
                "always" => NamedCodeBlockDefinitionNode::Always {
                    upper: upper.clone(),
                    body: StatementNode::Unresolved(def.statement.clone()),
                },
                name => NamedCodeBlockDefinitionNode::Unknown {
                    upper: upper.clone(),
                    name: name.to_string(),
                    body: StatementNode::Unresolved(def.statement.clone()),
                },
            };
            named_blocks.push(block);
        }
    }
    Ok(named_blocks)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::parse;

    // ─── вспомогательная функция ────────────────────────────────────────────

    /// Разбирает BuT-программу и строит семантическую модель.
    fn build(src: &str) -> Result<ModelNode, Diagnostic> {
        let (ast, _) = parse(src, 0).expect("parse error");
        construct_model(&ast, None, &[]).map(|model| model.take())
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
            let node = construct_model(m, None, &[]).unwrap();
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
        let node = construct_model(&ast, None, &[]).unwrap();
        // Inner — вложен в Outer, который в корневом контексте
        assert!(!node.take().has_states()); // корень не содержит состояний напрямую
    }
}
