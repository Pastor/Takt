//! Левая часть присваивания — **место записи** (фича 0249).
//!
//! ## Что нормируется
//!
//! Фича 0187 нормировала **позиции** присваивания: `:=` разбирается ровно в
//! трёх местах (оператор тела, шаг `for`, именованный аргумент вызова), всё
//! прочее отсекает грамматика (`SY-006`) или судья [`super::assignment_position`]
//! (`SE-095`). Но **левую часть** не судил никто: грамматика принимает слева
//! `Chain13<Precedence1Full>` — произвольное выражение.
//!
//! Этот модуль отвечает на вопрос «обозначает ли левая часть хранилище»:
//!
//! | Форма | Пример | Решение |
//! |---|---|---|
//! | переменная, порт, константа | `n := 1;`, `led := 1;` | место (носитель судится отдельно) |
//! | поле структуры | `p.x := 1;`, `p.y.v := 1;` | место, если место — основание |
//! | бит | `b.2 := 1;` | место, если место — основание |
//! | элемент массива | `arr[1] := 1;` | место |
//! | срез массива | `arr[0:2] := 1;` | место |
//! | ячейка `#АДРЕС` | `#0x200.5 := 1;` | место |
//! | скобки | `(n) := 1;` | прозрачны |
//! | неразрешённый узел | — | молчим: уже отвергнут `SE-025`/`SE-003` |
//! | всё прочее | `f(n)`, `5`, `n + m`, `-n`, `n as u8`, `!n` | **`SE-111`** |
//!
//! Носитель места судится отдельно: константа обозначает величину, а не
//! хранилище, — запись в неё **`SE-112`**.
//!
//! ## Чем платили за отсутствие правила (пробы 2026-08-18)
//!
//! Вход `fn f(x: u8) -> u8 { return x; } … always { f(n) := 1; }`: семантика
//! **принимает**, код возврата `taktc compile` — ноль, а дальше пять ответов:
//!
//! | Потребитель | Что происходило |
//! |---|---|
//! | `c` | `Badplace_f(model, model->n) = 1;` — `cc`: `expression is not assignable` |
//! | `rust` | `f(self.n) = 1;` — `rustc`: `E0070` |
//! | `st` | `Badplace_f(n) := 1;` — `iec2c`: «invalid variable before ':='» |
//! | `sv` | честный отказ `SV-002` |
//! | симулятор | `SIM-017` **в такте**, то есть уже на исполнении |
//!
//! Класс шире вызова функции: то же самое давали `5 := 1;` (`5 = 1;` в C),
//! `n + m := 1;`, `-n := 1;`, `n as u8 := 1;`, `!n := 1;`.
//!
//! ⚠️ **Запись в константу была хуже: там расходились не сообщения, а
//! значения.** Вход `const K: u8 := 5; … K := 1; n := K;` — эталон **молча
//! исполнял** запись (трасса `n=1`), а цель `c` печатала `CONST_K = 1;` при
//! `#define CONST_K 5`, то есть `5 = 1;`. Молчаливое расхождение эталона и
//! прошивки — ровно то, ради чего заведены сверки.
//!
//! ## Почему в семантике, а не в грамматике
//!
//! Сузить левую часть до нетерминала `Place` нельзя технически: `Place`
//! есть подмножество `Expression`, а `StatementExpr` принимает и то и другое —
//! разбирая `f(n)`, парсер обязан выбрать свёртку, ещё не видя `:=`. Это
//! reduce/reduce. И даже будь оно возможно, отказ выродился бы в `SY-002`
//! «нераспознанный токен `:=`» — сообщение о токене, а не о сделанном; тот же
//! довод грамматика уже фиксирует у `CallArg` (сужение отобрало бы у
//! `Tuner(1 := 5)` внятную `SE-076`).

#![deny(clippy::wildcard_enum_match_arm)]

use crate::diagnostics::{Diagnostic, Location};
use crate::semantic::validate::bodies::Position;
use crate::semantic::{ExpressionNode, VariableNode};

