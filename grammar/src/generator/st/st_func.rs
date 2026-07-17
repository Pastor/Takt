//! Печать функций Lam как `FUNCTION` IEC 61131-3.
//!
//! Задача 0041-04, часть 3. Дополняет `st_expr.rs` (выражения) и `st_stmt.rs`
//! (операторы).
//!
//! ## Возврат — присваивание имени функции
//!
//! В ST нет `return <значение>`: результат возвращается присваиванием **имени
//! функции**, а `RETURN;` лишь досрочно выходит. Поэтому `return a - b;` Lam →
//! `abs_diff := a - b; RETURN;` (форма проверена пробой на раннем возврате
//! `abs_diff`, `stacker.lam:100-103`).
//!
//! ## Три синтетические подпорки — и почему они неизбежны
//!
//! Пробы MatIEC (0041-04) вскрыли, что **`extern fn` Lam в стандартном ST
//! невыразим сразу по трём осям**. Все три бьют по `elevator.lam:93-115` (восемь
//! `extern fn` вида `motor_up();`). Ограничения — в **стандарте**, а не в
//! инструменте: `iec2c -h` сам называет послабления «a non-standard extension».
//!
//! | Препятствие | Факт | Подпорка |
//! |---|---|---|
//! | Функция без параметров | `-i : allow POUs with no in out and inout parameters (a non-standard extension!)`; пустой `VAR_INPUT` роняет `iec2c` **segfault**'ом | синтетический параметр [`SYNTHETIC_PARAM`]; вызов передаёт `0` |
//! | Функция, возвращающая `VOID` | `-b : allow functions returning VOID (a non-standard extension!)` | синтетический тип `USINT`, тело присваивает `0` |
//! | Вызов функции как оператор | `error: Function invocation in ST code is not allowed outside an expression` | присваивание в переменную-приёмник (`st_stmt`) |
//!
//! Подпорки выбраны так, чтобы вывод остался **стандартным ST**: альтернатива —
//! требовать от `iec2c` флагов `-i`/`-b`, но тогда порождённое перестанет
//! приниматься настоящим ПЛК, ради которого фича и делается.
//!
//! ## `extern fn` → заглушка + `ST-009`
//!
//! Тело внешней функции неизвестно, а `FUNCTION` без тела `iec2c` отвергает
//! («no body defined in function declaration», проба П9). Эмитится заглушка,
//! возвращающая нейтральное значение, **плюс предупреждение `ST-009`**: молчание
//! здесь дало бы ПЛК-код, который тихо ничего не делает вместо того, чтобы
//! крутить двигатель, — ровно класс дефекта фичи 0025.

use crate::diagnostics::{Diagnostic, Location};
use crate::generator::indent::Printer;
use crate::generator::st::st_expr::print_expression;
use crate::generator::st::st_stmt::{Hoisted, StmtOutput, print_statement};
use crate::generator::st::st_type::get_st_type;
use crate::semantic::type_node::TypeNode;
use crate::semantic::unused::{UsageSet, usage_from_stmt};
use crate::semantic::{
    ExpressionNode, FunctionDefinitionNode, ModelNode, StatementNode, VariableNode,
};
use std::cell::RefCell;
use std::rc::Rc;

/// Имя синтетического параметра у функции, объявленной без параметров.
const SYNTHETIC_PARAM: &str = "unused";

/// Тип, подставляемый вместо отсутствующего возвращаемого значения (`unit`)
/// и синтетическому параметру.
fn synthetic_type() -> TypeNode {
    TypeNode::Integer {
        bits: 8,
        signed: false,
    }
}

/// Возвращаемый тип функции для ST.
///
/// `unit` Lam → синтетический `USINT`: функции, возвращающие `VOID`, — не
/// стандарт (`iec2c -b`).
pub(crate) fn return_type_of(def: &FunctionDefinitionNode) -> TypeNode {
    match def {
        FunctionDefinitionNode::Local { ret, .. }
        | FunctionDefinitionNode::External { ret, .. } => {
            if matches!(ret, TypeNode::Unit) {
                synthetic_type()
            } else {
                ret.clone()
            }
        }
        FunctionDefinitionNode::Builtin(_, _, ret) => ret.clone(),
        FunctionDefinitionNode::None | FunctionDefinitionNode::Unresolved(_) => synthetic_type(),
    }
}

