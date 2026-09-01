//! Трансляция операторов Takt в Rust (задача 0050-07).
//!
//! ## Чем Rust проще ST
//!
//! Цель `st` вынуждена выкручиваться: объявление переменной **поднимается** в
//! шапку POU (в теле его быть не может), вызов функции в позиции оператора
//! требует изобретения переменной-приёмника, а `match` разворачивается в
//! `IF/ELSIF`, потому что образцы Takt — произвольные выражения. В Rust всё это
//! ложится один в один: `let` живёт в теле, вызов — законный оператор, `match`
//! есть в языке.
//!
//! Единственная тонкость — `match`: образцы Takt произвольны, а образцы Rust —
//! нет (`match x { y => … }` связал бы `y` как **новое имя**, а не сравнил с
//! ним). Поэтому `match` транслируется в цепочку `if`/`else if` — так же, как в
//! ST, но по другой причине: там не было конструкции, здесь она есть, но с
//! другой семантикой.

use crate::diagnostics::Diagnostic;
use crate::generator::indent::Printer;
use crate::generator::rust::rust_expr::{
    Scope, print_as_bool, print_expression, unsupported, unwrap_outer,
};
use crate::generator::rust::rust_live::{
    Folded, deferred_needs_mut, fold_assignment, fold_target, initializer_is_dead,
};
use crate::generator::rust::rust_name::rust_value_name;
use crate::generator::rust::rust_type::rust_type;
use crate::semantic::{ExpressionNode, MatchPatternNode, StatementNode};
use std::collections::{BTreeMap, BTreeSet};

/// Побочный результат печати тела: предупреждения, которые обязан увидеть автор.
#[derive(Default)]
pub(crate) struct StmtOutput {
    /// Предупреждения (`RS-010`).
    pub(crate) warnings: Vec<Diagnostic>,
}

/// Собирает имена, которым в теле хоть раз присваивают.
///
/// Нужно, чтобы решить, печатать ли `let` или `let mut`. В Takt изменяемость не
/// объявляется — `var` изменяем всегда, — а в Rust лишний `mut` это `unused_mut`,
/// то есть отказ гейта. Вывести его «по объявлению» нельзя: `var delta := 1;`,
/// которому больше не присваивают, обязан стать `let`, а не `let mut`.
///
/// Обход идёт по **всему** телу заранее: печать потоковая, и в точке `let`
/// будущие присваивания ещё не видны.
pub(crate) fn collect_assigned(stmt: &StatementNode, out: &mut BTreeSet<String>) {
    match stmt {
        StatementNode::Block(items) => items.iter().for_each(|i| collect_assigned(i, out)),
        StatementNode::Expression(expr, _) => collect_assigned_expr(expr, out),
        StatementNode::Variable(_, _, Some(init), _) => collect_assigned_expr(init, out),
        StatementNode::If { cond, then_, else_ } => {
            collect_assigned_expr(cond, out);
            collect_assigned(then_, out);
            if let Some(alt) = else_ {
                collect_assigned(alt, out);
            }
        }
        StatementNode::Loop { cond, body } => {
            if let Some(cond) = cond {
                collect_assigned_expr(cond, out);
            }
            collect_assigned(body, out);
        }
        StatementNode::For {
            init,
            cond,
            step,
            body,
        } => {
            if let Some(init) = init {
                collect_assigned(init, out);
            }
            if let Some(cond) = cond {
                collect_assigned_expr(cond, out);
            }
            if let Some(step) = step {
                collect_assigned_expr(step, out);
            }
            collect_assigned(body, out);
        }
        StatementNode::Match { expr, arms } => {
            collect_assigned_expr(expr, out);
            arms.iter().for_each(|a| collect_assigned(&a.body, out));
        }
        StatementNode::Return(Some(expr)) => collect_assigned_expr(expr, out),
        StatementNode::Return(None)
        | StatementNode::Variable(_, _, None, _)
        | StatementNode::Continue
        | StatementNode::Break
        | StatementNode::InlineFormula(_)
        | StatementNode::None
        | StatementNode::Unresolved(_) => {}
    }
}