/// Проверяет левую часть присваивания в выражении.
///
/// Судит **только** позицию оператора: присваивание в позиции значения не
/// разбирается грамматикой, а остаток (именованный аргумент вызова) — предмет
/// [`super::assignment_position`], где слева стоит имя параметра, а не место.
///
/// Возвращает **все** нарушения (накопительно, как `literal_range`, фича 0157):
/// редактор подчёркивает каждое, а не первое.
pub(super) fn check_expression(expr: &ExpressionNode, position: Position) -> Vec<Diagnostic> {
    let mut found = Vec::new();
    if position == Position::Statement {
        collect(expr, &mut found);
    }
    found
}

/// Обходит оператор, отдавая судье левую часть каждого присваивания.
///
/// Скобки прозрачны: `(led := 1);` — то же действие.
///
/// ⚠️ `_ => {}` здесь **уместен и потому разрешён явно**: обход отвечает на
/// вопрос «есть ли в операторе присваивание», а не разбирает язык. Правило
/// исчерпаемости стережёт [`classify`] — место, где новый узел обязан получить
/// решение «это хранилище или нет».
#[allow(clippy::wildcard_enum_match_arm)]
fn collect(expr: &ExpressionNode, found: &mut Vec<Diagnostic>) {
    match expr {
        ExpressionNode::Assign(left, _) => {
            if let Some(diagnostic) = judge(left, expr) {
                found.push(diagnostic);
            }
        }
        ExpressionNode::Parenthesis(inner) => collect(inner, found),
        _ => {}
    }
}

/// Что удалось опознать в левой части.
///
/// Носитель отдаётся **именем**, а не ссылкой в ячейку: заимствование
/// `Rc<RefCell<…>>` живёт короче вердикта, а судье достаточно имени для текста.
enum Place {
    /// Хранилище, в которое писать можно.
    Writable,
    /// Хранилище только для чтения: константа с таким именем.
    ReadOnly(String),
    /// Судить нечего — неразрешённый узел, молчим.
    Silent,
}

/// Позиция диагностики: у левой части, а при её отсутствии — у присваивания
/// целиком.
///
/// ⚠️ **Координата здесь скудна, и это замер, а не догадка.** У переменной и
/// функции `ExpressionNode::loc()` отдаёт позицию **объявления**, а не
/// использования: позиции вхождений живут отдельным слоем `semantic::usages`
/// (фича 0131) и в понижённом дереве тела отсутствуют. У чистого литерала
/// (`5 := 2;`) позиции нет вовсе — `Location::Builtin`. Запасной ход через
/// присваивание целиком подхватывает координату правой части, если она есть;
/// когда и там литерал, сообщение остаётся без префикса. Соседний судья
/// `SE-095` живёт с тем же ограничением (`Location::Codegen` в `target_of`).
fn where_to_point(left: &ExpressionNode, whole: &ExpressionNode) -> Location {
    let found = left.loc();
    if matches!(found, Location::Builtin) {
        whole.loc()
    } else {
        found
    }
}

/// Выносит вердикт по левой части: `None` — место законно.
fn judge(left: &ExpressionNode, whole: &ExpressionNode) -> Option<Diagnostic> {
    let loc = where_to_point(left, whole);
    match classify(left) {
        Err(kind) => Some(
            Diagnostic::error(
                loc,
                format!(
                    "левая часть присваивания — {kind}, а не место записи: писать можно в \
                     переменную, поле структуры, элемент или срез массива, отдельный бит, \
                     порт либо ячейку '#АДРЕС'"
                ),
            )
            .with_code("SE-111"),
        ),
        Ok(Place::ReadOnly(name)) => Some(
            Diagnostic::error(
                loc,
                format!(
                    "целью записи назначена константа '{name}': константа обозначает \
                     величину, а не хранилище. Объявите её как 'var {name}: …', если \
                     значение должно меняться"
                ),
            )
            .with_code("SE-112"),
        ),
        Ok(Place::Writable | Place::Silent) => None,
    }
}

