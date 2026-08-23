//! Проверки имён цели `sv`, требующие доступа к семантической модели.
//!
//! # Зачем отдельный модуль
//!
//! `check_sv_name` живёт в `sv_module` и работает со строкой. Проверка имён
//! **состояний** (фича 0200) строкой не обходится: перечислитель печатается по
//! `Name`, у которого позиции нет, а диагностика обязана указывать на
//! объявление автора — значит нужен доступ к `ModelNode`. Отдельный модуль, а
//! не `sv_fsm`, потому что тот упирается в лимит размера
//! (`scripts/check-module-size.sh`).

use crate::diagnostics::Diagnostic;
use crate::generator::sv::sv_map::SvMap;
use crate::generator::sv::sv_module::check_sv_name;
use crate::semantic::minimap::Name;

/// Проверяет имена состояний модели на пригодность для SystemVerilog.
///
/// Позиция берётся из семантической модели: перечислитель печатается по
/// `Name`, у которого позиции нет, а диагностика обязана указывать на
/// объявление автора.
pub(crate) fn check_state_names(map: &SvMap, model: &Name) -> Result<(), Diagnostic> {
    let raw = map.raw_model_at(model.clone())?;
    let borrowed = raw.borrow();
    for state in borrowed.states.values() {
        let (name, loc) = match state {
            crate::semantic::StateNode::Simple { name, loc, .. }
            | crate::semantic::StateNode::Implement { name, loc, .. } => (name, *loc),
            crate::semantic::StateNode::Unresolved => continue,
        };
        check_sv_name(name, loc)?;
    }
    Ok(())
}

// ── Имена сигналов ─────────────────────────────────────────────────────────
//
// Переехало из `sv_expr` фичей 0424 по границе ответственности: печать
// выражения отвечает на вопрос «как выглядит значение», а эти функции — на
// вопрос «как зовётся сигнал», и он один на все три печатника (выражения,
// условия, операторы).

use crate::generator::sv::sv_expr::Scope;
use crate::parser::ast::Member;
use crate::semantic::VariableNode;

/// Имя варианта перечисления в SV: `Action`/`Idle` → `ACTION_IDLE`.
///
/// Префикс именем перечисления обязателен: метки `enum` в SystemVerilog живут в
/// **общем пространстве имён модуля**, а не внутри своего типа (в отличие от
/// Rust, где `Action::Idle` квалифицировано). Два перечисления модели с
/// одноимённым вариантом (`Idle` у `Action` и у `Mode`) без префикса дали бы
/// повторное объявление.
pub(crate) fn sv_enum_variant_name(enum_name: &str, variant: &str) -> String {
    format!(
        "{}_{}",
        crate::generator::sv::sv_type::sv_enum_type_name(enum_name)
            .trim_end_matches("_e")
            .to_uppercase(),
        crate::semantic::naming::normalize_lowercase_snakecase(variant.to_string()).to_uppercase()
    )
}

/// Имя сигнала с учётом ЛОКАЛЬНЫХ имён печатаемой функции (фича 0424).
///
/// ⚠️ Признак «локальная переменная функции» в [`signal_of`] строится как «имя
/// не объявлено в модели» — и при СОВПАДЕНИИ имён он ложен: локальная `s`
/// функции получала префикс модели и писала в её сигнал. Список локальных
/// имён снимает двусмысленность там, где узел её не различает.
pub(crate) fn signal_of_in(
    var: &std::rc::Rc<std::cell::RefCell<VariableNode>>,
    scope: &Scope,
) -> Option<String> {
    if let VariableNode::Simple { name, .. } = &*var.borrow()
        && scope.locals.contains(name)
    {
        return Some(name.clone());
    }
    signal_of(var)
}