/// Имя функции.
fn name_of(def: &FunctionDefinitionNode) -> Option<&str> {
    match def {
        FunctionDefinitionNode::Local { name, .. }
        | FunctionDefinitionNode::External { name, .. } => Some(name),
        FunctionDefinitionNode::Builtin(name, _, _) => Some(name),
        FunctionDefinitionNode::None | FunctionDefinitionNode::Unresolved(_) => None,
    }
}

/// Параметры функции.
fn params_of(def: &FunctionDefinitionNode) -> Vec<(String, TypeNode)> {
    match def {
        FunctionDefinitionNode::Local { params, .. }
        | FunctionDefinitionNode::External { params, .. } => params.clone(),
        FunctionDefinitionNode::Builtin(_, params, _) => params
            .iter()
            .map(|(n, t)| (n.to_string(), t.clone()))
            .collect(),
        FunctionDefinitionNode::None | FunctionDefinitionNode::Unresolved(_) => Vec::new(),
    }
}

/// Переменные модели, которые функция читает или пишет в своём теле.
///
/// **Зачем.** В цели `c` функция получает первым параметром указатель на модель
/// (`static uint8_t Stacker_travel_time(const Stacker *model, …)`) и читает через
/// него порты и переменные (`stacker.c:29-56`). В IEC 61131-3 `FUNCTION` —
/// **чистая**: она видит только свои `VAR_INPUT`/`VAR_IN_OUT` и к переменным
/// вызывающего `FUNCTION_BLOCK` доступа не имеет. Гейт `iec2c` поймал это на
/// `travel_time`, который читает порт корня `pos_stack`:
/// «Ambiguous enumerate value or Variable not declared in this scope».
///
/// Поэтому такие переменные передаются функции по ссылке — `VAR_IN_OUT`, форма
/// проверена пробой (✅). Список — **единый источник истины** для объявления и
/// для аргументов вызова: разойдись они, ST либо не соберётся, либо свяжет не те
/// переменные.
///
/// Константы сюда **не** попадают: они неизменны и объявляются `VAR CONSTANT`
/// внутри самой функции (форма тоже проверена пробой).
pub(crate) fn state_params(
    def: &FunctionDefinitionNode,
    model: &ModelNode,
) -> Vec<(String, TypeNode)> {
    let FunctionDefinitionNode::Local { body, params, .. } = def else {
        return Vec::new();
    };
    let mut set = UsageSet::default();
    usage_from_stmt(body, &mut set);

    // Локальные объявления тела — не состояние модели.
    let mut locals: Vec<String> = params.iter().map(|(n, _)| n.clone()).collect();
    collect_locals(body, &mut locals);

    let mut out = Vec::new();
    let mut names: Vec<&String> = model.variables.keys().collect();
    names.sort();
    for name in names {
        if locals.contains(name) {
            continue;
        }
        if !set.variables.contains(name) && !set.ports.contains(name) {
            continue;
        }
        let (VariableNode::Simple { ty, .. } | VariableNode::Port { ty, .. }) =
            &model.variables[name]
        else {
            continue;
        };
        out.push((name.clone(), ty.clone()));
    }
    out
}

/// Собирает имена переменных, объявленных внутри тела.
fn collect_locals(stmt: &StatementNode, out: &mut Vec<String>) {
    match stmt {
        StatementNode::Variable(name, _, _) => out.push(name.clone()),
        StatementNode::Block(items) => items.iter().for_each(|s| collect_locals(s, out)),
        StatementNode::If { then_, else_, .. } => {
            collect_locals(then_, out);
            if let Some(e) = else_ {
                collect_locals(e, out);
            }
        }
        StatementNode::Loop { body, .. } => collect_locals(body, out),
        StatementNode::For { init, body, .. } => {
            if let Some(i) = init {
                collect_locals(i, out);
            }
            collect_locals(body, out);
        }
        StatementNode::Match { arms, .. } => arms.iter().for_each(|a| collect_locals(&a.body, out)),
        StatementNode::None
        | StatementNode::Unresolved(_)
        | StatementNode::Expression(_)
        | StatementNode::Return(_)
        | StatementNode::Continue
        | StatementNode::Break
        | StatementNode::InlineFormula(_) => {}
    }
}

