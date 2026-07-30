//! Проверка инстанцирования модели с аргументами (фича 0185).
//!
//! ## Временный сторож, а не постоянная проверка
//!
//! Аргументы `M(Y := 200)` разбираются и лежат в дереве
//! ([`Extend::Model`](crate::semantic::extend::Extend::Model)), но **ни один
//! потребитель их пока не применяет**: подстановка значений — задачи 0185-04
//! (режим `assign`) и 0185-05 (`specialize`). Без этой проверки такой вход
//! компилировался бы **молча неверно**: проба показала `model->limit = 100` при
//! написанном `Tuner(limit := 200)` — программа принята, а настройка потеряна.
//! Ровно класс дефекта, который фича 0184 закрыла дорогой ценой.
//!
//! Поэтому пока — явный отказ `SE-082`. Задача 0185-04 **обязана снять** и его,
//! и сторожащий тест: `parameter_argument_is_rejected_until_applied`.

use super::*;
use crate::semantic::extend::Extend;

/// Отказывает, если реализация несёт аргументы инстанцирования.
pub(super) fn check_instantiation_arguments(
    model: Rc<RefCell<ModelNode>>,
) -> Result<(), Diagnostic> {
    let model_ref = model.borrow();
    check_extend(&model_ref.implements)?;
    for state in model_ref.states.values() {
        if let StateNode::Implement { implements, .. } = state {
            check_extend(implements)?;
        }
    }
    Ok(())
}

/// Обходит реализацию: композиции прозрачны, интересны только ссылки на модель.
fn check_extend(extend: &Extend) -> Result<(), Diagnostic> {
    match extend {
        Extend::Model(target, loc, args) => match args.first() {
            None => Ok(()),
            Some(first) => {
                let name = target.borrow().name().to_string();
                Err(Diagnostic::error(
                    first.loc,
                    format!(
                        "Значения аргументов инстанцирования пока не применяются генератором: \
                         '{}' у модели '{name}' будет проигнорирован. Задайте значение в \
                         объявлении параметра либо инстанцируйте модель без аргументов",
                        first.name
                    ),
                )
                .with_note(*loc, format!("инстанцирование '{name}' здесь")))
                .map_err(|d| d.with_code("SE-082"))
            }
        },
        Extend::Parentless(inner) => check_extend(inner),
        Extend::Concatenation(items) | Extend::Parallel(items) => {
            for item in items {
                check_extend(item)?;
            }
            Ok(())
        }
        Extend::None | Extend::Unresolved(_) => Ok(()),
    }
}
