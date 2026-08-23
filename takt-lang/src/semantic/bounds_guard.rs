//! Guard границ массива в порождённом коде — по флагу (фича 0433).
//!
//! # Зачем
//!
//! Размер массива известен компилятору всегда, а индекс бывает переменным.
//! Замер 2026-08-23 на `d[i]` при `[u8; 3]`, где `i` растёт каждый такт, дал
//! **пять** разных поведений, и ни один инструмент не возразил:
//!
//! | Потребитель | Что делает |
//! |---|---|
//! | эталон | останов, `SIM-010` |
//! | `c`, `c-hal` | **чтение за границей массива** — неопределённое поведение |
//! | `rust` | паника `index out of bounds` |
//! | `st`, `st-at` | чтение по правилам MatIEC |
//! | `sv`, `sv-mmio` | индекс **усечён** по ширине (0365) |
//!
//! Литеральный и константный индекс отвергает семантика (`SE-028`, фичи
//! 0028 и 0434) — здесь речь о значении, известном лишь в такте.
//!
//! # Решение заказчика 2026-08-23
//!
//! Guard **сообщает наружу** и операцию не выполняет; флаг **выключен по
//! умолчанию** (иначе изменился бы вывод всего корпуса и цена прошивки).
//!
//! # Форма
//!
//! Проход оборачивает оператор, содержащий индексацию переменным индексом:
//!
//! ```text
//! p := d[i];
//! ⇓
//! if i < 3 { p := d[i]; } else { bounds_fault := 1; }
//! ```
//!
//! `bounds_fault` — синтетический **выходной порт** типа `bit`. Это и есть
//! «сообщить наружу»: у цели `c` он становится колбэком записи, у `rust` —
//! методом HAL, у `st` и `sv` — выходом модуля. Печатники целей не трогаются
//! вовсе — приём тот же, что у разворота среза (0400) и подъёма результата
//! вызова (0431/0432).
//!
//! ⚠️ **Оборачивается ОПЕРАТОР, а не выражение.** Разворот каждого доступа во
//! временную дал бы то же поведение, но потребовал бы знать тип элемента в
//! каждой позиции; условие же строится из размера массива, известного у базы.
//!
//! ⚠️ **Нижняя граница проверяется только у знакового индекса**: у
//! беззнакового `i >= 0` истинно всегда, и цели отвечали бы предупреждением
//! своего линта (`clippy`, `verilator`), а гейты считают его ошибкой.

use std::cell::RefCell;
use std::collections::HashSet;
use std::rc::Rc;

use crate::diagnostics::Location;
use crate::semantic::PortDirection;
use crate::semantic::type_node::TypeNode;
use crate::semantic::{
    ExpressionNode, FunctionDefinitionNode, ModelNode, NamedCodeBlockDefinitionNode, StateNode,
    StatementNode, VariableNode,
};

/// Имя синтетического порта-признака. Обязано быть допустимым идентификатором
/// **целевых** языков (урок 0400) и не сталкиваться с именем автора.
const FAULT_PORT: &str = "bounds_fault";

/// Вставляет guard границ во все тела дерева.
///
/// Зовётся конвейером цели при `GenerateOptions::bounds_check`; эталон
/// (`takt-sim`) зовёт его своим флагом. По умолчанию не зовётся никем — вывод
/// корпуса от фичи не меняется.
pub fn insert_bounds_guards(model: &Rc<RefCell<ModelNode>>) {
    let mut visited = HashSet::new();
    guard_model(model, &mut visited);
}

fn guard_model(model: &Rc<RefCell<ModelNode>>, visited: &mut HashSet<*const RefCell<ModelNode>>) {
    if !visited.insert(Rc::as_ptr(model)) {
        return;
    }
    let nested: Vec<Rc<RefCell<ModelNode>>> = model.borrow().models.values().cloned().collect();
    guard_bodies(model);
    for child in &nested {
        guard_model(child, visited);
    }
}