/// Константы модели, которые функция использует.
///
/// Объявляются `VAR CONSTANT` внутри самой функции: `FUNCTION` чистая, а
/// константа неизменна — дублировать её дешевле, чем плести через параметры.
fn const_params(def: &FunctionDefinitionNode, model: &ModelNode) -> Vec<String> {
    let FunctionDefinitionNode::Local { body, .. } = def else {
        return Vec::new();
    };
    let mut set = UsageSet::default();
    usage_from_stmt(body, &mut set);
    let mut names: Vec<String> = set
        .constants
        .iter()
        .filter(|n| model.variables.contains_key(*n))
        .cloned()
        .collect();
    names.sort();
    names
}

/// Печатает вызов функции как выражение ST.
///
/// У функции, объявленной без параметров, есть синтетический параметр, поэтому
/// вызов передаёт `0` — иначе `iec2c` ответит «no parameter defined in function
/// invocation».
///
/// # Ошибки
/// `ST-011` — функция не разрешена либо аргумент не транслируется.
pub(crate) fn print_call(
    def: &Rc<RefCell<FunctionDefinitionNode>>,
    args: &[ExpressionNode],
    model: &ModelNode,
) -> Result<String, Diagnostic> {
    let mut printed = Vec::new();
    for arg in args {
        printed.push(print_expression(arg, model)?);
    }
    print_call_in(def, &printed, model)
}

/// Печатает вызов по уже напечатанным аргументам.
///
/// Отдельный вход нужен печатнику условий: у `ConditionNode` своя грамматика
/// (инвариант ADR 0019), и её аргументы печатает он сам.
pub(crate) fn print_call_texts(
    def: &Rc<RefCell<FunctionDefinitionNode>>,
    args: &[String],
    model: &ModelNode,
) -> Result<String, Diagnostic> {
    print_call_in(def, args, model)
}

/// Общая печать вызова: объявленные аргументы плюс переменные состояния.
fn print_call_in(
    def_rc: &Rc<RefCell<FunctionDefinitionNode>>,
    args: &[String],
    model: &ModelNode,
) -> Result<String, Diagnostic> {
    let def = def_rc.borrow();
    let name = name_of(&def)
        .ok_or_else(|| unsupported("вызов неразрешённой функции (определение отсутствует)"))?;
    let mut printed: Vec<String> = args.to_vec();
    if printed.is_empty() && state_params(&def, model).is_empty() {
        // Синтетический параметр требует синтетического аргумента.
        printed.push("0".to_string());
    }
    // Переменные состояния идут ПОСЛЕ объявленных — тем же порядком, что в
    // `VAR_IN_OUT` функции: список общий (единый источник истины).
    for (var, _) in state_params(&def, model) {
        printed.push(var);
    }
    Ok(format!("{}({})", name, printed.join(", ")))
}

/// Печатает все функции моделей как `FUNCTION … END_FUNCTION`.
///
/// Функции печатаются **до** `FUNCTION_BLOCK`, которые их вызывают: опережающие
/// ссылки в ST — нестандартное расширение (`iec2c -p`).
///
/// Возвращает предупреждения `ST-009` по каждой `extern fn`.
///
/// # Ошибки
/// `ST-011`/`ST-002` — тело или тип функции не транслируются.
pub(crate) fn emit_functions(
    p: &mut Printer,
    models: &[(crate::semantic::minimap::Name, Rc<RefCell<ModelNode>>)],
) -> Result<Vec<Diagnostic>, Diagnostic> {
    let mut warnings = Vec::new();
    // Пространство имён функций в IEC — плоское, поэтому дедупликация по имени.
    // Одноимённые функции разных моделей столкнулись бы; столкновение поймает
    // `iec2c` («duplicate»), то есть громко, а не молча.
    let mut emitted: Vec<String> = Vec::new();

    for (_, model_rc) in models {
        let model = &*model_rc.borrow();
        let mut names: Vec<&String> = model.functions.keys().collect();
        names.sort();
        for key in names {
            let def = &model.functions[key];
            let Some(name) = name_of(def) else {
                continue;
            };
            if emitted.iter().any(|n| n == name) {
                continue;
            }
            emitted.push(name.to_string());
            emit_function(p, def, model, &mut warnings)?;
        }
    }
    Ok(warnings)
}

