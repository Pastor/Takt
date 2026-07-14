//! Интерпретатор операторов и вызовов функций.
//!
//! # Один интерпретатор, а не два
//!
//! Тела блоков (`enter`/`exit`/`always`) и тела функций исполняет **одна и та же**
//! функция [`exec_statement`]. Соблазн был скомпилировать блоки в замыкания (как
//! раньше), а тела `fn` интерпретировать отдельно — но это ровно тот грех, ради
//! устранения которого принят ADR 0025: два механизма расходятся. Поэтому
//! «компиляция» блока свелась к тонкой обёртке [`compile_block_body`], которая
//! заводит область видимости и зовёт тот же интерпретатор.
//!
//! # Область видимости
//!
//! `var` не должна попадать в переменные модели, поэтому тело блока исполняется в
//! [`BlockScope`] — **одном на весь блок** (имена собираются рекурсивно на этапе
//! компиляции). Область плоская осознанно: запись в **не-локальное** имя обязана
//! попадать в `write_ctx` (контекст модели), а не в контекст чтения (юнит) — на
//! этом держится видимость переменных между **параллельными** моделями, то есть
//! сценарии `stacker_*`.
//!
//! Тело функции исполняется в [`FunctionScope`]: параметры и локальные `var` —
//! свои, а запись в прочие имена делегируется наружу, в контекст вызывающего (и
//! далее в `write_ctx`).

use crate::context::Context;
use crate::eval::value::Value;
use crate::eval::{self as eval_core, ops};
use crate::expression::eval_expression;
use crate::unit::{Execution, Flow};
use grammar::diagnostics::{Diagnostic, Location};
use grammar::semantic::type_node::TypeNode;
use grammar::semantic::{ExpressionNode, FunctionDefinitionNode, MatchPatternNode, StatementNode};
use std::cell::{Cell, RefCell};
use std::collections::HashMap;
use std::rc::Rc;

/// Предохранитель от зацикливания: симулятор не имеет права зависнуть.
const MAX_ITERATIONS: u32 = 100_000;

/// S10: предел глубины рекурсии — симулятор не имеет права переполнить стек.
const MAX_CALL_DEPTH: u32 = 256;

thread_local! {
    /// Текущая глубина вложенных вызовов функций (S10).
    static CALL_DEPTH: Cell<u32> = const { Cell::new(0) };
}

// ── Области видимости ─────────────────────────────────────────────────────────

/// Область видимости тела блока: локальные `var` + запись наружу в `write_ctx`.
struct BlockScope<'a> {
    locals: HashMap<String, Value>,
    outer: &'a mut dyn Context,
    write: Rc<RefCell<dyn Context>>,
}

impl Context for BlockScope<'_> {
    fn get_value(&self, name: &str) -> Option<Value> {
        self.locals
            .get(name)
            .cloned()
            .or_else(|| self.outer.get_value(name))
    }

    fn set_value(&mut self, name: &str, value: Value) {
        if self.locals.contains_key(name) {
            self.locals.insert(name.to_string(), value);
        } else {
            // Не локальная — пишем туда же, куда писала прежняя реализация.
            self.write.borrow_mut().set_value(name, value);
        }
    }
}

/// Область видимости тела функции: параметры и локальные `var`.
struct FunctionScope<'a> {
    locals: HashMap<String, Value>,
    outer: &'a mut dyn Context,
}

impl Context for FunctionScope<'_> {
    fn get_value(&self, name: &str) -> Option<Value> {
        self.locals
            .get(name)
            .cloned()
            .or_else(|| self.outer.get_value(name))
    }

    fn set_value(&mut self, name: &str, value: Value) {
        if self.locals.contains_key(name) {
            self.locals.insert(name.to_string(), value);
        } else {
            // Запись в глобальное имя из тела функции уходит наружу по цепочке.
            self.outer.set_value(name, value);
        }
    }
}

// ── Служебное ─────────────────────────────────────────────────────────────────

/// Значение локальной переменной до инициализации.
fn default_value(ty: &TypeNode) -> Value {
    match ty {
        TypeNode::Bool => Value::Boolean(false),
        TypeNode::Rational => Value::Real(0.0),
        _ => Value::Number(0),
    }
}

