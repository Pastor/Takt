//! Разбор объявления значения модели: `var`, порт, `const`, `parameter`.
//!
//! Вынесен из `semantic/tree.rs` фичей 0185: файл давно сверх лимита размера
//! (`scripts/check-module-size.sh`), а «построить узел объявления» —
//! самостоятельная ответственность, отделимая от обхода элементов модели.

use crate::diagnostics::{Diagnostic, Location};
use crate::parser::ast::{Identifier, VariableDefine};
use crate::semantic::type_node::{TypeNode, construct_type};
use crate::semantic::{ExpressionNode, ModelNode, ParameterNode, VariableNode};
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
            // Адрес порта необязателен — если не задан, используем None.
            let expr = initializer
                .map(ExpressionNode::Unresolved)
                .unwrap_or(ExpressionNode::None);
            variables.insert(
                name.clone(),
                VariableNode::Port {
                    upper: Some(Rc::downgrade(&model_node)),
                    loc,
                    name: name.clone(),
                    ty: type_node,
                    expr,
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
