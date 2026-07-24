//! Эмиссия именованных блоков (`enter`/`exit`/`always`) в цель C.
//!
//! Вынесено из `c_model.rs` (фича 0083: лимит размера модуля). Два источника
//! блоков — состояние и **сама модель** (model-level `always`, фича 0083);
//! обе печати идут в одинаковых условиях (`owner` — модель), различаясь лишь
//! источником списка блоков.

use super::c_expr::{generate_code_block, generate_expr};
use crate::diagnostics::Diagnostic;
use crate::generator::c::c_map::CMap;
use crate::generator::indent::Printer;
use crate::semantic::minimap::Element;
use crate::semantic::type_node::TypeNode;
use crate::semantic::{ExpressionNode, ModelNode, StateNode};

/// Инициализация скалярной (не массивной) переменной в `_init`:
/// `model->name = <expr>;`.
///
/// 0080-01: **структура** присваивается СОСТАВНЫМ ЛИТЕРАЛОМ `(Type){...}` —
/// голый `model->p = {1, 2};` невалиден (`cc: expected expression`): фигурный
/// инициализатор допустим только в объявлении, не в присваивании.
pub(super) fn generate_scalar_init(
    printer: &mut Printer,
    map: &CMap,
    owner: &Element,
    var_name: &str,
    ty: &TypeNode,
    expr: &ExpressionNode,
) -> Result<(), Diagnostic> {
    let cast = if let TypeNode::Struct(s) = ty {
        format!("({})", s)
    } else {
        String::new()
    };
    printer.ident(&format!("model->{} = {}", var_name, cast));
    generate_expr(printer, map, owner, vec![], expr, 0, true)?;
    printer.print(";").nl();
    Ok(())
}

/// Генерирует именованные блоки состояния (`enter`/`exit`/`always`).
pub(super) fn generate_named_blocks(
    printer: &mut Printer,
    state: &StateNode,
    map: &CMap,
    owner: &Element,
    block_name: &str,
) -> Result<(), Diagnostic> {
    for block in state.get_named_blocks(block_name) {
        let Some(stmt) = block.statement() else {
            continue;
        };
        generate_code_block(printer, map, owner, vec![], stmt, true)?;
    }
    Ok(())
}

/// Генерирует именованные блоки **уровня модели** (фича 0083): `always` вне
/// состояния. Источник — сама модель, а не состояние; тело печатается в тех же
/// условиях (`owner` — модель).
pub(super) fn generate_model_named_blocks(
    printer: &mut Printer,
    model_node: &ModelNode,
    map: &CMap,
    owner: &Element,
    block_name: &str,
) -> Result<(), Diagnostic> {
    for block in model_node.get_named_blocks(block_name) {
        let Some(stmt) = block.statement() else {
            continue;
        };
        generate_code_block(printer, map, owner, vec![], stmt, true)?;
    }
    Ok(())
}
