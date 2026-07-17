//! Трансляция выражений и условий Lam в Rust (задача 0050-07).
//!
//! ## Почему два печатника, а не один
//!
//! [`print_expression`] и [`print_condition`] — разные функции по той же
//! причине, по какой `Condition` и `Expression` не слиты в языке (ADR 0019):
//! **у `=` разная семантика**. В выражении `=` — присваивание
//! (`ExpressionNode::Assign` → `=` в Rust), в условии `=` — равенство
//! (`ConditionNode::Equal` → `==`). Один печатник обязан был бы угадывать, и
//! угадал бы неверно.
//!
//! ## Чем Rust проще ST
//!
//! У цели `st` числа и биты — непересекающиеся миры: `n & m` при `n : USINT`
//! требует `BYTE_TO_USINT(USINT_TO_BYTE(n) AND USINT_TO_BYTE(m))`, а битового
//! доступа `x.0` в MatIEC нет вовсе (`CLAUDE.md`). В Rust побитовые операции на
//! целых **нативны** и транслируются один в один; `x.0` — обычная маска.
//!
//! ## Ветки `_` здесь нет
//!
//! Каждый непереводимый вариант назван явно и возвращает `Err`. Это не
//! педантизм: ровно `_ => None` позволил двум вычислителям симулятора молча
//! разойтись (ADR 0025). Добавление варианта в `ExpressionNode` обязано **валить
//! сборку**, а не тихо проходить мимо.

use crate::diagnostics::{Diagnostic, Location};
use crate::generator::rust::rust_name::{rust_type_name, rust_value_name};
use crate::generator::rust::rust_needs::function_needs;
use crate::generator::rust::rust_port::port_class;
use crate::parser::ast::Member;
use crate::semantic::type_node::TypeNode;
use crate::semantic::{
    ConditionNode, ExpressionNode, FunctionDefinitionNode, ModelNode, PortDirection, VariableNode,
};

/// Строит диагностику `RS-011` — конструкция не транслируется в Rust.
pub(crate) fn unsupported(what: &str) -> Diagnostic {
    Diagnostic::error(
        Location::Codegen,
        format!("Не транслируется в Rust: {}", what),
    )
    .with_code("RS-011")
}

/// Контекст трансляции: как печатать обращение к имени.
///
/// Обращение к переменной зависит от того, **где** мы печатаем:
///
/// | Где | `command` |
/// |---|---|
/// | в корневой модели | `self.command` |
/// | в под-модели (корневая переменная — параметр `&mut`) | `(*command)` |
/// | в теле `fn` (параметр) | `command` |
///
/// Это прямой аналог задачи, которую цель `st` решает через `VAR_IN_OUT`: в C
/// под-модель получает указатель `main` и пишет `main->command`, а в Rust так
/// нельзя — `self.cabin.tick(&mut self)` не заимствуется дважды. Поэтому
/// корневые переменные передаются под-модели отдельными `&mut`-параметрами
/// (заимствования непересекающихся полей законны).
pub(crate) struct Scope<'a> {
    /// Модель, в контексте которой печатается выражение.
    ///
    /// Нужна не для имён, а для **обратного** поиска: `command := Up` приходит
    /// сюда как `Assign(Variable(command), Number(2))` — вариант перечисления
    /// уже свёрнут в своё числовое значение. В C это неотличимо от присваивания
    /// числа и работает (перечисление там — целое), а в Rust `Command` и `2` —
    /// разные типы. Чтобы восстановить `Command::Stop`, нужен доступ к таблице
    /// перечислений модели.
    pub(crate) model: &'a ModelNode,
    /// Корневые переменные, переданные параметрами `&mut` → печатаются `(*x)`.
    pub(crate) shared: Vec<String>,
    /// Локальные имена (параметры `fn`, `var` в теле) → печатаются голым именем.
    pub(crate) locals: Vec<String>,
    /// Имена, которым в теле присваивают — для выбора `let` против `let mut`.
    ///
    /// В Lam изменяемость не объявляется (`var` изменяем всегда), в Rust лишний
    /// `mut` — это `unused_mut`, то есть отказ гейта. Заполняется обходом тела
    /// до печати (`rust_stmt::collect_assigned`).
    pub(crate) assigned: std::collections::BTreeSet<String>,
    /// Выражение доступа к HAL: `self.hal` в корне, `hal` в под-модели.
    pub(crate) hal: String,
    /// Доступен ли `self` (в теле свободной `fn` — нет).
    pub(crate) has_self: bool,
    /// Является ли [`hal`](Self::hal) уже ссылкой `&mut H`.
    ///
    /// У корня `hal` — ПОЛЕ типа `H`, и передать его дальше можно только как
    /// `&mut self.hal`. У под-модели и функции это уже `&mut H`, и `&mut hal`
    /// дало бы `&mut &mut H` — а `Hal` для `&mut H` не реализован. Нужно
    /// перезаимствование `&mut *hal`.
    pub(crate) hal_is_ref: bool,
    /// Экземпляры под-моделей текущей модели: уникальное имя → имя поля.
    ///
    /// Нужны ровно для одной конструкции — `S(Модель) = Состояние` (и её формы
    /// `Модель != Состояние`): чтобы сравнить состояние под-модели, надо знать,
    /// в каком поле она лежит. Цель `c` строит путь от `model->`/`main->`; здесь
    /// поля плоские, поэтому достаточно карты «модель → поле».
    pub(crate) instances: Vec<(String, String)>,
}

