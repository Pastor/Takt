//! Генерация C-выражений, операторов, блоков кода и разрешение переменных.
//!
//! Содержит всю логику генерации C-выражений из семантических узлов:
//! [`generate_expr`], [`generate_code_block`], [`generate_stmt_expression`],
//! а также вспомогательные функции разрешения имён переменных и функций.

use super::{PortClass, get_c_type, get_typed_variable};
use crate::diagnostics::{Diagnostic, Location};
use crate::generator::c::c_map::CMap;
use crate::generator::c::{
    FUNCTION_PORT_READ_BIT, FUNCTION_PORT_READ_FLOAT, FUNCTION_PORT_READ_NUMERIC,
    FUNCTION_PORT_WRITE_BIT, FUNCTION_PORT_WRITE_FLOAT, FUNCTION_PORT_WRITE_NUMERIC,
};
use crate::generator::indent::Printer;
use crate::parser::ast::{Condition, Member};
use crate::semantic::extend::Extend;
use crate::semantic::minimap::{Element, Name};
use crate::semantic::naming::normalize_lowercase_snakecase;
use crate::semantic::type_node::TypeNode;
use crate::semantic::{
    ConditionDefinitionNode, ConditionNode, ExpressionNode, Formula, FunctionDefinitionNode,
    ModelNode, StateNode, StatementNode, VariableNode,
};
use std::cell::RefCell;
use std::rc::Rc;

/// Возвращает C-имя функции (camelcase_name или внешнее имя).
pub(super) fn get_function_name(fun: &FunctionDefinitionNode) -> String {
    match fun {
        FunctionDefinitionNode::Local { upper, name, .. } => {
            let model_name = Name::from(upper.clone().unwrap().upgrade().unwrap());
            format!("{}_{}", model_name.unique_camelcase(), name)
        }
        FunctionDefinitionNode::External { name, .. } => name.clone(),
        FunctionDefinitionNode::Builtin(name, ..) => name.to_string(),
        _ => {
            unreachable!("Unresolved function definition");
        }
    }
}

/// Возвращает имя поля в родительской C-структуре для вложенной модели.
///
/// Ищет в родительской модели состояние с `implements = Extend::Model(эта_модель)`
/// и возвращает имя этого состояния в snake_case (именно оно используется как поле
/// в сгенерированной C-структуре). Если не найдено — возвращает `None`.
fn find_in_extend(
    extend: &Extend,
    target: &Rc<RefCell<ModelNode>>,
    state_name: &str,
) -> Option<String> {
    match extend {
        Extend::Model(m) => {
            if Rc::ptr_eq(m, target) {
                Some(normalize_lowercase_snakecase(state_name.to_string()))
            } else {
                None
            }
        }
        Extend::Concatenation(items) => {
            for (idx, item) in items.iter().enumerate() {
                if let Some(path) = find_in_concat(item, target, state_name, idx) {
                    return Some(path);
                }
            }
            None
        }
        Extend::Parallel(items) => {
            for (idx, item) in items.iter().enumerate() {
                if let Some(path) = find_in_parallel(item, target, state_name, idx) {
                    return Some(path);
                }
            }
            None
        }
        Extend::Parentless(inner) => find_in_extend(inner, target, state_name),
        _ => None,
    }
}

fn find_in_concat(
    extend: &Extend,
    target: &Rc<RefCell<ModelNode>>,
    state_name: &str,
    idx: usize,
) -> Option<String> {
    match extend {
        Extend::Model(m) => {
            if Rc::ptr_eq(m, target) {
                let model_name = m.borrow().name.clone().unwrap_or_default();
                Some(format!(
                    "{}_{}{}",
                    normalize_lowercase_snakecase(state_name.to_string()),
                    normalize_lowercase_snakecase(model_name),
                    idx
                ))
            } else {
                None
            }
        }
        Extend::Parallel(items) => {
            let prefix = format!(
                "{}_parallel{}",
                normalize_lowercase_snakecase(state_name.to_string()),
                idx
            );
            for (inner_idx, item) in items.iter().enumerate() {
                if let Some(path) = find_in_parallel(item, target, &prefix, inner_idx) {
                    return Some(path);
                }
            }
            None
        }
        Extend::Concatenation(items) => {
            for (inner_idx, item) in items.iter().enumerate() {
                if let Some(path) = find_in_concat(item, target, state_name, inner_idx) {
                    return Some(path);
                }
            }
            None
        }
        Extend::Parentless(inner) => find_in_concat(inner, target, state_name, idx),
        _ => None,
    }
}

fn find_in_parallel(
    extend: &Extend,
    target: &Rc<RefCell<ModelNode>>,
    prefix: &str,
    idx: usize,
) -> Option<String> {
    match extend {
        Extend::Model(m) => {
            if Rc::ptr_eq(m, target) {
                let model_name = m.borrow().name.clone().unwrap_or_default();
                Some(format!(
                    "{}.{}{}",
                    normalize_lowercase_snakecase(prefix.to_string()),
                    normalize_lowercase_snakecase(model_name),
                    idx
                ))
            } else {
                None
            }
        }
        Extend::Parallel(items) => {
            let nested_prefix = format!(
                "{}.parallel{}",
                normalize_lowercase_snakecase(prefix.to_string()),
                idx
            );
            for (inner_idx, item) in items.iter().enumerate() {
                if let Some(path) = find_in_parallel(item, target, &nested_prefix, inner_idx) {
                    return Some(path);
                }
            }
            None
        }
        Extend::Concatenation(items) => {
            for (inner_idx, item) in items.iter().enumerate() {
                if let Some(path) = find_in_parallel(item, target, prefix, inner_idx) {
                    return Some(path);
                }
            }
            None
        }
        Extend::Parentless(inner) => find_in_parallel(inner, target, prefix, idx),
        _ => None,
    }
}

/// Возвращает имя поля в родительской C-структуре для вложенной модели.
///
/// Ищет в родительской модели состояние, в реализации которого используется
/// эта модель, и возвращает путь к полю в сгенерированной C-структуре.
pub(super) fn field_name_in_parent(model_rc: &Rc<RefCell<ModelNode>>) -> Option<String> {
    let parent_rc = model_rc.borrow().upper.as_ref()?.upgrade()?;
    let parent = parent_rc.borrow();
    for (state_name, state_node) in &parent.states {
        if let StateNode::Implement { implements, .. } = state_node
            && let Some(path) = find_in_extend(implements, model_rc, state_name)
        {
            return Some(path);
        }
    }
    None
}

