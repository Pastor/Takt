//! Умолчание значения ТИПА как выражение семантики (фича 0466).
//!
//! # Зачем носитель
//!
//! Умолчание значения знали только цели, и каждая по-своему: `c` раскладывает
//! нули (`c_zero_init`, 0353), `rust` строит `default_value` (0351), `sv`
//! печатает `reset_literal` (0365). Пока умолчание требовалось лишь при
//! объявлении, это было верно: там форма — свойство целевого языка.
//!
//! Guard границ массива (0433) добавил позицию, где значение нужно **в
//! семантике**: оборачивая `return d[i];`, проход обязан сказать, что функция
//! вернёт, когда индекс вышел за границу. Печатники целей о guard не знают
//! вовсе — знание обязано быть выражением дерева.
//!
//! ⚠️ Носитель отдаёт **выражение**, а не текст: форму печатает цель, как и
//! прежде. Перечисление приходит числом — цели восстанавливают мнемонику по
//! типу приёмника (0167, 0281).
//!
//! ⚠️ `None` — «умолчания у типа нет», и это не ошибка: вход с таким типом
//! проход оставляет неизменным, называя границу. Так ведут себя `Inference`
//! (тип ещё не выведен) и служебные типы.

use std::cell::RefCell;
use std::rc::Rc;

use crate::semantic::enum_node::enum_default;
use crate::semantic::type_node::TypeNode;
use crate::semantic::{ExpressionNode, ModelNode};

/// Умолчание значения типа `ty` в области видимости модели `model`.
///
/// Умолчание **числовое** у скаляров (`0`/`false`) и **позиционный агрегат** у
/// массива и структуры — тот же порядок, что у инициализатора автора (0034).
pub fn default_expression(ty: &TypeNode, model: &ModelNode) -> Option<ExpressionNode> {
    match ty {
        // Скаляры: ноль представим у всех целей и означает одно и то же.
        TypeNode::Bit | TypeNode::Integer { .. } | TypeNode::Duration => {
            Some(ExpressionNode::Number(0))
        }
        TypeNode::Bool => Some(ExpressionNode::Bool(false)),
        // q(m, n): представление нуля — сам ноль при любом масштабе (0317).
        TypeNode::Fixed { .. } => Some(ExpressionNode::Number(0)),
        // Вещественное: ноль печатается литералом целевого языка.
        TypeNode::Rational => Some(ExpressionNode::Number(0)),
        // Перечисление: ПЕРВЫЙ ПО ТЕКСТУ вариант — решение заказчика 0391,
        // ноль может не принадлежать набору вовсе.
        TypeNode::Enum(name) => {
            let def = model.enums.get(name)?;
            let (_, value) = enum_default(&def.variants)?;
            Some(ExpressionNode::Number(value))
        }
        // Массив и структура: агрегат из умолчаний элементов, в объявленном
        // порядке — длина сверяется семантикой (`SE-123`, 0320).
        TypeNode::Array(len, elem) => {
            let one = default_expression(elem, model)?;
            Some(ExpressionNode::Initializer(vec![one; *len as usize]))
        }
        TypeNode::Struct(name) => {
            let def = model.structs.get(name)?;
            let mut fields = Vec::with_capacity(def.fields.len());
            for (_, field_ty) in &def.fields {
                fields.push(default_expression(field_ty, model)?);
            }
            Some(ExpressionNode::Initializer(fields))
        }
        // Умолчания нет: тип не выведен, служебный либо адресный.
        TypeNode::Inference
        | TypeNode::Address(_, _)
        | TypeNode::Unsupported
        | TypeNode::Unit
        | TypeNode::BuiltinString
        | TypeNode::BuiltinModel
        | TypeNode::BuiltinState
        | TypeNode::BuiltinNumeric => None,
    }
}

/// То же по ссылке на ячейку модели — удобство для проходов, держащих `Rc`.
pub fn default_of(ty: &TypeNode, model: &Rc<RefCell<ModelNode>>) -> Option<ExpressionNode> {
    default_expression(ty, &model.borrow())
}