impl Scope<'_> {
    /// Печатает HAL в позиции **аргумента** (`&mut …`).
    ///
    /// Отличается от [`hal_receiver`](Self::hal_receiver): получателю метода
    /// (`hal.read_bit(…)`) ссылка берётся автоматически, а аргументу — нет, и её
    /// форма зависит от того, поле перед нами или уже ссылка.
    pub(crate) fn hal_argument(&self, what: &str) -> Result<String, Diagnostic> {
        let hal = self.hal_receiver(what)?;
        if self.hal_is_ref {
            Ok(format!("&mut *{}", hal))
        } else {
            Ok(format!("&mut {}", hal))
        }
    }

    /// Печатает получатель HAL-вызова.
    ///
    /// # Ошибки
    /// [`RS-022`], если HAL в этой области недоступен. Без проверки получатель
    /// напечатался бы пустым, и вызов уехал бы в никуда: `.log_temp(x)` —
    /// синтаксическая ошибка, а не «почти работает». Это сторож против
    /// рассинхрона с предикатом `needs_hal`, решающим, давать ли модели `hal`.
    fn hal_receiver(&self, what: &str) -> Result<&str, Diagnostic> {
        if self.hal.is_empty() {
            return Err(Diagnostic::error(
                Location::Codegen,
                format!(
                    "{} требует доступа к HAL, но он в этой области недоступен",
                    what
                ),
            )
            .with_code("RS-022"));
        }
        Ok(&self.hal)
    }

    /// Печатает обращение к переменной модели по её имени.
    fn field(&self, raw: &str, loc: Location) -> Result<String, Diagnostic> {
        let name = rust_value_name(raw, loc)?;
        if self.locals.iter().any(|l| l == raw) {
            return Ok(name);
        }
        if self.shared.iter().any(|s| s == raw) {
            return Ok(format!("(*{})", name));
        }
        if !self.has_self {
            return Err(Diagnostic::error(
                loc,
                format!(
                    "Обращение к переменной '{}' модели из тела функции не \
                     транслируется в Rust: функция порождается свободной и \
                     состояния модели не видит. Передайте значение параметром",
                    raw
                ),
            )
            .with_code("RS-017"));
        }
        Ok(format!("self.{}", name))
    }
}

/// Печатает чтение порта: `hal.read_bit(InBitPort::Name)`.
fn read_port(
    name: &str,
    ty: &TypeNode,
    direction: PortDirection,
    scope: &Scope,
    loc: Location,
) -> Result<String, Diagnostic> {
    if direction == PortDirection::Out {
        return Err(Diagnostic::error(
            loc,
            format!(
                "Чтение выходного порта '{}' не транслируется в Rust: \
                 HAL-трейт даёт выходному порту только запись",
                name
            ),
        )
        .with_code("RS-018"));
    }
    let class = port_class(ty, name, loc)?;
    Ok(format!(
        "{}.{}({}::{})",
        scope.hal_receiver(&format!("чтение порта '{}'", name))?,
        class.read_fn(),
        class.in_enum(),
        rust_type_name(name, loc)?
    ))
}

/// Печатает запись в порт: `hal.write_bit(OutBitPort::Name, value)`.
pub(crate) fn write_port(
    name: &str,
    ty: &TypeNode,
    direction: PortDirection,
    value: &str,
    scope: &Scope,
    loc: Location,
) -> Result<String, Diagnostic> {
    if direction == PortDirection::In {
        return Err(Diagnostic::error(
            loc,
            format!(
                "Запись во входной порт '{}' не транслируется в Rust: \
                 HAL-трейт даёт входному порту только чтение",
                name
            ),
        )
        .with_code("RS-018"));
    }
    let class = port_class(ty, name, loc)?;
    Ok(format!(
        "{}.{}({}::{}, {})",
        scope.hal_receiver(&format!("запись в порт '{}'", name))?,
        class.write_fn(),
        class.out_enum(),
        rust_type_name(name, loc)?,
        value
    ))
}

/// Печатает обращение к переменной/константе/порту.
fn variable(var: &VariableNode, scope: &Scope) -> Result<String, Diagnostic> {
    match var {
        VariableNode::Simple { name, loc, .. } => scope.field(name, *loc),
        // Константы живут на уровне модуля (`const MAX: u8 = 10;`) — обращение
        // по имени без `self`.
        VariableNode::Const { name, loc, .. } => Ok(const_name(name, *loc)?),
        VariableNode::Port {
            name,
            ty,
            direction,
            loc,
            ..
        } => read_port(name, ty, *direction, scope, *loc),
        VariableNode::Unresolved => Err(unsupported("неразрешённая переменная")),
    }
}

/// Имя константы уровня модуля: `UPPER_SNAKE_CASE`.
pub(crate) fn const_name(raw: &str, loc: Location) -> Result<String, Diagnostic> {
    Ok(rust_value_name(raw, loc)?
        .trim_start_matches("r#")
        .to_uppercase())
}

/// Печатает вещественный литерал так, чтобы он был литералом `f64`.
///
/// `Rational` хранит **текст** (`"1"`, `"1.5"`), а `1` литералом `f64` в Rust не
/// является — без точки это целое, и `let x: f64 = 1;` не компилируется.
fn rational(text: &str, negative: bool) -> String {
    let sign = if negative { "-" } else { "" };
    if text.contains('.') || text.contains('e') || text.contains('E') {
        format!("{}{}", sign, text)
    } else {
        format!("{}{}.0", sign, text)
    }
}

/// Печатает бинарную операцию, **всегда** заключая её в скобки.
///
/// ## Скобки здесь — не стиль, а корректность
///
/// Приоритеты C и Rust **расходятся**, и расходятся молча. Проба 2026-07-16:
///
/// ```text
/// a = 2, b = 2, c = 1;
/// C:     a == b | c   →  1      (то есть (a == b) | c)
/// Rust:  a == b | c   →  false  (то есть a == (b | c) = 2 == 3)
/// ```
///
/// В C `|` слабее `==`, в Rust — **сильнее**. Один и тот же текст означает
/// разное. Печатать выражения «как в C» значило бы получать код, который
/// собирается и делает не то, — то есть ровно тот дефект, ради отсутствия
/// которого заведена цель (`elevator_mini` поймал это лишь потому, что операнд
/// оказался перечислением и дал ошибку типа; на целых всё бы «работало»).
///
/// Структурная расстановка скобок снимает вопрос приоритетов целиком: печатается
/// **дерево**, а не текст. Лишние внешние скобки снимает [`unwrap_outer`] в
/// позиции условия — единственном месте, где `unused_parens` на них ругается.
fn binary(
    a: &ExpressionNode,
    op: &str,
    b: &ExpressionNode,
    scope: &Scope,
) -> Result<String, Diagnostic> {
    Ok(format!(
        "({} {} {})",
        print_expression(a, scope)?,
        op,
        print_expression(b, scope)?
    ))
}

