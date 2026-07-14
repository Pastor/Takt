//! Компиляция операторов тела блока в исполнители.
//!
//! До задачи `0025-02b-1` `compile_statement` содержал `_ => vec![]` и молча
//! ронял **8 из 13** вариантов `StatementNode`: `Loop`, `For`, `Match`,
//! `Variable`, `Return`, `Continue`, `Break`, `InlineFormula`. Это тот же класс
//! дефекта, что Д1/Д2, но на уровне операторов — в инвентаре Д1–Д8 он не
//! значился и был найден при реализации 0025-02a.
//!
//! Теперь разбор исчерпывающий: каждый вариант либо исполняется, либо **явно**
//! отказывает с диагностикой.
//!
//! # Область видимости локальных переменных
//!
//! `var` внутри блока не должна попадать в переменные модели. Поэтому тело блока
//! исполняется в [`BlockScope`] — **одном на весь блок**: имена, объявленные в
//! любом месте его дерева, собираются на этапе компиляции
//! ([`collect_locals`]) и живут в самом scope.
//!
//! Область видимости **плоская** (без вложенных scope) осознанно. Запись в
//! не-локальное имя обязана попадать **в `write_ctx`**, а не в контекст чтения:
//! на этом держится разделение «читаем из юнита — пишем в модель», и на нём же —
//! видимость переменных между параллельными моделями (общий родитель), то есть
//! сценарии `stacker_*`. Цепочка вложенных scope делегировала бы запись
//! наверх по чтению и это разделение сломала бы. Платой является отсутствие
//! затенения одноимённых `var` во вложенных блоках — на практике это запрещает
//! семантический анализ.

use crate::context::Context;
use crate::eval::value::Value;
use crate::eval::{self as eval_core, ops};
use crate::expression::eval_expression;
use crate::unit::Execution;
use grammar::diagnostics::{Diagnostic, Location};
use grammar::semantic::type_node::TypeNode;
use grammar::semantic::{ExpressionNode, MatchArmNode, MatchPatternNode, StatementNode};
use std::cell::RefCell;
use std::collections::HashMap;
use std::rc::Rc;

/// Предохранитель от зацикливания: симулятор не имеет права зависнуть.
///
/// Пока `break`/`continue` не поддержаны (задача `0025-02b-2`), безусловный
/// `loop` завершится только по этому пределу — с диагностикой, а не молча.
const MAX_ITERATIONS: u32 = 100_000;

/// Область видимости блока: локальные переменные + делегирование наружу.
struct BlockScope<'a> {
    locals: HashMap<String, Value>,
    outer: &'a mut dyn Context,
    write: Rc<RefCell<dyn Context>>,
}