/// Обходит тела ОДНОЙ модели; порт-признак заводится, только если он нужен.
///
/// ⚠️ Тела изымаются на время обхода (`mem::take`): размер массива читается у
/// той же модели, и изменяемое заимствование этого не допускает (приём 0400).
fn guard_bodies(model: &Rc<RefCell<ModelNode>>) {
    let (mut functions, mut named_blocks, mut states) = {
        let mut b = model.borrow_mut();
        (
            std::mem::take(&mut b.functions),
            std::mem::take(&mut b.named_blocks),
            std::mem::take(&mut b.states),
        )
    };
    let mut used = false;
    {
        let borrowed = model.borrow();
        let fault = fault_cell(model);
        for func in functions.values_mut() {
            if let FunctionDefinitionNode::Local { body, .. } = func {
                guard_stmt(body, &borrowed, &fault, &mut used);
            }
        }
        for blk in named_blocks.iter_mut() {
            guard_block(blk, &borrowed, &fault, &mut used);
        }
        for st in states.values_mut() {
            match st {
                StateNode::Simple { named_blocks, .. }
                | StateNode::Implement { named_blocks, .. } => {
                    for blk in named_blocks.iter_mut() {
                        guard_block(blk, &borrowed, &fault, &mut used);
                    }
                }
                StateNode::Unresolved => {}
            }
        }
    }
    let mut b = model.borrow_mut();
    b.functions = functions;
    b.named_blocks = named_blocks;
    b.states = states;
    if used && !b.variables.contains_key(FAULT_PORT) {
        b.variables.insert(
            FAULT_PORT.to_string(),
            VariableNode::Port {
                upper: Some(Rc::downgrade(model)),
                loc: Location::Implicit,
                name: FAULT_PORT.to_string(),
                ty: TypeNode::Bit,
                address: ExpressionNode::None,
                init: ExpressionNode::None,
                direction: PortDirection::Out,
            },
        );
    }
}

/// Ячейка синтетического порта — одна на модель, чтобы все присваивания
/// ссылались на один узел.
fn fault_cell(model: &Rc<RefCell<ModelNode>>) -> Rc<RefCell<VariableNode>> {
    Rc::new(RefCell::new(VariableNode::Port {
        upper: Some(Rc::downgrade(model)),
        loc: Location::Implicit,
        name: FAULT_PORT.to_string(),
        ty: TypeNode::Bit,
        address: ExpressionNode::None,
        init: ExpressionNode::None,
        direction: PortDirection::Out,
    }))
}

fn guard_block(
    blk: &mut NamedCodeBlockDefinitionNode,
    model: &ModelNode,
    fault: &Rc<RefCell<VariableNode>>,
    used: &mut bool,
) {
    match blk {
        NamedCodeBlockDefinitionNode::Enter { body, .. }
        | NamedCodeBlockDefinitionNode::Exit { body, .. }
        | NamedCodeBlockDefinitionNode::Always { body, .. }
        | NamedCodeBlockDefinitionNode::Unknown { body, .. }
        | NamedCodeBlockDefinitionNode::Every { body, .. } => guard_stmt(body, model, fault, used),
        NamedCodeBlockDefinitionNode::None | NamedCodeBlockDefinitionNode::Unresolved(_, _) => {}
    }
}

/// Обходит оператор; обернуть можно только элемент блока — там есть куда
/// поставить `if`.
fn guard_stmt(
    stmt: &mut StatementNode,
    model: &ModelNode,
    fault: &Rc<RefCell<VariableNode>>,
    used: &mut bool,
) {
    match stmt {
        StatementNode::Block(items) => {
            for item in items.iter_mut() {
                guard_stmt(item, model, fault, used);
                wrap_item(item, model, fault, used);
            }
        }
        StatementNode::If { then_, else_, .. } => {
            guard_stmt(then_, model, fault, used);
            if let Some(alt) = else_ {
                guard_stmt(alt, model, fault, used);
            }
        }
        StatementNode::Loop { body, .. } => guard_stmt(body, model, fault, used),
        StatementNode::For { init, body, .. } => {
            if let Some(i) = init {
                guard_stmt(i, model, fault, used);
            }
            guard_stmt(body, model, fault, used);
        }
        StatementNode::Match { arms, .. } => {
            for arm in arms.iter_mut() {
                guard_stmt(&mut arm.body, model, fault, used);
            }
        }
        _ => {}
    }
}

/// Оборачивает один оператор, если в нём есть индексация переменным индексом.
fn wrap_item(
    item: &mut StatementNode,
    model: &ModelNode,
    fault: &Rc<RefCell<VariableNode>>,
    used: &mut bool,
) {
    // Составной оператор уже обойдён внутри — его элементы обёрнуты по одному.
    if matches!(
        item,
        StatementNode::Block(_)
            | StatementNode::If { .. }
            | StatementNode::Loop { .. }
            | StatementNode::For { .. }
            | StatementNode::Match { .. }
    ) {
        return;
    }
    let mut checks = Vec::new();
    collect_checks_stmt(item, model, &mut checks);
    let Some(cond) = conjunction(checks) else {
        return;
    };
    *used = true;
    let body = std::mem::take(item);
    *item = StatementNode::If {
        cond: Box::new(cond),
        then_: Box::new(body),
        else_: Box::new(StatementNode::Expression(
            Box::new(ExpressionNode::Assign(
                Box::new(ExpressionNode::Variable(Rc::clone(fault))),
                Box::new(ExpressionNode::Number(1)),
            )),
            Location::Implicit,
        ))
        .into(),
    };
}

