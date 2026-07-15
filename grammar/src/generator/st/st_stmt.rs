//! Печать операторов Lam ([`StatementNode`]) в Structured Text (IEC 61131-3).
//!
//! Задача 0041-04, часть 2. Дополняет `st_expr.rs` (часть 1: выражения и
//! условия). Функции (`FUNCTION`/`RETURN`, `extern fn`) — часть 3.
//!
//! ## Подъём объявлений (главное отличие от C)
//!
//! В Lam переменная объявляется по месту: `enter { var boost: u8 := 5; … }`
//! (`comprehensive.lam:58`). В IEC 61131-3 объявления живут **только в шапке
//! POU**, а не в теле. Поэтому [`print_statement`] **поднимает** объявление в
//! [`Hoisted`], а на его месте оставляет присваивание инициализатора.
//!
//! Разделение принципиально для семантики: поднимается **объявление**, а
//! инициализатор остаётся на исходном месте. Иначе `var i := 0` внутри цикла
//! инициализировался бы однажды, а не на каждом входе в блок.
//!
//! > ⚠ MatIEC **принимает** и `VAR … END_VAR` посреди тела (проверено пробой), но
//! > это его послабление, а не стандарт: `iec2c` тут не судья — он одинаково
//! > принимает обе формы. Выбор в пользу подъёма сделан по **стандарту**, потому
//! > что цель фичи — настоящий ПЛК, а не транспилятор.
//!
//! ## Циклы
//!
//! `loop`/`while` Lam → `WHILE … DO … END_WHILE;`. `for` Lam — **си-образный**
//! (`init; cond; step`), а `FOR` в IEC — **счётный** (`FOR i := 0 TO 3 BY 1`), то
//! есть прямого соответствия нет: си-образный `for` разворачивается в `WHILE` с
//! шагом в конце тела.

// Печатник операторов ещё никто не вызывает: его потребитель — `st_model.rs`
// (задача 0041-03), который пишется следующим. Разрешение снимается вместе с
// появлением вызывающего — та же причина и тот же приём, что в `st_expr.rs`.
#![allow(dead_code)]

use crate::diagnostics::{Diagnostic, Location};
use crate::generator::indent::Printer;
use crate::generator::st::st_expr::print_expression;
use crate::semantic::type_node::TypeNode;
use crate::semantic::{ExpressionNode, MatchPatternNode, ModelNode, StatementNode};

/// Объявление, поднятое из тела в шапку POU.
pub(crate) struct Hoisted {
    /// Имя переменной.
    pub name: String,
    /// Тип переменной.
    pub ty: TypeNode,
}

/// Побочные результаты печати тела: поднятые объявления и предупреждения.
#[derive(Default)]
pub(crate) struct StmtOutput {
    /// Объявления, которые вызывающий обязан напечатать в шапке POU.
    pub hoisted: Vec<Hoisted>,
    /// Предупреждения (`ST-010`), которые вызывающий обязан показать.
    pub warnings: Vec<Diagnostic>,
}