/// Собирает имена, которым присваивают, из выражения.
fn collect_assigned_expr(expr: &ExpressionNode, out: &mut BTreeSet<String>) {
    if let ExpressionNode::Assign(target, value) = expr {
        if let ExpressionNode::Variable(var) = &**target {
            out.insert(var.borrow().name().to_string());
        }
        // Запись в ПОЛЕ структуры (фича 0293) делает изменяемой саму переменную:
        // `r.output := …` требует `let mut r`. Прежде поля структур цель не
        // переводила вовсе, и этот случай не возникал.
        if let ExpressionNode::BitAccess(base, crate::parser::ast::Member::Identifier(_)) =
            &**target
            && let ExpressionNode::Variable(var) = &**base
        {
            out.insert(var.borrow().name().to_string());
        }
        collect_assigned_expr(target, out);
        collect_assigned_expr(value, out);
        return;
    }
    // Присваивание — оператор, а не подвыражение: в Takt его негде спрятать
    // глубже одного уровня. Обход ограничен теми узлами, что реально несут
    // вложенные выражения-операторы.
    match expr {
        ExpressionNode::Parenthesis(inner) | ExpressionNode::CodeBlock(inner, _) => {
            collect_assigned_expr(inner, out)
        }
        ExpressionNode::ConditionalOperator(a, b, c) => {
            collect_assigned_expr(a, out);
            collect_assigned_expr(b, out);
            collect_assigned_expr(c, out);
        }
        _ => {}
    }
}

/// Печатает свёрнутое значение объявления (`rust_live::Folded`).
///
/// Ветки печатаются как **выражения** (`if c { a } else { b }`), а не как
/// присваивания: в этом и смысл свёртки.
fn print_folded(
    folded: &Folded,
    ty: &crate::semantic::type_node::TypeNode,
    scope: &Scope,
) -> Result<String, Diagnostic> {
    match folded {
        Folded::Value(expr) => Ok(unwrap_outer(&crate::generator::rust::rust_expr::coerce_to(
            expr, ty, scope,
        )?)
        .to_string()),
        // Цепочка из `match` (фича 0216): образец сравнивается с ТИПОМ
        // разбираемого выражения — тем же приёмом, что и в печати оператора
        // `match`. Без обратного отображения `match mode { 0 => … }` при
        // `mode : Mode` дало бы сравнение перечисления с целым.
        Folded::Chain {
            subject,
            arms,
            otherwise,
        } => {
            let printed_subject = print_expression(subject, scope)?;
            let subject_type = crate::generator::rust::rust_expr::expression_type(subject);
            let mut text = String::new();
            for (patterns, value) in arms {
                let mut tests = Vec::new();
                for pattern in patterns {
                    let printed = match &subject_type {
                        Some(ty) => {
                            crate::generator::rust::rust_expr::coerce_to(pattern, ty, scope)?
                        }
                        None => print_expression(pattern, scope)?,
                    };
                    tests.push(format!("{printed_subject} == {printed}"));
                }
                let head = if text.is_empty() { "if" } else { " else if" };
                text.push_str(&format!(
                    "{head} {} {{ {} }}",
                    tests.join(" || "),
                    print_folded(value, ty, scope)?
                ));
            }
            let tail = print_folded(otherwise, ty, scope)?;
            // Ветвей с образцами может не быть вовсе (`match x { _ => … }`) —
            // тогда цепочки нет и печатается одно значение.
            if text.is_empty() {
                return Ok(tail);
            }
            text.push_str(&format!(" else {{ {tail} }}"));
            Ok(text)
        }
        Folded::Branch { cond, then_, else_ } => Ok(format!(
            "if {} {{ {} }} else {{ {} }}",
            unwrap_outer(&print_as_bool(cond, scope)?),
            print_folded(then_, ty, scope)?,
            print_folded(else_, ty, scope)?
        )),
    }
}

/// Печатает объявление, переехавшее к своему затирающему оператору.
///
/// `stmt` — тот самый оператор (безусловное присваивание либо `if/else`);
/// `rest` — что идёт за ним (нужно, чтобы решить вопрос о `mut`).
fn emit_folded_declaration(
    name: &str,
    ty: &crate::semantic::type_node::TypeNode,
    stmt: &StatementNode,
    rest: &[StatementNode],
    scope: &mut Scope,
    p: &mut Printer,
) -> Result<(), Diagnostic> {
    let folded = fold_assignment(name, stmt).ok_or_else(|| {
        unsupported(&format!(
            "объявление '{}': форма присваивания не сворачивается",
            name
        ))
    })?;
    let ident = rust_value_name(name, crate::diagnostics::Location::Codegen)?;
    let ty_name = rust_type(ty, &format!("переменная '{}'", name))?;
    let value = print_folded(&folded, ty, scope)?;
    // `mut` считается по остатку ПОСЛЕ точки инициализации: присваивание на
    // каждом пути здесь — инициализация, а не изменение.
    let mutable = if rest.iter().any(|s| assigns_later(name, s)) {
        "mut "
    } else {
        ""
    };
    p.ident(&format!(
        "let {}{}: {} = {};",
        mutable, ident, ty_name, value
    ))
    .nl();
    scope.locals.push(name.to_string());
    Ok(())
}

