//! Обход выражений подстановки и переименование её локальных имён (фича 0444).
//!
//! # Почему обход исчерпывающий
//!
//! Модуль объявляет `deny(clippy::wildcard_enum_match_arm)`: пропущенная форма
//! выражения дала бы **молчаливую** ошибку — подставленное тело сохранило бы
//! исходное имя параметра, то есть читало бы переменную места вызова. Это тот
//! же класс, ради которого исчерпывающими сделаны обход использований (0131) и
//! вычислитель эталона (0025).
//!
//! # Как идентифицируется имя
//!
//! Ссылка на параметр и на локальную переменную — **своя ячейка** у каждого
//! употребления (`expression::resolve_expr` создаёт `Rc` на месте), общей
//! ячейки у них нет. Поэтому переименование идёт **по имени**: ячейка с именем
//! из карты заменяется новой, с тем же типом и владельцем места вызова.
//!
//! ⚠️ Ячейка переменной МОДЕЛИ с таким же именем в теле функции появиться не
//! может: параметр затеняет одноимённое объявление при разрешении.
#![deny(clippy::wildcard_enum_match_arm)]

use std::cell::RefCell;
use std::collections::HashMap;
use std::rc::Rc;

use crate::diagnostics::Location;
use crate::semantic::{ExpressionNode, ModelNode, StatementNode, VariableNode};

/// Применяет `f` к каждому подвыражению **снизу вверх** (сперва потомки).
///
/// Порядок значим: вложенный вызов `outer(inner(x))` обязан быть подставлен
/// раньше внешнего, иначе объявления встанут в `prelude` в обратном порядке.
pub(crate) fn walk_expr_mut(expr: &mut ExpressionNode, f: &mut dyn FnMut(&mut ExpressionNode)) {
    match expr {
        ExpressionNode::ArraySubscript(a, b)
        | ExpressionNode::Power(a, b)
        | ExpressionNode::Multiply(a, b)
        | ExpressionNode::Divide(a, b)
        | ExpressionNode::Modulo(a, b)
        | ExpressionNode::Add(a, b)
        | ExpressionNode::Subtract(a, b)
        | ExpressionNode::ShiftLeft(a, b)
        | ExpressionNode::ShiftRight(a, b)
        | ExpressionNode::BitwiseAnd(a, b)
        | ExpressionNode::BitwiseXor(a, b)
        | ExpressionNode::BitwiseOr(a, b)
        | ExpressionNode::Less(a, b)
        | ExpressionNode::More(a, b)
        | ExpressionNode::LessEqual(a, b)
        | ExpressionNode::MoreEqual(a, b)
        | ExpressionNode::Equal(a, b)
        | ExpressionNode::NotEqual(a, b)
        | ExpressionNode::And(a, b)
        | ExpressionNode::Or(a, b)
        | ExpressionNode::Assign(a, b) => {
            walk_expr_mut(a, f);
            walk_expr_mut(b, f);
        }
        ExpressionNode::ConditionalOperator(a, b, c) => {
            walk_expr_mut(a, f);
            walk_expr_mut(b, f);
            walk_expr_mut(c, f);
        }
        ExpressionNode::ArraySlice(a, _, _)
        | ExpressionNode::Parenthesis(a)
        | ExpressionNode::BitAccess(a, _)
        | ExpressionNode::Not(a)
        | ExpressionNode::BitwiseNot(a)
        | ExpressionNode::UnaryPlus(a)
        | ExpressionNode::Negate(a)
        | ExpressionNode::Cast(a, _) => walk_expr_mut(a, f),
        ExpressionNode::CodeBlock(a, _) => walk_expr_mut(a, f),
        // ⚠️ Именованные аргументы (`NamedFunctionBox`) — узлы **сырого** АСД:
        // на уровне АСД вызов функции и инстанцирование модели неразличимы
        // (0187), и разрешённых подвыражений там нет. Обходится только база.
        ExpressionNode::NamedFunctionBox(a, _) => walk_expr_mut(a, f),
        ExpressionNode::Function(_, args)
        | ExpressionNode::Array(args)
        | ExpressionNode::Initializer(args) => {
            for arg in args.iter_mut() {
                walk_expr_mut(arg, f);
            }
        }
        // Листья: своих подвыражений не имеют.
        ExpressionNode::None
        | ExpressionNode::Unresolved(_)
        | ExpressionNode::Number(_)
        | ExpressionNode::Duration(_)
        | ExpressionNode::Rational(_, _)
        | ExpressionNode::String(_)
        | ExpressionNode::Type(_)
        | ExpressionNode::Address(_, _)
        | ExpressionNode::AnonPort(_)
        | ExpressionNode::Bool(_)
        | ExpressionNode::Variable(_)
        | ExpressionNode::Model(_)
        | ExpressionNode::Condition(_)
        | ExpressionNode::List(_) => {}
    }
    f(expr);
}

