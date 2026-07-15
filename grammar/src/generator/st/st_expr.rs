//! Печать выражений и условий Lam в Structured Text (IEC 61131-3).
//!
//! Задача 0041-04 (часть 1: выражения и условия; операторы, функции и `extern fn`
//! — часть 2). Аналог для цели `c` — `c_expr.rs`, 1736 строк, самый крупный файл
//! C-бэкенда и кандидат фичи 0027 на дробление. Этот модуль обязан не повторить
//! его судьбу: при выходе за ~1000 строк — делить.
//!
//! ## Два печатника, а не один
//!
//! [`ConditionNode`] и [`ExpressionNode`] печатаются **раздельно** — это
//! инвариант языка (ADR 0019, `CLAUDE.md`): у них разная семантика `=` (в условии
//! равенство, в выражении присваивание), и ADR 0019 **отверг** их слияние.
//! Целевой синтаксис у обоих общий, но входные грамматики — разные.
//!
//! ## Удачное совпадение с ST
//!
//! Lam и IEC 61131-3 используют **одни и те же** `:=` (присваивание) и `=`
//! (равенство), поэтому отображение почти тождественно — в отличие от цели `c`,
//! которая вынуждена печатать `==` (`stacker.c:146`).
//!
//! ## Факты MatIEC, определившие форму (пробы 0041-04, 2026-07-15)
//!
//! Три нормы плана опровергнуты проверкой; форма ниже — следствие фактов:
//!
//! - **Побитовые операции не работают на целых.** `n AND m` при `n : USINT` →
//!   `error: Data type mismatch for 'AND' expression`. В IEC `AND`/`OR`/`XOR`/`NOT`
//!   определены на **битовых строках** (`BYTE`/`WORD`/`DWORD`/`LWORD`), а не на
//!   числах. Поэтому побитовые операции идут через преобразование
//!   `USINT_TO_BYTE(…) AND USINT_TO_BYTE(…)` и обратно `BYTE_TO_USINT(…)`.
//! - **Битового доступа `x.0` нет вовсе.** Ни `n.0`, ни `w.0`, ни `w.%X0` MatIEC
//!   не принимает (`invalid expression after ':='`). Форма 3-й редакции ему
//!   неизвестна. Битовый доступ разворачивается в маску:
//!   `(USINT_TO_BYTE(x) AND 16#01) <> 16#00`.
//! - **Сдвигов-операторов нет.** `n << 1` — синтаксическая ошибка; `SHL`/`SHR` —
//!   функции, и тоже требуют битовой строки.
//!
//! Арифметика на битовых строках **запрещена** (`y + 1` при `y : BYTE` →
//! `Data type mismatch for '+'`), поэтому переменные остаются числовыми
//! (`USINT`), а преобразование делается **в месте операции**, а не в объявлении.

// Печатники ещё никто не вызывает: их потребитель — `st_model.rs` (задача
// 0041-03, эмиссия `CASE state OF`), который пишется следующим. Без этого
// разрешения модуль дал бы ~десяток предупреждений `dead_code`, а проект как раз
// сводит их к нулю (фича 0046). Разрешение снимается вместе с появлением
// вызывающего — тот же приём и по той же причине применён в `st_map.rs`
// (задача 0041-01).
#![allow(dead_code)]

use crate::diagnostics::{Diagnostic, Location};
use crate::parser::ast::Member;
use crate::semantic::type_node::TypeNode;
use crate::semantic::{ConditionNode, ExpressionNode, ModelNode, VariableNode};