/// Преобразует [`VariableNode`] в C-выражение для чтения.
///
/// - `Simple` с `loc == Implicit` — локальная переменная (stack), доступ по имени.
/// - `Simple` — переменная модели, доступ `main->field` или `main->model.field`.
/// - `Const` — `CONST_{MODEL}_{NAME}`.
/// - `Port` — вызов `(*main->read_bit)(PORT_..., bit, main->userdata)`.
pub(super) fn resolve_variable_c_expr(
    var: &VariableNode,
    params: &[(String, TypeNode)],
    map: &CMap,
    owner: &Element,
    has_model: bool,
) -> Result<String, Diagnostic> {
    match var {
        VariableNode::Simple {
            name, upper, loc, ..
        } => {
            // Локальная переменная (объявлена через register_local_var) имеет loc == Implicit
            if matches!(loc, Location::Implicit) {
                return Ok(normalize_lowercase_snakecase(name.clone()));
            }
            // Параметр функции — тоже доступ по имени
            if params.iter().any(|(p, _)| p == name) {
                return Ok(normalize_lowercase_snakecase(name.clone()));
            }
            // Переменная уровня модели → main->field
            if let Some(model_rc) = upper.as_ref().and_then(|w| w.upgrade()) {
                // Извлекаем имя модели до вызова field_name_in_parent, чтобы избежать
                // двойного заимствования model_rc.
                let model_name_opt = model_rc.borrow().name.clone();
                if model_name_opt.is_some() {
                    // Вложенная модель: поле структуры называется по имени состояния-контейнера,
                    // а не по имени самой модели. Ищем это состояние в родителе.
                    let field = field_name_in_parent(&model_rc).unwrap_or_else(|| {
                        normalize_lowercase_snakecase(model_name_opt.unwrap_or_default())
                    });
                    Ok(format!(
                        "model->{}.{}",
                        field,
                        normalize_lowercase_snakecase(name.clone())
                    ))
                } else {
                    // Корневая модель — поле напрямую
                    Ok(format!(
                        "model->{}",
                        normalize_lowercase_snakecase(name.clone())
                    ))
                }
            } else {
                Ok(format!(
                    "model->{}",
                    normalize_lowercase_snakecase(name.clone())
                ))
            }
        }
        VariableNode::Const { name, upper, .. } => {
            // CONST_{MODEL_UPPERCASE}_{CONST_UPPERCASE}
            if let Some(model_rc) = upper.as_ref().and_then(|w| w.upgrade()) {
                let model_name = Name::from(model_rc);
                Ok(format!(
                    "CONST_{}_{}",
                    model_name.unique_uppercase_snakecase(),
                    normalize_lowercase_snakecase(name.clone()).to_uppercase()
                ))
            } else {
                Ok(format!(
                    "CONST_{}",
                    normalize_lowercase_snakecase(name.clone()).to_uppercase()
                ))
            }
        }
        VariableNode::Port {
            name, ty, upper, ..
        } => {
            let model_name = if let Some(model_rc) = upper.as_ref().and_then(|w| w.upgrade()) {
                Name::from(model_rc)
            } else {
                return Err("Неразрешённый owner порта".into());
            };
            let cls = PortClass::from_type(ty);
            let variant = format!(
                "{}_{}",
                model_name.unique_uppercase_snakecase(),
                normalize_lowercase_snakecase(name.clone()).to_uppercase()
            );
            // В локальных функциях (has_model=false) первый параметр — `const Root *model`.
            // В tick/init корневой модели — тоже `model`. В tick/init подмодели — `main`.
            let ptr = if has_model && !owner.name().eq(&map.root_name()) {
                "main"
            } else {
                "model"
            };
            match cls {
                PortClass::Rational => Ok(format!(
                    "(*{ptr}->{read_float})({variant}, {ptr}->userdata)",
                    read_float = FUNCTION_PORT_READ_FLOAT
                )),
                PortClass::Numeric => Ok(format!(
                    "(*{ptr}->{read_numeric})({variant}, {ptr}->userdata)",
                    read_numeric = FUNCTION_PORT_READ_NUMERIC
                )),
                PortClass::Bit => Ok(format!(
                    "(*{ptr}->{read_bit})({variant}, {ptr}->userdata)",
                    read_bit = FUNCTION_PORT_READ_BIT
                )),
            }
        }
        VariableNode::Unresolved => Err("Неразрешённая переменная".into()),
    }
}

/// Разрешает путь доступа к [`VariableNode::Simple`] с учётом контекста генерации.
///
/// Сигнатуры C-функций:
///   - Tick/init (`has_model = true`):   `void SubModel_tick(SubModel *model, Root *main)`
///   - Локальная функция (`has_model = false`): `static T Model_fn(const Root *model, ...)`
///
/// Правила доступа:
/// - Переменная той же модели, что `owner`:
///   - `has_model = true`  → `model->var`
///   - `has_model = false` → `model->field.var` (через поле дочерней модели в Root)
/// - Переменная корневой модели, `owner` — вложенная:
///   - `has_model = true`  → `main->var`
///   - `has_model = false` → `model->var` (первый параметр — сама Root)
/// - Иначе → делегируем в [`resolve_variable_c_expr`]
pub(super) fn resolve_simple_var_in_context(
    var_name: &str,
    upper: &Option<std::rc::Weak<std::cell::RefCell<ModelNode>>>,
    params: &[(String, TypeNode)],
    owner: &Element,
    map: &CMap,
    has_model: bool,
) -> Option<String> {
    // Параметры функции — доступ по имени, обрабатывается в resolve_variable_c_expr
    if params.iter().any(|(p, _)| p == var_name) {
        return None;
    }
    let var_model_rc = upper.as_ref().and_then(|w| w.upgrade())?;
    let var_model_name = Name::from(var_model_rc.clone());
    let is_same_model = var_model_name.eq(&owner.name());
    let is_root_var = var_model_rc.borrow().upper.is_none();
    let is_root_owner = owner.name().eq(&map.root_name());
    let snake = normalize_lowercase_snakecase(var_name.to_string());
    if is_same_model {
        if has_model {
            // Переменная принадлежит текущей генерируемой модели, `model` доступен
            Some(format!("model->{}", snake))
        } else if is_root_var {
            // Локальная функция корневой модели: `const Root *model` → прямой доступ
            Some(format!("model->{}", snake))
        } else {
            // Локальная функция вложенной модели: первый параметр — `const Root *model`,
            // доступ через поле-контейнер дочерней модели.
            let field = field_name_in_parent(&var_model_rc)?;
            Some(format!("model->{}.{}", field, snake))
        }
    } else if is_root_var && !is_root_owner {
        // Переменная корневой модели, accessed из вложенной:
        // - tick/init: `main->var` (Root передаётся как `main`)
        // - локальная функция: `model->var` (первый параметр — сама Root)
        if has_model {
            Some(format!("main->{}", snake))
        } else {
            Some(format!("model->{}", snake))
        }
    } else {
        // Родительская модель обращается к переменной дочерней — стандартный путь
        None
    }
}

/// Генерирует имя макроса условия вида `COND_{MODEL}_{NAME}`.
pub(super) fn condition_macro_name(cond: &ConditionDefinitionNode) -> String {
    if let Some(model_rc) = cond.upper.as_ref().and_then(|w| w.upgrade()) {
        let model_name = Name::from(model_rc);
        format!(
            "COND_{}_{}",
            model_name.unique_uppercase_snakecase(),
            normalize_lowercase_snakecase(cond.name.clone()).to_uppercase()
        )
    } else {
        format!(
            "COND_{}",
            normalize_lowercase_snakecase(cond.name.clone()).to_uppercase()
        )
    }
}

/// Возвращает C-приоритет выражения (больше = сильнее связывает).
///
/// Используется для минимизации лишних скобок: обёртка добавляется только
/// если приоритет дочернего узла ниже требуемого минимума от родителя.
pub(super) fn expr_precedence(expr: &ExpressionNode) -> u8 {
    match expr {
        // Присваивание — наименьший приоритет
        ExpressionNode::Assign(..) => 1,
        // Тернарный оператор
        ExpressionNode::ConditionalOperator(..) => 2,
        // Логическое ИЛИ
        ExpressionNode::Or(..) => 3,
        // Логическое И
        ExpressionNode::And(..) => 4,
        // Побитовое ИЛИ
        ExpressionNode::BitwiseOr(..) => 5,
        // Побитовое исключающее ИЛИ
        ExpressionNode::BitwiseXor(..) => 6,
        // Побитовое И
        ExpressionNode::BitwiseAnd(..) => 7,
        // Равенство / неравенство
        ExpressionNode::Equal(..) | ExpressionNode::NotEqual(..) => 8,
        // Сравнение
        ExpressionNode::Less(..)
        | ExpressionNode::More(..)
        | ExpressionNode::LessEqual(..)
        | ExpressionNode::MoreEqual(..) => 9,
        // Битовые сдвиги
        ExpressionNode::ShiftLeft(..) | ExpressionNode::ShiftRight(..) => 10,
        // Аддитивные операторы
        ExpressionNode::Add(..) | ExpressionNode::Subtract(..) => 11,
        // Мультипликативные операторы
        ExpressionNode::Multiply(..) | ExpressionNode::Divide(..) | ExpressionNode::Modulo(..) => {
            12
        }
        // Унарные операторы и приведение типов
        ExpressionNode::Not(..)
        | ExpressionNode::BitwiseNot(..)
        | ExpressionNode::UnaryPlus(..)
        | ExpressionNode::Negate(..)
        | ExpressionNode::Cast(..) => 13,
        // Атомы: литералы, переменные, вызовы функций, скобки и т.п.
        _ => 15,
    }
}