/// Печатает оператор Lam в текст ST.
///
/// Объявления переменных **поднимаются** в `out.hoisted` (см. шапку модуля), а
/// на их месте печатается присваивание инициализатора.
///
/// # Ошибки
/// `ST-011` — узел не имеет представления в ST (R4: никакого тихого пропуска).
pub(crate) fn print_statement(
    stmt: &StatementNode,
    model: &ModelNode,
    p: &mut Printer,
    out: &mut StmtOutput,
    fn_name: Option<&str>,
) -> Result<(), Diagnostic> {
    match stmt {
        // Пустой оператор ничего не печатает: в ST лишняя `;` — синтаксическая
        // ошибка (проверено пробой: голая `;` в теле FB не разбирается).
        StatementNode::None => Ok(()),
        StatementNode::Block(items) => {
            for item in items {
                print_statement(item, model, p, out, fn_name)?;
            }
            Ok(())
        }
        // Голый вызов функции оператором быть НЕ МОЖЕТ: «Function invocation in
        // ST code is not allowed outside an expression». Lam так вызывает
        // (`log_temp(temperature);`, `motor_up();`), поэтому результат уходит в
        // переменную-приёмник, которую вызывающий объявит в шапке POU.
        StatementNode::Expression(expr) => {
            if let ExpressionNode::Function(def, args) = expr.as_ref() {
                let call = crate::generator::st::st_func::print_call(def, args, model)?;
                let ret = crate::generator::st::st_func::return_type_of(&def.borrow());
                let sink = sink_name(&ret, model)?;
                out.hoisted.push(Hoisted {
                    name: sink.clone(),
                    ty: ret,
                });
                p.ident(&format!("{} := {};", sink, call)).nl();
                return Ok(());
            }
            let text = print_expression(expr, model)?;
            p.ident(&format!("{};", text)).nl();
            Ok(())
        }
        StatementNode::If { cond, then_, else_ } => {
            p.ident(&format!("IF {} THEN", print_expression(cond, model)?))
                .nl();
            p.up();
            print_statement(then_, model, p, out, fn_name)?;
            p.down();
            if let Some(else_) = else_ {
                p.ident("ELSE").nl();
                p.up();
                print_statement(else_, model, p, out, fn_name)?;
                p.down();
            }
            p.ident("END_IF;").nl();
            Ok(())
        }
        // `loop`/`while` → `WHILE … DO`. Бесконечный цикл (`cond: None`) —
        // `WHILE TRUE DO`: в ПЛК он завесит скан-цикл, но это свойство модели, а
        // не трансляции; молча менять семантику нельзя.
        StatementNode::Loop { cond, body } => {
            let guard = match cond {
                Some(c) => print_expression(c, model)?,
                None => "TRUE".to_string(),
            };
            p.ident(&format!("WHILE {} DO", guard)).nl();
            p.up();
            print_statement(body, model, p, out, fn_name)?;
            p.down();
            p.ident("END_WHILE;").nl();
            Ok(())
        }
        StatementNode::For {
            init,
            cond,
            step,
            body,
        } => print_for(init, cond, step, body, model, p, out, fn_name),
        // Объявление: тип уезжает в шапку POU, инициализатор остаётся здесь.
        StatementNode::Variable(name, ty, init) => {
            out.hoisted.push(Hoisted {
                name: name.clone(),
                ty: ty.clone(),
            });
            if let Some(init) = init {
                let text = print_expression(init, model)?;
                p.ident(&format!("{} := {};", name, text)).nl();
            }
            Ok(())
        }
        // Возврат значения — присваивание имени функции; его подставляет печатник
        // функций (часть 3), поэтому здесь допустим только голый `RETURN`.
        StatementNode::Return(None) => {
            p.ident("RETURN;").nl();
            Ok(())
        }
        // В ST нет `return <значение>`: результат возвращается присваиванием
        // ИМЕНИ функции, а `RETURN;` лишь досрочно выходит.
        StatementNode::Return(Some(value)) => {
            let name = fn_name.ok_or_else(|| {
                unsupported(
                    "return со значением вне функции: присваивать нечему — имя \
                     функции неизвестно",
                )
            })?;
            let text = print_expression(value, model)?;
            p.ident(&format!("{} := {};", name, text)).nl();
            p.ident("RETURN;").nl();
            Ok(())
        }
        // В ST выход из цикла — `EXIT`, а не `break`.
        StatementNode::Break => {
            p.ident("EXIT;").nl();
            Ok(())
        }
        StatementNode::Continue => {
            p.ident("CONTINUE;").nl();
            Ok(())
        }
        // `match` → цепочка `IF/ELSIF`, а НЕ `CASE OF`. Причина: метки `CASE` в
        // IEC — литералы и диапазоны, а образцы Lam могут быть произвольными
        // выражениями (включая варианты перечислений, которые у нас стали
        // именованными константами, а не литералами). Цепочка сравнений
        // семантически тождественна и заведомо выразима.
        StatementNode::Match { expr, arms } => print_match(expr, arms, model, p, out, fn_name),
        // LTL-формулы в ST не транслируются. Предупреждение, а не тихий пропуск:
        // молчание здесь — ровно класс дефекта фичи 0025 (ср. фича 0035, где
        // формулы теряются молча уже в семантике).
        StatementNode::InlineFormula(formulas) => {
            if !formulas.is_empty() {
                out.warnings.push(
                    Diagnostic::warning(
                        Location::Codegen,
                        format!(
                            "LTL-формул ({}) в блоке кода: в Structured Text они не \
                             транслируются и в порождённый ПЛК-код не попадут",
                            formulas.len()
                        ),
                    )
                    .with_code("ST-010"),
                );
            }
            Ok(())
        }
        StatementNode::Unresolved(_) => Err(unsupported(
            "оператор не прошёл семантическое понижение (Unresolved)",
        )),
    }
}

