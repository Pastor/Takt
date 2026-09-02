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
    // Присваивание СРЕЗА печатается тем же поэлементным путём (фича 0355):
    // массив в C не присваивается, а `memcpy` потребовал бы `<string.h>` в
    // заголовке, который цель не подключает. Границы — литералы, проверенные
    // `SE-029`, поэтому длина известна и цикла не нужно.
    if let ExpressionNode::ArraySlice(src, from, to) = value.as_ref() {
        return emit_slice(
            printer, map, owner, params, target, src, *from, *to, has_model,
        );
    }
    // Массив копируется ПОЭЛЕМЕНТНО и тогда, когда справа не литерал (0490):
    // `seen := src;` при массивах печаталось как `model->seen = model->src;` —
    // в C массив не присваивается вовсе, и `cc` отвечает «array type … is not
    // assignable» при НУЛЕВОМ коде возврата `taktc`. Замер 2026-09-02: тот же
    // вход отвергают `st` и `st-at` (`iec2c`: «Incompatible data types»), а
    // `rust` и `sv` переводят — то есть потребители расходились.
    //
    // ⚠️ Структура из объёма исключена намеренно: в C она присваивается, и
    // замер это подтверждает (её принимают все восемь потребителей).
    // ⚠️ Справа обязана стоять ПЕРЕМЕННАЯ: только её можно индексировать.
    // Результат вызова поднимает во временную свой проход (0431/0432), и
    // разворот здесь ломал бы уже починенное.
    if matches!(value.as_ref(), ExpressionNode::Variable(_)) {
        return emit_array_copy(printer, map, owner, params, target, value, has_model);
    }
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
    // Агрегат раскрывается ДО ЛИСТЬЕВ общим носителем (фича 0366): рекурсия
    // здесь была копией той, что живёт в инициализации переменной модели, — а
    // правило одно, и печатали его две цели из четырёх.
    let fields_of = |name: &str| {
        owner_model
            .as_ref()
            .and_then(std::rc::Weak::upgrade)
            .and_then(|model| model.borrow().search_struct(name))
            .map(|def| def.fields)
    };
    for leaf in crate::generator::aggregate::leaves(Some(&ty), items, &fields_of) {
        let mut rhs = String::new();
        {
            let mut tmp = Printer::new(4, &mut rhs);
            generate_expr(
                &mut tmp,
                map,
                owner,
                params.clone(),
                leaf.value,
                0,
                has_model,
            )?;
        }
        let suffix = crate::generator::aggregate::c_like_suffix(&leaf.path);
        printer.ident(&format!("{base}{suffix} = {rhs};")).nl();
    }
    Ok(true)
}

/// Копирует массив поэлементно, когда справа не литерал (фича 0490).
///
/// Возвращает `false`, если приёмник массивом не является: тогда печатает
/// вызывающий обычным путём.
///
/// ⚠️ Бит-вектор `[bit;N≤64]` — упакованный СКАЛЯР (0078), он присваивается
/// целиком; поэлементная печать сделала бы из него массив, которого в выводе
/// нет.
fn emit_array_copy(
    printer: &mut Printer,
    map: &CMap,
    owner: &Element,
    params: Vec<(String, TypeNode)>,
    target: &ExpressionNode,
    value: &ExpressionNode,
    has_model: bool,
) -> Result<bool, Diagnostic> {
    let ExpressionNode::Variable(var) = target else {
        return Ok(false);
    };
    let ty = match &*var.borrow() {
        VariableNode::Simple { ty, .. } => ty.clone(),
        _ => return Ok(false),
    };
    let TypeNode::Array(count, _) = &ty else {
        return Ok(false);
    };
    if crate::semantic::bit_vector::is_bit_vector(&ty).is_some() {
        return Ok(false);
    }
    let count = *count as usize;
    let mut base = String::new();
    {
        let mut tmp = Printer::new(4, &mut base);
        generate_expr(&mut tmp, map, owner, params.clone(), target, 0, has_model)?;
    }
    let mut rhs = String::new();
    {
        let mut tmp = Printer::new(4, &mut rhs);
        generate_expr(&mut tmp, map, owner, params.clone(), value, 0, has_model)?;
    }
    for place in crate::generator::aggregate::places(None, Some(&ty), count) {
        printer
            .ident(&format!("{base}{} = {rhs}{};", place.suffix, place.suffix))
            .nl();
    }
    Ok(true)
}

/// Печатает присваивание среза поэлементно (фича 0355).
///
/// База обоих операндов берётся у общего печатника выражений: она бывает
/// `model->a`, `main->a` и просто `a`, и второй копии правила выбора здесь
/// быть не должно.
#[allow(clippy::too_many_arguments)]
fn emit_slice(
    printer: &mut Printer,
    map: &CMap,
    owner: &Element,
    params: Vec<(String, TypeNode)>,
    dst: &ExpressionNode,
    src: &ExpressionNode,
    from: Option<i128>,
    to: Option<i128>,
    has_model: bool,
) -> Result<bool, Diagnostic> {
    // Пригодны ОБА операнда: приёмник тоже обязан быть настоящим массивом.
    // `res := mem[1:2];` при `res: u8` эталон не исполняет (`SIM-006`), а
    // поэлементная печать дала бы `model->res[0] = …` над скаляром.
    //
    // Тип базы даёт общий носитель (фича 0358): она теперь выражение, а не
    // переменная, и `b.data[0:2]` разбирается тем же правилом.
    let Element::Model {
        name: owner_name, ..
    } = owner
    else {
        return Ok(false);
    };
    let model_rc = map.raw_model_at(owner_name.clone())?;
    let model_ref = model_rc.borrow();
    let dst_ok = crate::generator::slice::elementwise_len_of(dst, &model_ref).is_some();
    let src_len = if dst_ok {
        crate::generator::slice::elementwise_len_of(src, &model_ref)
    } else {
        None
    };
    // Непригодный операнд отдаётся ПРЕЖНЕМУ пути (`Ok(false)`), а не отвергается
    // здесь: отказ `CC-022` строит общий печатник выражений, и координату
    // оператора ему даёт `site::at` (фича 0277). Свой отказ пришёл бы с
    // позицией объявления — то есть с чужой верной координатой (класс 0264).
    let Some(src_len) = src_len else {
        return Ok(false);
    };
    let (start, len) = crate::generator::slice::bounds(from, to, src_len);
    let base_of = |node: &ExpressionNode| -> Result<String, Diagnostic> {
        let mut text = String::new();
        let mut tmp = Printer::new(4, &mut text);
        generate_expr(&mut tmp, map, owner, params.clone(), node, 0, has_model)?;
        Ok(text)
    };
    let dst_base = base_of(dst)?;
    let src_base = base_of(src)?;
    for k in 0..len {
        printer
            .ident(&format!("{dst_base}[{k}] = {src_base}[{}];", start + k))
            .nl();
    }
    Ok(true)
}