/// Печатает одну функцию.
fn emit_function(
    p: &mut Printer,
    def: &FunctionDefinitionNode,
    model: &ModelNode,
    warnings: &mut Vec<Diagnostic>,
) -> Result<(), Diagnostic> {
    let Some(name) = name_of(def) else {
        return Ok(());
    };
    // Встроенные функции языка предоставляет сам компилятор ST — не эмитим.
    if matches!(def, FunctionDefinitionNode::Builtin(_, _, _)) {
        return Ok(());
    }
    let ret_ty = get_st_type(&return_type_of(def), model)?;
    p.ident(&format!("FUNCTION {} : {}", name, ret_ty)).nl();

    // Параметры. Пустой `VAR_INPUT … END_VAR` недопустим (и роняет iec2c
    // segfault'ом), поэтому у беспараметрической функции — синтетический вход.
    let mut params = params_of(def);
    if params.is_empty() && state_params(def, model).is_empty() {
        params.push((SYNTHETIC_PARAM.to_string(), synthetic_type()));
    }
    // Пустой `VAR_INPUT … END_VAR` недопустим (и роняет iec2c segfault'ом):
    // если параметров нет, но есть состояние, секция просто не печатается.
    if !params.is_empty() {
        p.ident("VAR_INPUT").nl();
        p.up();
        for (pname, pty) in &params {
            let ty = get_st_type(pty, model)?;
            p.ident(&format!("{} : {};", pname, ty)).nl();
        }
        p.down();
        p.ident("END_VAR").nl();
    }

    // Переменные модели, которые тело трогает: `FUNCTION` в IEC чистая, поэтому
    // они передаются по ссылке (см. `state_params`).
    let state = state_params(def, model);
    if !state.is_empty() {
        p.ident("VAR_IN_OUT").nl();
        p.up();
        for (vname, vty) in &state {
            let ty = get_st_type(vty, model)?;
            p.ident(&format!("{} : {};", vname, ty)).nl();
        }
        p.down();
        p.ident("END_VAR").nl();
    }
    // Локальные объявления тела. Печатаются ДО `VAR CONSTANT` — и это не стиль:
    // MatIEC протаскивает квалификатор `CONSTANT` предыдущей секции на следующий
    // за ней `VAR`, из-за чего присваивание локальной переменной отвергается
    // («Assignment to CONSTANT variables is not allowed»). Проверено пробой:
    // порядок `VAR CONSTANT` → `VAR` невалиден, `VAR` → `VAR CONSTANT` валиден.
    // Дефект вскрыт фичей 0030: до неё ни одна функция корпуса не имела
    // локальных переменных, и порядок ничего не ломал.
    let hoisted = match def {
        FunctionDefinitionNode::Local { body, .. } => collect_hoisted(body, name, model)?,
        // Тела нет: у `extern fn` печатается заглушка (ниже), у остальных —
        // печатать нечего, поднимать тоже нечего.
        FunctionDefinitionNode::External { .. }
        | FunctionDefinitionNode::Builtin(_, _, _)
        | FunctionDefinitionNode::None
        | FunctionDefinitionNode::Unresolved(_) => Vec::new(),
    };
    emit_hoisted_var(p, &hoisted, model)?;

    // Константы модели дублируются внутрь функции.
    let consts = const_params(def, model);
    if !consts.is_empty() {
        p.ident("VAR CONSTANT").nl();
        p.up();
        for cname in &consts {
            let VariableNode::Const { ty, expr, .. } = &model.variables[cname] else {
                continue;
            };
            let ty_name = get_st_type(ty, model)?;
            let init = crate::generator::st::st_decl::literal_init(expr, ty);
            match init {
                Some(v) => p.ident(&format!("{} : {} := {};", cname, ty_name, v)).nl(),
                None => {
                    return Err(unsupported(&format!(
                        "константа '{}' с невычислимым инициализатором внутри функции",
                        cname
                    )));
                }
            };
        }
        p.down();
        p.ident("END_VAR").nl();
    }

    match def {
        FunctionDefinitionNode::External { .. } => {
            warnings.push(
                Diagnostic::warning(
                    Location::Codegen,
                    format!(
                        "Внешняя функция '{}': тело неизвестно, а IEC 61131-3 требует \
                         его от FUNCTION. Эмитирована заглушка, возвращающая {} — в \
                         ПЛК она НИЧЕГО НЕ СДЕЛАЕТ. Замените её реализацией вручную",
                        name,
                        neutral_value(&return_type_of(def), model)?
                    ),
                )
                .with_code("ST-009"),
            );
            p.up();
            let neutral = neutral_value(&return_type_of(def), model)?;
            p.ident(&format!("(* extern fn {} — заглушка (ST-009) *)", name))
                .nl();
            p.ident(&format!("{} := {};", name, neutral)).nl();
            p.down();
        }
        FunctionDefinitionNode::Local { body, ret, .. } => {
            emit_local_body(p, name, body, ret, model)?;
        }
        FunctionDefinitionNode::Builtin(_, _, _)
        | FunctionDefinitionNode::None
        | FunctionDefinitionNode::Unresolved(_) => {}
    }
    p.ident("END_FUNCTION").nl().nl();
    Ok(())
}

