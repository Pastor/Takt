//! Разбор объявления значения модели: `var`, порт, `const`, `parameter`.
//!
//! Вынесен из `semantic/tree.rs` фичей 0185: файл давно сверх лимита размера
//! (`scripts/check-module-size.sh`), а «построить узел объявления» —
//! самостоятельная ответственность, отделимая от обхода элементов модели.

use crate::diagnostics::{Diagnostic, Location};
use crate::parser::ast::{Identifier, VariableDefine};
use crate::semantic::type_node::{TypeNode, construct_type};
use crate::semantic::{
    ExpressionNode, ModelNode, ParameterNode, PortDirection, VariableNode, const_eval,
};
use std::cell::RefCell;
use std::collections::BTreeMap;
use std::rc::Rc;

/// Строит узел объявления и кладёт его в карту переменных модели.
///
/// Параметр (фича 0185) дополнительно попадает в `parameters` — в порядке
/// объявления: по нему строится ключ дедупликации специализаций
/// (`--parameters=specialize`), а детерминизм вывода (0048) требует, чтобы
/// порядок зависел только от входа.
pub(super) fn construct_declaration(
    def: &VariableDefine,
    model_node: Rc<RefCell<ModelNode>>,
    variables: &mut BTreeMap<String, VariableNode>,
    parameters: &mut Vec<ParameterNode>,
) -> Result<(), Diagnostic> {
    // Пока тип определяется только из явной аннотации.
    match def.clone() {
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
                    ty: construct_type(typ, Rc::clone(&model_node))?,
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
            address,
            initializer,
            direction,
        } => {
            let name = extract_name(name.clone(), loc)?;
            let type_node = construct_type(typ, Rc::clone(&model_node))?;
            if type_node == TypeNode::Inference {
                return Err(
                    Diagnostic::error(loc, "Порт должен иметь конкретный тип".to_string())
                        .with_code("SE-023"),
                );
            }
            // Два независимых выражения (фича 0187): размещение `at <адрес>` и
            // начальное значение `:=`. Каждое необязательно — адрес может
            // прийти по имени порта (оператор `address`, внешняя карта), и
            // полноту проверяет слой адресов, а не объявление.
            let address_node = address
                .clone()
                .map(ExpressionNode::Unresolved)
                .unwrap_or(ExpressionNode::None);
            let init_node = initializer
                .clone()
                .map(ExpressionNode::Unresolved)
                .unwrap_or(ExpressionNode::None);
            variables.insert(
                name.clone(),
                VariableNode::Port {
                    upper: Some(Rc::downgrade(&model_node)),
                    loc,
                    name: name.clone(),
                    ty: type_node,
                    address: address_node,
                    init: init_node,
                    direction,
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
                    ty: construct_type(typ, Rc::clone(&model_node))?,
                    expr: ExpressionNode::Unresolved(initializer),
                },
            )
        }
        // Параметр модели (фича 0185). В дереве он **обычная
        // переменная** с начальным значением: в режиме генерации по
        // умолчанию (`--parameters=assign`) параметр и есть поле
        // экземпляра, поэтому потребитель, ничего не знающий о
        // параметрах, обращается с ним верно. Отличие хранится
        // отдельно — в `ModelNode::parameters` (имя, позиция, порядок).
        VariableDefine::Parameter {
            loc,
            typ,
            name,
            initializer,
        } => {
            let name = extract_name(name.clone(), loc)?;
            // Параметр верхнего уровня файла инстанцировать нечем:
            // анонимный корень в выражении реализации по имени не
            // появляется (ADR 0185, п. 2). Отказ здесь — вместо
            // объявления, которое молча вело бы себя как `var`.
            if model_node.borrow().upper.is_none() && model_node.borrow().name.is_none() {
                return Err(Diagnostic::error(
                    loc,
                    format!(
                        "Параметр '{name}' объявлен вне модели: \
                         верхний уровень файла инстанцировать нечем — \
                         перенесите объявление в модель либо замените на 'var'"
                    ),
                )
                .with_code("SE-075"));
            }
            parameters.push(ParameterNode {
                name: name.clone(),
                loc,
                // «Изменяемый», пока анализ изменяемости (0185-06) не сказал
                // иное: неразмеченный параметр обязан вести себя как переменная.
                mutated: true,
            });
            variables.insert(
                name.clone(),
                VariableNode::Simple {
                    upper: Some(Rc::downgrade(&model_node)),
                    loc,
                    name: name.clone(),
                    ty: construct_type(typ, Rc::clone(&model_node))?,
                    expr: ExpressionNode::Unresolved(initializer),
                },
            )
        }
    };
    Ok(())
}

/// Разбирает объявление **внутри блока оператора**: имя, тип, инициализатор.
///
/// Отличается от [`construct_declaration`] тем, что локальное объявление не
/// становится членом модели: узел строит вызывающий
/// ([`StatementNode::Variable`](crate::semantic::StatementNode::Variable)).
pub(super) fn local_declaration(
    def: &VariableDefine,
    loc: Location,
    model: Rc<RefCell<ModelNode>>,
) -> Result<(String, TypeNode, Option<crate::parser::ast::Expression>), Diagnostic> {
    let named =
        |name: &Option<Identifier>| name.as_ref().map(|i| i.name.clone()).unwrap_or_default();
    match def {
        VariableDefine::Variable {
            name,
            typ,
            initializer,
            ..
        }
        | VariableDefine::Port {
            name,
            typ,
            initializer,
            ..
        } => Ok((
            named(name),
            construct_type(typ.clone(), model)?,
            initializer.clone(),
        )),
        VariableDefine::Constant {
            name,
            typ,
            initializer,
            ..
        } => Ok((
            named(name),
            construct_type(typ.clone(), model)?,
            Some(initializer.clone()),
        )),
        // Параметр в теле блока грамматикой не порождается
        // (`LocalVariableDefine` слова `parameter` не знает), но ветвь обязана
        // быть: расширив грамматику, разработчик получит здесь явный отказ, а
        // не молчаливое превращение параметра в локальную переменную (0185).
        VariableDefine::Parameter { name, .. } => Err(Diagnostic::error(
            loc,
            format!(
                "Параметр '{}' объявлен внутри блока: параметр задаётся в месте \
                 инстанцирования модели, поэтому объявляется только на уровне модели",
                named(name)
            ),
        )
        .with_code("SE-075")),
    }
}