/// Печатает выражение Lam в текст ST.
///
/// # Ошибки
/// `ST-011` — узел не имеет представления в ST (R4: никакого тихого пропуска).
pub(crate) fn print_expression(
    expr: &ExpressionNode,
    model: &ModelNode,
) -> Result<String, Diagnostic> {
    match expr {
        ExpressionNode::Number(n) => Ok(n.to_string()),
        ExpressionNode::Bool(b) => Ok(bool_literal(*b)),
        ExpressionNode::Rational(text, negative) => {
            Ok(format!("{}{}", if *negative { "-" } else { "" }, text))
        }
        ExpressionNode::Variable(var) => Ok(variable_name(&var.borrow())),
        ExpressionNode::Parenthesis(inner) => Ok(format!("({})", print_expression(inner, model)?)),
        // Логические операции: в ST те же слова, что в IEC-условиях.
        ExpressionNode::Not(a) => Ok(format!("NOT {}", print_expression(a, model)?)),
        ExpressionNode::And(a, b) => binary(a, "AND", b, model),
        ExpressionNode::Or(a, b) => binary(a, "OR", b, model),
        // Арифметика: синтаксис совпадает, кроме остатка (`%` → `MOD`).
        ExpressionNode::Add(a, b) => binary(a, "+", b, model),
        ExpressionNode::Subtract(a, b) => binary(a, "-", b, model),
        ExpressionNode::Multiply(a, b) => binary(a, "*", b, model),
        ExpressionNode::Divide(a, b) => binary(a, "/", b, model),
        ExpressionNode::Modulo(a, b) => binary(a, "MOD", b, model),
        ExpressionNode::Power(a, b) => binary(a, "**", b, model),
        ExpressionNode::UnaryPlus(a) => Ok(format!("+{}", print_expression(a, model)?)),
        ExpressionNode::Negate(a) => Ok(format!("-{}", print_expression(a, model)?)),
        // Сравнения: `!=` в ST записывается `<>`, остальные совпадают.
        ExpressionNode::Equal(a, b) => binary(a, "=", b, model),
        ExpressionNode::NotEqual(a, b) => binary(a, "<>", b, model),
        ExpressionNode::Less(a, b) => binary(a, "<", b, model),
        ExpressionNode::More(a, b) => binary(a, ">", b, model),
        ExpressionNode::LessEqual(a, b) => binary(a, "<=", b, model),
        ExpressionNode::MoreEqual(a, b) => binary(a, ">=", b, model),
        // Побитовые операции — только через битовую строку (см. шапку модуля).
        ExpressionNode::BitwiseAnd(a, b) => bitwise(a, "AND", b, model),
        ExpressionNode::BitwiseOr(a, b) => bitwise(a, "OR", b, model),
        ExpressionNode::BitwiseXor(a, b) => bitwise(a, "XOR", b, model),
        ExpressionNode::BitwiseNot(a) => {
            let bs = bit_string_of_expr(a, model)?;
            let inner = print_expression(a, model)?;
            Ok(format!("{}(NOT {}({}))", bs.from_fn, bs.to_fn, inner))
        }
        ExpressionNode::ShiftLeft(a, b) => shift(a, "SHL", b, model),
        ExpressionNode::ShiftRight(a, b) => shift(a, "SHR", b, model),
        ExpressionNode::BitAccess(inner, member) => bit_access(
            &|| print_expression(inner, model),
            inner_expr_type(inner),
            member,
            model,
        ),
        ExpressionNode::ArraySubscript(var, index) => Ok(format!(
            "{}[{}]",
            variable_name(&var.borrow()),
            print_expression(index, model)?
        )),
        // Присваивание — оператор ST, а не выражение; точку с запятой ставит
        // вызывающий (печатник операторов, часть 2 задачи).
        ExpressionNode::Assign(lhs, rhs) => Ok(format!(
            "{} := {}",
            print_expression(lhs, model)?,
            print_expression(rhs, model)?
        )),
        // Тернарный оператор Lam `c ? a : b` → SEL(G, IN0, IN1): при G=FALSE
        // берётся IN0, при TRUE — IN1, поэтому ветви идут в обратном порядке.
        ExpressionNode::ConditionalOperator(cond, then_, else_) => Ok(format!(
            "SEL({}, {}, {})",
            print_expression(cond, model)?,
            print_expression(else_, model)?,
            print_expression(then_, model)?
        )),
        ExpressionNode::Cast(inner, ty) => cast(inner, ty, model),
        // Узлы без представления в ST. Каждый назван поимённо — ветки `_` здесь
        // НЕТ: `ExpressionNode` не помечен `#[non_exhaustive]`, поэтому новый
        // вариант ЗАВАЛИТ сборку (гарантия ADR 0025), а не проскочит молча.
        ExpressionNode::None => Err(unsupported("пустое выражение")),
        ExpressionNode::Unresolved(_) => Err(unsupported(
            "выражение не прошло семантическое понижение (Unresolved)",
        )),
        ExpressionNode::ArraySlice(_, _, _) => Err(unsupported(
            "срез массива: в IEC 61131-3 нет операции среза",
        )),
        ExpressionNode::Function(_, _) => Err(unsupported(
            "вызов функции: печать функций — часть 2 задачи 0041-04",
        )),
        ExpressionNode::CodeBlock(_, _) => {
            Err(unsupported("блок кода как выражение не выразим в ST"))
        }
        ExpressionNode::NamedFunctionBox(_, _) => Err(unsupported(
            "вызов с именованными аргументами не выразим в ST",
        )),
        ExpressionNode::String(_) => Err(unsupported(
            "строковый литерал: цель ST строк не поддерживает",
        )),
        ExpressionNode::Type(_) => Err(unsupported("тип как выражение")),
        ExpressionNode::Address(_, _) => Err(unsupported(
            "адресный литерал: размещение портов — задача 0041-05 (AT %…)",
        )),
        ExpressionNode::Model(_) => Err(unsupported("модель как выражение")),
        ExpressionNode::Condition(_) => Err(unsupported(
            "именованное условие в выражении: печать — часть 2 задачи 0041-04",
        )),
        ExpressionNode::List(_) => Err(unsupported("список параметров как выражение")),
        ExpressionNode::Array(_) => Err(unsupported(
            "массивный литерал: агрегатная инициализация — часть 2 задачи 0041-04",
        )),
        ExpressionNode::Initializer(_) => Err(unsupported(
            "инициализатор: агрегатная инициализация — часть 2 задачи 0041-04",
        )),
    }
}