/// Преобразует [`ConditionNode`] в строку C-выражения.
///
/// Используется при генерации условий переходов для простых состояний.
/// Возвращает пустую строку для безусловных переходов (`ConditionNode::None`).
pub(super) fn generate_condition_expr(
    cond: &ConditionNode,
    map: &CMap,
    owner: &Element,
) -> Result<String, Diagnostic> {
    match cond {
        ConditionNode::None | ConditionNode::Unresolved(_) => Ok(String::new()),
        ConditionNode::Bool(b) => Ok(if *b { "true" } else { "false" }.to_string()),
        ConditionNode::Number(n) => Ok(n.to_string()),
        ConditionNode::Rational(s, neg) => {
            if *neg {
                Ok(format!("-{}", s))
            } else {
                Ok(s.clone())
            }
        }
        ConditionNode::String(parts) => Ok(format!("\"{}\"", parts.join(""))),
        ConditionNode::Not(inner) => Ok(format!(
            "!({})",
            generate_condition_expr(inner, map, owner)?
        )),
        ConditionNode::Parenthesis(inner) => {
            Ok(format!("({})", generate_condition_expr(inner, map, owner)?))
        }
        ConditionNode::Add(l, r) => Ok(format!(
            "{} + {}",
            generate_condition_expr(l, map, owner)?,
            generate_condition_expr(r, map, owner)?
        )),
        ConditionNode::Subtract(l, r) => Ok(format!(
            "{} - {}",
            generate_condition_expr(l, map, owner)?,
            generate_condition_expr(r, map, owner)?
        )),
        ConditionNode::And(l, r) => Ok(format!(
            "{} && {}",
            generate_condition_expr(l, map, owner)?,
            generate_condition_expr(r, map, owner)?
        )),
        ConditionNode::Or(l, r) => Ok(format!(
            "{} || {}",
            generate_condition_expr(l, map, owner)?,
            generate_condition_expr(r, map, owner)?
        )),
        ConditionNode::Less(l, r) => Ok(format!(
            "{} < {}",
            generate_condition_expr(l, map, owner)?,
            generate_condition_expr(r, map, owner)?
        )),
        ConditionNode::More(l, r) => Ok(format!(
            "{} > {}",
            generate_condition_expr(l, map, owner)?,
            generate_condition_expr(r, map, owner)?
        )),
        ConditionNode::LessEqual(l, r) => Ok(format!(
            "{} <= {}",
            generate_condition_expr(l, map, owner)?,
            generate_condition_expr(r, map, owner)?
        )),
        ConditionNode::MoreEqual(l, r) => Ok(format!(
            "{} >= {}",
            generate_condition_expr(l, map, owner)?,
            generate_condition_expr(r, map, owner)?
        )),
        ConditionNode::Equal(l, r) => {
            if let ConditionNode::Model(model) = l.as_ref() {
                let eq_name = if let ConditionNode::Variable(v, ..) = r.as_ref() {
                    let x = v.borrow();
                    x.name().to_string()
                } else if let ConditionNode::Unresolved(cond) = r.as_ref()
                    && let Condition::Variable(id) = cond
                {
                    id.name.clone()
                } else {
                    return Err(Diagnostic::error(
                        Location::Codegen,
                        format!("Выражение {:?} не разыменовано", r.as_ref()),
                    )
                    .with_code("CC-013"));
                };

                let model_name = Name::from(model.clone());
                let using_models = map.using_models();
                let element = using_models
                    .iter()
                    .find(|m| m.name().eq(&model_name))
                    .ok_or_else(|| {
                        Diagnostic::error(
                            Location::Codegen,
                            format!("Модель {} не найдена", &model_name),
                        )
                        .with_code("CC-012")
                    })?;

                let Element::Model { states, .. } = element else {
                    return Err(Diagnostic::error(
                        Location::Codegen,
                        format!("Элемент {} не является моделью", &model_name),
                    )
                    .with_code("CC-006"));
                };

                let state = states
                    .iter()
                    .find(|s| s.local() == eq_name)
                    .ok_or_else(|| {
                        Diagnostic::error(
                            Location::Codegen,
                            format!("Состояние {} не найдено", eq_name),
                        )
                        .with_code("CC-011")
                    })?;

                let is_same_model = model_name.eq(&owner.name());
                let is_root_model = model.borrow().upper.is_none();
                let is_root_owner = owner.name().eq(&map.root_name());

                let path = if is_same_model {
                    "model->state".to_string()
                } else if is_root_model && !is_root_owner {
                    "main->state".to_string()
                } else {
                    let field = field_name_in_parent(model).unwrap_or_else(|| {
                        normalize_lowercase_snakecase(
                            model.borrow().name.clone().unwrap_or_default(),
                        )
                    });
                    format!("model->{}.state", field)
                };

                let state_const = format!(
                    "{}_{}",
                    model_name.unique_uppercase_snakecase(),
                    normalize_lowercase_snakecase(state.local().to_string()).to_uppercase()
                );

                Ok(format!("{} == {}", path, state_const))
            } else {
                Ok(format!(
                    "{} == {}",
                    generate_condition_expr(l, map, owner)?,
                    generate_condition_expr(r, map, owner)?
                ))
            }
        }
        ConditionNode::NotEqual(l, r) => {
            if let ConditionNode::Model(model) = l.as_ref() {
                let eq_name = if let ConditionNode::Variable(v, ..) = r.as_ref() {
                    let x = v.borrow();
                    x.name().to_string()
                } else if let ConditionNode::Unresolved(cond) = r.as_ref()
                    && let Condition::Variable(id) = cond
                {
                    id.name.clone()
                } else {
                    return Err(Diagnostic::error(
                        Location::Codegen,
                        format!("Выражение {:?} не разыменовано", r.as_ref()),
                    )
                    .with_code("CC-013"));
                };

                let model_name = Name::from(model.clone());
                let using_models = map.using_models();
                let element = using_models
                    .iter()
                    .find(|m| m.name().eq(&model_name))
                    .ok_or_else(|| {
                        Diagnostic::error(
                            Location::Codegen,
                            format!("Модель {} не найдена", &model_name),
                        )
                        .with_code("CC-012")
                    })?;

                let Element::Model { states, .. } = element else {
                    return Err(Diagnostic::error(
                        Location::Codegen,
                        format!("Элемент {} не является моделью", &model_name),
                    )
                    .with_code("CC-006"));
                };

                let state = states
                    .iter()
                    .find(|s| s.local() == eq_name)
                    .ok_or_else(|| {
                        Diagnostic::error(
                            Location::Codegen,
                            format!("Состояние {} не найдено", eq_name),
                        )
                        .with_code("CC-011")
                    })?;

                let is_same_model = model_name.eq(&owner.name());
                let is_root_model = model.borrow().upper.is_none();
                let is_root_owner = owner.name().eq(&map.root_name());

                let path = if is_same_model {
                    "model->state".to_string()
                } else if is_root_model && !is_root_owner {
                    "main->state".to_string()
                } else {
                    let field = field_name_in_parent(model).unwrap_or_else(|| {
                        normalize_lowercase_snakecase(
                            model.borrow().name.clone().unwrap_or_default(),
                        )
                    });
                    format!("model->{}.state", field)
                };

                let state_const = format!(
                    "{}_{}",
                    model_name.unique_uppercase_snakecase(),
                    normalize_lowercase_snakecase(state.local().to_string()).to_uppercase()
                );

                Ok(format!("{} != {}", path, state_const))
            } else {
                Ok(format!(
                    "{} != {}",
                    generate_condition_expr(l, map, owner)?,
                    generate_condition_expr(r, map, owner)?
                ))
            }
        }
        ConditionNode::Variable(var_rc, _) => {
            let var = var_rc.borrow();
            if let VariableNode::Simple { upper, .. } = &*var
                && let Some(s) =
                    resolve_simple_var_in_context(var.name(), upper, &[], owner, map, true)
            {
                return Ok(s);
            }
            resolve_variable_c_expr(&var, &[], map, owner, true)
        }
        ConditionNode::EnumVariant(_, _, value) => Ok(value.to_string()),
        ConditionNode::ArraySubscript(var_rc, idx) => {
            let var = var_rc.borrow();
            if let VariableNode::Simple { upper, .. } = &*var
                && let Some(s) =
                    resolve_simple_var_in_context(var.name(), upper, &[], owner, map, true)
            {
                return Ok(format!("{}[{}]", s, idx));
            }
            let base = resolve_variable_c_expr(&var, &[], map, owner, true)?;
            Ok(format!("{}[{}]", base, idx))
        }
        ConditionNode::BitAccess(inner, member) => {
            match member {
                Member::Identifier(id) => {
                    // Доступ к полю структуры: inner.field
                    let inner_str = generate_condition_expr(inner, map, owner)?;
                    Ok(format!("{}.{}", inner_str, id.name))
                }
                Member::Number(n) => {
                    // Битовый доступ: проверяем, является ли inner портовой переменной
                    if let ConditionNode::Variable(var_rc, _) = inner.as_ref() {
                        let var = var_rc.borrow();
                        if let VariableNode::Port {
                            name, ty, upper, ..
                        } = &*var
                        {
                            let model_name =
                                if let Some(rc) = upper.as_ref().and_then(|w| w.upgrade()) {
                                    Name::from(rc)
                                } else {
                                    return Err(
                                        "Неразрешённый owner порта при BitAccess в условии".into(),
                                    );
                                };
                            let cls = PortClass::from_type(ty);
                            let variant = format!(
                                "{}_{}",
                                model_name.unique_uppercase_snakecase(),
                                normalize_lowercase_snakecase(name.clone()).to_uppercase()
                            );
                            // В условиях всегда has_model=true; ptr зависит от owner
                            let ptr = if owner.name().eq(&map.root_name()) {
                                "model"
                            } else {
                                "main"
                            };
                            return match cls {
                                PortClass::Bit => Ok(format!(
                                    "(*{ptr}->{read_bit})({variant}, {ptr}->userdata)",
                                    read_bit = FUNCTION_PORT_READ_BIT
                                )),
                                PortClass::Numeric => Ok(format!(
                                    "(((*{ptr}->{read_numeric})({variant}, {ptr}->userdata) >> {n}) & 1u)",
                                    read_numeric = FUNCTION_PORT_READ_NUMERIC
                                )),
                                PortClass::Rational => Err(Diagnostic::error(
                                    Location::Codegen,
                                    "BitAccess на float-порт не поддерживается в условии"
                                        .to_string(),
                                )
                                .with_code("CC-001")),
                            };
                        }
                    }
                    // Обычная переменная/выражение: ((inner >> N) & 1u)
                    let inner_str = generate_condition_expr(inner, map, owner)?;
                    Ok(format!("(({} >> {}) & 1u)", inner_str, n))
                }
            }
        }
        ConditionNode::Function(fun_rc, args, _) => {
            let fun = fun_rc.borrow();
            // Пропускаем неразрешённые и пустые функции — они не могут быть сгенерированы
            if !matches!(
                *fun,
                FunctionDefinitionNode::Local { .. }
                    | FunctionDefinitionNode::External { .. }
                    | FunctionDefinitionNode::Builtin { .. }
            ) {
                return Err(Diagnostic::error(
                    Location::Codegen,
                    "Неразрешённая функция в условии перехода".to_string(),
                )
                .with_code("CC-002"));
            }
            let fn_name = get_function_name(&fun);
            let args_strs: Result<Vec<_>, _> = args
                .iter()
                .map(|a| generate_condition_expr(a, map, owner))
                .collect();
            let args_strs = args_strs?;
            // Локальная функция в C принимает main/model как первый аргумент
            if matches!(*fun, FunctionDefinitionNode::Local { .. }) {
                let first_arg = if owner.name().eq(&map.root_name()) {
                    "model"
                } else {
                    "main"
                };
                let mut all_args = vec![first_arg.to_string()];
                all_args.extend(args_strs);
                Ok(format!("{}({})", fn_name, all_args.join(", ")))
            } else {
                Ok(format!("{}({})", fn_name, args_strs.join(", ")))
            }
        }
        ConditionNode::Model(_) | ConditionNode::State(_) => Err(Diagnostic::error(
            Location::Codegen,
            "Ссылки на модели и состояния не поддерживаются в условиях переходов".to_string(),
        )
        .with_code("CC-003")),
    }
}