/// Нужно ли отложенному объявлению умолчание (фича 0411).
///
/// Агрегат — массив или структура — в Rust обязан быть инициализирован
/// **целиком** до записи по индексу либо в поле: `let part: [u8; 2]; part[0] =
/// …;` даёт `E0381` («used binding `part` isn't initialized»), тогда как
/// эталон, `c`, `st` и `sv` тот же вход исполняют.
///
/// ⚠️ Скаляру умолчание не нужно и **вредно**: там отложенная форма законна, а
/// лишнее значение дало бы `unused_assignments` — отказ гейта под
/// `-D warnings` (тот самый класс, ради которого заведена 0216).
///
/// ⚠️ Бит-вектор `[bit; N ≤ 64]` — упакованный **скаляр** (0078), и умолчания
/// ему не нужно; при `N > 64` он массив слов, и нужно.
fn needs_deferred_default(ty: &crate::semantic::type_node::TypeNode) -> bool {
    use crate::semantic::type_node::TypeNode;
    match ty {
        TypeNode::Array(..) => {
            crate::semantic::bit_vector::is_bit_vector(ty).is_none_or(|bits| bits > 64)
        }
        TypeNode::Struct(_) => true,
        _ => false,
    }
}

/// Присваивают ли переменной в этом операторе (для решения о `mut`).
fn assigns_later(name: &str, stmt: &StatementNode) -> bool {
    let mut assigned = BTreeSet::new();
    collect_assigned(stmt, &mut assigned);
    assigned.contains(name)
}

/// Печатник ХВОСТОВОГО оператора блока.
///
/// Возвращает `true`, если оператор напечатан им, и `false` — если печатать его
/// надо обычным путём. Существует ради тела функции: там завершающий `return x;`
/// обязан стать хвостовым выражением `x` (`needless_return`).
pub(crate) type TailPrinter<'a> = &'a dyn Fn(
    &StatementNode,
    &mut Scope,
    &mut Printer,
    &mut StmtOutput,
) -> Result<bool, Diagnostic>;

/// Печатает блок операторов.
///
/// Общая точка для тела блока и тела функции: обе обязаны одинаково переносить
/// объявления с мёртвым инициализатором (`rust_live`). Раньше тело функции
/// печаталось своим циклом и переноса не делало — из-за чего `travel_time` в
/// `stacker.takt` оставался с отложенными объявлениями (`needless_late_init`).
///
/// `tail` — если задан, ПОСЛЕДНИЙ оператор печатается этой функцией (у тела
/// функции это хвостовое выражение вместо `return`).
pub(crate) fn print_block(
    items: &[StatementNode],
    tail: Option<TailPrinter>,
    scope: &mut Scope,
    p: &mut Printer,
    out: &mut StmtOutput,
) -> Result<(), Diagnostic> {
    // Объявление с мёртвым инициализатором ПЕРЕЕЗЖАЕТ к оператору, который его
    // затирает, и сворачивается в `let x = …`. Иначе остаётся отложенная форма
    // (`let x: T;`), которую clippy не принимает (`needless_late_init`). Корпус
    // требует именно переезда: `travel_time` в `stacker.takt` объявляет
    // `ds`/`dr`/`dy`/`t` подряд, а затирает их ниже — по одному `if/else`.
    //
    // Безопасность переезда обосновывает `rust_live::fold_target`: между старым
    // и новым местом переменная не упоминается.
    let mut folded_at: BTreeMap<usize, (String, crate::semantic::type_node::TypeNode)> =
        BTreeMap::new();
    let mut moved: BTreeSet<usize> = BTreeSet::new();
    for (i, item) in items.iter().enumerate() {
        let StatementNode::Variable(name, ty, Some(_), _) = item else {
            continue;
        };
        let rest_of = &items[i + 1..];
        if !initializer_is_dead(name, rest_of) {
            continue;
        }
        let Some(offset) = fold_target(name, rest_of) else {
            continue;
        };
        let target = i + 1 + offset;
        // На один оператор — одно объявление: `if/else`, затирающий две
        // переменные сразу, свернуть в одно `let` нельзя.
        if folded_at.contains_key(&target) {
            continue;
        }
        folded_at.insert(target, (name.clone(), ty.clone()));
        moved.insert(i);
    }

    let last_idx = items.len().saturating_sub(1);
    let mut idx = 0;
    while idx < items.len() {
        // Объявление уехало вниз — здесь не печатаем ничего.
        if moved.contains(&idx) {
            idx += 1;
            continue;
        }
        if let Some((name, ty)) = folded_at.get(&idx) {
            emit_folded_declaration(name, ty, &items[idx], &items[idx + 1..], scope, p)?;
            idx += 1;
            continue;
        }
        // Хвост печатается особым образом только если он и правда последний и
        // его не поглотила свёртка.
        if let Some(tail) = tail
            && idx == last_idx
            && tail(&items[idx], scope, p, out)?
        {
            idx += 1;
            continue;
        }
        let eaten = print_statement_ctx(&items[idx], &items[idx + 1..], scope, p, out)?;
        idx += 1 + eaten;
    }
    // Неиспользуемая локальная гасится заглушкой (фича 0376): без неё
    // `rustc -D warnings` отвечает «unused variable», то есть вывод не
    // собирается под флагами гейта этой же цели при нулевом коде возврата
    // `taktc`. Идиома — та же, что у неиспользуемого параметра (0337); место —
    // конец блока, где переменная ещё в области видимости.
    for name in crate::generator::local_stub::unused_locals(items) {
        p.ident(&format!(
            "let _ = {};",
            crate::semantic::naming::normalize_lowercase_snakecase(name)
        ))
        .nl();
    }
    Ok(())
}

