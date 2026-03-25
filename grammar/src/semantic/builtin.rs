//! Встроенные функции языка BuT.
//!
//! Встроенные функции не объявляются пользователем явно — они всегда доступны
//! в любом контексте программы BuT. Реестр определён как статическая PHF-таблица
//! [`BUILTIN_FUNCTIONS`].
//!
//! ## Текущий список встроенных функций
//!
//! | Имя     | Параметры                      | Возвращаемый тип  | Описание                              |
//! |---------|--------------------------------|-------------------|---------------------------------------|
//! | `debug` | `text: BuiltinString`          | `Unit`            | Вывод отладочного сообщения           |
//! | `S`     | `model: BuiltinModel`          | `BuiltinState`    | Получение начального состояния модели |

use crate::diagnostics::Diagnostic;
use crate::semantic::{FunctionNode, TypeNode};
use phf::phf_map;
use std::convert::Into;

/// Статическая таблица встроенных функций языка BuT.
///
/// Ключ — имя функции, значение — [`FunctionNode::Builtin`].
const BUILTIN_FUNCTIONS: phf::Map<&'static str, FunctionNode> = phf_map! {
    "debug" => FunctionNode::Builtin("debug", &[("text", TypeNode::BuiltinString)], TypeNode::Unit),
    "S" => FunctionNode::Builtin("S", &[("model", TypeNode::BuiltinModel)], TypeNode::BuiltinState),
};

/// Возвращает [`FunctionNode`] встроенной функции по имени.
///
/// # Ошибки
///
/// Возвращает [`Diagnostic`], если функция с указанным именем не найдена
/// в реестре встроенных функций.
///
/// # Примеры
///
/// ```but
/// debug("значение x");   // встроенная функция debug
/// start A = S(M) { }    // встроенная функция S
/// ```
pub fn builtin_function(name: &str) -> Result<&FunctionNode, Diagnostic> {
    Ok(BUILTIN_FUNCTIONS
        .get(name)
        .ok_or_else(|| format!("Неизвестная функция '{}'", name).as_str().into())?)
}

// ── Тесты ─────────────────────────────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use super::*;
    use crate::semantic::TypeNode;

    /// `builtin_function("debug")` возвращает корректный узел.
    #[test]
    fn builtin_debug_exists() {
        let func = builtin_function("debug").unwrap();
        assert!(
            matches!(func, FunctionNode::Builtin("debug", _, TypeNode::Unit)),
            "debug должна быть Builtin с возвратом Unit"
        );
    }

    /// `builtin_function("S")` возвращает корректный узел.
    #[test]
    fn builtin_s_exists() {
        let func = builtin_function("S").unwrap();
        assert!(
            matches!(func, FunctionNode::Builtin("S", _, TypeNode::BuiltinState)),
            "S должна быть Builtin с возвратом BuiltinState"
        );
    }

    /// Контрпример: неизвестная встроенная функция → ошибка.
    ///
    /// # Контрпример (BuT)
    /// ```but
    /// ghost();   // ошибка: функция ghost не определена
    /// ```
    #[test]
    fn unknown_builtin_is_error() {
        let result = builtin_function("ghost");
        assert!(result.is_err(), "неизвестная функция должна давать ошибку");
        let err = result.unwrap_err();
        assert!(
            err.message.contains("ghost"),
            "сообщение должно содержать имя функции: {}",
            err.message
        );
    }

    /// Параметры `debug`: один параметр `text: BuiltinString`.
    #[test]
    fn builtin_debug_has_correct_params() {
        if let FunctionNode::Builtin(_, params, _) = builtin_function("debug").unwrap() {
            assert_eq!(params.len(), 1);
            assert_eq!(params[0].0, "text");
            assert_eq!(params[0].1, TypeNode::BuiltinString);
        } else {
            panic!("ожидался FunctionNode::Builtin");
        }
    }

    /// Параметры `S`: один параметр `model: BuiltinModel`.
    #[test]
    fn builtin_s_has_correct_params() {
        if let FunctionNode::Builtin(_, params, ret) = builtin_function("S").unwrap() {
            assert_eq!(params.len(), 1);
            assert_eq!(params[0].0, "model");
            assert_eq!(params[0].1, TypeNode::BuiltinModel);
            assert_eq!(*ret, TypeNode::BuiltinState);
        } else {
            panic!("ожидался FunctionNode::Builtin");
        }
    }
}
