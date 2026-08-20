//! Чтение **неопределённой памяти** в инициализаторе объявления: `SE-099`
//! (ячейка по адресу, фича 0189) и `SE-113` (порт, фича 0266).
//!
//! ## Почему это ошибка, а не значение
//!
//! Инициализатор объявления вычисляется **до первого такта**: эталон делает это
//! вычислителем начальных значений (`takt-sim/src/unit/initial.rs`), цели —
//! печатью в `_init` / `new()` / ветви сброса. Ни у ячейки, ни у порта значения
//! в этот момент нет: у эталона памяти и входов ещё не существует, у цели
//! `c-hal` чтение регистра на этапе инициализации — уже обращение к железу.
//!
//! Без запрета стороны расходятся **молча**. Замер фичи 0266 на входе
//! `in sensor: bit; var mirror: u8 := sensor;` — три разных поведения у
//! принимающих потребителей и два разных отказа у остальных:
//!
//! | Потребитель | Что происходит |
//! |---|---|
//! | эталон | `mirror = 0` — порт не читается вовсе |
//! | `c` | чтение порта через HAL-колбэк прямо в `_init` |
//! | `c-hal` | чтение **регистра железа** при инициализации |
//! | `st`, `st-at` | инициализатор **теряется молча** |
//! | `rust` | отказ `RS-022` |
//! | `sv`, `sv-mmio` | отказ `SV-002` |
//!
//! ⚠️ **Правило ОДНО, а кодов два.** Ячейка и порт — одна и та же память,
//! названная по-разному, поэтому обход инициализаторов здесь **один**: два
//! обхода одного места разъезжаются (урок 0203). Коды разные потому, что разный
//! обход в тексте: `#АДРЕС` убирают в тело, порт — тоже в тело, но сообщение
//! называет его по имени.
//!
//! ⚠️ Общего поведения у этой записи **не существует** (ADR 0266): ветвь сброса
//! цели `sv` выражений не вычисляет — это свойство синтеза, а не недоделка, — а
//! у эталона до первого такта входа нет. Поэтому запрет, а не нормирование.
//!
//! Разрешённое место чтения — **тело**: `always { x := #0x100 as u8; }`,
//! `always { mirror := sensor; }`.

use super::*;

/// Проверяет инициализаторы объявлений модели на чтение неопределённой памяти.
pub(super) fn validate_undefined_reads_in_initializers(
    model: Rc<RefCell<ModelNode>>,
) -> Vec<Diagnostic> {
    let borrowed = model.borrow();
    // Накопление по объявлениям (фича 0151).
    let mut out = Vec::new();
    for variable in borrowed.variables.values() {
        match variable {
            VariableNode::Unresolved => {}
            VariableNode::Simple {
                expr, loc, name, ..
            }
            | VariableNode::Const {
                expr, loc, name, ..
            } => {
                out.extend(check(expr, *loc, name).err());
            }
            // У порта два выражения (фича 0187): размещение и начальное
            // значение. Чтение неопределённой памяти незаконно в обоих — в
            // адресе оно к тому же не свернулось бы в константу (`SE-055`).
            VariableNode::Port {
                address,
                init,
                loc,
                name,
                ..
            } => {
                out.extend(check(address, *loc, name).err());
                out.extend(check(init, *loc, name).err());
            }
        }
    }
    out
}

/// Что неопределённого нашлось в инициализаторе.
///
/// Порядок вариантов значения не имеет: обход возвращает **первую** находку, а
/// накопление идёт по объявлениям (правило 0151 — одна диагностика на элемент,
/// ранний выход внутри одного выражения сохранён).
enum UndefinedRead {
    /// Обращение к ячейке по адресу — `#АДРЕС` (фича 0189).
    Cell,
    /// Чтение порта по имени (фича 0266).
    Port(String),
    /// Вызов **внешней** функции (фича 0305): её значение при компиляции
    /// неизвестно по определению — тело живёт вне программы.
    ExternCall(String),
}

/// Ищет чтение неопределённой памяти в выражении инициализатора.
fn check(expr: &ExpressionNode, loc: Location, name: &str) -> Result<(), Diagnostic> {
    match find_undefined_read(expr) {
        None => Ok(()),
        Some(UndefinedRead::Cell) => Err(Diagnostic::error(
            loc,
            format!(
                "инициализатор '{name}' обращается к ячейке по адресу: содержимое памяти \
                 до первого такта неизвестно, и эталон с целью разошлись бы молча. \
                 Читайте ячейку в теле состояния — например, 'always {{ {name} := \
                 #0xАДРЕС as ТИП; }}'"
            ),
        )
        .with_code("SE-099")),
        Some(UndefinedRead::Port(port)) => Err(Diagnostic::error(
            loc,
            format!(
                "инициализатор '{name}' читает порт '{port}': значение порта до первого \
                 такта не определено, и потребители разошлись бы молча — эталон дал бы \
                 ноль, цель 'c-hal' прочла бы регистр, а 'st' потеряла бы инициализатор. \
                 Читайте порт в теле состояния — например, 'always {{ {name} := {port}; }}'"
            ),
        )
        .with_code("SE-113")),
        Some(UndefinedRead::ExternCall(func)) => Err(Diagnostic::error(
            loc,
            format!(
                "инициализатор '{name}' зовёт внешнюю функцию '{func}': её значение при \
                 компиляции неизвестно — тело живёт вне программы, а начальное значение \
                 выставляется до первого такта. Прежде потребители расходились молча: \
                 эталон давал ноль, 'st' теряла инициализатор без единого слова, а 'c', \
                 'rust' и 'sv' отказывали. Зовите функцию в теле состояния — например, \
                 'always {{ {name} := {func}(); }}'"
            ),
        )
        .with_code("SE-084")),
    }
}