/// Извлекает имя элемента Takt из узла переменной.
///
/// Возвращает **имя Takt**, а не имя сигнала: отображение в сигнал делает
/// [`Scope`], который один знает про префиксы уплощения.
pub(crate) fn signal_of(var: &std::rc::Rc<std::cell::RefCell<VariableNode>>) -> Option<String> {
    match &*var.borrow() {
        // Переменная модели получает префикс ВЛАДЕЛЬЦА, а не «модели, которую
        // сейчас печатаем»: имя владельца берётся из самой переменной (`upper`),
        // поэтому одноимённые переменные разных под-моделей расходятся сами
        // собой, без карты имён и без риска, что карта отобразит не ту.
        VariableNode::Simple { upper, name, .. } => {
            let Some(owner) = upper.as_ref().and_then(|u| u.upgrade()) else {
                // Владельца нет — это не переменная модели, а локальная
                // переменная функции: у неё нет ни регистра, ни префикса.
                return Some(name.clone());
            };
            // ⚠️ Параметры и локальные переменные функции — ТОЖЕ `Simple` и
            // тоже ссылаются на модель. Отличить их можно ровно одним: они не
            // объявлены в самой модели. Без этой проверки `travel_time(to_stack)`
            // печатал бы `stacker_to_stack` — сигнал, которого не существует
            // (проба 2026-07-16: `Can't find definition of variable`).
            if !owner.borrow().variables.contains_key(name) {
                return Some(name.clone());
            }
            let model: crate::semantic::minimap::Name = owner.into();
            Some(format!("{}_{}", model.unique_lowercase_snakecase(), name))
        }
        // Порт — вывод кристалла, его имя задал автор и оно уникально по модулю
        // (`collect_ports` дедуплицирует). Константа — `localparam` уровня
        // модуля: префикс ей не нужен и только мешал бы читать — **кроме**
        // константы, выведенной из параметра модели (фича 0185).
        VariableNode::Const { upper, name, .. } => Some(const_signal(upper.as_ref(), name)),
        VariableNode::Port { name, .. } => Some(name.clone()),
        VariableNode::Unresolved => None,
    }
}

/// Имя `localparam` константы — **с префиксом владельца** (фича 0193; форма
/// заведена задачей 0185-06 для констант-параметров).
///
/// ⚠️ Композиция в цели `sv` **уплощается**, и `localparam` живёт на уровне
/// модуля: две модели с одноимённой константой разных значений давали **одно**
/// объявление, и вторая молча получала значение первой (проба: `model A { const
/// K := 2; } model B { const K := 3; }` давала один `localparam K = 2`).
/// Поэтому префикс несут **все** константы, а не только выведенные из параметра
/// модели — правило то же, что у регистров (`Simple` выше), и берётся оно у
/// **владельца**, а не у «модели, которую печатаем». Исключение для обычных
/// констант, заведённое 0185-06 ради неизменного вывода корпуса, снято ADR 0193.
///
/// ⚠️ Тем же именем идёт дедупликация объявлений (`sv_const::emit_constants`) и
/// согласуется ключ «константа используется»
/// ([`crate::semantic::unused::const_key`]): печать и фильтрация — одно правило.
pub(crate) fn const_signal(
    upper: Option<&std::rc::Weak<std::cell::RefCell<crate::semantic::ModelNode>>>,
    name: &str,
) -> String {
    let Some(owner) = upper.and_then(|u| u.upgrade()) else {
        return name.to_string();
    };
    let model: crate::semantic::minimap::Name = owner.into();
    format!("{}_{}", model.unique_lowercase_snakecase(), name)
}

/// Печатает доступ к члену (`x.0` → `x[0]`, `p.field` → `p.field`).
///
/// **Битового доступа как отдельной конструкции в SV нет и не нужно:** вектор
/// индексируется тем же `[]`, что и массив. Это заметно проще цели `st`, где
/// `x.0` разворачивается в маску `(USINT_TO_BYTE(x) AND 16#01) <> 16#00` —
/// MatIEC не знает ни `x.0`, ни `%X0` (`CLAUDE.md`, фича 0041).
pub(crate) fn print_member(base: &str, member: &Member) -> String {
    match member {
        Member::Number(index) => format!("{}[{}]", base, index),
        Member::Identifier(id) => format!("{}.{}", base, id.name),
    }
}