/// Генерирует список C-аргументов для вызова функции.
fn generate_args(
    map: &CMap,
    owner: &Element,
    params: &[(String, TypeNode)],
    args: &[ExpressionNode],
    has_model: bool,
) -> Result<Vec<String>, Diagnostic> {
    let mut result = Vec::new();
    for arg in args {
        let mut s = String::new();
        let mut tmp = Printer::new(4, &mut s);
        generate_stmt_expression(&mut tmp, map, owner, params.to_vec(), arg, has_model)?;
        result.push(s);
    }
    Ok(result)
}

/// Генерирует C-вызов функции.
///
/// - `Local` → `{ModelCamelCase}_{name}(main, args...)`
/// - `External` → `{name}(args...)`
/// - `Builtin("min"|"max"|"abs"|"clamp")` → раскрывается как тернарное выражение
/// - `Builtin("debug"|"S")` → возвращает ошибку (не транслируется в C)
fn generate_function_call(
    printer: &mut Printer,
    map: &CMap,
    owner: &Element,
    params: Vec<(String, TypeNode)>,
    fun_def: &FunctionDefinitionNode,
    args: &[ExpressionNode],
    has_model: bool,
) -> Result<(), Diagnostic> {
    match fun_def {
        FunctionDefinitionNode::Local { upper, name, .. } => {
            let model_rc =
                upper
                    .as_ref()
                    .and_then(|w| w.upgrade())
                    .ok_or_else(|| -> Diagnostic {
                        "Неразрешённый owner функции".into()
                    })?;
            let model_name = Name::from(model_rc);
            let func_name = format!("{}_{}", model_name.unique_camelcase(), name);
            let arg_strs = generate_args(map, owner, &params, args, has_model)?;
            // В корневой модели (или вне контекста tick/init) первый аргумент — `model`,
            // в подмоделях — `main` (указатель на корневую модель)
            let first_arg = if !has_model || owner.name().eq(&map.root_name()) {
                "model"
            } else {
                "main"
            };
            let mut all_args = vec![first_arg.to_string()];
            all_args.extend(arg_strs);
            printer.print(&format!("{}({})", func_name, all_args.join(", ")));
        }
        FunctionDefinitionNode::External { name, .. } => {
            let arg_strs = generate_args(map, owner, &params, args, has_model)?;
            printer.print(&format!("{}({})", name, arg_strs.join(", ")));
        }
        FunctionDefinitionNode::Builtin(builtin_name, _, _) => match *builtin_name {
            "min" => {
                let arg_strs = generate_args(map, owner, &params, args, has_model)?;
                if arg_strs.len() >= 2 {
                    printer.print(&format!(
                        "((({a}) < ({b})) ? ({a}) : ({b}))",
                        a = arg_strs[0],
                        b = arg_strs[1]
                    ));
                }
            }
            "max" => {
                let arg_strs = generate_args(map, owner, &params, args, has_model)?;
                if arg_strs.len() >= 2 {
                    printer.print(&format!(
                        "((({a}) > ({b})) ? ({a}) : ({b}))",
                        a = arg_strs[0],
                        b = arg_strs[1]
                    ));
                }
            }
            "abs" => {
                let arg_strs = generate_args(map, owner, &params, args, has_model)?;
                if !arg_strs.is_empty() {
                    printer.print(&format!("((({x}) < 0) ? -({x}) : ({x}))", x = arg_strs[0]));
                }
            }
            "clamp" => {
                let arg_strs = generate_args(map, owner, &params, args, has_model)?;
                if arg_strs.len() >= 3 {
                    printer.print(&format!(
                        "((({x}) < ({lo})) ? ({lo}) : ((({x}) > ({hi})) ? ({hi}) : ({x})))",
                        x = arg_strs[0],
                        lo = arg_strs[1],
                        hi = arg_strs[2]
                    ));
                }
            }
            "debug" | "S" => {
                return Err(format!(
                    "Встроенная функция '{}' не поддерживается в C генераторе",
                    builtin_name
                )
                .as_str()
                .into());
            }
            other => {
                return Err(format!("Неизвестная встроенная функция '{}'", other)
                    .as_str()
                    .into());
            }
        },
        _ => {
            return Err("Неразрешённое определение функции".into());
        }
    }
    Ok(())
}