/// Сворачиваемо ли последнее выражение хвостовой позиции в значение (0058-01).
///
/// **Единственный источник истины** правила «всё или ничего» (ADR 0058, R4):
/// печатник [`print_tail`] вызывается только после `true` и форму узла повторно
/// не разбирает. Разойдись предикат и печатник — тот же класс дефекта, что
/// `function_needs` / `rust_expr::call` («разойдись они, код не собрался бы»).
pub(crate) fn tail_foldable(stmt: &StatementNode) -> bool {
    match stmt {
        // `return e;` в хвосте → `e`.
        StatementNode::Return(Some(_)) => true,
        // Блок сворачиваем, если сворачиваем его ПОСЛЕДНИЙ оператор (операторы
        // перед ним печатаются как есть — R8).
        StatementNode::Block(items) => items.last().is_some_and(tail_foldable),
        // `if/else` сворачиваем ⟺ сворачиваемы ОБЕ ветки (рекурсивно).
        StatementNode::If {
            then_,
            else_: Some(else_),
            ..
        } => tail_foldable(then_) && tail_foldable(else_),
        // `if` без `else` (значения ветки-пропуска нет) и всё прочее — нет.
        _ => false,
    }
}

/// Печатает хвостовую позицию тела/ветки как **выражение** (0058-02).
///
/// `return e;` → `e`; завершающий `if/else` со сворачиваемыми ветвями →
/// `if c { <хвост> } else { <хвост> }` (рекурсивно, цепочка `else if` — та же
/// рекурсия по `else_ = If`). Возвращает `true`, если оператор напечатан как
/// хвост; `false` — печатать обычным путём (`return` остаётся, поведение как
/// сегодня; ADR 0058, R5).
///
/// Годится как [`TailPrinter`] для [`print_block`] — сигнатура совпадает.
pub(crate) fn print_tail(
    stmt: &StatementNode,
    scope: &mut Scope,
    p: &mut Printer,
    out: &mut StmtOutput,
) -> Result<bool, Diagnostic> {
    match stmt {
        StatementNode::Return(Some(expr)) => {
            // Хвостовое выражение — тот же приёмник, что и `return` (фича
            // 0336): путей печати возврата ДВА, и правило обязано стоять в
            // обоих — иначе оно действует через раз, в зависимости от того,
            // последний ли это оператор тела.
            let text = match &scope.return_type {
                Some(ty) => crate::generator::rust::rust_expr::coerce_to(expr, ty, scope)?,
                None => print_expression(expr, scope)?,
            };
            p.ident(unwrap_outer(&text)).nl();
            Ok(true)
        }
        // Хвостовой `if/else` — только когда сворачиваемы ОБЕ ветки (предикат —
        // единственное место решения). Условие через `unwrap_outer`: иначе
        // `if (c) {…}` словил бы `unused_parens` (A-3 ADR).
        StatementNode::If {
            cond,
            then_,
            else_: Some(else_),
        } if tail_foldable(stmt) => {
            p.ident(&format!(
                "if {} {{",
                unwrap_outer(&print_as_bool(cond, scope)?)
            ))
            .nl();
            p.up();
            print_tail_branch(then_, scope, p, out)?;
            p.down();
            p.ident("} else {").nl();
            p.up();
            print_tail_branch(else_, scope, p, out)?;
            p.down();
            p.ident("}").nl();
            Ok(true)
        }
        _ => Ok(false),
    }
}

