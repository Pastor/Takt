//! Число элементов агрегата обязано отвечать объявлению — `SE-123`
//! (фича 0320).
//!
//! # Что было
//!
//! Длину агрегата не сверял с объявлением **никто**, и один вход означал разное.
//! Замер 2026-08-20 на `var a: [u8; 2] := {1, 2, 3};`:
//!
//! | Потребитель | Ответ |
//! |---|---|
//! | эталон | хранит **три** элемента в двухэлементном массиве |
//! | **`rust`** | печатает `a: [1, 2, 3]` в поле `[u8; 2]` — **`E0308`** при нулевом коде возврата |
//! | **`st`, `st-at`** | **теряют инициализатор молча** (`a : ARRAY [0..1] OF USINT;`) |
//! | `c`, `c-hal` | `CC-017` |
//! | `sv`, `sv-mmio` | `SV-002` |
//!
//! Недостача хуже: `var a: [u8; 2] := {1};` — эталон падает `SIM-010` **в
//! такте** (индекс вне границ), а `rust` печатает `a: [1]`.
//!
//! Та же беда у структуры: `var g: Gains := {2, 3, 4};` при двух полях — эталон
//! строит **массив** и отвечает `SIM-012`, цели `c` и `st` переводят.
//!
//! # Почему запрет
//!
//! Общего поведения у записи не существует: усечь (как хочет `st`), расширить
//! (как делает эталон) или отказать (как `c`) — три разных языка. Отказ
//! приводит девять потребителей к одному ответу.

use super::*;
use crate::semantic::type_node::TypeNode;

/// Проверяет длину агрегатов в инициализаторах объявлений модели.
pub(super) fn check_aggregate_lengths(model: Rc<RefCell<ModelNode>>) -> Vec<Diagnostic> {
    let (vars, structs) = {
        let borrowed = model.borrow();
        (
            borrowed.variables.values().cloned().collect::<Vec<_>>(),
            borrowed.structs.clone(),
        )
    };
    // Накопление по объявлениям (правило 0151): каждое объявление высказывается.
    let mut found = Vec::new();
    for var in &vars {
        let (Some(ty), Some(expr)) = (declared_type(var), initializer(var)) else {
            continue;
        };
        let (ExpressionNode::Array(items) | ExpressionNode::Initializer(items)) = expr else {
            continue;
        };
        let name = var.name();
        match ty {
            TypeNode::Array(size, _) => {
                let expected = usize::from(size);
                if items.len() != expected {
                    found.push(mismatch(
                        var.loc(),
                        &format!("массив '{name}'"),
                        expected,
                        items.len(),
                    ));
                }
            }
            // Структура: «длина» — число объявленных полей. Порядок полей
            // значим (0034), поэтому недостача не может означать «остальные по
            // умолчанию».
            TypeNode::Struct(struct_name) => {
                let Some(def) = structs.get(&struct_name) else {
                    continue;
                };
                if items.len() != def.fields.len() {
                    found.push(mismatch(
                        var.loc(),
                        &format!("структура '{name}' типа '{struct_name}'"),
                        def.fields.len(),
                        items.len(),
                    ));
                }
            }
            _ => {}
        }
    }
    found
}

/// Диагностика `SE-123`: называет вид, объявленное и переданное числа.
fn mismatch(loc: Location, what: &str, expected: usize, got: usize) -> Diagnostic {
    Diagnostic::error(
        loc,
        format!(
            "{what}: объявлено элементов {expected}, в инициализаторе {got}. Прежде такая \
             запись означала разное: эталон хранил лишние, цель 'rust' печатала невалидный \
             код, а 'st' теряла инициализатор без единого слова"
        ),
    )
    .with_code("SE-123")
}

/// Объявленный тип переменной либо константы.
fn declared_type(var: &VariableNode) -> Option<TypeNode> {
    match var {
        VariableNode::Simple { ty, .. } | VariableNode::Const { ty, .. } => Some(ty.clone()),
        // Порт агрегатом не инициализируется: его начальное значение —
        // скаляр (0187), и правило здесь ни при чём.
        VariableNode::Port { .. } | VariableNode::Unresolved => None,
    }
}

/// Инициализатор объявления, если он есть.
fn initializer(var: &VariableNode) -> Option<&ExpressionNode> {
    match var {
        VariableNode::Simple { expr, .. } | VariableNode::Const { expr, .. } => Some(expr),
        VariableNode::Port { .. } | VariableNode::Unresolved => None,
    }
}