/// Печатает условие Lam в текст ST.
///
/// Отдельный печатник — инвариант ADR 0019: в условии `=` это **равенство**,
/// а не присваивание.
///
/// # Ошибки
/// `ST-011` — узел не имеет представления в ST.
pub(crate) fn print_condition(
    cond: &ConditionNode,
    model: &ModelNode,
) -> Result<String, Diagnostic> {
    match cond {
        ConditionNode::Number(n) => Ok(n.to_string()),
        ConditionNode::Bool(b) => Ok(bool_literal(*b)),
        ConditionNode::Rational(text, negative) => {
            Ok(format!("{}{}", if *negative { "-" } else { "" }, text))
        }
        ConditionNode::Variable(var, _) => Ok(variable_name(&var.borrow())),
        ConditionNode::Parenthesis(inner) => Ok(format!("({})", print_condition(inner, model)?)),
        ConditionNode::Not(a) => Ok(format!("NOT {}", print_condition(a, model)?)),
        ConditionNode::And(a, b) => binary_cond(a, "AND", b, model),
        ConditionNode::Or(a, b) => binary_cond(a, "OR", b, model),
        ConditionNode::Add(a, b) => binary_cond(a, "+", b, model),
        ConditionNode::Subtract(a, b) => binary_cond(a, "-", b, model),
        // Ключевое отличие от цели `c`: там печатается `==`, здесь `=`.
        ConditionNode::Equal(a, b) => binary_cond(a, "=", b, model),
        ConditionNode::NotEqual(a, b) => binary_cond(a, "<>", b, model),
        ConditionNode::Less(a, b) => binary_cond(a, "<", b, model),
        ConditionNode::More(a, b) => binary_cond(a, ">", b, model),
        ConditionNode::LessEqual(a, b) => binary_cond(a, "<=", b, model),
        ConditionNode::MoreEqual(a, b) => binary_cond(a, ">=", b, model),
        ConditionNode::BitAccess(inner, member) => bit_access(
            &|| print_condition(inner, model),
            inner_cond_type(inner),
            member,
            model,
        ),
        ConditionNode::ArraySubscript(var, index) => Ok(format!(
            "{}[{}]",
            variable_name(&var.borrow()),
            print_condition(index, model)?
        )),
        // Вариант перечисления → именованная константа, которую объявляет
        // `st_decl` (откат Option C: перечислимых типов MatIEC не знает).
        ConditionNode::EnumVariant(enum_node, variant, _) => {
            Ok(format!("{}_{}", enum_node.borrow().name, variant))
        }
        // Узлы без представления в ST — поимённо, без ветки `_`.
        ConditionNode::None => Err(unsupported("пустое условие")),
        ConditionNode::Unresolved(_) => Err(unsupported(
            "условие не прошло семантическое понижение (Unresolved)",
        )),
        ConditionNode::Function(_, _, _) => Err(unsupported(
            "вызов функции в условии: печать функций — часть 2 задачи 0041-04",
        )),
        ConditionNode::String(_) => Err(unsupported(
            "строковый литерал: цель ST строк не поддерживает",
        )),
        ConditionNode::Model(_) => Err(unsupported("модель как условие")),
        ConditionNode::State(_) => Err(unsupported(
            "состояние как условие: сравнение с состоянием — задача 0041-03",
        )),
    }
}