/// Собирает объявления, поднимаемые из тела функции в её секцию `VAR`.
///
/// Тело печатается «вхолостую»: в ST объявления обязаны стоять ДО тела, поэтому
/// узнать их состав можно только пройдя тело заранее.
fn collect_hoisted(
    body: &crate::semantic::StatementNode,
    name: &str,
    model: &ModelNode,
) -> Result<Vec<Hoisted>, Diagnostic> {
    let mut probe = String::new();
    let mut out = StmtOutput::default();
    {
        let mut probe_p = Printer::new(4, &mut probe);
        print_statement(body, model, &mut probe_p, &mut out, Some(name))?;
    }
    Ok(out.hoisted)
}

/// Печатает секцию `VAR` поднятых объявлений (пустую — не печатает: пустой
/// `VAR … END_VAR` для `iec2c` невалиден).
fn emit_hoisted_var(
    p: &mut Printer,
    hoisted: &[Hoisted],
    model: &ModelNode,
) -> Result<(), Diagnostic> {
    if hoisted.is_empty() {
        return Ok(());
    }
    p.ident("VAR").nl();
    p.up();
    let mut seen: Vec<&str> = Vec::new();
    for h in hoisted {
        if seen.contains(&h.name.as_str()) {
            continue;
        }
        seen.push(&h.name);
        let ty = get_st_type(&h.ty, model)?;
        p.ident(&format!("{} : {};", h.name, ty)).nl();
    }
    p.down();
    p.ident("END_VAR").nl();
    Ok(())
}

/// Печатает тело локальной функции. Секцию `VAR` печатает вызывающий — до
/// `VAR CONSTANT` (см. `emit_function`).
fn emit_local_body(
    p: &mut Printer,
    name: &str,
    body: &crate::semantic::StatementNode,
    ret: &TypeNode,
    model: &ModelNode,
) -> Result<(), Diagnostic> {
    p.up();
    let mut out2 = StmtOutput::default();
    print_statement(body, model, p, &mut out2, Some(name))?;
    // Функция, объявленная как `unit`, значение не возвращает — но в ST тип
    // синтетический, поэтому имя обязано быть присвоено хотя бы раз.
    if matches!(ret, TypeNode::Unit) {
        let neutral = neutral_value(&synthetic_type(), model)?;
        p.ident(&format!("{} := {};", name, neutral)).nl();
    }
    p.down();
    Ok(())
}

/// Нейтральное значение типа — для заглушек и синтетических возвратов.
fn neutral_value(ty: &TypeNode, model: &ModelNode) -> Result<String, Diagnostic> {
    Ok(match ty {
        TypeNode::Bit | TypeNode::Bool => "FALSE".to_string(),
        TypeNode::Rational => "0.0".to_string(),
        TypeNode::Integer { .. } => "0".to_string(),
        // Для прочих типов нейтральное значение не очевидно — лучше отказ, чем
        // выдумка: заглушка и так подменяет поведение, молча угадывать нельзя.
        _ => {
            return Err(unsupported(&format!(
                "нейтральное значение для типа '{}' (заглушка внешней функции)",
                get_st_type(ty, model).unwrap_or_else(|_| ty.to_string())
            )));
        }
    })
}