/// Печатает си-образный `for` Lam как `WHILE` со счётчиком.
///
/// `FOR` в IEC — **счётный** (`FOR i := 0 TO 3 BY 1 DO`), а `for` Lam несёт
/// произвольные `cond` и `step`, поэтому прямого соответствия нет.
///
/// # Ошибки
/// `ST-011`, если тело содержит `continue`: в си-образном `for` шаг выполняется и
/// после `continue`, а в `WHILE` — нет. Развернуть такой цикл, не изменив
/// семантику, нельзя, поэтому отказ громкий, а не тихое расхождение.
#[allow(clippy::ref_option)]
fn print_for(
    init: &Option<Box<StatementNode>>,
    cond: &Option<Box<ExpressionNode>>,
    step: &Option<Box<ExpressionNode>>,
    body: &StatementNode,
    model: &ModelNode,
    p: &mut Printer,
    out: &mut StmtOutput,
    fn_name: Option<&str>,
) -> Result<(), Diagnostic> {
    if step.is_some() && contains_continue(body) {
        return Err(unsupported(
            "continue внутри for с шагом: в Lam шаг выполняется и после continue, \
             а в WHILE-развёртке ST — нет; тождественной развёртки не существует",
        ));
    }
    if let Some(init) = init {
        print_statement(init, model, p, out, fn_name)?;
    }
    let guard = match cond {
        Some(c) => print_expression(c, model)?,
        None => "TRUE".to_string(),
    };
    p.ident(&format!("WHILE {} DO", guard)).nl();
    p.up();
    print_statement(body, model, p, out, fn_name)?;
    if let Some(step) = step {
        let text = print_expression(step, model)?;
        p.ident(&format!("{};", text)).nl();
    }
    p.down();
    p.ident("END_WHILE;").nl();
    Ok(())
}

/// Печатает `match` как цепочку `IF/ELSIF/ELSE`.
fn print_match(
    expr: &ExpressionNode,
    arms: &[crate::semantic::MatchArmNode],
    model: &ModelNode,
    p: &mut Printer,
    out: &mut StmtOutput,
    fn_name: Option<&str>,
) -> Result<(), Diagnostic> {
    let subject = print_expression(expr, model)?;
    let mut printed_if = false;
    let mut wildcard: Option<&StatementNode> = None;

    for arm in arms {
        // Ветка `_` печатается последней как `ELSE`, где бы она ни стояла.
        if arm
            .patterns
            .iter()
            .any(|p| matches!(p, MatchPatternNode::Wildcard))
        {
            wildcard = Some(&arm.body);
            continue;
        }
        let mut tests = Vec::new();
        for pattern in &arm.patterns {
            let MatchPatternNode::Value(value) = pattern else {
                continue;
            };
            tests.push(format!("{} = {}", subject, print_expression(value, model)?));
        }
        if tests.is_empty() {
            continue;
        }
        let guard = tests.join(" OR ");
        p.ident(&format!(
            "{} {} THEN",
            if printed_if { "ELSIF" } else { "IF" },
            guard
        ))
        .nl();
        p.up();
        print_statement(&arm.body, model, p, out, fn_name)?;
        p.down();
        printed_if = true;
    }

    match (printed_if, wildcard) {
        // Есть ветви и есть `_` → обычный ELSE.
        (true, Some(body)) => {
            p.ident("ELSE").nl();
            p.up();
            print_statement(body, model, p, out, fn_name)?;
            p.down();
            p.ident("END_IF;").nl();
        }
        (true, None) => {
            p.ident("END_IF;").nl();
        }
        // Только `_` → тело исполняется безусловно; `IF` не нужен.
        (false, Some(body)) => print_statement(body, model, p, out, fn_name)?,
        (false, None) => {}
    }
    Ok(())
}

/// Есть ли `continue` в теле (не заходя во вложенные циклы — там он свой).
fn contains_continue(stmt: &StatementNode) -> bool {
    match stmt {
        StatementNode::Continue => true,
        StatementNode::Block(items) => items.iter().any(contains_continue),
        StatementNode::If { then_, else_, .. } => {
            contains_continue(then_) || else_.as_ref().is_some_and(|e| contains_continue(e))
        }
        StatementNode::Match { arms, .. } => arms.iter().any(|a| contains_continue(&a.body)),
        // Вложенные циклы перехватывают свой `continue` — дальше не смотрим.
        StatementNode::Loop { .. } | StatementNode::For { .. } => false,
        StatementNode::None
        | StatementNode::Unresolved(_)
        | StatementNode::Expression(_)
        | StatementNode::Variable(_, _, _)
        | StatementNode::Return(_)
        | StatementNode::Break
        | StatementNode::InlineFormula(_) => false,
    }
}