/// Генерирует C-выражение из семантического узла с учётом приоритета операторов.
///
/// Скобки добавляются автоматически только там, где это необходимо для
/// сохранения семантики: если `expr_precedence(expr) < min_prec`.
///
/// Используйте `min_prec = 0` для выражений верхнего уровня.
pub(super) fn generate_expr(
    printer: &mut Printer,
    map: &CMap,
    owner: &Element,
    params: Vec<(String, TypeNode)>,
    expr: &ExpressionNode,
    min_prec: u8,
    has_model: bool,
) -> Result<(), Diagnostic> {
    let my_prec = expr_precedence(expr);
    let wrap = my_prec < min_prec;
    if wrap {
        printer.print("(");
    }
    match expr {
        ExpressionNode::None | ExpressionNode::Unresolved(_) => {
            return Err("Неразрешённое выражение".into());
        }

        // ── Литералы ──────────────────────────────────────────────────────────
        ExpressionNode::Number(n) => {
            printer.print(&n.to_string());
        }
        ExpressionNode::Bool(value) => {
            printer.print(if *value { "true" } else { "false" });
        }
        ExpressionNode::String(v) => {
            printer.print("\"").print(&v.join("")).print("\"");
        }
        ExpressionNode::Rational(s, neg) => {
            if *neg {
                printer.print("-");
            }
            printer.print(s);
        }

        // ── Унарные операторы ──────────────────────────────────────────────────
        // min_prec=14 для операнда: бинарные выражения (prec≤13) будут обёрнуты;
        // также исключает двусмысленные `--x` и `++x` (унарный + унарный).
        ExpressionNode::Not(e) => {
            printer.print("!");
            generate_expr(printer, map, owner, params, e, 14, has_model)?;
        }
        ExpressionNode::BitwiseNot(e) => {
            printer.print("~");
            generate_expr(printer, map, owner, params, e, 14, has_model)?;
        }
        ExpressionNode::UnaryPlus(e) => {
            printer.print("+");
            generate_expr(printer, map, owner, params, e, 14, has_model)?;
        }
        ExpressionNode::Negate(e) => {
            printer.print("-");
            generate_expr(printer, map, owner, params, e, 14, has_model)?;
        }

        // ── Степень → pow() ────────────────────────────────────────────────────
        ExpressionNode::Power(l, r) => {
            printer.print("pow((double)(");
            generate_expr(printer, map, owner, params.clone(), l, 0, has_model)?;
            printer.print("), (double)(");
            generate_expr(printer, map, owner, params, r, 0, has_model)?;
            printer.print("))");
        }

        // ── Бинарные арифметические ────────────────────────────────────────────
        // Левый операнд: допускается тот же приоритет (левоассоциативность).
        // Правый операнд: требует более высокого приоритета (wrap при равном).
        ExpressionNode::Multiply(l, r) => {
            generate_expr(printer, map, owner, params.clone(), l, 12, has_model)?;
            printer.print(" * ");
            generate_expr(printer, map, owner, params, r, 13, has_model)?;
        }
        ExpressionNode::Divide(l, r) => {
            generate_expr(printer, map, owner, params.clone(), l, 12, has_model)?;
            printer.print(" / ");
            generate_expr(printer, map, owner, params, r, 13, has_model)?;
        }
        ExpressionNode::Modulo(l, r) => {
            generate_expr(printer, map, owner, params.clone(), l, 12, has_model)?;
            printer.print(" % ");
            generate_expr(printer, map, owner, params, r, 13, has_model)?;
        }
        ExpressionNode::Add(l, r) => {
            generate_expr(printer, map, owner, params.clone(), l, 11, has_model)?;
            printer.print(" + ");
            generate_expr(printer, map, owner, params, r, 12, has_model)?;
        }
        ExpressionNode::Subtract(l, r) => {
            generate_expr(printer, map, owner, params.clone(), l, 11, has_model)?;
            printer.print(" - ");
            generate_expr(printer, map, owner, params, r, 12, has_model)?;
        }

        // ── Битовые сдвиги ────────────────────────────────────────────────────
        ExpressionNode::ShiftLeft(l, r) => {
            generate_expr(printer, map, owner, params.clone(), l, 10, has_model)?;
            printer.print(" << ");
            generate_expr(printer, map, owner, params, r, 11, has_model)?;
        }
        ExpressionNode::ShiftRight(l, r) => {
            generate_expr(printer, map, owner, params.clone(), l, 10, has_model)?;
            printer.print(" >> ");
            generate_expr(printer, map, owner, params, r, 11, has_model)?;
        }

        // ── Побитовые операторы ────────────────────────────────────────────────
        ExpressionNode::BitwiseAnd(l, r) => {
            generate_expr(printer, map, owner, params.clone(), l, 7, has_model)?;
            printer.print(" & ");
            generate_expr(printer, map, owner, params, r, 8, has_model)?;
        }
        ExpressionNode::BitwiseXor(l, r) => {
            generate_expr(printer, map, owner, params.clone(), l, 6, has_model)?;
            printer.print(" ^ ");
            generate_expr(printer, map, owner, params, r, 7, has_model)?;
        }
        ExpressionNode::BitwiseOr(l, r) => {
            generate_expr(printer, map, owner, params.clone(), l, 5, has_model)?;
            printer.print(" | ");
            generate_expr(printer, map, owner, params, r, 6, has_model)?;
        }

        // ── Сравнение ─────────────────────────────────────────────────────────
        ExpressionNode::Less(l, r) => {
            generate_expr(printer, map, owner, params.clone(), l, 9, has_model)?;
            printer.print(" < ");
            generate_expr(printer, map, owner, params, r, 10, has_model)?;
        }
        ExpressionNode::More(l, r) => {
            generate_expr(printer, map, owner, params.clone(), l, 9, has_model)?;
            printer.print(" > ");
            generate_expr(printer, map, owner, params, r, 10, has_model)?;
        }
        ExpressionNode::LessEqual(l, r) => {
            generate_expr(printer, map, owner, params.clone(), l, 9, has_model)?;
            printer.print(" <= ");
            generate_expr(printer, map, owner, params, r, 10, has_model)?;
        }
        ExpressionNode::MoreEqual(l, r) => {
            generate_expr(printer, map, owner, params.clone(), l, 9, has_model)?;
            printer.print(" >= ");
            generate_expr(printer, map, owner, params, r, 10, has_model)?;
        }
        ExpressionNode::Equal(l, r) => {
            generate_expr(printer, map, owner, params.clone(), l, 8, has_model)?;
            printer.print(" == ");
            generate_expr(printer, map, owner, params, r, 9, has_model)?;
        }
        ExpressionNode::NotEqual(l, r) => {
            generate_expr(printer, map, owner, params.clone(), l, 8, has_model)?;
            printer.print(" != ");
            generate_expr(printer, map, owner, params, r, 9, has_model)?;
        }

        // ── Логические ────────────────────────────────────────────────────────
        ExpressionNode::And(l, r) => {
            generate_expr(printer, map, owner, params.clone(), l, 4, has_model)?;
            printer.print(" && ");
            generate_expr(printer, map, owner, params, r, 5, has_model)?;
        }
        ExpressionNode::Or(l, r) => {
            generate_expr(printer, map, owner, params.clone(), l, 3, has_model)?;
            printer.print(" || ");
            generate_expr(printer, map, owner, params, r, 4, has_model)?;
        }

        // ── Специальные ───────────────────────────────────────────────────────
        // Явные скобки из исходного кода — всегда генерируем как есть.
        ExpressionNode::Parenthesis(e) => {
            printer.print("(");
            generate_expr(printer, map, owner, params, e, 0, has_model)?;
            printer.print(")");
        }

        // Тернарный оператор: условие обёртывается при prec ≤ ||, чтобы
        // присваивание или вложенный тернарный в условии был явно выделен.
        ExpressionNode::ConditionalOperator(cond, then_, else_) => {
            generate_expr(printer, map, owner, params.clone(), cond, 4, has_model)?;
            printer.print(" ? ");
            generate_expr(printer, map, owner, params.clone(), then_, 0, has_model)?;
            printer.print(" : ");
            generate_expr(printer, map, owner, params, else_, 0, has_model)?;
        }

        ExpressionNode::Assign(l, r) => {
            // Запись в порт → write_bit / write_float
            if let ExpressionNode::Variable(var_rc) = l.as_ref() {
                let var = var_rc.borrow();
                if let VariableNode::Port {
                    name, ty, upper, ..
                } = &*var
                {
                    let model_name =
                        if let Some(model_rc) = upper.as_ref().and_then(|w| w.upgrade()) {
                            Name::from(model_rc)
                        } else {
                            return Err("Неразрешённый owner порта при записи".into());
                        };
                    let cls = PortClass::from_type(ty);
                    let variant = format!(
                        "{}_{}",
                        model_name.unique_uppercase_snakecase(),
                        normalize_lowercase_snakecase(name.clone()).to_uppercase()
                    );
                    let mut rhs_str = String::new();
                    {
                        let mut tmp = Printer::new(4, &mut rhs_str);
                        generate_expr(&mut tmp, map, owner, params, r, 0, has_model)?;
                    }
                    let ptr = if has_model && !owner.name().eq(&map.root_name()) {
                        "main"
                    } else {
                        "model"
                    };
                    match cls {
                        PortClass::Rational => {
                            printer.print(&format!(
                                "(*{ptr}->{write_float})({variant}, {rhs_str}, {ptr}->userdata)",
                                write_float = FUNCTION_PORT_WRITE_FLOAT
                            ));
                        }
                        PortClass::Numeric => {
                            printer.print(&format!(
                                "(*{ptr}->{write_numeric})({variant}, {rhs_str}, {ptr}->userdata)",
                                write_numeric = FUNCTION_PORT_WRITE_NUMERIC
                            ));
                        }
                        PortClass::Bit => {
                            printer.print(&format!(
                                "(*{ptr}->{write_bit})({variant}, {rhs_str}, {ptr}->userdata)",
                                write_bit = FUNCTION_PORT_WRITE_BIT
                            ));
                        }
                    }
                    return Ok(());
                }
            }
            // BitAccess как lvalue: inner.N = val
            if let ExpressionNode::BitAccess(inner_expr, Member::Number(n)) = l.as_ref() {
                // Порт.бит = val → write_bit(PORT, N, val, userdata)
                if let ExpressionNode::Variable(var_rc) = inner_expr.as_ref() {
                    let var = var_rc.borrow();
                    if let VariableNode::Port {
                        name, ty, upper, ..
                    } = &*var
                    {
                        let model_name = if let Some(rc) = upper.as_ref().and_then(|w| w.upgrade())
                        {
                            Name::from(rc)
                        } else {
                            return Err("Неразрешённый owner порта при BitAccess записи".into());
                        };
                        let cls = PortClass::from_type(ty);
                        if cls == PortClass::Rational {
                            return Err(Diagnostic::error(
                                Location::Codegen,
                                "BitAccess на float-порт не поддерживается при записи".to_string(),
                            )
                            .with_code("CC-001"));
                        }
                        let variant = format!(
                            "{}_{}",
                            model_name.unique_uppercase_snakecase(),
                            normalize_lowercase_snakecase(name.clone()).to_uppercase()
                        );
                        let mut rhs_str = String::new();
                        {
                            let mut tmp = Printer::new(4, &mut rhs_str);
                            generate_expr(&mut tmp, map, owner, params, r, 0, has_model)?;
                        }
                        let ptr = if has_model && !owner.name().eq(&map.root_name()) {
                            "main"
                        } else {
                            "model"
                        };
                        match cls {
                            PortClass::Bit => {
                                printer.print(&format!(
                                    "(*{ptr}->{write_bit})({variant}, {rhs_str}, {ptr}->userdata)",
                                    write_bit = FUNCTION_PORT_WRITE_BIT
                                ));
                            }
                            PortClass::Numeric => {
                                printer.print(&format!(
                                    "(*{ptr}->{write_numeric})({variant}, \
                                    ((*{ptr}->{read_numeric})({variant}, {ptr}->userdata) \
                                    & ~(1LL << {n})) | (({rhs_str} & 1LL) << {n}), {ptr}->userdata)",
                                    write_numeric = FUNCTION_PORT_WRITE_NUMERIC,
                                    read_numeric = FUNCTION_PORT_READ_NUMERIC,
                                ));
                            }
                            PortClass::Rational => unreachable!(),
                        }
                        return Ok(());
                    }
                }
                // Обычная переменная.бит = val
                // x = (x & ~(1u << N)) | ((val & 1u) << N)
                let mut lhs_str = String::new();
                {
                    let mut tmp = Printer::new(4, &mut lhs_str);
                    generate_expr(
                        &mut tmp,
                        map,
                        owner,
                        params.clone(),
                        inner_expr,
                        0,
                        has_model,
                    )?;
                }
                let mut rhs_str = String::new();
                {
                    let mut tmp = Printer::new(4, &mut rhs_str);
                    generate_expr(&mut tmp, map, owner, params, r, 0, has_model)?;
                }
                printer.print(&format!(
                    "{0} = ({0} & ~(1u << {1})) | (({2} & 1u) << {1})",
                    lhs_str, n, rhs_str
                ));
                return Ok(());
            }
            // Обычное присваивание (право-ассоциативно: тот же prec не оборачивается)
            generate_expr(printer, map, owner, params.clone(), l, 1, has_model)?;
            printer.print(" = ");
            generate_expr(printer, map, owner, params, r, 1, has_model)?;
        }

        ExpressionNode::ArraySubscript(var_rc, idx) => {
            let var = var_rc.borrow();
            let var_expr = if let VariableNode::Simple { upper, .. } = &*var {
                resolve_simple_var_in_context(var.name(), upper, &params, owner, map, has_model)
                    .map_or_else(|| resolve_variable_c_expr(&*var, &params, map, owner, has_model), Ok)?
            } else {
                resolve_variable_c_expr(&*var, &params, map, owner, has_model)?
            };
            printer.print(&format!("{}[{}]", var_expr, idx));
        }

        ExpressionNode::Variable(var_rc) => {
            let var = var_rc.borrow();
            let var_expr = if let VariableNode::Simple { upper, loc, .. } = &*var {
                // Локальные переменные (loc == Implicit) доступны по имени напрямую,
                // а не через model->name, даже если они принадлежат той же модели.
                if matches!(loc, crate::diagnostics::Location::Implicit) {
                    normalize_lowercase_snakecase(var.name().to_string())
                } else {
                    resolve_simple_var_in_context(var.name(), upper, &params, owner, map, has_model)
                        .map_or_else(|| resolve_variable_c_expr(&*var, &params, map, owner, has_model), Ok)?
                }
            } else {
                resolve_variable_c_expr(&*var, &params, map, owner, has_model)?
            };
            printer.print(&var_expr);
        }

        ExpressionNode::Condition(cond_rc) => {
            let cond = cond_rc.borrow();
            let cond_str = condition_macro_name(&*cond);
            printer.print(&cond_str);
        }

        ExpressionNode::Function(fun_rc, args) => {
            let fun = fun_rc.borrow();
            generate_function_call(printer, map, owner, params, &*fun, args, has_model)?;
        }

        ExpressionNode::Initializer(elems) => {
            printer.print("{");
            for (i, elem) in elems.iter().enumerate() {
                if i > 0 {
                    printer.print(", ");
                }
                generate_expr(printer, map, owner, params.clone(), elem, 0, has_model)?;
            }
            printer.print("}");
        }

        ExpressionNode::Array(elems) => {
            printer.print("{");
            for (i, elem) in elems.iter().enumerate() {
                if i > 0 {
                    printer.print(", ");
                }
                generate_expr(printer, map, owner, params.clone(), elem, 0, has_model)?;
            }
            printer.print("}");
        }

        ExpressionNode::Cast(expr, typ) => {
            let model = map.raw_model_at(owner.name())?;
            let model = &*model.borrow();
            let type_c = get_c_type(typ, model).unwrap_or_else(|| "int".to_string());
            // Приводимое выражение оборачивается при prec < UNARY (13),
            // то есть при наличии бинарных операторов: (int)(a + b).
            printer.print("(").print(&type_c).print(")");
            generate_expr(printer, map, owner, params, expr, 13, has_model)?;
        }

        // ── Неподдерживаемые ──────────────────────────────────────────────────
        ExpressionNode::ArraySlice(_, _, _) => {
            return Err("ArraySlice не поддерживается в C генераторе".into());
        }
        ExpressionNode::BitAccess(inner, member) => {
            match member {
                Member::Identifier(id) => {
                    // Доступ к полю структуры: inner.field — используем максимальный приоритет
                    generate_expr(printer, map, owner, params, inner, 15, has_model)?;
                    printer.print(&format!(".{}", id.name));
                }
                Member::Number(n) => {
                    // Битовый доступ к порту: (*main->read_bit)(PORT_X, N, main->userdata)
                    if let ExpressionNode::Variable(var_rc) = inner.as_ref() {
                        let var = var_rc.borrow();
                        if let VariableNode::Port {
                            name, ty, upper, ..
                        } = &*var
                        {
                            let model_name =
                                if let Some(rc) = upper.as_ref().and_then(|w| w.upgrade()) {
                                    Name::from(rc)
                                } else {
                                    return Err("Неразрешённый owner порта при BitAccess".into());
                                };
                            let cls = PortClass::from_type(ty);
                            let variant = format!(
                                "{}_{}",
                                model_name.unique_uppercase_snakecase(),
                                normalize_lowercase_snakecase(name.clone()).to_uppercase()
                            );
                            let ptr = if has_model && !owner.name().eq(&map.root_name()) {
                                "main"
                            } else {
                                "model"
                            };
                            match cls {
                                PortClass::Bit => {
                                    printer.print(&format!(
                                        "(*{ptr}->{read_bit})({variant}, {ptr}->userdata)",
                                        read_bit = FUNCTION_PORT_READ_BIT
                                    ));
                                }
                                PortClass::Numeric => {
                                    printer.print(&format!(
                                        "(((*{ptr}->{read_numeric})({variant}, {ptr}->userdata) >> {n}) & 1u)",
                                        read_numeric = FUNCTION_PORT_READ_NUMERIC
                                    ));
                                }
                                PortClass::Rational => {
                                    return Err(Diagnostic::error(
                                        Location::Codegen,
                                        "BitAccess на float-порт не поддерживается".to_string(),
                                    )
                                    .with_code("CC-001"));
                                }
                            }
                            return Ok(());
                        }
                    }
                    // Обычная переменная/выражение: ((inner >> N) & 1u)
                    printer.print("((");
                    generate_expr(printer, map, owner, params, inner, 0, has_model)?;
                    printer.print(&format!(" >> {}) & 1u)", n));
                }
            }
        }
        ExpressionNode::CodeBlock(_, _) => {
            return Err("CodeBlock не поддерживается как выражение в C генераторе".into());
        }
        ExpressionNode::NamedFunctionBox(_, _) => {
            return Err("NamedFunctionBox не поддерживается в C генераторе".into());
        }
        ExpressionNode::List(_) => {
            return Err("List не поддерживается в C генераторе".into());
        }
        ExpressionNode::Type(_) => {
            return Err("Type не поддерживается как выражение в C генераторе".into());
        }
        ExpressionNode::Address(_, _) => {
            return Err("Address не поддерживается как выражение в C генераторе".into());
        }
        ExpressionNode::Model(_) => {
            return Err("Model не поддерживается как выражение в C генераторе".into());
        }
    }
    if wrap {
        printer.print(")");
    }
    Ok(())
}