/// Опознаёт место записи, либо называет **словом** вид того, что стоит слева.
///
/// ⚠️ Разбор исчерпывающий (ветки `_` нет) намеренно: новый узел языка обязан
/// **завалить сборку** этого модуля, а не молча выйти из-под правила. Тот же
/// приём, что у `semantic/usages/walk.rs`, `parser/depth` и соседнего
/// `assignment_position`.
///
fn classify(expr: &ExpressionNode) -> Result<Place, &'static str> {
    match expr {
        // Скобки прозрачны, доступ к члену и биту — место, если место основание.
        ExpressionNode::Parenthesis(inner) | ExpressionNode::BitAccess(inner, _) => classify(inner),
        ExpressionNode::Variable(var)
        | ExpressionNode::ArraySubscript(var, _)
        | ExpressionNode::ArraySlice(var, _, _) => match &*var.borrow() {
            VariableNode::Const { name, .. } => Ok(Place::ReadOnly(name.clone())),
            // `Unresolved` уже отвергнут `SE-025`/`SE-003` — не удваиваем.
            VariableNode::Unresolved => Ok(Place::Silent),
            VariableNode::Simple { .. } | VariableNode::Port { .. } => Ok(Place::Writable),
        },
        // Ячейка по адресу (фича 0189): объявления у неё нет по построению.
        ExpressionNode::AnonPort(_) => Ok(Place::Writable),
        // Сырой АСД: до понижения формы левой части не видно.
        ExpressionNode::Unresolved(_) => Ok(Place::Silent),

        ExpressionNode::Function(_, _) => Err("вызов функции"),
        ExpressionNode::NamedFunctionBox(_, _) => Err("вызов с именованными аргументами"),
        ExpressionNode::CodeBlock(_, _) => Err("блок кода"),
        ExpressionNode::Number(_)
        | ExpressionNode::Duration(_)
        | ExpressionNode::Rational(_, _)
        | ExpressionNode::String(_)
        | ExpressionNode::Bool(_) => Err("литерал"),
        ExpressionNode::Address(_, _) => Err("адресный литерал"),
        ExpressionNode::Array(_) | ExpressionNode::Initializer(_) => Err("составной литерал"),
        ExpressionNode::Type(_) => Err("имя типа"),
        ExpressionNode::Model(_) => Err("модель"),
        ExpressionNode::Condition(_) => Err("именованное условие"),
        ExpressionNode::List(_) => Err("список параметров"),
        ExpressionNode::None => Err("пустое выражение"),
        ExpressionNode::Not(_) => Err("логическое отрицание"),
        ExpressionNode::BitwiseNot(_) => Err("побитовое отрицание"),
        ExpressionNode::UnaryPlus(_) => Err("унарный плюс"),
        ExpressionNode::Negate(_) => Err("смена знака"),
        ExpressionNode::Cast(_, _) => Err("приведение типа"),
        ExpressionNode::Power(_, _)
        | ExpressionNode::Multiply(_, _)
        | ExpressionNode::Divide(_, _)
        | ExpressionNode::Modulo(_, _)
        | ExpressionNode::Add(_, _)
        | ExpressionNode::Subtract(_, _) => Err("арифметическое выражение"),
        ExpressionNode::ShiftLeft(_, _)
        | ExpressionNode::ShiftRight(_, _)
        | ExpressionNode::BitwiseAnd(_, _)
        | ExpressionNode::BitwiseXor(_, _)
        | ExpressionNode::BitwiseOr(_, _) => Err("побитовое выражение"),
        ExpressionNode::Less(_, _)
        | ExpressionNode::More(_, _)
        | ExpressionNode::LessEqual(_, _)
        | ExpressionNode::MoreEqual(_, _)
        | ExpressionNode::Equal(_, _)
        | ExpressionNode::NotEqual(_, _) => Err("сравнение"),
        ExpressionNode::And(_, _) | ExpressionNode::Or(_, _) => Err("логическое выражение"),
        ExpressionNode::ConditionalOperator(_, _, _) => Err("условный оператор"),
        ExpressionNode::Assign(_, _) => Err("присваивание"),
    }
}