/// Печатает сравнение, приводя операнды друг к другу по типу.
///
/// Нужно ровно там же, где и [`coerce_to`]: вариант перечисления приходит из
/// семантики **числом** (`ExpressionNode` варианта не имеет), и `c == 0` при
/// `c : Constant` — ошибка типа. Цель `c` этого не замечает: перечисление там
/// целое.
fn comparison(
    a: &ExpressionNode,
    op: &str,
    b: &ExpressionNode,
    scope: &Scope,
) -> Result<String, Diagnostic> {
    // Тип берётся у той стороны, где он известен; вторая к нему приводится.
    let left = match expression_type(b) {
        Some(ty) if expression_type(a).is_none() => coerce_to(a, &ty, scope)?,
        _ => print_expression(a, scope)?,
    };
    let right = match expression_type(a) {
        Some(ty) => coerce_to(b, &ty, scope)?,
        None => print_expression(b, scope)?,
    };
    // `x = 0` при булевом `x` даёт после приведения `x == false`, а clippy
    // требует отрицания (`bool_comparison`: «equality checks against false can
    // be replaced by a negation»). Форма `x.2 = 0` в корпусе обычна
    // (`extend_complex.lam`), поэтому случай не теоретический.
    if let Some(simplified) = bool_comparison(&left, op, &right) {
        return Ok(simplified);
    }
    Ok(format!("({} {} {})", left, op, right))
}

/// Упрощает сравнение с булевым литералом: `x == false` → `(!x)`.
fn bool_comparison(left: &str, op: &str, right: &str) -> Option<String> {
    let (value, other) = match (left, right) {
        ("true" | "false", _) => (left, right),
        (_, "true" | "false") => (right, left),
        _ => return None,
    };
    // Отрицание нужно, когда сравнение с `false` на равенство либо с `true` на
    // неравенство.
    let negate = match (op, value) {
        ("==", "false") | ("!=", "true") => true,
        ("==", "true") | ("!=", "false") => false,
        _ => return None,
    };
    Some(if negate {
        format!("(!{})", other)
    } else {
        other.to_string()
    })
}

/// Снимает внешние скобки, если ими обёрнуто выражение целиком.
///
/// Нужен в позиции условия: `if (x != 0) {` даёт `unnecessary parentheses
/// around 'if' condition` — ошибку под `-D warnings` (проба 2026-07-16).
/// Внутренние скобки при этом обязаны остаться: именно они и держат приоритет.
pub(crate) fn unwrap_outer(text: &str) -> &str {
    let bytes = text.as_bytes();
    if bytes.first() != Some(&b'(') || bytes.last() != Some(&b')') {
        return text;
    }
    // Первая скобка обязана закрываться ПОСЛЕДНЕЙ, иначе `(a) | (b)` потерял бы
    // куски.
    let mut depth = 0usize;
    for (i, ch) in text.char_indices() {
        match ch {
            '(' => depth += 1,
            ')' => {
                depth -= 1;
                if depth == 0 && i + ch.len_utf8() != text.len() {
                    return text;
                }
            }
            _ => {}
        }
    }
    &text[1..text.len() - 1]
}

/// Транслирует выражение Lam в выражение Rust.
///
/// # Ошибки
/// [`RS-011`] на непереводимой конструкции — **не** тихий пропуск.
pub(crate) fn print_expression(expr: &ExpressionNode, scope: &Scope) -> Result<String, Diagnostic> {
    match expr {
        ExpressionNode::Number(n) => Ok(n.to_string()),
        ExpressionNode::Rational(text, negative) => Ok(rational(text, *negative)),
        ExpressionNode::Bool(b) => Ok(b.to_string()),
        ExpressionNode::Variable(var) => variable(&var.borrow(), scope),
        // Скобки автора структурно уже учтены: печатник расставляет свои вокруг
        // каждого узла. Печатать ещё одни значило бы получать `((x))` — а это
        // `unused_parens`.
        ExpressionNode::Parenthesis(inner) => print_expression(inner, scope),

        // Унарные.
        ExpressionNode::Not(a) => Ok(format!("(!{})", print_expression(a, scope)?)),
        // В Rust `!` — и логическое, и побитовое отрицание (в C — `~`).
        ExpressionNode::BitwiseNot(a) => Ok(format!("(!{})", print_expression(a, scope)?)),
        ExpressionNode::UnaryPlus(a) => print_expression(a, scope),
        ExpressionNode::Negate(a) => Ok(format!("(-{})", print_expression(a, scope)?)),

        // Арифметика.
        ExpressionNode::Multiply(a, b) => binary(a, "*", b, scope),
        ExpressionNode::Divide(a, b) => binary(a, "/", b, scope),
        ExpressionNode::Modulo(a, b) => binary(a, "%", b, scope),
        ExpressionNode::Add(a, b) => binary(a, "+", b, scope),
        ExpressionNode::Subtract(a, b) => binary(a, "-", b, scope),

        // Побитовые — нативны (в ST требовали бы BYTE_TO_USINT(...)).
        ExpressionNode::ShiftLeft(a, b) => binary(a, "<<", b, scope),
        ExpressionNode::ShiftRight(a, b) => binary(a, ">>", b, scope),
        ExpressionNode::BitwiseAnd(a, b) => binary(a, "&", b, scope),
        ExpressionNode::BitwiseXor(a, b) => binary(a, "^", b, scope),
        ExpressionNode::BitwiseOr(a, b) => binary(a, "|", b, scope),

        // Сравнения. Операнды приводятся друг к другу по типу: `c = X` при
        // `c : Constant` приходит как `Equal(Variable(c), Number(0))` —
        // семантика уже свернула вариант в число (см. `coerce_to`).
        ExpressionNode::Less(a, b) => comparison(a, "<", b, scope),
        ExpressionNode::More(a, b) => comparison(a, ">", b, scope),
        ExpressionNode::LessEqual(a, b) => comparison(a, "<=", b, scope),
        ExpressionNode::MoreEqual(a, b) => comparison(a, ">=", b, scope),
        ExpressionNode::Equal(a, b) => comparison(a, "==", b, scope),
        ExpressionNode::NotEqual(a, b) => comparison(a, "!=", b, scope),

        // Логические.
        ExpressionNode::And(a, b) => binary(a, "&&", b, scope),
        ExpressionNode::Or(a, b) => binary(a, "||", b, scope),

        // `=` в выражении — ПРИСВАИВАНИЕ (ADR 0019/0021), а не сравнение.
        // Запись в порт — не присваивание, а вызов метода HAL.
        ExpressionNode::Assign(target, value) => assign(target, value, scope),

        ExpressionNode::ConditionalOperator(cond, then_, else_) => Ok(format!(
            "if {} {{ {} }} else {{ {} }}",
            print_expression(cond, scope)?,
            print_expression(then_, scope)?,
            print_expression(else_, scope)?
        )),

        // `x.0` → маска. В MatIEC битового доступа нет вовсе; здесь — обычная
        // арифметика, но она типозависима: у `bool` бита 0 нет.
        ExpressionNode::BitAccess(inner, member) => bit_access(inner, member, scope),

        ExpressionNode::ArraySubscript(var, index) => Ok(format!(
            "{}[{} as usize]",
            variable(&var.borrow(), scope)?,
            print_expression(index, scope)?
        )),

        ExpressionNode::Function(def, args) => call(def, args, scope),

        ExpressionNode::Cast(inner, ty) => {
            let target = crate::generator::rust::rust_type::rust_type(ty, "приведение типа")?;
            Ok(format!(
                "({} as {})",
                print_expression(inner, scope)?,
                target
            ))
        }

        ExpressionNode::Array(items) | ExpressionNode::Initializer(items) => {
            let printed = items
                .iter()
                .map(|item| print_expression(item, scope))
                .collect::<Result<Vec<_>, _>>()?;
            Ok(format!("[{}]", printed.join(", ")))
        }

        // Ниже — непереводимое. Ветки `_` нет намеренно (ADR 0025): добавление
        // варианта в `ExpressionNode` обязано валить сборку.
        ExpressionNode::None => Err(unsupported("пустое выражение")),
        ExpressionNode::Unresolved(_) => Err(unsupported("неразрешённое выражение")),
        ExpressionNode::ArraySlice(_, _, _) => Err(unsupported(
            "срез массива: в Lam он не имеет типа-владельца, а в no_std нет alloc",
        )),
        ExpressionNode::CodeBlock(_, _) => Err(unsupported("блок кода в позиции выражения")),
        ExpressionNode::NamedFunctionBox(_, _) => {
            Err(unsupported("вызов с именованными аргументами"))
        }
        ExpressionNode::Power(_, _) => Err(unsupported(
            "возведение в степень: в no_std нет f64::powf (нужен libm), \
             а целочисленный pow отличается семантикой переполнения",
        )),
        ExpressionNode::String(_) => Err(unsupported(
            "строковый литерал вне вызова debug: в no_std нет владеющей строки",
        )),
        ExpressionNode::Type(_) => Err(unsupported("тип в позиции выражения")),
        ExpressionNode::Address(_, _) => Err(unsupported(
            "адресный литерал: цель rust карту адресов не потребляет \
             (порты идут через HAL-трейт)",
        )),
        ExpressionNode::Model(_) => Err(unsupported("модель в позиции выражения")),
        ExpressionNode::Condition(_) => Err(unsupported("именованное условие в позиции выражения")),
        ExpressionNode::List(_) => Err(unsupported("список параметров в позиции выражения")),
    }
}

