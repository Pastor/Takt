//! Присваивание АГРЕГАТА в теле — цель `c` (фича 0340).
//!
//! # Что было
//!
//! `a := {3, 4};` печаталось как `model->a = {3, 4};` — в C такой формы нет
//! вовсе, и `cc` отвечает «expected expression» при **нулевом** коде возврата
//! `taktc` (класс 0262). Массив в C не присваивается даже составным литералом
//! (`(int[2]){…}` присвоить нельзя), структура — присваивается, но обе формы
//! выразимы **поэлементно**, и одна форма на оба случая проще двух.
//!
//! ⚠️ Фича 0330 чинила тот же класс у целей `st` и `sv` и утверждала в
//! комментарии, что поэлементная форма «совпадает с тем, что печатает цель
//! `c`». Замер 2026-08-20 это **опроверг** — цель печатала агрегат как есть.
//! Утверждение о чужом коде проверяется прогоном, а не чтением (класс 0292).

use super::*;

/// Печатает присваивание агрегата, если оператор им является.
///
/// Возвращает `false`, если узел — не присваивание агрегата: тогда печатает
/// вызывающий обычным путём.
pub(in crate::generator::c) fn emit(
    printer: &mut Printer,
    map: &CMap,
    owner: &Element,
    params: Vec<(String, TypeNode)>,
    expr: &ExpressionNode,
    has_model: bool,
) -> Result<bool, Diagnostic> {
    let ExpressionNode::Assign(target, value) = expr else {
        return Ok(false);
    };
    let (ExpressionNode::Initializer(items) | ExpressionNode::Array(items)) = value.as_ref() else {
        return Ok(false);
    };
    let ExpressionNode::Variable(var) = target.as_ref() else {
        return Ok(false);
    };
    let (ty, owner_model) = match &*var.borrow() {
        VariableNode::Simple { ty, upper, .. } => (ty.clone(), upper.clone()),
        _ => return Ok(false),
    };

    // Базу печатает общий печатник выражений: она бывает `model->a`,
    // `main->a` и просто `a` — правило выбора живёт там, и второй его копии
    // здесь быть не должно.
    let mut base = String::new();
    {
        let mut tmp = Printer::new(4, &mut base);
        generate_expr(
            &mut tmp,
            map,
            owner,
            params.clone(),
            target.as_ref(),
            0,
            has_model,
        )?;
    }

    // Поля структуры ищутся у ВЛАДЕЛЬЦА переменной: карта уровней их не
    // хранит, а объявление структуры видно оттуда же, откуда объявлена сама
    // переменная (`search_struct` поднимается по родителям).
    let fields = match &ty {
        TypeNode::Struct(name) => owner_model
            .as_ref()
            .and_then(std::rc::Weak::upgrade)
            .and_then(|model| model.borrow().search_struct(name))
            .map(|def| def.fields),
        _ => None,
    };
    let places = crate::generator::aggregate::places(fields.as_deref(), Some(&ty), items.len());
    for (item, place) in items.iter().zip(places) {
        let mut rhs = String::new();
        {
            let mut tmp = Printer::new(4, &mut rhs);
            generate_expr(&mut tmp, map, owner, params.clone(), item, 0, has_model)?;
        }
        printer
            .ident(&format!("{base}{} = {rhs};", place.suffix))
            .nl();
    }
    Ok(true)
}