/// Строит диагностику `ST-011` — узел без представления в ST.
fn unsupported(what: &str) -> Diagnostic {
    Diagnostic::error(
        Location::Codegen,
        format!("Не транслируется в Structured Text: {}", what),
    )
    .with_code("ST-011")
}

/// Литерал `BOOL`.
///
/// MatIEC принимает и числовые `0`/`1` для `BOOL` (проверено пробой — вопреки
/// ожиданию плана), но `2` уже отвергает. Печатаем всегда `FALSE`/`TRUE`:
/// это стандартная форма и она читается однозначно.
fn bool_literal(value: bool) -> String {
    if value { "TRUE" } else { "FALSE" }.to_string()
}

/// Возвращает имя переменной для ST.
///
/// В ST порт — **обычная переменная**, поэтому слой косвенности цели `c`
/// (`(*main->read_numeric)(…)`) здесь исчезает: печатается просто имя.
fn variable_name(var: &VariableNode) -> String {
    match var {
        VariableNode::Simple { name, .. }
        | VariableNode::Port { name, .. }
        | VariableNode::Const { name, .. } => name.clone(),
        VariableNode::Unresolved => "(*неразрешённая переменная*)".to_string(),
    }
}

/// Печатает бинарную операцию выражения.
fn binary(
    a: &ExpressionNode,
    op: &str,
    b: &ExpressionNode,
    model: &ModelNode,
) -> Result<String, Diagnostic> {
    Ok(format!(
        "{} {} {}",
        print_expression(a, model)?,
        op,
        print_expression(b, model)?
    ))
}

/// Печатает бинарную операцию условия.
fn binary_cond(
    a: &ConditionNode,
    op: &str,
    b: &ConditionNode,
    model: &ModelNode,
) -> Result<String, Diagnostic> {
    Ok(format!(
        "{} {} {}",
        print_condition(a, model)?,
        op,
        print_condition(b, model)?
    ))
}

/// Битовая строка, соответствующая целому типу: имя и функции преобразования.
///
/// Имена проверены пробой на всех восьми целых типах (`USINT`…`LINT`).
struct BitString {
    /// Функция «целое → битовая строка» (например, `USINT_TO_BYTE`).
    to_fn: String,
    /// Функция «битовая строка → целое» (например, `BYTE_TO_USINT`).
    from_fn: String,
    /// Число шестнадцатеричных цифр в литерале маски (2 для `BYTE`, 4 для `WORD`…).
    hex_digits: usize,
    /// Разрядность типа.
    bits: u8,
}

/// Подбирает битовую строку для целого типа Lam.
fn bit_string_of_type(ty: &TypeNode) -> Option<BitString> {
    let TypeNode::Integer { bits, signed } = ty else {
        return None;
    };
    let (bs, int_name, digits) = match bits {
        8 => ("BYTE", if *signed { "SINT" } else { "USINT" }, 2),
        16 => ("WORD", if *signed { "INT" } else { "UINT" }, 4),
        32 => ("DWORD", if *signed { "DINT" } else { "UDINT" }, 8),
        64 => ("LWORD", if *signed { "LINT" } else { "ULINT" }, 16),
        _ => return None,
    };
    Some(BitString {
        to_fn: format!("{}_TO_{}", int_name, bs),
        from_fn: format!("{}_TO_{}", bs, int_name),
        hex_digits: digits,
        bits: *bits,
    })
}