/// Соединяет проверки конъюнкцией; `None` — проверять нечего.
fn conjunction(mut checks: Vec<ExpressionNode>) -> Option<ExpressionNode> {
    let first = checks.pop()?;
    Some(checks.into_iter().fold(first, |acc, c| {
        ExpressionNode::And(Box::new(acc), Box::new(c))
    }))
}

fn collect_checks_stmt(stmt: &StatementNode, model: &ModelNode, out: &mut Vec<ExpressionNode>) {
    match stmt {
        StatementNode::Expression(expr, _) => collect_checks_expr(expr, model, out),
        StatementNode::Variable(_, _, Some(init), _) => collect_checks_expr(init, model, out),
        StatementNode::Return(Some(expr)) => collect_checks_expr(expr, model, out),
        _ => {}
    }
}

/// Собирает проверки границ по выражению.
///
/// ⚠️ Обход не исчерпывающий по построению: пропущенная форма даёт **прежнее**
/// поведение (доступ без guard), а не порчу вывода.
fn collect_checks_expr(expr: &ExpressionNode, model: &ModelNode, out: &mut Vec<ExpressionNode>) {
    match expr {
        ExpressionNode::ArraySubscript(base, index) => {
            collect_checks_expr(base, model, out);
            collect_checks_expr(index, model, out);
            if let Some(check) = bound_check(base, index, model) {
                out.push(check);
            }
        }
        ExpressionNode::Assign(l, r)
        | ExpressionNode::Add(l, r)
        | ExpressionNode::Subtract(l, r)
        | ExpressionNode::Multiply(l, r)
        | ExpressionNode::Divide(l, r)
        | ExpressionNode::Modulo(l, r)
        | ExpressionNode::BitwiseAnd(l, r)
        | ExpressionNode::BitwiseOr(l, r)
        | ExpressionNode::BitwiseXor(l, r)
        | ExpressionNode::ShiftLeft(l, r)
        | ExpressionNode::ShiftRight(l, r) => {
            collect_checks_expr(l, model, out);
            collect_checks_expr(r, model, out);
        }
        ExpressionNode::Parenthesis(e)
        | ExpressionNode::Cast(e, _)
        | ExpressionNode::Negate(e)
        | ExpressionNode::BitwiseNot(e)
        | ExpressionNode::Not(e)
        | ExpressionNode::BitAccess(e, _) => collect_checks_expr(e, model, out),
        ExpressionNode::Function(_, args) => {
            for a in args {
                collect_checks_expr(a, model, out);
            }
        }
        _ => {}
    }
}

/// Проверка «индекс в границах» для одного доступа.
///
/// `None` — база не массив известного размера либо индекс уже проверен
/// статически (литерал; константу к этому моменту свернула стадия 2).
fn bound_check(
    base: &ExpressionNode,
    index: &ExpressionNode,
    model: &ModelNode,
) -> Option<ExpressionNode> {
    if matches!(index, ExpressionNode::Number(_)) {
        return None; // судит `SE-028`
    }
    let Some(TypeNode::Array(size, _)) =
        crate::semantic::validate::base_type::base_type(base, model)
    else {
        return None;
    };
    let upper = ExpressionNode::Less(
        Box::new(index.clone()),
        Box::new(ExpressionNode::Number(i128::from(size))),
    );
    // Нижняя граница — только у ЗНАКОВОГО индекса: у беззнакового `i >= 0`
    // истинно всегда, и линт цели ответил бы предупреждением, а гейты считают
    // его ошибкой.
    if signed_index(index, model) {
        return Some(ExpressionNode::And(
            Box::new(upper),
            Box::new(ExpressionNode::MoreEqual(
                Box::new(index.clone()),
                Box::new(ExpressionNode::Number(0)),
            )),
        ));
    }
    Some(upper)
}

fn signed_index(index: &ExpressionNode, model: &ModelNode) -> bool {
    matches!(
        crate::semantic::validate::base_type::base_type(index, model),
        Some(TypeNode::Integer { signed: true, .. })
    )
}
