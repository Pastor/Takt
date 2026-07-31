//! Начальное значение порта: где оно законно и что с ним пока происходит
//! (фича 0187, задача 02).
//!
//! ## Две проверки
//!
//! - **`SE-092`** — начальное значение у **входного** порта. Значение входа
//!   приходит извне: датчик, регистр, соседнее устройство. Задавать его нечем и
//!   незачем, а запись `in P: u8 := 5;` почти наверняка означает, что автор
//!   принял `:=` за адрес (до фичи 0187 так и было).
//! - **`SE-093`** — временное: значение **пока не эмитируется** целями. Хранить
//!   его семантика уже умеет, а выставлять до первого такта — работа задач
//!   0187-03…05. Молчать нельзя: программа собралась бы, а значение исчезло —
//!   ровно тот молчаливый пропуск, против которого заведена вся фича.
//!
//! ⚠️ `SE-093` **снимается** по мере реализации целей; его номер после этого не
//! переиспользуется (реестр диагностик, правило 0077).

use crate::diagnostics::Diagnostic;
use crate::semantic::{ExpressionNode, ModelNode, PortDirection, VariableNode};
use std::cell::RefCell;
use std::rc::Rc;

/// `SE-092`: начальное значение у входного порта — **ошибка**.
pub(super) fn check_port_initializers(model: Rc<RefCell<ModelNode>>) -> Vec<Diagnostic> {
    let mut found = Vec::new();
    collect(&model, &mut found, Kind::Error);
    found
}

/// `SE-093`: начальное значение выходного порта пока не эмитируется —
/// **предупреждение**.
///
/// ⚠️ Живёт отдельной функцией, потому что `validate_model_all` собирает
/// **ошибки**: предупреждение, положенное туда, печаталось бы как «Ошибка
/// компиляции». Единая точка предупреждений — `semantic::warnings`.
pub(crate) fn port_init_warnings(model: Rc<RefCell<ModelNode>>) -> Vec<Diagnostic> {
    let mut found = Vec::new();
    collect(&model, &mut found, Kind::Warning);
    found
}

/// Что именно собирает обход за один проход.
#[derive(Clone, Copy, PartialEq, Eq)]
enum Kind {
    /// Только `SE-092` (вход с начальным значением).
    Error,
    /// Только `SE-093` (значение выхода пока не эмитируется).
    Warning,
}

fn collect(model: &Rc<RefCell<ModelNode>>, out: &mut Vec<Diagnostic>, kind: Kind) {
    let (vars, nested) = {
        let b = model.borrow();
        (
            b.variables.values().cloned().collect::<Vec<_>>(),
            b.models.values().map(Rc::clone).collect::<Vec<_>>(),
        )
    };
    for var in &vars {
        let VariableNode::Port {
            init,
            direction,
            name,
            loc,
            ..
        } = var
        else {
            continue;
        };
        if matches!(init, ExpressionNode::None) {
            continue;
        }
        if *direction == PortDirection::In {
            if kind != Kind::Error {
                continue;
            }
            out.push(
                Diagnostic::error(
                    *loc,
                    format!(
                        "входной порт '{name}' не может иметь начального значения: значение \
                         входа приходит извне. Если имелся в виду адрес, укажите его \
                         размещением — `at <адрес>`"
                    ),
                )
                .with_code("SE-092"),
            );
            continue;
        }
        if kind != Kind::Warning {
            continue;
        }
        out.push(
            Diagnostic::warning(
                *loc,
                format!(
                    "начальное значение порта '{name}' пока не выставляется целями генерации: \
                     значение сохранено, но до первого такта порт его не получит"
                ),
            )
            .with_code("SE-093"),
        );
    }
    for child in &nested {
        collect(child, out, kind);
    }
}