/// Генерирует C-выражение из семантического узла выражения.
///
/// Функция пишет в `printer` без начального отступа и без завершающего `;\n`.
/// Отступ и разделители добавляет вызывающий код.
/// Является обёрткой над [`generate_expr`] с `min_prec = 0`.
pub(super) fn generate_stmt_expression(
    printer: &mut Printer,
    map: &CMap,
    owner: &Element,
    params: Vec<(String, TypeNode)>,
    expr: &ExpressionNode,
    has_model: bool,
) -> Result<(), Diagnostic> {
    generate_expr(printer, map, owner, params, expr, 0, has_model)
}

pub(super) fn generate_formula_check(
    printer: &mut Printer,
    map: &CMap,
    owner: &Element,
    formula: &Formula,
) -> Result<(), Diagnostic> {
    match formula {
        Formula::None => {}
        Formula::Formulas(formulas) => {
            for f in formulas {
                generate_formula_check(printer, map, owner, f)?;
            }
        }
        Formula::Guard(cond) => {
            let cond_expr = generate_condition_expr(cond, map, owner)?;
            if !cond_expr.is_empty() {
                printer.ident(&format!("assert({});", cond_expr)).nl();
            }
        }
        Formula::LTL(_) => {
            // LTL-формулы в C-коде пока не проверяются
        }
    }
    Ok(())
}