/// Печатает ветку хвостового `if/else` — **через `print_block`** с тем же
/// печатником хвоста.
///
/// Мимо `print_block` нельзя (R9): он несёт свёртку мёртвого инициализатора
/// (`rust_live`); ветка в обход потеряла бы перенос объявлений и словила
/// `needless_late_init`. Это же сохраняет операторы перед хвостом ветки (R8).
fn print_tail_branch(
    stmt: &StatementNode,
    scope: &mut Scope,
    p: &mut Printer,
    out: &mut StmtOutput,
) -> Result<(), Diagnostic> {
    match stmt {
        StatementNode::Block(items) => print_block(items, Some(&print_tail), scope, p, out),
        // Не-Block сворачиваемый хвост (голый `return e` или вложенный `if/else`):
        // предикат уже гарантировал сворачиваемость, print_tail его напечатает.
        other => print_tail(other, scope, p, out).map(|_| ()),
    }
}

/// Печатает оператор Takt в Rust.
///
/// # Ошибки
/// [`RS-011`] на непереводимой конструкции — не тихий пропуск.
pub(crate) fn print_statement(
    stmt: &StatementNode,
    scope: &mut Scope,
    p: &mut Printer,
    out: &mut StmtOutput,
) -> Result<(), Diagnostic> {
    print_statement_ctx(stmt, &[], scope, p, out).map(|_| ())
}