/// Печатает присваивание; запись в порт превращает в вызов HAL.
fn assign(
    target: &ExpressionNode,
    value: &ExpressionNode,
    scope: &Scope,
) -> Result<String, Diagnostic> {
    if let ExpressionNode::Variable(var) = target {
        let borrowed = var.borrow();
        if let VariableNode::Port {
            name,
            ty,
            direction,
            loc,
            ..
        } = &*borrowed
        {
            let printed = coerce_to(value, ty, scope)?;
            return write_port(name, ty, *direction, unwrap_outer(&printed), scope, *loc);
        }
        if let VariableNode::Const { name, loc, .. } = &*borrowed {
            return Err(Diagnostic::error(
                *loc,
                format!("Присваивание в константу '{}' недопустимо", name),
            )
            .with_code("RS-019"));
        }
    }
    let target_text = print_expression(target, scope)?;
    // `x := x + 1` → `x += 1`. Не косметика: clippy считает `x = x + 1` ручной
    // реализацией составного присваивания (`assign_op_pattern`) и под
    // `-D warnings` отвергает. Совпадение операнда проверяется по НАПЕЧАТАННОМУ
    // тексту, а не по узлам: текст — это ровно то, что увидит компилятор.
    if let Some(compound) = compound_assign(&target_text, value, scope)? {
        return Ok(compound);
    }
    let ty = expression_type(target);
    let printed = match &ty {
        Some(ty) => coerce_to(value, ty, scope)?,
        None => print_expression(value, scope)?,
    };
    // Присваиваемое значение — ещё одна позиция, где внешние скобки лишние:
    // `x = (a - b);` даёт `unnecessary parentheses around assigned value`.
    Ok(format!("{} = {}", target_text, unwrap_outer(&printed)))
}

/// Строит составное присваивание (`x += 1`), если значение имеет форму `x op …`.
fn compound_assign(
    target_text: &str,
    value: &ExpressionNode,
    scope: &Scope,
) -> Result<Option<String>, Diagnostic> {
    let (op, lhs, rhs) = match value {
        ExpressionNode::Add(a, b) => ("+=", a, b),
        ExpressionNode::Subtract(a, b) => ("-=", a, b),
        ExpressionNode::Multiply(a, b) => ("*=", a, b),
        ExpressionNode::Divide(a, b) => ("/=", a, b),
        ExpressionNode::Modulo(a, b) => ("%=", a, b),
        ExpressionNode::BitwiseAnd(a, b) => ("&=", a, b),
        ExpressionNode::BitwiseOr(a, b) => ("|=", a, b),
        ExpressionNode::BitwiseXor(a, b) => ("^=", a, b),
        ExpressionNode::ShiftLeft(a, b) => ("<<=", a, b),
        ExpressionNode::ShiftRight(a, b) => (">>=", a, b),
        _ => return Ok(None),
    };
    if print_expression(lhs, scope)? != target_text {
        return Ok(None);
    }
    let rhs_text = print_expression(rhs, scope)?;
    Ok(Some(format!(
        "{} {} {}",
        target_text,
        op,
        unwrap_outer(&rhs_text)
    )))
}