/// Собирает имена, объявленные в дереве оператора (область плоская).
fn collect_locals(stmt: &StatementNode, out: &mut Vec<(String, Value)>) {
    match stmt {
        StatementNode::Variable(name, ty, _) => out.push((name.clone(), default_value(ty))),
        StatementNode::Block(stmts) => stmts.iter().for_each(|s| collect_locals(s, out)),
        StatementNode::If { then_, else_, .. } => {
            collect_locals(then_, out);
            if let Some(else_) = else_ {
                collect_locals(else_, out);
            }
        }
        StatementNode::Loop { body, .. } => collect_locals(body, out),
        StatementNode::For { init, body, .. } => {
            if let Some(init) = init {
                collect_locals(init, out);
            }
            collect_locals(body, out);
        }
        StatementNode::Match { arms, .. } => {
            arms.iter().for_each(|arm| collect_locals(&arm.body, out));
        }
        StatementNode::None
        | StatementNode::Unresolved(_)
        | StatementNode::Expression(_)
        | StatementNode::Return(_)
        | StatementNode::Break
        | StatementNode::Continue
        | StatementNode::InlineFormula(_) => {}
    }
}

/// Сообщает об ошибке времени выполнения.
///
/// Полноценный канал (`TickResult` → `RunResult` → код возврата CLI) — задача
/// `0025-05`. До неё ошибка **печатается**, а не теряется.
pub(crate) fn report(what: &str, diagnostic: &Diagnostic) {
    eprintln!(
        "[симуляция] {what}: {} ({})",
        diagnostic.message,
        diagnostic.code.as_deref().unwrap_or("SIM-000")
    );
}

// ── Точка входа: тело именованного блока ─────────────────────────────────────

/// Оборачивает тело блока в область видимости и зовёт интерпретатор.
pub(crate) fn compile_block_body(
    stmt: &StatementNode,
    write_ctx: Rc<RefCell<dyn Context>>,
) -> Vec<Execution> {
    if matches!(stmt, StatementNode::None) {
        return vec![];
    }
    let stmt = stmt.clone();
    let mut declared = Vec::new();
    collect_locals(&stmt, &mut declared);
    let f: Execution = Rc::new(move |ctx: &mut dyn Context| {
        let mut scope = BlockScope {
            locals: declared.iter().cloned().collect(),
            outer: ctx,
            write: write_ctx.clone(),
        };
        match exec_statement(&stmt, &mut scope) {
            Ok(_) => Flow::Normal,
            Err(diagnostic) => {
                report("оператор пропущен", &diagnostic);
                Flow::Normal
            }
        }
    });
    vec![f]
}

// ── Интерпретатор ─────────────────────────────────────────────────────────────

/// Исполняет оператор.
///
/// Разбор исчерпывающий: каждый вариант `StatementNode` обработан явно.
pub(crate) fn exec_statement(
    stmt: &StatementNode,
    ctx: &mut dyn Context,
) -> Result<Flow, Diagnostic> {
    match stmt {
        StatementNode::None | StatementNode::Unresolved(_) => Ok(Flow::Normal),
        StatementNode::Block(stmts) => {
            for stmt in stmts {
                match exec_statement(stmt, ctx)? {
                    Flow::Normal => {}
                    // Поток управления пробрасывается наружу до цикла/функции.
                    flow => return Ok(flow),
                }
            }
            Ok(Flow::Normal)
        }
        StatementNode::Expression(expr) => exec_expression(expr, ctx),
        StatementNode::If { cond, then_, else_ } => {
            if eval_bool(cond, ctx)? {
                exec_statement(then_, ctx)
            } else if let Some(else_) = else_ {
                exec_statement(else_, ctx)
            } else {
                Ok(Flow::Normal)
            }
        }
        StatementNode::Variable(name, ty, init) => {
            let Some(init) = init else {
                // Без инициализатора значение уже расставлено областью видимости.
                return Ok(Flow::Normal);
            };
            let value = eval_expression(init, ctx)?;
            let value = eval_core::coerce_to_type(value, ty)
                .map_err(|e| e.to_diagnostic(Location::Builtin))?;
            ctx.set_value(name, value);
            Ok(Flow::Normal)
        }
        StatementNode::Loop { cond, body } => exec_loop(cond.as_deref(), body, ctx),
        StatementNode::For {
            init,
            cond,
            step,
            body,
        } => exec_for(init.as_deref(), cond.as_deref(), step.as_deref(), body, ctx),
        StatementNode::Match { expr, arms } => {
            let subject = eval_expression(expr, ctx)?;
            for arm in arms {
                if arm_matches(&arm.patterns, &subject, ctx)? {
                    return exec_statement(&arm.body, ctx);
                }
            }
            Ok(Flow::Normal)
        }
        StatementNode::Return(expr) => {
            let value = match expr {
                Some(expr) => Some(eval_expression(expr, ctx)?),
                None => None,
            };
            Ok(Flow::Return(value))
        }
        StatementNode::Break => Ok(Flow::Break),
        StatementNode::Continue => Ok(Flow::Continue),
        // `Guard`/LTL — метаданные верификации, а не исполняемый код: осознанный
        // no-op. Проверка инвариантов симулятором — кандидат «assert/invariant».
        StatementNode::InlineFormula(_) => Ok(Flow::Normal),
    }
}