/// Генерирует C-оператор из семантического узла.
///
/// Для `Block` рекурсивно генерирует все вложенные операторы.
/// Для `Expression` генерирует выражение с отступом и `;`.
/// Поддерживает `If`, `Loop`, `For`, `Variable`, `Return`, `Continue`, `Break`.
pub(super) fn generate_code_block(
    printer: &mut Printer,
    map: &CMap,
    owner: &Element,
    params: Vec<(String, TypeNode)>,
    body: &StatementNode,
    has_model: bool,
) -> Result<(), Diagnostic> {
    match body {
        StatementNode::None => {}
        StatementNode::Unresolved(_) => {}

        StatementNode::Block(block) => {
            for stmt in block {
                generate_code_block(printer, map, owner, params.clone(), stmt, has_model)?;
            }
        }

        StatementNode::Expression(expr) => {
            // Генерируем во временный буфер, чтобы безопасно пропустить
            // неподдерживаемые встроенные функции (debug, S) без порчи вывода
            let mut expr_buf = String::new();
            let result = {
                let mut tmp = Printer::new(4, &mut expr_buf);
                generate_stmt_expression(&mut tmp, map, owner, params, expr, has_model)
            };
            match result {
                Ok(()) if !expr_buf.is_empty() => {
                    printer.ident(&expr_buf).print(";").nl();
                }
                Ok(()) => {}
                Err(_) => {
                    // Пропускаем неподдерживаемые выражения (S и т.п.)
                }
            }
        }

        StatementNode::If { cond, then_, else_ } => {
            // Печатаем первый if
            printer.ident("if (");
            generate_stmt_expression(printer, map, owner, params.clone(), cond, has_model)?;
            printer.print(") {").up().nl();
            generate_code_block(printer, map, owner, params.clone(), then_, has_model)?;

            // Обходим цепочку else/else-if: если else-ветка — одиночный if,
            // схлопываем в `} else if (...)`, чтобы не создавать лишней вложенности
            let mut current_else = else_.as_deref();
            loop {
                match current_else {
                    None => {
                        // Нет else — закрываем последний блок
                        printer.down().ident("}").nl();
                        break;
                    }
                    Some(StatementNode::If {
                        cond: ec,
                        then_: et,
                        else_: ee,
                    }) => {
                        // else-ветка — одиночный if: схлопываем в else if
                        printer.down().ident("} else if (");
                        generate_stmt_expression(
                            printer,
                            map,
                            owner,
                            params.clone(),
                            ec,
                            has_model,
                        )?;
                        printer.print(") {").up().nl();
                        generate_code_block(printer, map, owner, params.clone(), et, has_model)?;
                        current_else = ee.as_deref();
                    }
                    Some(else_stmt) => {
                        // else-ветка — произвольный блок
                        printer.down().ident("} else {").up().nl();
                        generate_code_block(
                            printer,
                            map,
                            owner,
                            params.clone(),
                            else_stmt,
                            has_model,
                        )?;
                        printer.down().ident("}").nl();
                        break;
                    }
                }
            }
        }

        StatementNode::Loop { cond, body } => {
            match cond {
                None => {
                    // Бесконечный цикл
                    printer.ident("while (true) {").up().nl();
                }
                Some(cond_expr) => {
                    // Цикл с условием
                    printer.ident("while (");
                    generate_stmt_expression(
                        printer,
                        map,
                        owner,
                        params.clone(),
                        cond_expr,
                        has_model,
                    )?;
                    printer.print(") {").up().nl();
                }
            }
            generate_code_block(printer, map, owner, params.clone(), body, has_model)?;
            printer.down().ident("}").nl();
        }

        StatementNode::For {
            init,
            cond,
            step,
            body,
        } => {
            let has_var_init = matches!(
                init.as_ref().map(|b| b.as_ref()),
                Some(StatementNode::Variable(..))
            );

            if has_var_init {
                // Объявление переменной выносим перед `for` в обёртку `{}`
                printer.ident("{").nl();
                printer.up();
                if let Some(init_stmt) = init {
                    generate_code_block(printer, map, owner, params.clone(), init_stmt, has_model)?;
                }
                printer.ident("for (;");
                if let Some(cond_expr) = cond {
                    printer.print(" ");
                    generate_stmt_expression(
                        printer,
                        map,
                        owner,
                        params.clone(),
                        cond_expr,
                        has_model,
                    )?;
                }
                printer.print(";");
                if let Some(step_expr) = step {
                    printer.print(" ");
                    generate_stmt_expression(
                        printer,
                        map,
                        owner,
                        params.clone(),
                        step_expr,
                        has_model,
                    )?;
                }
                printer.print(") {").up().nl();
                generate_code_block(printer, map, owner, params.clone(), body, has_model)?;
                printer.down().ident("}").nl();
                printer.down();
                printer.ident("}").nl();
            } else {
                printer.ident("for (");
                if let Some(init_stmt) = init {
                    // Инициализация — только выражение (без отступа и точки с запятой)
                    if let StatementNode::Expression(expr) = init_stmt.as_ref() {
                        generate_stmt_expression(
                            printer,
                            map,
                            owner,
                            params.clone(),
                            expr,
                            has_model,
                        )?;
                    }
                }
                printer.print(";");
                if let Some(cond_expr) = cond {
                    printer.print(" ");
                    generate_stmt_expression(
                        printer,
                        map,
                        owner,
                        params.clone(),
                        cond_expr,
                        has_model,
                    )?;
                }
                printer.print(";");
                if let Some(step_expr) = step {
                    printer.print(" ");
                    generate_stmt_expression(
                        printer,
                        map,
                        owner,
                        params.clone(),
                        step_expr,
                        has_model,
                    )?;
                }
                printer.print(") {").up().nl();
                generate_code_block(printer, map, owner, params.clone(), body, has_model)?;
                printer.down().ident("}").nl();
            }
        }

        StatementNode::Variable(name, ty, init) => {
            let model = map.raw_model_at(owner.name())?;
            let model_ref = model.borrow();
            let snake_name = normalize_lowercase_snakecase(name.clone());
            let decl = get_typed_variable(ty, Some(snake_name.clone()), &*model_ref)
                .unwrap_or_else(|| format!("int {}", snake_name));
            printer.ident(&decl);
            if let Some(init_expr) = init {
                printer.print(" = ");
                generate_stmt_expression(printer, map, owner, params, init_expr, has_model)?;
            }
            printer.print(";").nl();
        }

        StatementNode::Return(ret) => {
            printer.ident("return");
            if let Some(expr) = ret {
                printer.print(" ");
                generate_stmt_expression(printer, map, owner, params, expr, has_model)?;
            }
            printer.print(";").nl();
        }

        StatementNode::Continue => {
            printer.ident("continue;").nl();
        }

        StatementNode::Break => {
            printer.ident("break;").nl();
        }

        StatementNode::InlineFormula(formulas) => {
            if map.guard_enable() {
                for formula in formulas {
                    generate_formula_check(printer, map, owner, formula)?;
                }
            }
        }
    }
    Ok(())
}