/// Печатает выражение с оглядкой на **целевой** тип.
///
/// Существует ради перечислений. `command := Up` приходит сюда как
/// `Assign(Variable(command), Number(2))`: семантика уже свернула вариант в его
/// числовое значение, и `ExpressionNode` варианта перечисления просто не имеет.
/// Цель `c` печатает `model->command = 2;`, и это **работает** — перечисление в
/// C есть целое. В Rust `Command` и `2` — разные типы, поэтому вариант нужно
/// восстановить по значению.
///
/// Тот же приём чинит `bit`-порт: `ElevatorMotor_Up := 1` при `bool`-порте.
pub(crate) fn coerce_to(
    value: &ExpressionNode,
    target: &TypeNode,
    scope: &Scope,
) -> Result<String, Diagnostic> {
    match (target, value) {
        (TypeNode::Enum(enum_name), ExpressionNode::Number(n)) => {
            let def = scope
                .model
                .search_enum(enum_name)
                .ok_or_else(|| unsupported(&format!("перечисление '{}' не найдено", enum_name)))?;
            let variant = def.variants.iter().find(|(_, v)| v == n).ok_or_else(|| {
                // Значение вне набора вариантов — в C это молча легло бы в
                // переменную, в Rust представить нечем. Диагностика честнее.
                unsupported(&format!(
                    "значение {} не соответствует ни одному варианту перечисления '{}'",
                    n, enum_name
                ))
            })?;
            Ok(format!(
                "{}::{}",
                rust_type_name(enum_name, def.loc)?,
                rust_type_name(&variant.0, def.loc)?
            ))
        }
        // `bit`/`bool` в Lam принимает 0/1; в Rust это `false`/`true`.
        (TypeNode::Bit | TypeNode::Bool, ExpressionNode::Number(n)) => match n {
            0 => Ok("false".to_string()),
            1 => Ok("true".to_string()),
            other => Err(unsupported(&format!(
                "значение {} не представимо в bool (допустимо 0 или 1)",
                other
            ))),
        },
        // Вещественному полю целый литерал не подходит: `1` не является литералом f64.
        (TypeNode::Rational, ExpressionNode::Number(n)) => Ok(format!("{}.0", n)),
        _ => print_expression(value, scope),
    }
}

/// Печатает битовый доступ `x.N` как маску.
///
/// В MatIEC битового доступа нет вовсе (ни `x.0`, ни `%X0`), и цель `st`
/// разворачивает его в маску по нужде. Здесь маска — тоже единственная форма,
/// но по другой причине: у чисел Rust битового синтаксиса просто нет.
fn bit_access(
    inner: &ExpressionNode,
    member: &Member,
    scope: &Scope,
) -> Result<String, Diagnostic> {
    let base = print_expression(inner, scope)?;
    let bit = member_index(member)?;
    Ok(bit_mask(&base, bit))
}

/// Строит маску битового доступа `x.N`.
///
/// Узел заключается в скобки ЦЕЛИКОМ — как и любой бинарный (см. [`binary`]).
/// Без внешних скобок `x.1 | flag` дало бы `(x >> 1) & 1 != 0 | flag`, что в
/// Rust читается как `… != (0 | flag)`: `|` сильнее `!=`. Поймано гейтом на
/// `elevator.lam` — тот же класс дефекта, ради которого печатник вообще
/// расставляет скобки структурно.
///
/// Сдвиг на 0 не эмитится: `x >> 0` — операция без эффекта
/// (`clippy::identity_op`), то есть отказ гейта. Нулевой бит в корпусе обычен
/// (`SENSORS_CAB.0`), поэтому случай не теоретический.
fn bit_mask(base: &str, bit: u64) -> String {
    if bit == 0 {
        return format!("(({} & 1) != 0)", base);
    }
    format!("((({} >> {}) & 1) != 0)", base, bit)
}

/// Извлекает номер бита из члена `x.N`.
///
/// Доступ по имени (`x.field`) битовым не является: это обращение к полю
/// структуры, и молча выдать за него маску значило бы породить тихо неверный код.
fn member_index(member: &Member) -> Result<u64, Diagnostic> {
    match member {
        Member::Number(index) if *index >= 0 => Ok(*index as u64),
        Member::Number(index) => Err(unsupported(&format!(
            "битовый доступ с отрицательным индексом '{}'",
            index
        ))),
        Member::Identifier(name) => Err(unsupported(&format!(
            "доступ к члену '.{}': поля структур в цели rust пока не транслируются",
            name.name
        ))),
    }
}

/// Печатает вызов функции: встроенной, локальной либо внешней.
fn call(
    def: &std::rc::Rc<std::cell::RefCell<FunctionDefinitionNode>>,
    args: &[ExpressionNode],
    scope: &Scope,
) -> Result<String, Diagnostic> {
    let printed = args
        .iter()
        .map(|a| print_expression(a, scope))
        .collect::<Result<Vec<_>, _>>()?;
    let borrowed = def.borrow();
    match &*borrowed {
        FunctionDefinitionNode::Builtin(name, _, _) => builtin(name, &printed, args, scope),
        local @ FunctionDefinitionNode::Local { name, loc, .. } => Ok(format!(
            "{}({})",
            rust_value_name(name, *loc)?,
            call_arguments(local, &printed, scope)?.join(", ")
        )),
        // `extern fn` → метод HAL (решение (а) задачи 0050-07). Вариант
        // `extern "C" { fn … }` отвергнут: он потребовал бы `unsafe` в
        // порождаемом коде и сломал бы R10 — то есть всю дельту фичи к цели `c`.
        FunctionDefinitionNode::External { name, loc, .. } => Ok(format!(
            "{}.{}({})",
            scope.hal_receiver(&format!("вызов внешней функции '{}'", name))?,
            rust_value_name(name, *loc)?,
            printed.join(", ")
        )),
        FunctionDefinitionNode::None => Err(unsupported("пустое определение функции")),
        FunctionDefinitionNode::Unresolved(_) => Err(unsupported("неразрешённая функция")),
    }
}