/// Печатает оператор, зная **остаток блока**.
///
/// Остаток нужен ровно одному узлу — объявлению переменной: только по нему видно,
/// затирается ли инициализатор до первого чтения (`rust_live`). Печать
/// потоковая, поэтому «посмотреть вперёд» иначе нельзя.
/// Возвращает число операторов из `rest`, **поглощённых** этим вызовом.
///
/// Ненулевым бывает только у объявления с мёртвым инициализатором: оно
/// сворачивает следующее присваивание в своё значение (`let x = if … {…} else
/// {…};`), и печатать его второй раз нельзя.
pub(crate) fn print_statement_ctx(
    stmt: &StatementNode,
    rest: &[StatementNode],
    scope: &mut Scope,
    p: &mut Printer,
    out: &mut StmtOutput,
) -> Result<usize, Diagnostic> {
    match stmt {
        StatementNode::None => Ok(0),
        StatementNode::Block(items) => print_block(items, None, scope, p, out).map(|_| 0),
        StatementNode::Expression(expr, loc) => {
            // Место оператора — для отказов печати выражений (фича 0308):
            // своей позиции у них нет (решение 0056), и до этой фичи цель
            // печатала отказ без координаты вовсе.
            crate::generator::site::enter(*loc);
            // Присваивание СРЕЗА печатается поэлементно (фича 0355) — так же,
            // как у `c`, `st` и `sv`. Выражением его не напечатать: `&self.src[0..2]`
            // имеет тип `[u8]`, не `Sized`, а `copy_from_slice` — оператор, не
            // выражение. Границы — литералы, проверенные `SE-029`.
            if let ExpressionNode::Assign(target, value) = expr.as_ref()
                && let ExpressionNode::ArraySlice(src, from, to) = value.as_ref()
            {
                // База — выражение (фича 0358): печатается тем же печатником.
                let dst = print_expression(target, scope)?;
                let base = print_expression(src, scope)?;
                // Пригодны ОБА операнда: приёмник тоже обязан быть настоящим
                // массивом (`res := mem[1:2];` при `res: u8` эталон не
                // исполняет — `SIM-006`). Тип базы даёт общий носитель (0358).
                let dst_ok =
                    crate::generator::slice::elementwise_len_of(target, scope.model).is_some();
                let src_len = if dst_ok {
                    crate::generator::slice::elementwise_len_of(src, scope.model)
                } else {
                    None
                };
                // Срез над бит-вектором поэлементно не выразим (0078): вход
                // уходит прежним путём — к отказу `RS-011`, как у эталона.
                if let Some(src_len) = src_len {
                    let (start, len) = crate::generator::slice::bounds(*from, *to, src_len);
                    for k in 0..len {
                        p.ident(&format!("{dst}[{k}] = {base}[{}];", start + k))
                            .nl();
                    }
                    return Ok(0);
                }
            }
            p.ident(&format!("{};", print_expression(expr, scope)?))
                .nl();
            Ok(0)
        }
        StatementNode::Variable(name, ty, init, loc) => {
            // Объявление тела объявляет своё место (фича 0468): позиция у него
            // есть с 0386, а отказ печати типа или инициализатора приходил без
            // координаты.
            crate::generator::site::enter(*loc);
            let ident = rust_value_name(name, crate::diagnostics::Location::Codegen)?;
            let ty_name = rust_type(ty, &format!("переменная '{}'", name))?;
            // `mut` ставится по факту присваивания, а не по объявлению: в Takt
            // `var` изменяем всегда, в Rust лишний `mut` — это `unused_mut`.
            let mutable = if scope.assigned.contains(name) {
                "mut "
            } else {
                ""
            };
            // Инициализатор, затираемый до первого чтения, не печатается:
            // `let mut ds: u8 = 0;` перед `if/else`, пишущим обе ветки, даёт
            // `unused_assignments` — отказ гейта. Анализ консервативен, а ошибка
            // в сторону «мёртв» даёт ошибку компиляции, а не тихо неверный код
            // (см. `rust_live`).
            let dead = init.is_some() && initializer_is_dead(name, rest);
            match init {
                // Мёртвый инициализатор: печатается не он, а то значение, что
                // его затирает. `mut` тут считается ИНАЧЕ — присваивание на
                // каждом пути является инициализацией, а не изменением (см.
                // `deferred_needs_mut`).
                Some(_) if dead => {
                    let deferred_mut = if deferred_needs_mut(name, rest) {
                        "mut "
                    } else {
                        ""
                    };
                    // Свёртка в `let x = …`: отложенное объявление clippy тоже
                    // не принимает (`needless_late_init`), и справедливо — так
                    // читается лучше. Свернуть удаётся всегда, когда вердикт
                    // «мёртв» дало распознанное присваивание; иначе печатается
                    // отложенная форма.
                    match rest.first().and_then(|next| fold_assignment(name, next)) {
                        Some(folded) => {
                            let value = print_folded(&folded, ty, scope)?;
                            p.ident(&format!(
                                "let {}{}: {} = {};",
                                deferred_mut, ident, ty_name, value
                            ))
                            .nl();
                            scope.locals.push(name.clone());
                            // Присваивание поглощено — печатать его второй раз
                            // значило бы присвоить дважды.
                            return Ok(1);
                        }
                        None => {
                            p.ident(&format!("let {}{}: {};", deferred_mut, ident, ty_name))
                                .nl();
                        }
                    }
                }
                Some(expr) => {
                    p.ident(&format!(
                        "let {}{}: {} = {};",
                        mutable,
                        ident,
                        ty_name,
                        unwrap_outer(&crate::generator::rust::rust_expr::coerce_to(
                            expr, ty, scope
                        )?)
                    ))
                    .nl();
                }
                // Без инициализатора: первое присваивание — это ИНИЦИАЛИЗАЦИЯ,
                // а не изменение, и `mut` ей не нужен (фича 0410). Прежде он
                // печатался безусловно, и `rustc -D warnings` отвечал
                // «variable does not need to be mutable» при **нулевом** коде
                // возврата `taktc` — на записи, которую исполняют эталон и
                // остальные семь целей.
                //
                // Признак — тот же `deferred_needs_mut` (0216), которым уже
                // живёт соседняя ветвь мёртвого инициализатора: второе знание
                // о том, «что считать изменением», разошлось бы с первым.
                None => {
                    let deferred_mut = if deferred_needs_mut(name, rest) {
                        "mut "
                    } else {
                        ""
                    };
                    // Свёртка в `let x: T = …` — та же, что у ветви мёртвого
                    // инициализатора: отложенное объявление clippy не
                    // принимает (`needless_late_init`), и это **отказ** гейта
                    // цели под `-D warnings`.
                    match rest.first().and_then(|next| fold_assignment(name, next)) {
                        Some(folded) => {
                            let value = print_folded(&folded, ty, scope)?;
                            p.ident(&format!(
                                "let {}{}: {} = {};",
                                deferred_mut, ident, ty_name, value
                            ))
                            .nl();
                            scope.locals.push(name.clone());
                            // Присваивание поглощено — печатать его второй раз
                            // значило бы присвоить дважды.
                            return Ok(1);
                        }
                        None => {
                            // АГРЕГАТ обязан получить умолчание (фича 0411):
                            // Rust требует инициализировать массив (и
                            // структуру) целиком **до** записи по индексу или
                            // в поле — иначе `E0381` «used binding isn't
                            // initialized» при нулевом коде возврата `taktc`.
                            // Скаляру умолчание не нужно и вредно: там
                            // отложенная форма законна, а лишнее значение дало
                            // бы `unused_assignments` (урок 0216).
                            if needs_deferred_default(ty) {
                                let value = crate::generator::rust::rust_decl::default_value(
                                    ty,
                                    scope.model,
                                )?;
                                p.ident(&format!("let mut {}: {} = {};", ident, ty_name, value))
                                    .nl();
                            } else {
                                p.ident(&format!("let {}{}: {};", deferred_mut, ident, ty_name))
                                    .nl();
                            }
                        }
                    }
                }
            }
            // Имя объявлено в ТЕЛЕ и полем модели не является. Без этой записи
            // печатник обратился бы к нему как `self.i` — то есть искал бы поле,
            // которого нет («no field `i` on type …»). Регистрация идёт ПОСЛЕ
            // печати инициализатора: `var x := x;` в Takt читает ВНЕШНИЙ `x`.
            scope.locals.push(name.clone());
            Ok(0)
        }
        StatementNode::If { cond, then_, else_ } => {
            p.ident(&format!(
                "if {} {{",
                unwrap_outer(&print_as_bool(cond, scope)?)
            ))
            .nl();
            p.up();
            print_statement(then_, scope, p, out)?;
            p.down();
            match else_ {
                Some(alt) => {
                    p.ident("} else {").nl();
                    p.up();
                    print_statement(alt, scope, p, out)?;
                    p.down();
                    p.ident("}").nl();
                }
                None => {
                    p.ident("}").nl();
                }
            }
            Ok(0)
        }
        StatementNode::Loop { cond, body } => {
            match cond {
                Some(c) => {
                    p.ident(&format!(
                        "while {} {{",
                        unwrap_outer(&print_as_bool(c, scope)?)
                    ))
                    .nl();
                }
                None => {
                    p.ident("loop {").nl();
                }
            }
            p.up();
            print_statement(body, scope, p, out)?;
            p.down();
            p.ident("}").nl();
            Ok(0)
        }
        // `for` Takt — это C-подобный `for(init; cond; step)`, а не итератор.
        // Разворачивается в блок с `while`: иначе `step` пришлось бы дублировать
        // перед каждым `continue`.
        StatementNode::For {
            init,
            cond,
            step,
            body,
        } => {
            p.ident("{").nl();
            p.up();
            if let Some(init) = init {
                print_statement(init, scope, p, out)?;
            }
            match cond {
                Some(c) => {
                    p.ident(&format!(
                        "while {} {{",
                        unwrap_outer(&print_as_bool(c, scope)?)
                    ))
                    .nl();
                }
                None => {
                    p.ident("loop {").nl();
                }
            }
            p.up();
            print_statement(body, scope, p, out)?;
            if let Some(step) = step {
                p.ident(&format!("{};", print_expression(step, scope)?))
                    .nl();
            }
            p.down();
            p.ident("}").nl();
            p.down();
            p.ident("}").nl();
            Ok(0)
        }
        StatementNode::Return(value) => {
            match value {
                // `return (x);` — ещё одна позиция, где скобки лишние.
                Some(expr) => {
                    // `return` — позиция приёмника с известным типом (фича
                    // 0336): `return 1;` при `-> bit` обязано печататься
                    // `true`, вариант перечисления — именем, разряд — числом.
                    let printed = match &scope.return_type {
                        Some(ty) => crate::generator::rust::rust_expr::coerce_to(expr, ty, scope)?,
                        None => print_expression(expr, scope)?,
                    };
                    p.ident(&format!("return {};", unwrap_outer(&printed))).nl();
                }
                None => {
                    p.ident("return;").nl();
                }
            }
            Ok(0)
        }
        StatementNode::Continue => {
            p.ident("continue;").nl();
            Ok(0)
        }
        StatementNode::Break => {
            p.ident("break;").nl();
            Ok(0)
        }
        // Образцы Takt — произвольные выражения, а `match x { y => … }` в Rust
        // СВЯЗАЛ БЫ `y` как новое имя вместо сравнения с ним. Молчаливое
        // связывание дало бы всегда-истинную первую ветку — то есть тихо
        // неверный автомат. Поэтому цепочка `if`/`else if`.
        StatementNode::Match { expr, arms } => {
            let subject = print_expression(expr, scope)?;
            // Образец сравнивается с ТИПОМ разбираемого выражения: `match mode
            // { 0 => … }` при `mode : Mode` приходит числом, и без обратного
            // отображения получилось бы `self.mode == 0` — сравнение
            // перечисления с целым.
            let subject_type = crate::generator::rust::rust_expr::expression_type(expr);
            let mut first = true;
            let mut wildcard: Option<&StatementNode> = None;
            for arm in arms {
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
                    let printed = match &subject_type {
                        Some(ty) => crate::generator::rust::rust_expr::coerce_to(value, ty, scope)?,
                        None => print_expression(value, scope)?,
                    };
                    tests.push(format!("{} == {}", subject, printed));
                }
                if tests.is_empty() {
                    continue;
                }
                let head = if first { "if" } else { "} else if" };
                first = false;
                p.ident(&format!("{} {} {{", head, tests.join(" || "))).nl();
                p.up();
                print_statement(&arm.body, scope, p, out)?;
                p.down();
            }
            match (wildcard, first) {
                // Только `_`-ветка: цепочки нет, тело печатается как есть.
                (Some(body), true) => print_statement(body, scope, p, out)?,
                (Some(body), false) => {
                    p.ident("} else {").nl();
                    p.up();
                    print_statement(body, scope, p, out)?;
                    p.down();
                    p.ident("}").nl();
                }
                (None, false) => {
                    p.ident("}").nl();
                }
                (None, true) => {}
            }
            Ok(0)
        }
        // Формула в теле блока — предмет верификации (`taktc verify`), а не
        // трансляции: LTL описывает бесконечные прогоны, которых в прошивке нет.
        // Предупреждение, а НЕ тишина: молчаливая потеря `: [LTL]` — ровно тот
        // дефект, который закрыла фича 0035.
        StatementNode::InlineFormula(_) => {
            out.warnings.push(
                Diagnostic::warning(
                    crate::diagnostics::Location::Codegen,
                    "LTL-формула в теле блока не транслируется в Rust: \
                     проверяйте её через 'taktc verify'. Порождённый код формулу \
                     не содержит"
                        .to_string(),
                )
                .with_code("RS-010"),
            );
            Ok(0)
        }
        StatementNode::Unresolved(_) => Err(unsupported("неразрешённый оператор")),
    }
}

