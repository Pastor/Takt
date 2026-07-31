//! Предупреждение `SE-091`: инициализатор порта — адрес, а не значение (фича 0176).
//!
//! ## Зачем
//!
//! По [ADR 0020](../../../docs/adr/0020-port-address-decl.md) инициализатор порта
//! задаёт **адрес**, а [0070](../../../docs/adr/0070-port-initializer-address-role.md)
//! сняла с портов проверку значения бита — то есть `out ready: bit := 0;`
//! законно и означает **адрес 0**. Автор при этом почти наверняка писал
//! «начальное значение»: та же форма у обычной переменной значит именно его.
//!
//! Цена ошибки — не косметическая. Цель `c-hal` печатает такому порту
//! `{ (uintptr_t)0x0u, 0, 1 }`, и дефолтный HAL пишет по **нулевому указателю**;
//! гейт `cc -c` этот код принимает, потому что синтаксически он безупречен. В
//! корпусе форма встречалась пять раз, в документе — ещё пять, причём рядом с
//! комментарием «датчик»: читателя учили ловушке.
//!
//! ## Почему предупреждение, а не ошибка
//!
//! Адрес 0 или 1 теоретически возможен (регистр по нулевому смещению), и делать
//! незаконной форму, валидную сегодня, ради подозрения нельзя (решение заказчика
//! 2026-07-31). Намерение выражается явно формой `0x0:бит` — узел `Address`, а не
//! `Number`, и предупреждение молчит.
//!
//! ## Почему в семантике, а не в слое адресов
//!
//! Слой адресов зовут только адрес-потребляющие цели (`c-hal`, `st-at`,
//! `sv-mmio`), а ловушка — свойство **исходника**: увидеть её должен и тот, кто
//! собирает целью `c` (примеры документа собираются именно так).

use crate::diagnostics::Diagnostic;
use crate::semantic::{ExpressionNode, ModelNode, VariableNode};
use std::cell::RefCell;
use std::rc::Rc;

/// Собирает `SE-091` по всем портам модели и её под-моделей.
pub fn check_port_address_looks_like_value(model: Rc<RefCell<ModelNode>>) -> Vec<Diagnostic> {
    let mut warnings = Vec::new();
    collect(&model, &mut warnings);
    warnings
}

fn collect(model: &Rc<RefCell<ModelNode>>, out: &mut Vec<Diagnostic>) {
    let borrowed = model.borrow();
    for var in borrowed.variables.values() {
        let VariableNode::Port {
            expr, loc, name, ..
        } = var
        else {
            continue;
        };
        // Только голое число 0/1. Форма `0x0:бит` даёт `ExpressionNode::Address`
        // — там намерение «это адрес» написано автором явно.
        let ExpressionNode::Number(value) = expr else {
            continue;
        };
        if !matches!(value, 0 | 1) {
            continue;
        }
        out.push(
            Diagnostic::warning(
                *loc,
                format!(
                    "инициализатор порта '{name}' задаёт АДРЕС, а не начальное значение: \
                     адрес {value} почти наверняка не то, что имелось в виду. Укажите \
                     настоящий адрес (`0xADDR` или `0xADDR:бит`) либо уберите \
                     инициализатор — начального значения у порта нет, его значение \
                     приходит извне"
                ),
            )
            .with_code("SE-091"),
        );
    }
    let nested: Vec<Rc<RefCell<ModelNode>>> = borrowed.models.values().map(Rc::clone).collect();
    drop(borrowed);
    for child in nested {
        collect(&child, out);
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn warnings_for(src: &str) -> Vec<Diagnostic> {
        let (ast, _) = crate::parse(src, 0).expect("исходник пробы разбирается");
        let model = crate::semantic::tree::construct_model(&ast, None, &[]).expect("модель");
        check_port_address_looks_like_value(model)
    }

    #[test]
    fn zero_initializer_on_port_is_reported() {
        let w = warnings_for(
            "out ready: bit := 0;\nvar n: u8 := 0;\nstart S { always { ready := 1; } ref S: n = 9; }\n",
        );
        assert_eq!(w.len(), 1, "адрес 0 обязан быть назван: {w:?}");
        assert_eq!(w[0].code.as_deref(), Some("SE-091"));
        assert!(
            w[0].message.contains("ready"),
            "сообщение обязано называть порт: {}",
            w[0].message
        );
    }

    #[test]
    fn one_initializer_on_port_is_reported_too() {
        let w = warnings_for("in btn: bit := 1;\nstart S { ref S: btn = 1; }\n");
        assert_eq!(w.len(), 1, "адрес 1 столь же подозрителен: {w:?}");
    }

    #[test]
    fn real_address_is_silent() {
        let w = warnings_for("in btn: bit := 0x100:3;\nstart S { ref S: btn = 1; }\n");
        assert!(w.is_empty(), "настоящий адрес — не повод для шума: {w:?}");
    }

    #[test]
    fn explicit_zero_address_with_bit_is_silent() {
        // `0x0:0` — узел `Address`: автор написал, что это адрес. Это и есть
        // способ заглушить предупреждение, названный в его тексте.
        let w = warnings_for("in btn: bit := 0x0:0;\nstart S { ref S: btn = 1; }\n");
        assert!(w.is_empty(), "явная адресная форма молчит: {w:?}");
    }

    #[test]
    fn plain_variable_is_not_a_port() {
        // У переменной инициализатор — значение, и это законно.
        let w = warnings_for("var flag: bit := 0;\nstart S { ref S: flag = 1; }\n");
        assert!(w.is_empty(), "переменная порта не касается: {w:?}");
    }

    #[test]
    fn nested_model_ports_are_covered() {
        let w = warnings_for(
            "model Child {\n  out ready: bit := 0;\n  var n: u8 := 0;\n  \
             start S { always { ready := 1; } ref S: n = 9; }\n}\n\
             start Root = Child;\n",
        );
        assert_eq!(w.len(), 1, "порт под-модели обязан проверяться: {w:?}");
    }
}