/// Строит полный список аргументов вызова локальной функции.
///
/// Аргументы обязаны совпадать с параметрами, которые печатает
/// [`rust_func`](crate::generator::rust::rust_func) для той же функции: и то и
/// другое считает **один** предикат [`function_needs`]. Разойдись они —
/// порождённый код не собрался бы (а в худшем случае связал бы не те значения).
///
/// Порядок — тот же, что в сигнатуре: `hal`, объявленные параметры, переменные
/// модели (в порядке `BTreeMap`).
fn call_arguments(
    def: &FunctionDefinitionNode,
    printed: &[String],
    scope: &Scope,
) -> Result<Vec<String>, Diagnostic> {
    let needs = function_needs(def, scope.model, &mut std::collections::BTreeSet::new())?;
    let mut args = Vec::new();
    // Аргумент — позиция, где внешние скобки лишние: `f((*y))` даёт
    // `unnecessary parentheses around function argument`.
    args.extend(printed.iter().map(|a| unwrap_outer(a).to_string()));
    // Переменная модели печатается ТАК, КАК ВИДНА ВЫЗЫВАЮЩЕМУ: `self.x` в
    // корне, `(*x)` в под-модели, `x` в теле другой функции. Именно поэтому
    // `shared_variables` обязана включать переменные вызываемых функций —
    // иначе под-модели нечего было бы передать.
    for vname in needs.vars.keys() {
        let text = scope.field(vname, Location::Codegen)?;
        args.push(unwrap_outer(&text).to_string());
    }
    // HAL — ПОСЛЕДНИМ аргументом, зеркально сигнатуре: иначе вызов вида
    // `f(&mut hal, hal.read_u8(…))` взял бы `hal` изменяемо дважды (E0499).
    if needs.hal {
        let name = match def {
            FunctionDefinitionNode::Local { name, .. } => name.clone(),
            _ => String::new(),
        };
        args.push(scope.hal_argument(&format!("вызов функции '{}'", name))?);
    }
    Ok(args)
}

/// Печатает вызов встроенной функции.
///
/// Проба 2026-07-16: `min`/`max`/`abs`/`clamp` доступны на `u8`, `i32` и `f64`
/// **без `libm`** — цена профиля `no_std` для встроенных функций нулевая.
fn builtin(
    name: &str,
    printed: &[String],
    args: &[ExpressionNode],
    scope: &Scope,
) -> Result<String, Diagnostic> {
    match (name, printed.len()) {
        ("min", 2) => Ok(format!("{}.min({})", printed[0], printed[1])),
        ("max", 2) => Ok(format!("{}.max({})", printed[0], printed[1])),
        ("abs", 1) => Ok(format!("{}.abs()", printed[0])),
        ("clamp", 3) => Ok(format!(
            "{}.clamp({}, {})",
            printed[0], printed[1], printed[2]
        )),
        // `debug` → метод HAL (решение (а) задачи 0050-07). В no_std нет printf,
        // но профиль no_std не означает «без вывода» — он означает «вывод решает
        // пользователь». Тихо отбросить нельзя: ровно этот дефект закрыла фича
        // 0035 (`: [LTL]` молча терялась).
        ("debug", 1) => {
            let ExpressionNode::String(parts) = &args[0] else {
                return Err(unsupported(
                    "debug с нестроковым аргументом: в no_std форматирования нет, \
                     HAL-метод принимает готовую строку",
                ));
            };
            Ok(format!(
                "{}.debug(\"{}\")",
                scope.hal_receiver("встроенная функция 'debug'")?,
                escape(&parts.join(""))
            ))
        }
        ("S", 1) => Err(unsupported(
            "встроенная функция S вне условия 'S(Модель) = Состояние'",
        )),
        (other, n) => Err(unsupported(&format!(
            "встроенная функция '{}' с {} аргументами",
            other, n
        ))),
    }
}

/// Экранирует строковый литерал Rust.
fn escape(text: &str) -> String {
    text.replace('\\', "\\\\").replace('"', "\\\"")
}

/// Возвращает тип выражения, если он выводится статически.
///
/// Нужен для приведения к `bool` в позиции условия: в C `if (x)` при `x : u8`
/// законно, в Rust — ошибка типа. Без типа операнда угадывать нельзя — тот же
/// урок, что у ST (`ST-011`: «без типа операнда имя функции не построить»).
pub(crate) fn expression_type(expr: &ExpressionNode) -> Option<TypeNode> {
    match expr {
        ExpressionNode::Bool(_) => Some(TypeNode::Bool),
        ExpressionNode::Number(_) => Some(TypeNode::Integer {
            bits: 32,
            signed: true,
        }),
        ExpressionNode::Rational(_, _) => Some(TypeNode::Rational),
        ExpressionNode::Variable(var) => Some(var.borrow().ty().clone()),
        ExpressionNode::Parenthesis(inner) => expression_type(inner),
        ExpressionNode::Cast(_, ty) => Some(ty.clone()),
        // Сравнения и логические операции дают `bool` независимо от операндов.
        ExpressionNode::Less(_, _)
        | ExpressionNode::More(_, _)
        | ExpressionNode::LessEqual(_, _)
        | ExpressionNode::MoreEqual(_, _)
        | ExpressionNode::Equal(_, _)
        | ExpressionNode::NotEqual(_, _)
        | ExpressionNode::And(_, _)
        | ExpressionNode::Or(_, _)
        | ExpressionNode::Not(_)
        | ExpressionNode::BitAccess(_, _) => Some(TypeNode::Bool),
        ExpressionNode::ArraySubscript(var, _) => match var.borrow().ty() {
            TypeNode::Array(_, elem) => Some((**elem).clone()),
            _ => None,
        },
        // Тип вызова — ОБЪЯВЛЕННЫЙ возврат функции. Без этого `if is_ready()`
        // при `fn is_ready() -> bool` не приводится к bool: тип «не выводится»,
        // и честная диагностика RS-011 срабатывает там, где всё известно.
        ExpressionNode::Function(def, _) => function_return(&def.borrow()),
        _ => None,
    }
}

/// Возвращает объявленный тип результата функции.
fn function_return(def: &FunctionDefinitionNode) -> Option<TypeNode> {
    match def {
        FunctionDefinitionNode::Local { ret, .. }
        | FunctionDefinitionNode::External { ret, .. } => Some(ret.clone()),
        // У встроенных возврат описан тем же полем (`min`/`max`/… — Numeric,
        // `debug` — Unit): угадывать не требуется.
        FunctionDefinitionNode::Builtin(_, _, ret) => Some(ret.clone()),
        FunctionDefinitionNode::None | FunctionDefinitionNode::Unresolved(_) => None,
    }
}