#[cfg(test)]
mod tail_tests {
    use super::tail_foldable;
    use crate::semantic::{ExpressionNode, StatementNode};

    fn ret(n: i128) -> StatementNode {
        StatementNode::Return(Some(Box::new(ExpressionNode::Number(n))))
    }
    fn if_(then_: StatementNode, else_: Option<StatementNode>) -> StatementNode {
        StatementNode::If {
            cond: Box::new(ExpressionNode::Bool(true)),
            then_: Box::new(then_),
            else_: else_.map(Box::new),
        }
    }

    /// `return e;` в хвосте — сворачиваем (правило 1 ADR).
    #[test]
    fn return_is_foldable() {
        assert!(tail_foldable(&ret(1)));
    }

    /// `if/else` со сворачиваемыми ветвями — сворачиваем (правило 2).
    #[test]
    fn if_else_both_return_is_foldable() {
        assert!(tail_foldable(&if_(ret(1), Some(ret(2)))));
    }

    /// Блок сворачиваем по СВОЕМУ последнему оператору (операторы до него — как
    /// есть, R8).
    #[test]
    fn block_folds_on_last_statement() {
        let block = StatementNode::Block(vec![
            StatementNode::Expression(
                Box::new(ExpressionNode::Number(0)),
                crate::diagnostics::Location::default(),
            ),
            ret(1),
        ]);
        assert!(tail_foldable(&block));
    }

    /// **T11 (негативный сторож):** `if` БЕЗ `else` не сворачиваем (значения
    /// ветки-пропуска нет, правило 3). Мутация «сворачиваем» обязана валить тест.
    #[test]
    fn if_without_else_is_not_foldable() {
        assert!(!tail_foldable(&if_(ret(1), None)));
    }

    /// Смешанные ветки (одна не завершается сворачиваемым хвостом) — не
    /// сворачиваем (правило 2: «всё или ничего»).
    #[test]
    fn mixed_branches_are_not_foldable() {
        let else_no_return = StatementNode::Expression(
            Box::new(ExpressionNode::Number(5)),
            crate::diagnostics::Location::default(),
        );
        assert!(!tail_foldable(&if_(ret(1), Some(else_no_return))));
    }

    /// Цепочка `else if` (вложенный `if/else` в `else_`) — сворачиваема целиком.
    #[test]
    fn else_if_chain_is_foldable() {
        let inner = if_(ret(2), Some(ret(3)));
        assert!(tail_foldable(&if_(ret(1), Some(inner))));
    }
}