/// Тип операнда-выражения, если его удаётся определить статически.
///
/// Определяется только для переменных и скобок вокруг них: этого хватает корпусу,
/// а общий вывод типов выражения — не дело печатника. Если тип неизвестен,
/// вызывающий обязан вернуть `ST-011`, а не догадываться.
fn inner_expr_type(expr: &ExpressionNode) -> Option<TypeNode> {
    match expr {
        ExpressionNode::Variable(var) => variable_type(&var.borrow()),
        ExpressionNode::Parenthesis(inner) => inner_expr_type(inner),
        ExpressionNode::Cast(_, ty) => Some(ty.clone()),
        _ => None,
    }
}

/// Тип операнда-условия, если его удаётся определить статически.
fn inner_cond_type(cond: &ConditionNode) -> Option<TypeNode> {
    match cond {
        ConditionNode::Variable(var, _) => variable_type(&var.borrow()),
        ConditionNode::Parenthesis(inner) => inner_cond_type(inner),
        _ => None,
    }
}

/// Тип переменной.
fn variable_type(var: &VariableNode) -> Option<TypeNode> {
    match var {
        VariableNode::Simple { ty, .. }
        | VariableNode::Port { ty, .. }
        | VariableNode::Const { ty, .. } => Some(ty.clone()),
        VariableNode::Unresolved => None,
    }
}

/// Подбирает битовую строку для операнда выражения.
fn bit_string_of_expr(expr: &ExpressionNode, _model: &ModelNode) -> Result<BitString, Diagnostic> {
    inner_expr_type(expr)
        .as_ref()
        .and_then(bit_string_of_type)
        .ok_or_else(|| {
            unsupported(
                "побитовая операция над операндом, чей целый тип не определяется \
                 статически: в IEC 61131-3 такие операции требуют битовой строки \
                 (BYTE/WORD/DWORD/LWORD), и разрядность обязана быть известна",
            )
        })
}

/// Печатает побитовую операцию через преобразование в битовую строку.
///
/// `n & m` → `BYTE_TO_USINT(USINT_TO_BYTE(n) AND USINT_TO_BYTE(m))`: прямое
/// `n AND m` MatIEC отвергает («Data type mismatch for 'AND' expression»).
fn bitwise(
    a: &ExpressionNode,
    op: &str,
    b: &ExpressionNode,
    model: &ModelNode,
) -> Result<String, Diagnostic> {
    let bs = bit_string_of_expr(a, model)?;
    Ok(format!(
        "{}({}({}) {} {}({}))",
        bs.from_fn,
        bs.to_fn,
        print_expression(a, model)?,
        op,
        bs.to_fn,
        print_expression(b, model)?
    ))
}

/// Печатает сдвиг через `SHL`/`SHR` над битовой строкой.
///
/// Оператора `<<` в ST **нет** (синтаксическая ошибка), а `SHL` на числовом типе
/// отвергается — нужна битовая строка.
fn shift(
    a: &ExpressionNode,
    func: &str,
    b: &ExpressionNode,
    model: &ModelNode,
) -> Result<String, Diagnostic> {
    let bs = bit_string_of_expr(a, model)?;
    Ok(format!(
        "{}({}({}({}), {}))",
        bs.from_fn,
        func,
        bs.to_fn,
        print_expression(a, model)?,
        print_expression(b, model)?
    ))
}