/// Исполняет выражение-оператор.
///
/// Присваивание — с приведением к типу цели (S9). Прочие выражения (в первую
/// очередь вызовы функций, Д3) вычисляются ради побочного эффекта.
fn exec_expression(expr: &ExpressionNode, ctx: &mut dyn Context) -> Result<Flow, Diagnostic> {
    match expr {
        ExpressionNode::Assign(lhs, rhs) => {
            let ExpressionNode::Variable(var_rc) = lhs.as_ref() else {
                return Err(Diagnostic::error(
                    Location::Builtin,
                    "присваивание не в переменную пока не поддерживается симулятором".to_string(),
                )
                .with_code("SIM-017"));
            };
            let (name, ty, loc) = {
                let b = var_rc.borrow();
                (b.name().to_string(), b.ty().clone(), b.loc())
            };
            let value = eval_expression(rhs, ctx)?;
            let value = eval_core::coerce_to_type(value, &ty).map_err(|e| e.to_diagnostic(loc))?;
            ctx.set_value(&name, value);
            Ok(Flow::Normal)
        }
        // Д3: вызов-оператор (`log_temp(x);`) — раньше молча отбрасывался.
        _ => {
            eval_expression(expr, ctx)?;
            Ok(Flow::Normal)
        }
    }
}

fn exec_loop(
    cond: Option<&ExpressionNode>,
    body: &StatementNode,
    ctx: &mut dyn Context,
) -> Result<Flow, Diagnostic> {
    let mut iterations = 0_u32;
    loop {
        if let Some(cond) = cond {
            if !eval_bool(cond, ctx)? {
                break;
            }
        }
        match exec_statement(body, ctx)? {
            Flow::Normal | Flow::Continue => {}
            Flow::Break => break,
            // `return` изнутри цикла выходит из функции целиком.
            flow @ Flow::Return(_) => return Ok(flow),
        }
        iterations += 1;
        if iterations >= MAX_ITERATIONS {
            return Err(Diagnostic::error(
                Location::Builtin,
                format!("превышен предел итераций цикла ({MAX_ITERATIONS})"),
            )
            .with_code("SIM-018"));
        }
    }
    Ok(Flow::Normal)
}

fn exec_for(
    init: Option<&StatementNode>,
    cond: Option<&ExpressionNode>,
    step: Option<&ExpressionNode>,
    body: &StatementNode,
    ctx: &mut dyn Context,
) -> Result<Flow, Diagnostic> {
    if let Some(init) = init {
        exec_statement(init, ctx)?;
    }
    let mut iterations = 0_u32;
    loop {
        if let Some(cond) = cond {
            if !eval_bool(cond, ctx)? {
                break;
            }
        }
        match exec_statement(body, ctx)? {
            Flow::Normal | Flow::Continue => {}
            Flow::Break => break,
            flow @ Flow::Return(_) => return Ok(flow),
        }
        if let Some(step) = step {
            exec_expression(step, ctx)?;
        }
        iterations += 1;
        if iterations >= MAX_ITERATIONS {
            return Err(Diagnostic::error(
                Location::Builtin,
                format!("превышен предел итераций цикла for ({MAX_ITERATIONS})"),
            )
            .with_code("SIM-018"));
        }
    }
    Ok(Flow::Normal)
}

fn arm_matches(
    patterns: &[MatchPatternNode],
    subject: &Value,
    ctx: &mut dyn Context,
) -> Result<bool, Diagnostic> {
    for pattern in patterns {
        let matched = match pattern {
            MatchPatternNode::Wildcard => true,
            MatchPatternNode::Value(expr) => {
                let value = eval_expression(expr, ctx)?;
                let equal = ops::apply_binary(ops::BinOp::Equal, subject, &value)
                    .map_err(|e| e.to_diagnostic(Location::Builtin))?;
                ops::to_bool(&equal).map_err(|e| e.to_diagnostic(Location::Builtin))?
            }
        };
        if matched {
            return Ok(true);
        }
    }
    Ok(false)
}

fn eval_bool(expr: &ExpressionNode, ctx: &mut dyn Context) -> Result<bool, Diagnostic> {
    let value = eval_expression(expr, ctx)?;
    ops::to_bool(&value).map_err(|e| e.to_diagnostic(Location::Builtin))
}