impl<'a> BlockScope<'a> {
    fn new(
        declared: &[(String, Value)],
        outer: &'a mut dyn Context,
        write: Rc<RefCell<dyn Context>>,
    ) -> Self {
        Self {
            locals: declared.iter().cloned().collect(),
            outer,
            write,
        }
    }
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

/// Значение локальной переменной до инициализации.
fn default_value(ty: &TypeNode) -> Value {
    match ty {
        TypeNode::Bool => Value::Boolean(false),
        TypeNode::Rational => Value::Real(0.0),
        // Прочие (целые, `bit`, `enum`, адреса) — нулевое целое.
        _ => Value::Number(0),
    }
}

/// Собирает все имена, объявленные в дереве оператора (область плоская — см.
/// заголовок модуля).
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

/// Компилирует **тело именованного блока** (`enter`/`exit`/`always`).
///
/// Единственная точка, где заводится область видимости: сюда попадают все `var`
/// дерева, а запись в прочие имена уходит в `write_ctx`.
pub(crate) fn compile_block_body(
    stmt: &StatementNode,
    write_ctx: Rc<RefCell<dyn Context>>,
) -> Vec<Execution> {
    let inner = compile_statement(stmt, write_ctx.clone());
    if inner.is_empty() {
        return vec![];
    }
    let mut declared = Vec::new();
    collect_locals(stmt, &mut declared);
    let write = write_ctx;
    let f: Execution = Rc::new(move |ctx: &mut dyn Context| {
        let mut scope = BlockScope::new(&declared, ctx, write.clone());
        for f in &inner {
            f(&mut scope);
        }
    });
    vec![f]
}

/// Сообщает об ошибке времени выполнения.
///
/// Полноценный канал (`TickResult` → `RunResult` → код возврата CLI) — задача
/// `0025-05`. До неё ошибка **печатается**, а не теряется: тихий пропуск и есть
/// корневая причина фичи 0025.
pub(crate) fn report(what: &str, diagnostic: &Diagnostic) {
    eprintln!(
        "[симуляция] {what}: {} ({})",
        diagnostic.message,
        diagnostic.code.as_deref().unwrap_or("SIM-000")
    );
}

fn unsupported(what: &str) -> Diagnostic {
    Diagnostic::error(
        Location::Builtin,
        format!("{what} пока не поддерживается симулятором"),
    )
    .with_code("SIM-017")
}

/// Компилирует оператор в список исполнителей (closures).
///
/// `write_ctx` — контекст для записи (цепочка `ModelNodeContext`); `ctx` в
/// замыканиях — контекст для чтения (содержит значения входных портов).
pub(crate) fn compile_statement(
    stmt: &StatementNode,
    write_ctx: Rc<RefCell<dyn Context>>,
) -> Vec<Execution> {
    match stmt {
        StatementNode::None | StatementNode::Unresolved(_) => vec![],
        StatementNode::Block(stmts) => compile_block(stmts, write_ctx),
        StatementNode::Expression(expr) => compile_expression(expr, write_ctx),
        StatementNode::If { cond, then_, else_ } => {
            compile_if(cond, then_, else_.as_deref(), write_ctx)
        }
        StatementNode::Variable(name, ty, init) => {
            compile_variable(name, ty, init.as_deref(), write_ctx)
        }
        StatementNode::Loop { cond, body } => compile_loop(cond.as_deref(), body, write_ctx),
        StatementNode::For {
            init,
            cond,
            step,
            body,
        } => compile_for(
            init.as_deref(),
            cond.as_deref(),
            step.as_deref(),
            body,
            write_ctx,
        ),
        StatementNode::Match { expr, arms } => compile_match(expr, arms, write_ctx),
        // Требуют потока управления в `Execution` (сейчас — `Fn(…)` без
        // возвращаемого значения). Задача `0025-02b-2`. Отказ **явный**: молча
        // ронять оператор — это и есть дефект, который чинит фича.
        StatementNode::Return(_) => vec![reporting("оператор return")],
        StatementNode::Break => vec![reporting("оператор break")],
        StatementNode::Continue => vec![reporting("оператор continue")],
        // Встроенные формулы (`Guard`/LTL) — метаданные верификации, а не
        // исполняемый код: осознанный no-op, а не забытая ветка. Проверка
        // инвариантов симулятором — кандидат «assert/invariant» из FEATURES.md.
        StatementNode::InlineFormula(_) => vec![],
    }
}

/// Исполнитель, который сообщает о неподдержанном операторе (один раз за шаг).
fn reporting(what: &str) -> Execution {
    let diagnostic = unsupported(what);
    let what = what.to_string();
    Rc::new(move |_ctx: &mut dyn Context| report(&what, &diagnostic))
}

/// Блок — просто последовательность: область видимости общая на всё тело
/// (см. заголовок модуля), заводит её [`compile_block_body`].
fn compile_block(stmts: &[StatementNode], write_ctx: Rc<RefCell<dyn Context>>) -> Vec<Execution> {
    stmts
        .iter()
        .flat_map(|s| compile_statement(s, write_ctx.clone()))
        .collect()
}

/// Объявление локальной переменной: `var имя: тип := инициализатор;`
fn compile_variable(
    name: &str,
    ty: &TypeNode,
    init: Option<&ExpressionNode>,
    _write_ctx: Rc<RefCell<dyn Context>>,
) -> Vec<Execution> {
    let Some(init) = init else {
        // Без инициализатора значение уже расставлено `BlockScope::new`.
        return vec![];
    };
    let name = name.to_string();
    let ty = ty.clone();
    let init = init.clone();
    let f: Execution = Rc::new(move |ctx: &mut dyn Context| {
        match eval_expression(&init, ctx).and_then(|v| {
            eval_core::coerce_to_type(v, &ty).map_err(|e| e.to_diagnostic(Location::Builtin))
        }) {
            Ok(value) => ctx.set_value(&name, value),
            Err(diagnostic) => report(&format!("инициализация '{name}' пропущена"), &diagnostic),
        }
    });
    vec![f]
}

/// `loop [условие] { тело }` — в том числе `while` (условие задано).
fn compile_loop(
    cond: Option<&ExpressionNode>,
    body: &StatementNode,
    write_ctx: Rc<RefCell<dyn Context>>,
) -> Vec<Execution> {
    let body_fns = compile_statement(body, write_ctx);
    let cond = cond.cloned();
    let f: Execution = Rc::new(move |ctx: &mut dyn Context| {
        let mut iterations = 0_u32;
        loop {
            if let Some(cond) = &cond {
                match eval_condition_value(cond, ctx) {
                    Ok(true) => {}
                    Ok(false) => break,
                    Err(diagnostic) => {
                        report("условие цикла", &diagnostic);
                        break;
                    }
                }
            }
            for f in &body_fns {
                f(ctx);
            }
            iterations += 1;
            if iterations >= MAX_ITERATIONS {
                report(
                    "цикл прерван",
                    &Diagnostic::error(
                        Location::Builtin,
                        format!("превышен предел итераций ({MAX_ITERATIONS})"),
                    )
                    .with_code("SIM-018"),
                );
                break;
            }
        }
    });
    vec![f]
}

/// `for init; cond; step { тело }`
fn compile_for(
    init: Option<&StatementNode>,
    cond: Option<&ExpressionNode>,
    step: Option<&ExpressionNode>,
    body: &StatementNode,
    write_ctx: Rc<RefCell<dyn Context>>,
) -> Vec<Execution> {
    // Счётчик `for` объявляется в `init` и попадает в общую область видимости
    // тела блока (её заводит `compile_block_body`) — см. заголовок модуля.
    let init_fns = init
        .map(|s| compile_statement(s, write_ctx.clone()))
        .unwrap_or_default();
    let body_fns = compile_statement(body, write_ctx.clone());
    let step_fns = step
        .map(|e| compile_expression(e, write_ctx.clone()))
        .unwrap_or_default();
    let cond = cond.cloned();

    let f: Execution = Rc::new(move |ctx: &mut dyn Context| {
        for f in &init_fns {
            f(ctx);
        }
        let mut iterations = 0_u32;
        loop {
            if let Some(cond) = &cond {
                match eval_condition_value(cond, ctx) {
                    Ok(true) => {}
                    Ok(false) => break,
                    Err(diagnostic) => {
                        report("условие цикла for", &diagnostic);
                        break;
                    }
                }
            }
            for f in &body_fns {
                f(ctx);
            }
            for f in &step_fns {
                f(ctx);
            }
            iterations += 1;
            if iterations >= MAX_ITERATIONS {
                report(
                    "цикл for прерван",
                    &Diagnostic::error(
                        Location::Builtin,
                        format!("превышен предел итераций ({MAX_ITERATIONS})"),
                    )
                    .with_code("SIM-018"),
                );
                break;
            }
        }
    });
    vec![f]
}

/// `match выражение { образцы => тело, … }` — первая совпавшая ветка.
fn compile_match(
    expr: &ExpressionNode,
    arms: &[MatchArmNode],
    write_ctx: Rc<RefCell<dyn Context>>,
) -> Vec<Execution> {
    let expr = expr.clone();
    let compiled: Vec<(Vec<MatchPatternNode>, Vec<Execution>)> = arms
        .iter()
        .map(|arm| {
            (
                arm.patterns.clone(),
                compile_statement(&arm.body, write_ctx.clone()),
            )
        })
        .collect();

    let f: Execution = Rc::new(move |ctx: &mut dyn Context| {
        let subject = match eval_expression(&expr, ctx) {
            Ok(value) => value,
            Err(diagnostic) => {
                report("выражение match", &diagnostic);
                return;
            }
        };
        for (patterns, body) in &compiled {
            if arm_matches(patterns, &subject, ctx) {
                for f in body {
                    f(ctx);
                }
                return;
            }
        }
    });
    vec![f]
}

fn arm_matches(patterns: &[MatchPatternNode], subject: &Value, ctx: &mut dyn Context) -> bool {
    patterns.iter().any(|pattern| match pattern {
        MatchPatternNode::Wildcard => true,
        MatchPatternNode::Value(expr) => match eval_expression(expr, ctx) {
            Ok(value) => ops::apply_binary(ops::BinOp::Equal, subject, &value)
                .and_then(|v| ops::to_bool(&v))
                .unwrap_or(false),
            Err(diagnostic) => {
                report("образец match", &diagnostic);
                false
            }
        },
    })
}

/// Вычисляет выражение и приводит к логическому.
fn eval_condition_value(expr: &ExpressionNode, ctx: &dyn Context) -> Result<bool, Diagnostic> {
    let value = eval_expression(expr, ctx)?;
    ops::to_bool(&value).map_err(|e| e.to_diagnostic(Location::Builtin))
}

/// `if (cond) { then } [else { else }]`
fn compile_if(
    cond: &ExpressionNode,
    then_: &StatementNode,
    else_: Option<&StatementNode>,
    write_ctx: Rc<RefCell<dyn Context>>,
) -> Vec<Execution> {
    let cond_clone = cond.clone();
    let then_fns = compile_statement(then_, write_ctx.clone());
    let else_fns = else_
        .map(|s| compile_statement(s, write_ctx))
        .unwrap_or_default();

    if then_fns.is_empty() && else_fns.is_empty() {
        return vec![];
    }

    let f: Execution = Rc::new(move |ctx: &mut dyn Context| {
        let branch_true = match eval_condition_value(&cond_clone, ctx) {
            Ok(taken) => taken,
            Err(diagnostic) => {
                report("условие if", &diagnostic);
                false
            }
        };
        if branch_true {
            for f in &then_fns {
                f(ctx);
            }
        } else {
            for f in &else_fns {
                f(ctx);
            }
        }
    });
    vec![f]
}

/// Компилирует выражение-оператор.
///
/// # Приведение типа при записи (S9)
///
/// Значение приводится к объявленному типу цели (`VariableNode::ty()`) **здесь**,
/// а не внутри `Context::set_value`: метод объявлен без `Result`, а S2 (знаковое
/// переполнение) обязан уметь отказать. Обоснование —
/// `docs/development/0025-01-eval-core.md`.
pub(crate) fn compile_expression(
    expr: &ExpressionNode,
    write_ctx: Rc<RefCell<dyn Context>>,
) -> Vec<Execution> {
    match expr {
        ExpressionNode::Assign(lhs, rhs) => {
            let ExpressionNode::Variable(var_rc) = lhs.as_ref() else {
                return vec![reporting("присваивание не в переменную")];
            };
            let (name, ty, loc) = {
                let b = var_rc.borrow();
                (b.name().to_string(), b.ty().clone(), b.loc())
            };
            let rhs_clone = (**rhs).clone();
            let f: Execution = Rc::new(move |ctx: &mut dyn Context| {
                // Раньше здесь было `if let Some(value) = …` без ветки `else` —
                // невычислимое выражение молча пропускало присваивание (Д2).
                match eval_expression(&rhs_clone, ctx).and_then(|value| {
                    eval_core::coerce_to_type(value, &ty).map_err(|e| e.to_diagnostic(loc))
                }) {
                    Ok(value) => ctx.set_value(&name, value),
                    Err(diagnostic) => {
                        report(&format!("присваивание '{name}' пропущено"), &diagnostic)
                    }
                }
            });
            vec![f]
        }
        // Прочие выражения-операторы — прежде всего вызовы функций (Д3):
        // требуется интерпретатор тела `fn`, задача `0025-02b-2`.
        _ => vec![reporting("выражение-оператор")],
    }
}