/// Печатает доступ к члену: бит числа либо поле структуры.
///
/// Битовый доступ разворачивается в маску — формы `x.0` в MatIEC **нет вовсе**
/// (ни `n.0`, ни `w.0`, ни `w.%X0`):
/// `sensors_cab.0` → `(USINT_TO_BYTE(sensors_cab) AND 16#01) <> 16#00`.
fn bit_access(
    print_inner: &dyn Fn() -> Result<String, Diagnostic>,
    inner_ty: Option<TypeNode>,
    member: &Member,
    _model: &ModelNode,
) -> Result<String, Diagnostic> {
    match member {
        // Поле структуры: синтаксис ST совпадает с Lam.
        Member::Identifier(id) => Ok(format!("{}.{}", print_inner()?, id.name)),
        Member::Number(n) => {
            let inner = print_inner()?;
            let ty = inner_ty.ok_or_else(|| {
                unsupported(
                    "битовый доступ к операнду, чей тип не определяется статически: \
                     разрядность нужна, чтобы построить маску",
                )
            })?;
            // Бит 0 булева значения — оно само; иных битов у BOOL нет.
            if matches!(ty, TypeNode::Bit | TypeNode::Bool) {
                return if *n == 0 {
                    Ok(inner)
                } else {
                    Err(unsupported(&format!(
                        "бит {} у однобитного значения: в IEC 61131-3 у BOOL нет битов, \
                         кроме нулевого",
                        n
                    )))
                };
            }
            let bs = bit_string_of_type(&ty).ok_or_else(|| {
                unsupported(&format!(
                    "битовый доступ к типу '{}': маска строится только для целых \
                     типов IEC (8/16/32/64 бита)",
                    ty
                ))
            })?;
            if *n < 0 || *n >= bs.bits as i64 {
                return Err(unsupported(&format!(
                    "бит {} вне разрядности типа '{}' ({} бит)",
                    n, ty, bs.bits
                )));
            }
            let mask = 1u128 << n;
            Ok(format!(
                "({}({}) AND 16#{:0width$X}) <> 16#{:0width$X}",
                bs.to_fn,
                inner,
                mask,
                0,
                width = bs.hex_digits
            ))
        }
    }
}