/// Применяет `f` к каждому выражению оператора (рекурсивно по телам).
pub(crate) fn walk_stmt_exprs(stmt: &StatementNode, f: &mut dyn FnMut(&ExpressionNode)) {
    let mut copy = stmt.clone();
    walk_stmt_exprs_mut(&mut copy, &mut |e| f(e));
}

/// Изменяемый вариант [`walk_stmt_exprs`].
pub(crate) fn walk_stmt_exprs_mut(
    stmt: &mut StatementNode,
    f: &mut dyn FnMut(&mut ExpressionNode),
) {
    match stmt {
        StatementNode::Block(items) => {
            for item in items.iter_mut() {
                walk_stmt_exprs_mut(item, f);
            }
        }
        StatementNode::Expression(expr, _) => walk_expr_mut(expr, f),
        StatementNode::If { cond, then_, else_ } => {
            walk_expr_mut(cond, f);
            walk_stmt_exprs_mut(then_, f);
            if let Some(alt) = else_ {
                walk_stmt_exprs_mut(alt, f);
            }
        }
        StatementNode::Loop { cond, body } => {
            if let Some(c) = cond {
                walk_expr_mut(c, f);
            }
            walk_stmt_exprs_mut(body, f);
        }
        StatementNode::For {
            init,
            cond,
            step,
            body,
        } => {
            if let Some(i) = init {
                walk_stmt_exprs_mut(i, f);
            }
            if let Some(c) = cond {
                walk_expr_mut(c, f);
            }
            if let Some(s) = step {
                walk_expr_mut(s, f);
            }
            walk_stmt_exprs_mut(body, f);
        }
        StatementNode::Variable(_, _, init, _) => {
            if let Some(e) = init {
                walk_expr_mut(e, f);
            }
        }
        StatementNode::Return(expr) => {
            if let Some(e) = expr {
                walk_expr_mut(e, f);
            }
        }
        StatementNode::Match { expr, arms } => {
            walk_expr_mut(expr, f);
            for arm in arms.iter_mut() {
                walk_stmt_exprs_mut(&mut arm.body, f);
            }
        }
        // Операторы без выражений.
        StatementNode::None
        | StatementNode::Unresolved(_)
        | StatementNode::Continue
        | StatementNode::Break
        | StatementNode::InlineFormula(_) => {}
    }
}

/// Переименовывает локальные имена подставленного оператора.
pub(crate) fn rename_stmt(
    stmt: &mut StatementNode,
    map: &HashMap<String, String>,
    owner: &Rc<RefCell<ModelNode>>,
) {
    if let StatementNode::Variable(name, _, _, _) = stmt
        && let Some(fresh) = map.get(name)
    {
        *name = fresh.clone();
    }
    if let StatementNode::Block(items) = stmt {
        for item in items.iter_mut() {
            rename_stmt(item, map, owner);
        }
        return;
    }
    if let StatementNode::If { then_, else_, .. } = stmt {
        rename_stmt(then_, map, owner);
        if let Some(alt) = else_ {
            rename_stmt(alt, map, owner);
        }
    }
    if let StatementNode::Loop { body, .. } = stmt {
        rename_stmt(body, map, owner);
    }
    if let StatementNode::For { init, body, .. } = stmt {
        if let Some(i) = init {
            rename_stmt(i, map, owner);
        }
        rename_stmt(body, map, owner);
    }
    if let StatementNode::Match { arms, .. } = stmt {
        for arm in arms.iter_mut() {
            rename_stmt(&mut arm.body, map, owner);
        }
    }
    walk_stmt_exprs_mut(stmt, &mut |expr| rename_one(expr, map, owner));
}

/// Заменяет ячейку переменной, если её имя переименовано.
fn rename_one(
    expr: &mut ExpressionNode,
    map: &HashMap<String, String>,
    owner: &Rc<RefCell<ModelNode>>,
) {
    let ExpressionNode::Variable(cell) = expr else {
        return;
    };
    let (name, ty) = match &*cell.borrow() {
        VariableNode::Simple { name, ty, .. }
        | VariableNode::Port { name, ty, .. }
        | VariableNode::Const { name, ty, .. } => (name.clone(), ty.clone()),
        VariableNode::Unresolved => return,
    };
    let Some(fresh) = map.get(&name) else {
        return;
    };
    *expr = ExpressionNode::Variable(Rc::new(RefCell::new(VariableNode::Simple {
        upper: Some(Rc::downgrade(owner)),
        loc: Location::Implicit,
        name: fresh.clone(),
        ty,
        expr: ExpressionNode::None,
    })));
}
