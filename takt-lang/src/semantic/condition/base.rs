//! База постфиксной индексации в УСЛОВИИ (фича 0358).
//!
//! Условие — своё дерево (`ConditionNode`), и общий носитель типа
//! (`semantic::validate::base_type`) работает по `ExpressionNode`. Поэтому знание о
//! цепочке места здесь **своё**, но узкое: переменная массива либо поле
//! структуры типа `[T; N]`.
//!
//! Живёт отдельным модулем, потому что `condition/mod.rs` упирается в лимит
//! размера (правило `docs/CODE.md`): новое выносится, а не дописывается.

use super::*;

/// Как назвать базу условия в диагностике (фича 0358).
///
/// Имя переменной, когда оно есть: диагностика обязана называть предмет, а не
/// говорить о «значении» (тест `diagnostic_code_presence_tests` сторожит это).
pub(super) fn cond_base_label(base: &ConditionNode) -> String {
    match base {
        ConditionNode::Variable(var, _) => format!("'{}'", var.borrow().name()),
        ConditionNode::Parenthesis(inner) => cond_base_label(inner),
        ConditionNode::BitAccess(_, crate::parser::ast::Member::Identifier(field)) => {
            format!("поле '{}'", field.name)
        }
        ConditionNode::Unresolved(crate::parser::ast::Condition::Variable(id)) => {
            format!("'{}'", id.name)
        }
        _ => "индексируемое значение".to_string(),
    }
}

/// Приводит ли база условия к массиву (фича 0358).
///
/// Условие — своё дерево, поэтому знание о цепочке места здесь **своё**, но
/// узкое: переменная массива либо поле структуры типа `[T; N]`. Ошибка в
/// сторону «не массив» громкая (`SE-117`), поэтому неизвестное считается
/// массивом — за пропуском стоят проверки целей и эталона.
pub(super) fn cond_base_is_array(base: &ConditionNode, model: &ModelNode) -> bool {
    match base {
        ConditionNode::Variable(var, _) => {
            matches!(&*var.borrow(), VariableNode::Simple { ty, .. }
                if matches!(ty, TypeNode::Array(..) | TypeNode::Inference))
        }
        ConditionNode::Parenthesis(inner) => cond_base_is_array(inner, model),
        ConditionNode::BitAccess(inner, crate::parser::ast::Member::Identifier(field)) => {
            match cond_field_type(inner, field, model) {
                Some(ty) => matches!(ty, TypeNode::Array(..) | TypeNode::Inference),
                None => true,
            }
        }
        // Неразрешённое имя индексировать нельзя: прежде ветвь искала
        // переменную и, не найдя, отвечала `SE-117` — поведение сохраняется.
        ConditionNode::Unresolved(_) => false,
        // Прочее (вызов, арифметика) — не место; судить не берёмся.
        _ => true,
    }
}

/// Тип поля `field` у базы условия, если он выводится.
fn cond_field_type(
    base: &ConditionNode,
    field: &crate::parser::ast::Identifier,
    model: &ModelNode,
) -> Option<TypeNode> {
    let base_ty = match base {
        ConditionNode::Variable(var, _) => match &*var.borrow() {
            VariableNode::Simple { ty, .. } => ty.clone(),
            _ => return None,
        },
        ConditionNode::Parenthesis(inner) => return cond_field_type(inner, field, model),
        _ => return None,
    };
    let TypeNode::Struct(name) = base_ty else {
        return None;
    };
    model
        .search_struct(&name)?
        .fields
        .iter()
        .find(|(f, _)| *f == field.name)
        .map(|(_, t)| t.clone())
}