/// Печатает приведение типа через функцию преобразования IEC (`<ИЗ>_TO_<В>`).
fn cast(inner: &ExpressionNode, ty: &TypeNode, model: &ModelNode) -> Result<String, Diagnostic> {
    let to = super::st_type::get_st_type(ty, model)?;
    let from_ty = inner_expr_type(inner).ok_or_else(|| {
        unsupported(
            "приведение операнда, чей тип не определяется статически: имя функции \
             преобразования IEC строится из ОБОИХ типов (<ИЗ>_TO_<В>)",
        )
    })?;
    let from = super::st_type::get_st_type(&from_ty, model)?;
    if from == to {
        return print_expression(inner, model);
    }
    Ok(format!(
        "{}_TO_{}({})",
        from,
        to,
        print_expression(inner, model)?
    ))
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::semantic::tree::construct_model;
    use std::cell::RefCell;
    use std::rc::Rc;

    /// Строит модель из исходника Lam.
    fn model_of(src: &str) -> Rc<RefCell<ModelNode>> {
        let (ast, _) = crate::parse(src, 0).unwrap();
        construct_model(&ast, None, &[]).unwrap()
    }

    /// Печатает условие `cond C = <текст>;` из модели.
    fn cond_of(src_cond: &str) -> String {
        let src = format!(
            "var n: u8 := 0;\nvar m: u8 := 0;\nvar b: bit := 0;\n\
             cond C = {};\nstart S {{ ref D: C; }}\nstate D {{}}",
            src_cond
        );
        let rc = model_of(&src);
        let model = rc.borrow();
        let node = model.conditions.get("C").expect("нет условия C").clone();
        let value = node.value.clone();
        print_condition(&value, &model).expect("условие должно печататься")
    }

    /// Печатает инициализатор переменной `x` как выражение.
    fn expr_of(decl: &str) -> String {
        let src = format!(
            "var n: u8 := 0;\nvar m: u8 := 0;\nvar b: bit := 0;\n{}\n\
             start S {{ always {{ n := n; }} }}",
            decl
        );
        let rc = model_of(&src);
        let model = rc.borrow();
        let var = model.variables.get("x").expect("нет переменной x");
        let VariableNode::Simple { expr, .. } = var else {
            panic!("x не простая переменная");
        };
        print_expression(expr, &model).expect("выражение должно печататься")
    }

    /// `=` в условии — равенство. Цель `c` печатает здесь `==` (`stacker.c:146`).
    #[test]
    fn test_condition_equal_prints_single_equals_not_double() {
        assert_eq!(cond_of("n = m"), "n = m");
    }

    /// `&`/`|`/`!` над булевыми — словами IEC.
    ///
    /// Сверка с `stacker.c:83`: `lift_request && !(lift_op) && …` → в ST
    /// `lift_request AND NOT lift_op AND …`.
    #[test]
    fn test_condition_logical_operators_use_iec_words() {
        assert_eq!(cond_of("b & !b"), "b AND NOT b");
        assert_eq!(cond_of("b | !b"), "b OR NOT b");
    }

    /// `!=` в ST записывается `<>`.
    #[test]
    fn test_condition_not_equal_is_angle_brackets() {
        assert_eq!(cond_of("n != m"), "n <> m");
    }

    /// Реляционные операторы совпадают с Lam.
    #[test]
    fn test_condition_relational_operators_match_lam() {
        assert_eq!(cond_of("n < m"), "n < m");
        assert_eq!(cond_of("n <= m"), "n <= m");
        assert_eq!(cond_of("n > m"), "n > m");
        assert_eq!(cond_of("n >= m"), "n >= m");
    }

    /// Битовый доступ разворачивается в маску: формы `x.0` в MatIEC нет вовсе.
    ///
    /// Форма проверена пробой: `(USINT_TO_BYTE(n) AND 16#01) <> 16#00` — код 0.
    #[test]
    fn test_condition_bit_access_expands_to_mask() {
        assert_eq!(
            cond_of("n.0"),
            "(USINT_TO_BYTE(n) AND 16#01) <> 16#00",
            "битового доступа x.0 в IEC 61131-3 (диалект MatIEC) нет"
        );
    }

    /// Маска старшего бита учитывает номер бита, а не только тип.
    #[test]
    fn test_condition_bit_access_uses_correct_mask_for_high_bit() {
        assert_eq!(cond_of("n.7"), "(USINT_TO_BYTE(n) AND 16#80) <> 16#00");
    }

    /// Бит 0 однобитного значения — само значение; маска ему не нужна.
    #[test]
    fn test_condition_bit_zero_of_bool_is_the_value_itself() {
        assert_eq!(cond_of("b.0"), "b");
    }

    /// Остаток от деления в ST — `MOD`, а не `%`.
    #[test]
    fn test_expression_modulo_is_mod_keyword() {
        assert_eq!(expr_of("var x: u8 := n % m;"), "n MOD m");
    }

    /// Арифметика и сравнения переносятся тождественно.
    #[test]
    fn test_expression_arithmetic_is_identical() {
        assert_eq!(expr_of("var x: u8 := n + m * 2;"), "n + m * 2");
    }

    /// Побитовое И идёт через битовую строку: `n AND m` на USINT MatIEC отвергает.
    ///
    /// Форма проверена пробой: `BYTE_TO_USINT(USINT_TO_BYTE(n) AND USINT_TO_BYTE(m))`.
    #[test]
    fn test_expression_bitwise_and_goes_through_bit_string() {
        assert_eq!(
            expr_of("var x: u8 := n & m;"),
            "BYTE_TO_USINT(USINT_TO_BYTE(n) AND USINT_TO_BYTE(m))",
            "побитовые операции в IEC определены на битовых строках, не на числах"
        );
    }

    /// Сдвиг — функция `SHL` над битовой строкой: оператора `<<` в ST нет.
    #[test]
    fn test_expression_shift_left_is_shl_over_bit_string() {
        assert_eq!(
            expr_of("var x: u8 := n << 1;"),
            "BYTE_TO_USINT(SHL(USINT_TO_BYTE(n), 1))"
        );
    }

    /// Булев литерал печатается словом, а не числом.
    #[test]
    fn test_expression_bool_literal_is_keyword() {
        assert_eq!(expr_of("var x: bit := true;"), "TRUE");
    }

    /// Непечатаемый узел даёт `ST-011`, а не тихий пропуск (R4).
    #[test]
    fn test_unsupported_node_is_st011_error() {
        let rc = model_of("var n: u8 := 0;\nstart S { always { n := n; } }");
        let model = rc.borrow();
        let err = print_expression(&ExpressionNode::None, &model)
            .expect_err("пустое выражение обязано отвергаться");
        assert_eq!(err.code.as_deref(), Some("ST-011"));
    }

    /// Строковый литерал не транслируется — с кодом, а не молча.
    #[test]
    fn test_string_literal_is_st011_error() {
        let rc = model_of("var n: u8 := 0;\nstart S { always { n := n; } }");
        let model = rc.borrow();
        let err = print_expression(&ExpressionNode::String(vec!["s".into()]), &model)
            .expect_err("строка обязана отвергаться");
        assert_eq!(err.code.as_deref(), Some("ST-011"));
    }
}
