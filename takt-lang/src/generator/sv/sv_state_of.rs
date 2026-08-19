//! Форма `S(Модель) = Состояние` в цели `sv` (фича 0267).
//!
//! Отдельный модуль, потому что `sv_expr.rs` пришпилен лимитом размера, а знание
//! «как адресуется состояние соседа» самостоятельно: оно повторяет правило
//! именования регистров из `sv_fsm` и обязано меняться вместе с ним.

use super::sv_expr::Scope;
use crate::semantic::ConditionNode;

/// Разбирает форму `S(Модель) = Состояние` и даёт имена сигнала и варианта
/// (фича 0267).
///
/// Возвращает `None`, если условие к этой форме не относится — тогда печатается
/// обычное сравнение.
///
/// ## Почему цель `sv` это умеет, а `rust` и `st` — нет
///
/// Композиция здесь **уплощается**: регистры состояний всех уровней живут в
/// одном модуле, и состояние соседа — обычный сигнал того же `always_comb`. У
/// `rust` под-модель получает только себя и `<Root>Shared` (решение 0059), у
/// `st` экземпляры под-моделей — поля родительского `FUNCTION_BLOCK`; там для
/// той же записи нужен протокол «под-модель видит корень», как `main` у цели
/// `c`. Разбор — ADR 0267.
///
/// ⚠️ Правая часть приходит **неразрешённой**: `Done` — состояние
/// модели-аргумента, а не той, где записано условие (инвариант проекта: проход
/// `resolve_state_references` запрещён). Имя берётся строкой — так же поступают
/// цели `c` и `rust`.
///
/// ⚠️ Имя регистра строится тем же правилом, что в `sv_fsm::state_reg_name`:
/// у корня — `state`, у под-модели — с префиксом уникального имени. Корень
/// узнаётся по отсутствию родителя, а не по имени: второе знание о корне
/// разъехалось бы с первым.
pub(in crate::generator::sv) fn state_comparison(
    left: &ConditionNode,
    right: &ConditionNode,
) -> Option<(String, String)> {
    let model = crate::semantic::condition::state_of::state_of_model(left)?;
    let state = crate::semantic::condition::state_of::compared_state_name(right)?;
    let name = crate::semantic::minimap::Name::from(std::rc::Rc::clone(model));
    let is_root = model.borrow().upper.is_none();
    let reg = if is_root {
        "state".to_string()
    } else {
        format!("{}_state", name.unique_lowercase_snakecase())
    };
    let variant = format!(
        "{}_{}",
        name.unique_uppercase_snakecase(),
        crate::semantic::naming::normalize_lowercase_snakecase(state).to_uppercase()
    );
    Some((reg, variant))
}

/// Печатает сравнение состояния под-модели, если условие имеет форму
/// `S(Модель) = Состояние` (или `!=`); иначе — `None`.
///
/// Чтение идёт через [`Scope::read`], то есть из рабочей копии `_next`:
/// наблюдатель обязан увидеть переход соседа **на том же такте**, как в эталоне
/// и в цели `c` (правило 0245). Чтение регистра дало бы состояние предыдущего
/// такта — модуль при этом валиден и синтезируем, а трасса разъезжается, и
/// поймать это может только потактовая сверка.
pub(in crate::generator::sv) fn print(node: &ConditionNode, scope: &Scope) -> Option<String> {
    let (left, right, op) = match node {
        ConditionNode::Equal(l, r) => (l, r, "=="),
        ConditionNode::NotEqual(l, r) => (l, r, "!="),
        _ => return None,
    };
    let (reg, variant) = state_comparison(left, right)?;
    Some(format!("({} {} {})", scope.read(&reg), op, variant))
}