/// Что неопределённого есть в поддереве выражения — первая находка или `None`.
///
/// Обход **один на оба правила**: ячейка и порт запрещены в одном и том же
/// месте по одной и той же причине, а два обхода одного места разъезжаются
/// (урок 0203). Полноты добиваться незачем — форма запрещена целиком, и
/// достаточно найти хотя бы одно вхождение на любом уровне.
fn find_undefined_read(expr: &ExpressionNode) -> Option<UndefinedRead> {
    match expr {
        ExpressionNode::AnonPort(_) => Some(UndefinedRead::Cell),
        // Порт узнаётся по ВИДУ объявления, а не по имени: вид проставлен при
        // объявлении и от вывода типов не зависит, поэтому снимок в ячейке
        // ссылки (0204) здесь достоверен. Направление роли не играет — чтение
        // выходного порта отвергает `SE-027` раньше и по своей причине.
        ExpressionNode::Variable(var_rc) => match &*var_rc.borrow() {
            VariableNode::Port { name, .. } => Some(UndefinedRead::Port(name.clone())),
            _ => None,
        },
        ExpressionNode::Parenthesis(inner)
        | ExpressionNode::BitAccess(inner, _)
        | ExpressionNode::CodeBlock(inner, _)
        | ExpressionNode::NamedFunctionBox(inner, _)
        | ExpressionNode::Not(inner)
        | ExpressionNode::UnaryPlus(inner)
        | ExpressionNode::Negate(inner)
        | ExpressionNode::Cast(inner, _)
        | ExpressionNode::BitwiseNot(inner) => find_undefined_read(inner),
        ExpressionNode::Power(left, right)
        | ExpressionNode::Multiply(left, right)
        | ExpressionNode::Divide(left, right)
        | ExpressionNode::Modulo(left, right)
        | ExpressionNode::Add(left, right)
        | ExpressionNode::Subtract(left, right)
        | ExpressionNode::ShiftLeft(left, right)
        | ExpressionNode::ShiftRight(left, right)
        | ExpressionNode::BitwiseAnd(left, right)
        | ExpressionNode::BitwiseXor(left, right)
        | ExpressionNode::BitwiseOr(left, right)
        | ExpressionNode::Less(left, right)
        | ExpressionNode::More(left, right)
        | ExpressionNode::LessEqual(left, right)
        | ExpressionNode::MoreEqual(left, right)
        | ExpressionNode::Equal(left, right)
        | ExpressionNode::NotEqual(left, right)
        | ExpressionNode::And(left, right)
        | ExpressionNode::Or(left, right)
        | ExpressionNode::Assign(left, right) => {
            find_undefined_read(left).or_else(|| find_undefined_read(right))
        }
        ExpressionNode::ConditionalOperator(cond, then_, else_) => find_undefined_read(cond)
            .or_else(|| find_undefined_read(then_))
            .or_else(|| find_undefined_read(else_)),
        // Вызов функции: сперва спрашиваем САМУ функцию — внешняя запрещена
        // здесь целиком (фича 0305), — затем обходим аргументы.
        ExpressionNode::Function(def, args) => {
            // ⚠️ Форм внешней функции в ячейке ДВЕ. Инициализатор разрешается на
            // стадии 2, а тела функций строятся на стадии 5 — поэтому в снимке
            // ячейки лежит `Unresolved(FunctionDefine { external: true, … })`, а
            // не `External`. Проверять только второй вариант значит не поймать
            // ничего: первая редакция так и делала, и проба молчала.
            let external = match &*def.borrow() {
                FunctionDefinitionNode::External { name, .. } => Some(name.clone()),
                FunctionDefinitionNode::Unresolved(raw) if raw.external => raw
                    .name
                    .as_ref()
                    .map(|id| id.name.clone())
                    .or(Some(String::from("внешняя функция"))),
                _ => None,
            };
            if let Some(func) = external {
                return Some(UndefinedRead::ExternCall(func));
            }
            args.iter().find_map(find_undefined_read)
        }
        ExpressionNode::Array(args) | ExpressionNode::Initializer(args) => {
            args.iter().find_map(find_undefined_read)
        }
        ExpressionNode::ArraySubscript(_, index) => find_undefined_read(index),
        // Прочее вложенных выражений не несёт.
        ExpressionNode::None
        | ExpressionNode::Unresolved(_)
        | ExpressionNode::ArraySlice(_, _, _)
        | ExpressionNode::Number(_)
        | ExpressionNode::Duration(_)
        | ExpressionNode::Rational(_, _)
        | ExpressionNode::String(_)
        | ExpressionNode::Type(_)
        | ExpressionNode::Address(_, _)
        | ExpressionNode::Bool(_)
        | ExpressionNode::Model(_)
        | ExpressionNode::Condition(_)
        | ExpressionNode::List(_) => None,
    }
}