/// Строит диагностику `ST-011`.
fn unsupported(what: &str) -> Diagnostic {
    Diagnostic::error(
        Location::Codegen,
        format!("Не транслируется в Structured Text: {}", what),
    )
    .with_code("ST-011")
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::semantic::tree::construct_model;

    /// Печатает все функции модели.
    fn functions_of(src: &str) -> (String, Vec<Diagnostic>) {
        let (ast, _) = crate::parse(src, 0).unwrap();
        let rc = construct_model(&ast, None, &[]).unwrap();
        let name = crate::semantic::minimap::Map::create(Rc::clone(&rc))
            .unwrap()
            .root_name();
        let models = vec![(name, Rc::clone(&rc))];
        let mut text = String::new();
        let warnings = {
            let mut p = Printer::new(4, &mut text);
            emit_functions(&mut p, &models).expect("функции должны печататься")
        };
        (text, warnings)
    }

    /// Локальная функция → `FUNCTION имя : ТИП` … `END_FUNCTION`.
    #[test]
    fn test_local_function_emits_function_pou() {
        let (st, _) = functions_of(
            "fn add1(n: u8) -> u8 { return n + 1; }\nvar x: u8 := 0;\n\
             start S { always { x := add1(x); } }",
        );
        assert!(st.contains("FUNCTION add1 : USINT"), "нет FUNCTION:\n{st}");
        assert!(st.contains("n : USINT;"), "нет параметра:\n{st}");
        assert!(st.contains("END_FUNCTION"), "нет END_FUNCTION:\n{st}");
    }

    /// Возврат — присваивание имени функции плюс `RETURN;`: `return <знач>` в ST нет.
    #[test]
    fn test_return_becomes_assignment_to_function_name() {
        let (st, _) = functions_of(
            "fn add1(n: u8) -> u8 { return n + 1; }\nvar x: u8 := 0;\n\
             start S { always { x := add1(x); } }",
        );
        assert!(
            st.contains("add1 := n + 1;"),
            "возврат обязан быть присваиванием имени функции:\n{st}"
        );
        assert!(st.contains("RETURN;"), "нет RETURN:\n{st}");
    }

    /// Ранний возврат внутри `if` (форма `abs_diff`, `stacker.lam:100-103`).
    #[test]
    fn test_early_return_inside_if() {
        let (st, _) = functions_of(
            "fn abs_diff(a: u8, b: u8) -> u8 { if a > b { return a - b; } return b - a; }\n\
             var x: u8 := 0;\nstart S { always { x := abs_diff(x, 1); } }",
        );
        assert!(st.contains("IF a > b THEN"), "нет ветвления:\n{st}");
        assert!(
            st.contains("abs_diff := a - b;"),
            "нет раннего возврата:\n{st}"
        );
        assert!(
            st.contains("abs_diff := b - a;"),
            "нет позднего возврата:\n{st}"
        );
    }

    /// Локальные переменные функции поднимаются в её `VAR` — до тела.
    #[test]
    fn test_function_locals_are_hoisted_before_body() {
        let (st, _) = functions_of(
            "fn f(n: u8) -> u8 { var t: u8 := 0; t := t + n; return t; }\n\
             var x: u8 := 0;\nstart S { always { x := f(x); } }",
        );
        let var_pos = st.find("\nVAR\n").expect("нет секции VAR функции");
        let body_pos = st.find("t := 0;").expect("нет тела");
        assert!(var_pos < body_pos, "VAR обязан идти до тела:\n{st}");
        assert!(st.contains("t : USINT;"), "локальная не поднята:\n{st}");
    }

    /// Локальный `VAR` обязан идти ДО `VAR CONSTANT` (фича 0030).
    ///
    /// Не стиль, а условие валидности: MatIEC протаскивает квалификатор
    /// `CONSTANT` предыдущей секции на следующий за ней `VAR`, и присваивание
    /// локальной переменной отвергается («Assignment to CONSTANT variables is
    /// not allowed»). Проверено пробой на `iec2c`: порядок `VAR CONSTANT` → `VAR`
    /// невалиден, обратный — валиден.
    ///
    /// Сторож нужен потому, что **гейт этот дефект не ловил**: до 0030 ни одна
    /// функция корпуса не имела локальных переменных одновременно с константой,
    /// то есть дефект был латентным при зелёном `iec2c`.
    #[test]
    fn test_function_var_precedes_var_constant() {
        let (st, _) = functions_of(
            "const LIM: u8 := 3;\n\
             fn f(n: u8) -> u8 { var t: u8 := 0; t := n + LIM; return t; }\n\
             var x: u8 := 0;\nstart S { always { x := f(x); } }",
        );
        let var_pos = st.find("\nVAR\n").expect("нет секции VAR функции");
        let const_pos = st
            .find("VAR CONSTANT")
            .expect("нет секции VAR CONSTANT функции");
        assert!(
            var_pos < const_pos,
            "локальный VAR обязан идти до VAR CONSTANT — иначе iec2c считает \
             локальные переменные константами:\n{st}"
        );
    }

    /// `extern fn` → заглушка с телом **плюс** предупреждение `ST-009`.
    ///
    /// Тела у внешней функции нет, а IEC требует его от `FUNCTION` (проба П9).
    /// Молчание дало бы ПЛК-код, который тихо ничего не делает.
    #[test]
    fn test_extern_fn_emits_stub_and_warns_st009() {
        let (st, warnings) = functions_of(
            "extern fn log_it(v: u8);\nvar x: u8 := 0;\n\
             start S { always { log_it(x); } }",
        );
        assert!(
            st.contains("FUNCTION log_it : USINT"),
            "нет заглушки:\n{st}"
        );
        assert!(
            st.contains("log_it := 0;"),
            "заглушка обязана иметь тело:\n{st}"
        );
        assert_eq!(warnings.len(), 1, "extern fn обязана предупреждать");
        assert_eq!(warnings[0].code.as_deref(), Some("ST-009"));
    }

    /// Функция без параметров получает синтетический вход.
    ///
    /// Вход из `elevator.lam:93`: `extern fn motor_up();`. Пустой
    /// `VAR_INPUT … END_VAR` не только недопустим, но и **роняет `iec2c`
    /// segfault'ом**, а беспараметрический POU — нестандартное расширение
    /// (`iec2c -i`).
    #[test]
    fn test_parameterless_function_gets_synthetic_input() {
        let (st, _) = functions_of(
            "extern fn motor_up();\nvar x: u8 := 0;\nstart S { always { motor_up(); } }",
        );
        assert!(
            st.contains("unused : USINT;"),
            "беспараметрическая функция обязана получить синтетический вход:\n{st}"
        );
        assert!(
            !st.contains("VAR_INPUT\nEND_VAR"),
            "пустой VAR_INPUT роняет iec2c segfault'ом:\n{st}"
        );
    }

    /// Функция, возвращающая `unit`, получает синтетический тип: `VOID` в ST —
    /// нестандартное расширение (`iec2c -b`).
    #[test]
    fn test_unit_returning_function_gets_synthetic_return_type() {
        let (st, _) = functions_of(
            "extern fn motor_up();\nvar x: u8 := 0;\nstart S { always { motor_up(); } }",
        );
        assert!(
            st.contains("FUNCTION motor_up : USINT"),
            "unit обязан стать синтетическим USINT:\n{st}"
        );
    }

    /// Вызов беспараметрической функции передаёт синтетический аргумент.
    #[test]
    fn test_call_of_parameterless_function_passes_synthetic_argument() {
        let (ast, _) = crate::parse(
            "extern fn motor_up();\nvar x: u8 := 0;\nstart S { always { motor_up(); } }",
            0,
        )
        .unwrap();
        let rc = construct_model(&ast, None, &[]).unwrap();
        let model = rc.borrow();
        let def = model.search_func("motor_up").expect("нет функции");
        let text = print_call(&def, &[], &model).unwrap();
        assert_eq!(
            text, "motor_up(0)",
            "синтетический параметр требует аргумента"
        );
    }
}
