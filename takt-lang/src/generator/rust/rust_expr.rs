//! Трансляция выражений и условий Takt в Rust (задача 0050-07).
//!
//! ## Почему два печатника, а не один
//!
//! [`print_expression`] и [`print_condition`] — разные функции по той же
//! причине, по какой `Condition` и `Expression` не слиты (ADR 0019): **у `=`
//! разная семантика** — в выражении присваивание (`=`), в условии равенство
//! (`==`). Один печатник угадывал бы, и угадал бы неверно.
//!
//! ## Чем Rust проще ST
//!
//! У цели `st` числа и биты — непересекающиеся миры (`n & m` требует обёрток
//! `BYTE_TO_USINT`, `CLAUDE.md`); в Rust побитовые операции на целых нативны.
//!
//! ## Ветки `_` здесь нет
//!
//! Каждый непереводимый вариант назван явно и возвращает `Err`: ровно `_ => None`
//! позволил вычислителям симулятора молча разойтись (ADR 0025). Новый вариант
//! `ExpressionNode` обязан **валить сборку**, а не тихо проходить мимо.

use crate::diagnostics::{Diagnostic, Location};
use crate::generator::rust::rust_fixed::{self, FixedOp};
use crate::generator::rust::rust_name::{rust_type_name, rust_value_name};
use crate::generator::rust::rust_needs::function_needs;
use crate::generator::rust::rust_port::port_class;
use crate::parser::ast::Member;
use crate::semantic::type_node::TypeNode;
use crate::semantic::{
    ExpressionNode, FunctionDefinitionNode, ModelNode, PortDirection, VariableNode,
};

