//! Разрешение условий на рёбрах `ref` семантического дерева BuT.
//!
//! Функция [`resolve_state_references`] выполняет дополнительный проход
//! разрешения условий для всех `ref`-переходов состояния: заменяет
//! [`ConditionNode::Unresolved`] полностью разрешёнными семантическими условиями
//! с помощью [`resolve_condition`].
//!
//! Родительская модель берётся непосредственно из узла состояния (`state.upper()`),
//! поэтому явная передача `model` в параметрах не требуется.

use crate::diagnostics::Diagnostic;
use crate::semantic::condition::resolve_condition;
use crate::semantic::{ConditionNode, ReferenceNode, StateNode};

/// Разрешает список условий `ref`-ссылок, заменяя [`ConditionNode::Unresolved`]
/// полностью разрешёнными семантическими условиями.
///
/// Контекст для разрешения берётся из [`StateNode::upper`] — ссылки на
/// родительскую модель, которая уже содержит переменные, условия и функции.
///
/// # Ошибки
///
/// Пробрасывает [`Diagnostic`] из [`resolve_condition`], если условие не
/// удаётся разрешить (например, неизвестная переменная или функция).
fn resolve_references(
    references: &[ReferenceNode<StateNode>],
    state: &StateNode,
) -> Result<Vec<ReferenceNode<StateNode>>, Diagnostic> {
    // Берём родительскую модель из узла состояния
    let model = match state.upper() {
        Some(m) => m,
        // Состояние без родителя: условия разрешить невозможно, возвращаем как есть
        None => return Ok(references.to_vec()),
    };

    let mut new_references = Vec::with_capacity(references.len());
    for reference in references {
        if let ConditionNode::Unresolved(cond) = reference.cond.clone() {
            let resolved_cond = resolve_condition(&cond, model.clone())?;
            let resolved_reference = ReferenceNode {
                cond: resolved_cond,
                ..reference.clone()
            };
            new_references.push(resolved_reference);
        } else {
            new_references.push(reference.clone());
        }
    }
    Ok(new_references)
}

/// Разрешает условия всех `ref`-переходов состояния.
///
/// Использует [`StateNode::upper`] для получения контекста модели,
/// что позволяет не передавать `model` как отдельный параметр.
///
/// Обрабатывает как [`StateNode::Simple`], так и [`StateNode::Implement`]
/// (включая поле `next`). Для [`StateNode::Unresolved`] возвращает состояние
/// без изменений.
pub fn resolve_state_references(state: &StateNode) -> Result<StateNode, Diagnostic> {
    match state {
        StateNode::Simple {
            upper,
            loc,
            references,
            name,
            named_blocks,
            kind,
        } => Ok(StateNode::Simple {
            upper: upper.clone(),
            loc: *loc,
            named_blocks: named_blocks.clone(),
            name: name.clone(),
            references: resolve_references(references, state)?,
            kind: kind.clone(),
        }),
        StateNode::Implement {
            upper,
            loc,
            references,
            name,
            named_blocks,
            implements,
            next,
            kind,
        } => Ok(StateNode::Implement {
            upper: upper.clone(),
            loc: *loc,
            named_blocks: named_blocks.clone(),
            name: name.clone(),
            references: resolve_references(references, state)?,
            implements: implements.clone(),
            next: next.clone(),
            kind: kind.clone(),
        }),
        other => Ok(other.clone()),
    }
}