/// Имя переменной-приёмника для результата вызова-оператора.
///
/// Приёмник свой на каждый тип: в ST у переменной один тип, а вызовы в теле
/// могут возвращать разное.
fn sink_name(ty: &TypeNode, model: &ModelNode) -> Result<String, Diagnostic> {
    let st = crate::generator::st::st_type::get_st_type(ty, model)?;
    Ok(format!("_st_discard_{}", st.to_lowercase()))
}

/// Строит диагностику `ST-011` — узел без представления в ST.
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
    use crate::semantic::{NamedCodeBlockDefinitionNode, StateNode};

    /// Печатает тело блока `always` стартового состояния.
    fn always_of(body: &str) -> (String, StmtOutput) {
        let src = format!(
            "var n: u8 := 0;\nvar m: u8 := 0;\nvar b: bit := 0;\n\
             start S {{ always {{ {} }} }}",
            body
        );
        let (ast, _) = crate::parse(&src, 0).unwrap();
        let rc = construct_model(&ast, None, &[]).unwrap();
        let model = rc.borrow();
        let block = always_body(&model, "S");
        let mut text = String::new();
        let mut out = StmtOutput::default();
        {
            let mut p = Printer::new(4, &mut text);
            print_statement(&block, &model, &mut p, &mut out, None).expect("должно печататься");
        }
        (text, out)
    }

    /// Достаёт тело блока `always` состояния.
    fn always_body(model: &ModelNode, state: &str) -> StatementNode {
        let node = model.states.get(state).expect("нет состояния");
        let (StateNode::Simple { named_blocks, .. } | StateNode::Implement { named_blocks, .. }) =
            node
        else {
            panic!("состояние не разрешено");
        };
        named_blocks
            .iter()
            .find_map(|b| match b {
                NamedCodeBlockDefinitionNode::Always { body, .. } => Some(body.clone()),
                _ => None,
            })
            .expect("нет блока always")
    }

    /// `if` → `IF … THEN … END_IF;` — с обязательным закрытием.
    #[test]
    fn test_if_is_closed_with_end_if() {
        let (st, _) = always_of("if n > 1 { n := 1; }");
        assert!(st.contains("IF n > 1 THEN"), "нет IF:\n{st}");
        assert!(st.contains("n := 1;"), "нет тела:\n{st}");
        assert!(st.contains("END_IF;"), "IF обязан закрываться:\n{st}");
    }

    /// `if/else` → `IF … ELSE … END_IF;`.
    #[test]
    fn test_if_else_prints_else_branch() {
        let (st, _) = always_of("if n > 1 { n := 1; } else { n := 2; }");
        assert!(st.contains("ELSE"), "нет ветки ELSE:\n{st}");
        assert_eq!(st.matches("END_IF;").count(), 1, "лишние END_IF:\n{st}");
    }

    /// `while` → `WHILE … DO … END_WHILE;`.
    #[test]
    fn test_while_becomes_while_do() {
        let (st, _) = always_of("while n < 3 { n := n + 1; }");
        assert!(st.contains("WHILE n < 3 DO"), "нет WHILE:\n{st}");
        assert!(st.contains("END_WHILE;"), "WHILE обязан закрываться:\n{st}");
    }

    /// Си-образный `for` разворачивается в `WHILE`: `FOR` в IEC — счётный,
    /// прямого соответствия произвольным `cond`/`step` у него нет.
    ///
    /// Шаг обязан печататься **в конце тела**, иначе цикл не сойдётся.
    #[test]
    fn test_c_style_for_unrolls_into_while_with_step_at_end() {
        let (st, out) = always_of("for var i: u8 := 0; i < 3; i := i + 1 { n := n + 1; }");
        assert!(st.contains("i := 0;"), "нет инициализации:\n{st}");
        assert!(st.contains("WHILE i < 3 DO"), "нет WHILE:\n{st}");
        let body = st.find("n := n + 1;").expect("нет тела");
        let step = st.find("i := i + 1;").expect("нет шага");
        assert!(step > body, "шаг обязан идти после тела:\n{st}");
        assert!(
            out.hoisted.iter().any(|h| h.name == "i"),
            "счётчик обязан подниматься в шапку POU"
        );
    }

    /// Объявление в теле поднимается, а инициализатор остаётся на месте.
    ///
    /// Вход из `comprehensive.lam:58`: `enter { var boost: u8 := 5; … }`.
    /// В IEC объявления живут только в шапке POU.
    #[test]
    fn test_local_variable_declaration_is_hoisted_but_initializer_stays() {
        let (st, out) = always_of("var boost: u8 := 5; n := n + boost;");
        assert!(
            !st.contains("VAR"),
            "объявление не должно печататься в теле:\n{st}"
        );
        assert!(st.contains("boost := 5;"), "инициализатор остаётся:\n{st}");
        assert_eq!(out.hoisted.len(), 1, "объявление обязано подняться");
        assert_eq!(out.hoisted[0].name, "boost");
    }

    /// `match` → цепочка `IF/ELSIF/ELSE`, а не `CASE`: метки `CASE` в IEC —
    /// литералы, а образцы Lam могут быть выражениями.
    #[test]
    fn test_match_becomes_if_elsif_chain() {
        let (st, _) = always_of("match n { 1 => { m := 1; } 2 => { m := 2; } _ => { m := 0; } }");
        assert!(st.contains("IF n = 1 THEN"), "нет первой ветви:\n{st}");
        assert!(st.contains("ELSIF n = 2 THEN"), "нет второй ветви:\n{st}");
        assert!(st.contains("ELSE"), "нет ветви _:\n{st}");
        assert!(st.contains("END_IF;"), "цепочка обязана закрываться:\n{st}");
    }

    /// `break` в ST — `EXIT`.
    #[test]
    fn test_break_is_exit_keyword() {
        let (st, _) = always_of("while n < 3 { break; }");
        assert!(st.contains("EXIT;"), "break обязан стать EXIT:\n{st}");
    }

    /// `continue` внутри `for` с шагом — отказ, а не тихое расхождение.
    ///
    /// В Lam шаг выполняется и после `continue`; в `WHILE`-развёртке — нет.
    /// Тождественной развёртки не существует, поэтому `ST-011`.
    #[test]
    fn test_continue_inside_for_with_step_is_rejected_not_silently_wrong() {
        let src = "var n: u8 := 0;\nstart S { always { \
                   for var i: u8 := 0; i < 3; i := i + 1 { continue; } } }";
        let (ast, _) = crate::parse(src, 0).unwrap();
        let rc = construct_model(&ast, None, &[]).unwrap();
        let model = rc.borrow();
        let block = always_body(&model, "S");
        let mut text = String::new();
        let mut out = StmtOutput::default();
        let mut p = Printer::new(4, &mut text);
        let err = print_statement(&block, &model, &mut p, &mut out, None)
            .expect_err("continue в for с шагом обязан отвергаться");
        assert_eq!(err.code.as_deref(), Some("ST-011"));
    }

    /// LTL-формула даёт предупреждение `ST-010`, а не тихий пропуск.
    #[test]
    fn test_inline_formula_warns_st010() {
        use crate::semantic::Formula;
        let rc = {
            let (ast, _) =
                crate::parse("var n: u8 := 0;\nstart S { always { n := n; } }", 0).unwrap();
            construct_model(&ast, None, &[]).unwrap()
        };
        let model = rc.borrow();
        let stmt = StatementNode::InlineFormula(vec![Formula::Formulas(Vec::new())]);
        let mut text = String::new();
        let mut out = StmtOutput::default();
        {
            let mut p = Printer::new(4, &mut text);
            print_statement(&stmt, &model, &mut p, &mut out, None).unwrap();
        }
        assert_eq!(out.warnings.len(), 1, "формула обязана дать предупреждение");
        assert_eq!(out.warnings[0].code.as_deref(), Some("ST-010"));
        assert!(text.is_empty(), "формула ничего не печатает в ST");
    }

    /// Пустая `InlineFormula` предупреждения не даёт: терять нечего.
    #[test]
    fn test_empty_inline_formula_is_silent() {
        let rc = {
            let (ast, _) =
                crate::parse("var n: u8 := 0;\nstart S { always { n := n; } }", 0).unwrap();
            construct_model(&ast, None, &[]).unwrap()
        };
        let model = rc.borrow();
        let mut text = String::new();
        let mut out = StmtOutput::default();
        let mut p = Printer::new(4, &mut text);
        print_statement(
            &StatementNode::InlineFormula(Vec::new()),
            &model,
            &mut p,
            &mut out,
            None,
        )
        .unwrap();
        assert!(out.warnings.is_empty(), "пустая формула не теряет ничего");
    }
}