// Печатник условий вынесен в `rust_cond` (0088). Реэкспорт держит путь
// `rust_expr::condition_as_bool` для потребителей (правило 11) — импорт в
// `rust_model.rs` не меняется.
pub(crate) use crate::generator::rust::rust_bit::{bit_mask, member_index};
pub(crate) use crate::generator::rust::rust_cond::condition_as_bool;

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
/// Общая переменная корня печатается `self.shared.busy` у владельца-корня и
/// `shared.busy` у под-модели (параметр `&mut Shared`); своя — всегда `self.x`;
/// в теле свободной `fn` — голое имя. Это аналог `VAR_IN_OUT` цели `st`: в C
/// под-модель берёт указатель `main`, а в Rust `self.cabin.tick(&mut self)`
/// заимствовался бы дважды, поэтому общие переменные свёрнуты в `Shared` и идут
/// одним параметром `&mut Shared` (фича 0059).
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
    /// Общие переменные корня — в структуре `Shared` (фича 0059); печать зависит
    /// от [`shared_via_self`](Self::shared_via_self).
    pub(crate) shared: Vec<String>,
    /// Владеет ли модель полем `self.shared` (корень) — иначе получает параметром.
    pub(crate) shared_via_self: bool,
    /// Локальные имена (параметры `fn`, `var` в теле) → печатаются голым именем.
    pub(crate) locals: Vec<String>,
    /// Имена, которым в теле присваивают — для выбора `let` против `let mut`.
    ///
    /// В Takt изменяемость не объявляется (`var` изменяем всегда), в Rust лишний
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
    /// Профиль времени (фича 0134): нужен печатнику выдержки `after` в `rust_cond`
    /// (счётчик тактов vs метка `now_ms`). Берётся из [`RustMap::time_profile`].
    pub(crate) time_profile: crate::semantic::duration::TimeProfile,
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
    pub(crate) fn hal_receiver(&self, what: &str) -> Result<&str, Diagnostic> {
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
            // Общая переменная — в `Shared` (фича 0059); владелец-корень vs параметр.
            let base = if self.shared_via_self {
                "self.shared"
            } else {
                "shared"
            };
            return Ok(format!("{}.{}", base, name));
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
pub(crate) fn variable(var: &VariableNode, scope: &Scope) -> Result<String, Diagnostic> {
    match var {
        VariableNode::Simple { name, loc, .. } => scope.field(name, *loc),
        // Константы живут на уровне модуля (`const MAX: u8 = 10;`) — обращение
        // по имени без `self`.
        VariableNode::Const {
            upper, name, loc, ..
        } => Ok(const_ident(upper.as_ref(), name, *loc)?),
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

/// Имя объявления константы — **с префиксом владельца** (фича 0193; форма
/// заведена задачей 0185-06 для констант-параметров).
///
/// ⚠️ Модуль в цели `rust` один на всю программу, и `const` в нём — **общее**
/// пространство имён: две модели с одноимённой константой разных значений
/// давали одно объявление, и вторая молча получала значение первой (проба:
/// `model A { const K := 2; } model B { const K := 3; }` → единственный
/// `const K: u8 = 2`, то есть `y` считался по чужому значению). Поэтому
/// квалифицируются **все** константы, а не только выведенные из параметра
/// модели: имя обязано быть свойством **объявления**, а не программы — иначе
/// добавление модели переименовывало бы константу в другой, уже написанной
/// (ADR 0193, отвергнутый Option B).
///
/// Ровно то же правило и в цели `sv` (`sv_expr::const_signal`); цель `c`
/// квалифицирует константы с самого начала, цель `st` держит их полями
/// `FUNCTION_BLOCK` — там коллизии нет по устройству.
///
/// ⚠️ Ключ дедупликации объявлений и ключ «константа используется»
/// ([`crate::semantic::unused::const_key`]) обязаны согласоваться с **этим**
/// именем: печать и фильтрация — одно правило, а не два похожих.
pub(crate) fn const_ident(
    upper: Option<&std::rc::Weak<std::cell::RefCell<crate::semantic::ModelNode>>>,
    name: &str,
    loc: Location,
) -> Result<String, Diagnostic> {
    let ident = const_name(name, loc)?;
    let Some(owner) = upper.and_then(|u| u.upgrade()) else {
        return Ok(ident);
    };
    let model: crate::semantic::minimap::Name = owner.into();
    Ok(format!("{}_{}", model.unique_uppercase_snakecase(), ident))
}

/// Печатает вещественный литерал так, чтобы он был литералом `f64`.
///
/// `Rational` хранит **текст** (`"1"`, `"1.5"`), а `1` литералом `f64` в Rust не
/// является — без точки это целое, и `let x: f64 = 1;` не компилируется.
pub(crate) fn rational(text: &str, negative: bool) -> String {
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

/// Q-путь бинарной операции (0061): `Some` тогда и только тогда, когда `expr`
/// имеет тип `q(m, n)` — иначе вызывающий печатает обычную арифметику.
fn fixed_binary(
    expr: &ExpressionNode,
    op: FixedOp,
    a: &ExpressionNode,
    b: &ExpressionNode,
    scope: &Scope,
) -> Option<Result<String, Diagnostic>> {
    rust_fixed::fixed_format(expr).map(|(m, n, sat)| rust_fixed::binary(op, a, b, scope, m, n, sat))
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
    // (`extend_complex.takt`), поэтому случай не теоретический.
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

/// Транслирует выражение Takt в выражение Rust.
///
/// # Ошибки
/// [`RS-011`] на непереводимой конструкции — **не** тихий пропуск.
pub(crate) fn print_expression(expr: &ExpressionNode, scope: &Scope) -> Result<String, Diagnostic> {
    match expr {
        // Длительность (фича 0183) печатается **миллисекундами**; пересчёт зовёт
        // общий слой — своей арифметики времени генератор не заводит.
        ExpressionNode::Duration(nanos) => Ok(crate::semantic::duration::value_millis(
            *nanos,
            Location::Codegen,
            "литерал длительности",
        )?
        .to_string()),
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
        ExpressionNode::Negate(a) => match rust_fixed::fixed_format(expr) {
            Some((m, n, sat)) => rust_fixed::negate(a, scope, m, n, sat),
            None => Ok(format!("(-{})", print_expression(a, scope)?)),
        },

        // Арифметика. Над q(m, n) — масштабирующая Q-арифметика (0061).
        ExpressionNode::Multiply(a, b) => fixed_binary(expr, FixedOp::Multiply, a, b, scope)
            .unwrap_or_else(|| wrapping_or_plain(a, "*", "wrapping_mul", b, scope)),
        ExpressionNode::Divide(a, b) => fixed_binary(expr, FixedOp::Divide, a, b, scope)
            .unwrap_or_else(|| binary(a, "/", b, scope)),
        ExpressionNode::Modulo(a, b) => binary(a, "%", b, scope),
        ExpressionNode::Add(a, b) => fixed_binary(expr, FixedOp::Add, a, b, scope)
            .unwrap_or_else(|| wrapping_or_plain(a, "+", "wrapping_add", b, scope)),
        ExpressionNode::Subtract(a, b) => fixed_binary(expr, FixedOp::Subtract, a, b, scope)
            .unwrap_or_else(|| wrapping_or_plain(a, "-", "wrapping_sub", b, scope)),

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
        ExpressionNode::BitAccess(inner, member) => {
            crate::generator::rust::rust_bit::bit_access(inner, member, scope)
        }

        ExpressionNode::ArraySubscript(var, index) => Ok(format!(
            "{}[{} as usize]",
            variable(&var.borrow(), scope)?,
            print_expression(index, scope)?
        )),

        ExpressionNode::Function(def, args) => call(def, args, scope),

        ExpressionNode::Cast(inner, ty) => {
            // Fixed-point (0061): масштабирующее приведение, когда источник либо
            // цель — q(m, n); иначе обычный `as`.
            if matches!(ty, TypeNode::Fixed { .. }) || rust_fixed::fixed_format(inner).is_some() {
                rust_fixed::cast(inner, ty, scope)
            } else {
                let target = crate::generator::rust::rust_type::rust_type(ty, "приведение типа")?;
                Ok(format!(
                    "({} as {})",
                    print_expression(inner, scope)?,
                    target
                ))
            }
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
            "срез массива: в Takt он не имеет типа-владельца, а в no_std нет alloc",
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
        // Анонимное обращение (фича 0189): у цели `rust` порт — метод
        // HAL-трейта, адреса она не знает (решение 4A ADR 0189).
        ExpressionNode::AnonPort(_) => Err(unsupported(
            "обращение к ячейке по адресу ('#0x…'): цель rust адресов не знает — \
             доступ по адресу дают цели 'c-hal', 'st-at' и 'sv-mmio'",
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
    // Запись одного разряда (фича 0250). Прежде эта ветви не было, и печатник
    // левой части выдавал ЧТЕНИЕ бита: `(((self.b >> 2) & 1) != 0) = true;` —
    // `rustc` отвечал E0070, то есть цель рапортовала об успехе и клала на
    // диск файл, который не собирается.
    if let ExpressionNode::BitAccess(inner, Member::Number(bit)) = target {
        return crate::generator::rust::rust_bit::assign_bit(inner, *bit, value, scope);
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
    // Q-арифметика (0061) НЕ сворачивается: `x := x * y` над q — это масштабный
    // `lam_q`-путь, а не нативное `x *= y` (то дало бы целочисленное умножение
    // представлений без сдвига на n — молча неверный результат и паника на
    // переполнении в debug).
    if rust_fixed::fixed_format(value).is_some() {
        return Ok(None);
    }
    // Беззнаковая арифметика печатается обёрткой (`wrapping_*`, фича 0127):
    // свернуть её в `x += 1` нельзя — `+=` в debug паникует на переполнении, а
    // правило языка требует обёртки mod 2^N.
    if is_wrapping_arith(value) {
        return Ok(None);
    }
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

/// Печатает арифметику беззнакового целого **обёрткой** (`wrapping_*`).
///
/// Правило языка (фича 0127, S1 анализа 0025): переполнение беззнакового целого
/// — обёртка mod 2^N. В C и SV это поведение получается само (тип фиксированной
/// ширины), а Rust в debug-профиле **паникует** на `+`: `attempt to add with
/// overflow`. То есть без обёртки цель `rust` расходилась с эталоном ровно там,
/// где счётчик доходит до края, — а счётчики есть в каждом примере.
///
/// Для **знакового** переполнения обёртка НЕ печатается: по правилу S2 такая
/// программа ошибочна (в C это UB, симулятор даёт `SIM-003`), и паника debug —
/// более полезный исход, чем тихий переход через край.
fn wrapping_or_plain(
    a: &ExpressionNode,
    op: &str,
    wrapping: &str,
    b: &ExpressionNode,
    scope: &Scope,
) -> Result<String, Diagnostic> {
    if is_unsigned_int(a) || is_unsigned_int(b) {
        let left = print_expression(a, scope)?;
        let right = print_expression(b, scope)?;
        return Ok(format!("{left}.{wrapping}({})", unwrap_outer(&right)));
    }
    binary(a, op, b, scope)
}

/// Беззнаковое ли целое у выражения (тип известен и не знаковый).
fn is_unsigned_int(expr: &ExpressionNode) -> bool {
    matches!(
        rust_fixed::expression_type(expr),
        Some(TypeNode::Integer { signed: false, .. })
    )
}

/// Арифметический узел, который печатается обёрткой (см. [`wrapping_or_plain`]).
fn is_wrapping_arith(expr: &ExpressionNode) -> bool {
    match expr {
        ExpressionNode::Add(a, b)
        | ExpressionNode::Subtract(a, b)
        | ExpressionNode::Multiply(a, b) => is_unsigned_int(a) || is_unsigned_int(b),
        _ => false,
    }
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
/// Тот же приём чинит `bit`-порт: `elevator_motor_up := 1` при `bool`-порте.
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
        // `bit`/`bool` в Takt принимает 0/1; в Rust это `false`/`true`.
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
pub(crate) fn call_arguments(
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

// `expression_type` вынесен в `rust_fixed` (0061): вывод типа тематически рядом
// с детектором Q-формата. (`function_return` уехал вместе с печатником условий в
// `rust_cond`, фича 0088.)
pub(crate) use crate::generator::rust::rust_fixed::expression_type;

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