/// Имя объявления либо отказ: безымянное объявление разбором не отсеивается.
fn extract_name(id: Option<Identifier>, loc: Location) -> Result<String, Diagnostic> {
    match id {
        Some(id) => Ok(id.name.clone()),
        None => {
            Err(Diagnostic::error(loc, "Идентификатор не задан".to_string()).with_code("SE-021"))
        }
    }
}

/// Разрешает выражение объявления, если оно ещё «сырое» (`Unresolved`).
///
/// Вынесено сюда (фича 0187): у порта таких выражений **два** — размещение и
/// начальное значение, — и повтор `match` для каждого раздул бы `tree.rs`, уже
/// стоящий в реестре размера.
pub(crate) fn resolve_declaration_expression(
    expr: ExpressionNode,
    model: &Rc<RefCell<ModelNode>>,
) -> Result<ExpressionNode, Diagnostic> {
    match expr {
        ExpressionNode::Unresolved(raw) => {
            crate::semantic::expression::construct_expression(raw, vec![], Rc::clone(model))
        }
        other => Ok(other),
    }
}

/// Разрешает «сырые» выражения объявления переменной (`Unresolved` → дерево).
///
/// Вынесено из `tree.rs` (фича 0187): у порта выражений **два** — размещение и
/// начальное значение, — и разбор каждого на месте раздул бы файл, стоящий в
/// реестре размера. Логика прежняя: узел без «сырых» выражений возвращается как
/// есть.
pub(crate) fn resolve_variable_expressions(
    var: VariableNode,
    model: &Rc<RefCell<ModelNode>>,
) -> Result<VariableNode, Diagnostic> {
    Ok(match var {
        VariableNode::Simple {
            upper,
            loc,
            name,
            ty,
            expr,
        } => VariableNode::Simple {
            upper,
            loc,
            name,
            ty,
            expr: resolve_declaration_expression(expr, model)?,
        },
        VariableNode::Const {
            upper,
            loc,
            name,
            ty,
            expr,
        } => VariableNode::Const {
            upper,
            loc,
            name,
            ty,
            expr: resolve_declaration_expression(expr, model)?,
        },
        VariableNode::Port {
            upper,
            loc,
            name,
            ty,
            address,
            init,
            direction,
        } => VariableNode::Port {
            upper,
            loc,
            address: resolve_declaration_expression(address, model)?,
            init: resolve_port_init(init, &name, direction, loc, model)?,
            name,
            ty,
            direction,
        },
        VariableNode::Unresolved => VariableNode::Unresolved,
    })
}

/// Разрешает **начальное значение** порта, сворачивая его в литерал
/// (фича 0187, задача 03).
///
/// # Почему литерал, а не выражение
///
/// Значение выставляется **до первого такта**, и выставляют его шесть разных
/// потребителей: `_init` цели `c`, `new()`/`init()` цели `rust`, ветвь сброса
/// `sv`, инициализатор объявления `st`, старт порта в симуляторе. Выражение
/// печатается **в контексте владельца**, а места эмиссии у целей разные: у
/// цели `rust` под-модель конструируется без доступа к HAL, поэтому значения
/// портов всего дерева выставляет корень — и имя, законное в под-модели, там
/// уже не разрешается. Свёртка снимает вопрос целиком: за границей семантики
/// выражения не существует, разойтись целям не по чему (тот же приём, что у
/// константной выдержки `after`, ADR 0143).
///
/// # Что принимается
///
/// Всё, что вычисляет [`const_eval`]: литералы, константы модели (в том числе
/// цепочкой) и арифметика над ними. Прочее — **`SE-094`** с названной причиной:
/// молчаливая потеря значения здесь дороже отказа.
///
/// ⚠️ У **входного** порта значение не сворачивается: его там не бывает вовсе
/// (`SE-092`), и свёртка перехватила бы диагностику, подменив её жалобой на
/// невычислимость.
fn resolve_port_init(
    init: ExpressionNode,
    name: &str,
    direction: PortDirection,
    loc: Location,
    model: &Rc<RefCell<ModelNode>>,
) -> Result<ExpressionNode, Diagnostic> {
    if direction == PortDirection::In {
        return resolve_declaration_expression(init, model);
    }
    let ExpressionNode::Unresolved(raw) = &init else {
        return Ok(init);
    };
    let literal = const_eval::fold_to_literal(raw, model).map_err(|cause| {
        Diagnostic::error(
            loc,
            format!(
                "начальное значение порта '{name}' выставляется до первого такта, \
                 поэтому обязано быть известно при компиляции: {}",
                cause.message
            ),
        )
        .with_code("SE-094")
    })?;
    resolve_declaration_expression(ExpressionNode::Unresolved(literal), model)
}