// ── Вызов функции ─────────────────────────────────────────────────────────────

/// Вызывает функцию: связывает параметры, исполняет тело, возвращает значение.
///
/// Используется обоими адаптерами — выражений и условий (Д3, Д4).
pub(crate) fn call_function(
    func: &Rc<RefCell<FunctionDefinitionNode>>,
    args: &[Value],
    ctx: &mut dyn Context,
) -> Result<Value, Diagnostic> {
    let (params, ret, body, loc, name) = {
        let borrowed = func.borrow();
        match &*borrowed {
            FunctionDefinitionNode::Local {
                params,
                ret,
                body,
                loc,
                name,
                ..
            } => (
                params.clone(),
                ret.clone(),
                body.clone(),
                *loc,
                name.clone(),
            ),
            // Тела нет: процедура — no-op; функция со значением — отказ, а не
            // тихий ноль (решение ADR).
            FunctionDefinitionNode::External { ret, loc, name, .. } => {
                return if matches!(ret, TypeNode::Unit) {
                    Ok(Value::Number(0))
                } else {
                    Err(Diagnostic::error(
                        *loc,
                        format!(
                            "внешняя функция '{name}' не имеет тела: симуляция значения невозможна"
                        ),
                    )
                    .with_code("SIM-019"))
                };
            }
            FunctionDefinitionNode::Builtin(name, _, _) => {
                return Err(Diagnostic::error(
                    Location::Builtin,
                    format!("встроенная функция '{name}' пока не поддерживается симулятором"),
                )
                .with_code("SIM-020"));
            }
            FunctionDefinitionNode::None | FunctionDefinitionNode::Unresolved(_) => {
                return Err(Diagnostic::error(
                    Location::Builtin,
                    "неразрешённая функция не может быть вызвана".to_string(),
                )
                .with_code("SIM-016"));
            }
        }
    };

    if params.len() != args.len() {
        return Err(Diagnostic::error(
            loc,
            format!(
                "функция '{name}': ожидалось аргументов {}, передано {}",
                params.len(),
                args.len()
            ),
        )
        .with_code("SIM-021"));
    }

    // S10: без предела рекурсия переполнила бы стек — симулятор упал бы вместо
    // диагностики.
    let depth = CALL_DEPTH.with(|d| {
        let next = d.get() + 1;
        d.set(next);
        next
    });
    let result = call_local(&params, &ret, &body, args, ctx, depth, loc, &name);
    CALL_DEPTH.with(|d| d.set(d.get() - 1));
    result
}

#[allow(clippy::too_many_arguments)]
fn call_local(
    params: &[(String, TypeNode)],
    ret: &TypeNode,
    body: &StatementNode,
    args: &[Value],
    ctx: &mut dyn Context,
    depth: u32,
    loc: Location,
    name: &str,
) -> Result<Value, Diagnostic> {
    if depth > MAX_CALL_DEPTH {
        return Err(Diagnostic::error(
            loc,
            format!("превышена глубина рекурсии ({MAX_CALL_DEPTH}) при вызове '{name}'"),
        )
        .with_code("SIM-022"));
    }

    // Параметры приводятся к объявленным типам — как при записи в переменную.
    let mut locals = HashMap::new();
    for ((param, ty), value) in params.iter().zip(args) {
        let coerced =
            eval_core::coerce_to_type(value.clone(), ty).map_err(|e| e.to_diagnostic(loc))?;
        locals.insert(param.clone(), coerced);
    }
    let mut declared = Vec::new();
    collect_locals(body, &mut declared);
    locals.extend(declared);

    let mut scope = FunctionScope { locals, outer: ctx };
    let flow = exec_statement(body, &mut scope)?;

    match flow {
        Flow::Return(Some(value)) => {
            eval_core::coerce_to_type(value, ret).map_err(|e| e.to_diagnostic(loc))
        }
        // Процедура без значения либо `return;` — числовой ноль как нейтральное
        // значение (вызов-оператор результат игнорирует).
        Flow::Return(None) | Flow::Normal => {
            if matches!(ret, TypeNode::Unit) {
                Ok(Value::Number(0))
            } else {
                Err(
                    Diagnostic::error(loc, format!("функция '{name}' не вернула значение"))
                        .with_code("SIM-023"),
                )
            }
        }
        Flow::Break | Flow::Continue => Err(Diagnostic::error(
            loc,
            format!("'{name}': break/continue вне цикла"),
        )
        .with_code("SIM-024")),
    }
}