/// Печатает выражение в позиции **условия**, приводя его к `bool`.
///
/// `if x` при `x : u8` в C законно, в Rust — ошибка типа. Приведение делается
/// здесь, а не в общем печатнике: в позиции значения то же `x` обязано остаться
/// числом.
pub(crate) fn print_as_bool(expr: &ExpressionNode, scope: &Scope) -> Result<String, Diagnostic> {
    let printed = print_expression(expr, scope)?;
    match expression_type(expr) {
        Some(TypeNode::Bool) | Some(TypeNode::Bit) => Ok(printed),
        Some(TypeNode::Rational) => Ok(format!("({} != 0.0)", printed)),
        Some(TypeNode::Integer { .. }) => Ok(format!("({} != 0)", printed)),
        // Тип не выведен — угадывать нельзя. Молчаливое `!= 0` при `bool` дало бы
        // ошибку сборки в порождённом коде, то есть у пользователя, а не здесь.
        _ => Err(unsupported(&format!(
            "условие '{}': тип не выводится, приведение к bool построить нельзя",
            printed
        ))),
    }
}

/// Транслирует условие Lam в выражение `bool` Rust.
///
/// # Ошибки
/// [`RS-011`] на непереводимой конструкции.
pub(crate) fn print_condition(cond: &ConditionNode, scope: &Scope) -> Result<String, Diagnostic> {
    match cond {
        ConditionNode::Number(n) => Ok(n.to_string()),
        ConditionNode::Rational(text, negative) => Ok(rational(text, *negative)),
        ConditionNode::Bool(b) => Ok(b.to_string()),
        ConditionNode::Variable(var, _) => variable(&var.borrow(), scope),
        ConditionNode::Parenthesis(inner) => print_condition(inner, scope),
        ConditionNode::Not(a) => Ok(format!("(!{})", condition_as_bool(a, scope)?)),

        // `=` в УСЛОВИИ — равенство (ADR 0019). Именно ради этого различия
        // печатник условий отделён от печатника выражений.
        //
        // Спецформа `S(Модель) = Состояние` перехватывается ДО общего случая:
        // её операнды — модель и имя состояния, а не значения, и обычным
        // сравнением их не напечатать.
        ConditionNode::Equal(a, b) => match state_comparison(a, b, "==", scope)? {
            Some(text) => Ok(text),
            None => cond_binary(a, "==", b, scope),
        },
        ConditionNode::NotEqual(a, b) => match state_comparison(a, b, "!=", scope)? {
            Some(text) => Ok(text),
            None => cond_binary(a, "!=", b, scope),
        },
        ConditionNode::Less(a, b) => cond_binary(a, "<", b, scope),
        ConditionNode::More(a, b) => cond_binary(a, ">", b, scope),
        ConditionNode::LessEqual(a, b) => cond_binary(a, "<=", b, scope),
        ConditionNode::MoreEqual(a, b) => cond_binary(a, ">=", b, scope),
        ConditionNode::Add(a, b) => cond_binary(a, "+", b, scope),
        ConditionNode::Subtract(a, b) => cond_binary(a, "-", b, scope),

        // `&`/`|` в условии Lam — побитовые (как в C). На `bool` в Rust они
        // определены и дают `bool`, поэтому трансляция один в один законна и
        // для булевых операндов, и для целых.
        ConditionNode::And(a, b) => cond_bool_binary(a, "&", b, scope),
        ConditionNode::Or(a, b) => cond_bool_binary(a, "|", b, scope),

        ConditionNode::EnumVariant(def, variant, _) => {
            let enum_name = rust_type_name(&def.borrow().name, def.borrow().loc)?;
            Ok(format!(
                "{}::{}",
                enum_name,
                rust_type_name(variant, def.borrow().loc)?
            ))
        }

        ConditionNode::ArraySubscript(var, index) => Ok(format!(
            "{}[{} as usize]",
            variable(&var.borrow(), scope)?,
            print_condition(index, scope)?
        )),

        ConditionNode::BitAccess(inner, member) => {
            let base = print_condition(inner, scope)?;
            Ok(bit_mask(&base, member_index(member)?))
        }

        ConditionNode::Function(def, args, loc) => {
            let printed = args
                .iter()
                .map(|a| print_condition(a, scope))
                .collect::<Result<Vec<_>, _>>()?;
            let borrowed = def.borrow();
            match &*borrowed {
                FunctionDefinitionNode::Builtin(name, _, _) => Err(Diagnostic::error(
                    *loc,
                    format!(
                        "Встроенная функция '{}' в условии перехода не транслируется \
                         в Rust: поддержано только 'S(Модель) = Состояние'",
                        name
                    ),
                )
                .with_code("RS-011")),
                local @ FunctionDefinitionNode::Local { name, .. } => Ok(format!(
                    "{}({})",
                    rust_value_name(name, *loc)?,
                    call_arguments(local, &printed, scope)?.join(", ")
                )),
                FunctionDefinitionNode::External { name, .. } => Ok(format!(
                    "{}.{}({})",
                    scope.hal_receiver(&format!("вызов внешней функции '{}'", name))?,
                    rust_value_name(name, *loc)?,
                    printed.join(", ")
                )),
                FunctionDefinitionNode::None | FunctionDefinitionNode::Unresolved(_) => {
                    Err(unsupported("неразрешённая функция в условии"))
                }
            }
        }

        ConditionNode::None => Err(unsupported("пустое условие")),
        ConditionNode::Unresolved(_) => Err(unsupported("неразрешённое условие")),
        ConditionNode::String(_) => Err(unsupported("строковый литерал в условии")),
        ConditionNode::Model(_, _) => Err(unsupported(
            "модель в позиции условия вне формы 'S(Модель) = Состояние'",
        )),
        ConditionNode::State(_) => Err(unsupported(
            "состояние в позиции условия вне формы 'S(Модель) = Состояние'",
        )),
    }
}

/// Распознаёт спецформу `S(Модель) = Состояние` и печатает сравнение состояний.
///
/// Возвращает `None`, если условие к этой форме не относится — тогда печатается
/// обычное сравнение.
///
/// ## Почему имя состояния берётся строкой
///
/// Правая часть (`End` в `S(Ping) = End`) приходит **неразрешённой**: `End` —
/// состояние модели-аргумента, а не той, где записано условие, и семантика
/// разрешить его не может (`CLAUDE.md`: проход `resolve_state_references`
/// запрещён — он ломает ровно эту конструкцию, охраняется тестом
/// `syntax_simple`). Разрешение выполняется здесь, в области видимости
/// модели-аргумента. Цель `c` поступает так же (`generate_state_comparison`).
fn state_comparison(
    left: &ConditionNode,
    right: &ConditionNode,
    op: &str,
    scope: &Scope,
) -> Result<Option<String>, Diagnostic> {
    let Some(model) = model_of(left) else {
        return Ok(None);
    };
    let state_name = match right {
        ConditionNode::Variable(v, ..) => v.borrow().name().to_string(),
        // Неразрешённое имя — ШТАТНЫЙ случай (см. заголовок функции).
        ConditionNode::Unresolved(crate::parser::ast::Condition::Variable(id)) => id.name.clone(),
        // Имя, случайно совпавшее с состоянием объемлющей модели: семантика
        // разрешила его в ЧУЖОЙ области. Берём только имя.
        ConditionNode::State(state) => state.borrow().name().to_string(),
        _ => return Ok(None),
    };

    let unique = crate::semantic::minimap::Name::from(std::rc::Rc::clone(&model));
    let field = scope
        .instances
        .iter()
        .find(|(u, _)| u == unique.unique())
        .map(|(_, f)| f.clone())
        .ok_or_else(|| {
            unsupported(&format!(
                "условие по состоянию модели '{}': её экземпляр не найден среди \
                 под-моделей текущего состояния",
                unique.local()
            ))
        })?;

    // Поле `state` и перечисление состояний приватны, но лежат в ЭТОМ ЖЕ
    // модуле — обращение законно. Имя перечисления строится той же формулой,
    // что и в `rust_model::StateTable`.
    Ok(Some(format!(
        "(self.{}.state {} {}State::{})",
        field,
        op,
        unique.unique_camelcase(),
        rust_type_name(&state_name, Location::Codegen)?
    )))
}

/// Извлекает модель из левой части: `Модель` либо `S(Модель)`.
fn model_of(
    cond: &ConditionNode,
) -> Option<std::rc::Rc<std::cell::RefCell<crate::semantic::ModelNode>>> {
    match cond {
        ConditionNode::Model(model, _) => Some(std::rc::Rc::clone(model)),
        ConditionNode::Function(fun, args, _) => {
            if !matches!(&*fun.borrow(), FunctionDefinitionNode::Builtin("S", ..)) {
                return None;
            }
            // Арность `S` — ровно один параметр, проверена семантикой.
            match args.first().map(|a| a.as_ref())? {
                ConditionNode::Model(model, _) => Some(std::rc::Rc::clone(model)),
                _ => None,
            }
        }
        _ => None,
    }
}

/// Печатает бинарное условие. Скобки — см. [`binary`].
fn cond_binary(
    a: &ConditionNode,
    op: &str,
    b: &ConditionNode,
    scope: &Scope,
) -> Result<String, Diagnostic> {
    Ok(format!(
        "({} {} {})",
        print_condition(a, scope)?,
        op,
        print_condition(b, scope)?
    ))
}

/// Печатает `&`/`|`, приводя операнды к `bool`.
///
/// Операнд-порт (`ElevatorMotor_SensorU`) в Lam используется как условие
/// напрямую; в Rust `bool & bool` законно, но `u8 & bool` — нет.
fn cond_bool_binary(
    a: &ConditionNode,
    op: &str,
    b: &ConditionNode,
    scope: &Scope,
) -> Result<String, Diagnostic> {
    Ok(format!(
        "({} {} {})",
        condition_as_bool(a, scope)?,
        op,
        condition_as_bool(b, scope)?
    ))
}

/// Возвращает тип условия, если он выводится статически.
pub(crate) fn condition_type(cond: &ConditionNode) -> Option<TypeNode> {
    match cond {
        ConditionNode::Bool(_) => Some(TypeNode::Bool),
        ConditionNode::Number(_) => Some(TypeNode::Integer {
            bits: 32,
            signed: true,
        }),
        ConditionNode::Rational(_, _) => Some(TypeNode::Rational),
        ConditionNode::Variable(var, _) => Some(var.borrow().ty().clone()),
        ConditionNode::Parenthesis(inner) => condition_type(inner),
        ConditionNode::Equal(_, _)
        | ConditionNode::NotEqual(_, _)
        | ConditionNode::Less(_, _)
        | ConditionNode::More(_, _)
        | ConditionNode::LessEqual(_, _)
        | ConditionNode::MoreEqual(_, _)
        | ConditionNode::Not(_)
        | ConditionNode::And(_, _)
        | ConditionNode::Or(_, _)
        | ConditionNode::BitAccess(_, _) => Some(TypeNode::Bool),
        ConditionNode::ArraySubscript(var, _) => match var.borrow().ty() {
            TypeNode::Array(_, elem) => Some((**elem).clone()),
            _ => None,
        },
        ConditionNode::Function(def, _, _) => function_return(&def.borrow()),
        // Вариант перечисления имеет тип своего перечисления: нужен, чтобы
        // `command = Up` сравнивалось, а не приводилось к bool.
        ConditionNode::EnumVariant(def, _, _) => Some(TypeNode::Enum(def.borrow().name.clone())),
        _ => None,
    }
}

/// Печатает условие, приводя его к `bool`.
///
/// Точка входа для guard'ов рёбер: `ref Next: x;` при `x : u8` в C означает
/// `if (x)`, в Rust — `if x != 0`.
pub(crate) fn condition_as_bool(cond: &ConditionNode, scope: &Scope) -> Result<String, Diagnostic> {
    let printed = print_condition(cond, scope)?;
    match condition_type(cond) {
        Some(TypeNode::Bool) | Some(TypeNode::Bit) => Ok(printed),
        Some(TypeNode::Rational) => Ok(format!("({} != 0.0)", printed)),
        Some(TypeNode::Integer { .. }) => Ok(format!("({} != 0)", printed)),
        _ => Err(unsupported(&format!(
            "условие '{}': тип не выводится, приведение к bool построить нельзя",
            printed
        ))),
    }
}
