//! Построение семантических узлов типов языка BuT.
//!
//! Основная функция [`construct_type`] преобразует АСД-тип [`Type`]
//! в семантический [`TypeNode`].
//!
//! ## Поддерживаемые типы
//!
//! | АСД (`ast::Type`)              | Семантический узел             |
//! |-------------------------------|--------------------------------|
//! | `Type::Bit`                   | `TypeNode::Bit`                |
//! | `Type::Bool`                  | `TypeNode::Bool`               |
//! | `Type::Rational`              | `TypeNode::Rational`           |
//! | `Type::Unit`                  | `TypeNode::Unit`               |
//! | `Type::Array { N, T }`        | `TypeNode::Array(N, T)`        |
//! | `Type::Address { addr, bit }` | `TypeNode::Address(addr, bit)` |
//! | `Type::Alias("bit")`          | `TypeNode::Bit`                |
//! | `Type::Alias("bool")`         | `TypeNode::Bool`               |
//! | `Type::Alias("float")`        | `TypeNode::Rational`           |
//! | `Type::Alias("unit")`         | `TypeNode::Unit`               |
//! | `Type::Alias(local)`          | значение из таблицы типов      |
//! | `Type::Function { .. }`       | `TypeNode::Unsupported`        |
//! | `None`                        | `TypeNode::Inference`          |

use crate::diagnostics::Diagnostic;
use crate::parser::ast::Type;
use crate::semantic::TypeNode;
use std::collections::HashMap;

/// Строит [`TypeNode`] из опционального АСД-типа [`Type`].
///
/// Если тип не задан (`None`), возвращает [`TypeNode::Inference`] —
/// заглушку для последующего вывода типа.
///
/// # Ошибки
///
/// Возвращает [`Diagnostic`], если тип является псевдонимом, которого
/// нет в таблице типов `map`.
pub(crate) fn construct_type(
    typ: Option<Type>,
    map: &HashMap<String, TypeNode>,
) -> Result<TypeNode, Diagnostic> {
    if typ.is_none() {
        return Ok(TypeNode::Inference);
    }
    match typ.unwrap() {
        Type::Address { address, bit } => Ok(TypeNode::Address(address, bit)),
        Type::Bit => Ok(TypeNode::Bit),
        Type::Bool => Ok(TypeNode::Bool),
        Type::Rational => Ok(TypeNode::Rational),
        Type::Alias(def) => match def.name.as_str() {
            "bit" => Ok(TypeNode::Bit),
            "bool" => Ok(TypeNode::Bool),
            "float" => Ok(TypeNode::Rational),
            "unit" => Ok(TypeNode::Unit),
            local => Ok(map
                .get(local)
                .ok_or_else(|| {
                    format!("Локальный тип '{}' не найден", &def.name)
                        .as_str()
                        .into()
                })?
                .clone()),
        },
        Type::Array {
            element_type,
            element_count,
            ..
        } => Ok(TypeNode::Array(
            element_count,
            Box::new(construct_type(Some(*element_type), map)?),
        )),
        Type::Function { .. } => Ok(TypeNode::Unsupported),
        Type::Unit => Ok(TypeNode::Unit),
    }
}

// ── Тесты ─────────────────────────────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use super::*;
    use crate::diagnostics::Location;
    use crate::parser::ast::{Identifier, Type};

    fn empty_map() -> HashMap<String, TypeNode> {
        HashMap::new()
    }

    // ── Примитивные типы ──────────────────────────────────────────────────────

    /// `None` → `TypeNode::Inference`.
    #[test]
    fn none_gives_inference() {
        assert_eq!(construct_type(None, &empty_map()).unwrap(), TypeNode::Inference);
    }

    /// `Type::Bit` → `TypeNode::Bit`.
    #[test]
    fn bit_gives_bit() {
        assert_eq!(construct_type(Some(Type::Bit), &empty_map()).unwrap(), TypeNode::Bit);
    }

    /// `Type::Bool` → `TypeNode::Bool`.
    #[test]
    fn bool_gives_bool() {
        assert_eq!(construct_type(Some(Type::Bool), &empty_map()).unwrap(), TypeNode::Bool);
    }

    /// `Type::Rational` → `TypeNode::Rational`.
    #[test]
    fn rational_gives_rational() {
        assert_eq!(
            construct_type(Some(Type::Rational), &empty_map()).unwrap(),
            TypeNode::Rational
        );
    }

    /// `Type::Unit` → `TypeNode::Unit`.
    #[test]
    fn unit_gives_unit() {
        assert_eq!(
            construct_type(Some(Type::Unit), &empty_map()).unwrap(),
            TypeNode::Unit
        );
    }

    /// `Type::Address { addr, bit }` → `TypeNode::Address(addr, bit)`.
    #[test]
    fn address_gives_address() {
        let ty = Type::Address {
            address: 0x1234,
            bit: Some(3),
        };
        assert_eq!(
            construct_type(Some(ty), &empty_map()).unwrap(),
            TypeNode::Address(0x1234, Some(3))
        );
    }

    /// `Type::Address` без бита → `TypeNode::Address(addr, None)`.
    #[test]
    fn address_without_bit() {
        let ty = Type::Address {
            address: 0xFF,
            bit: None,
        };
        assert_eq!(
            construct_type(Some(ty), &empty_map()).unwrap(),
            TypeNode::Address(0xFF, None)
        );
    }

    // ── Псевдонимы встроенных типов ───────────────────────────────────────────

    fn alias(name: &str) -> Type {
        Type::Alias(Identifier::new(name))
    }

    /// `Alias("bit")` → `TypeNode::Bit`.
    #[test]
    fn alias_bit_gives_bit() {
        assert_eq!(construct_type(Some(alias("bit")), &empty_map()).unwrap(), TypeNode::Bit);
    }

    /// `Alias("bool")` → `TypeNode::Bool`.
    #[test]
    fn alias_bool_gives_bool() {
        assert_eq!(construct_type(Some(alias("bool")), &empty_map()).unwrap(), TypeNode::Bool);
    }

    /// `Alias("float")` → `TypeNode::Rational`.
    #[test]
    fn alias_float_gives_rational() {
        assert_eq!(
            construct_type(Some(alias("float")), &empty_map()).unwrap(),
            TypeNode::Rational
        );
    }

    /// `Alias("unit")` → `TypeNode::Unit`.
    #[test]
    fn alias_unit_gives_unit() {
        assert_eq!(
            construct_type(Some(alias("unit")), &empty_map()).unwrap(),
            TypeNode::Unit
        );
    }

    // ── Пользовательские псевдонимы ───────────────────────────────────────────

    /// Псевдоним из таблицы типов разрешается в соответствующий `TypeNode`.
    ///
    /// # Пример (BuT)
    /// ```but
    /// type u8 = [bit;8];
    /// var x: u8 = 0;   // alias "u8" → Array(8, Bit)
    /// ```
    #[test]
    fn local_alias_resolves_from_map() {
        let mut map = HashMap::new();
        map.insert("u8".to_string(), TypeNode::Array(8, Box::new(TypeNode::Bit)));
        assert_eq!(
            construct_type(Some(alias("u8")), &map).unwrap(),
            TypeNode::Array(8, Box::new(TypeNode::Bit))
        );
    }

    /// Контрпример: псевдоним, отсутствующий в таблице, — ошибка.
    ///
    /// # Контрпример (BuT)
    /// ```but
    /// var x: Unknown = 0;   // ошибка: тип Unknown не объявлен
    /// ```
    #[test]
    fn unknown_alias_is_error() {
        let result = construct_type(Some(alias("Unknown")), &empty_map());
        assert!(result.is_err(), "неизвестный псевдоним должен давать ошибку");
        let err = result.unwrap_err();
        assert!(
            err.message.contains("Unknown"),
            "сообщение должно содержать имя неизвестного типа: {}",
            err.message
        );
    }

    // ── Массивы ───────────────────────────────────────────────────────────────

    /// `Type::Array { N=8, T=Bit }` → `TypeNode::Array(8, Bit)`.
    #[test]
    fn array_bit_8() {
        let ty = Type::Array {
            loc: Location::default(),
            element_type: Box::new(Type::Bit),
            element_count: 8,
        };
        assert_eq!(
            construct_type(Some(ty), &empty_map()).unwrap(),
            TypeNode::Array(8, Box::new(TypeNode::Bit))
        );
    }

    /// Вложенный массив: `[[bit;4];2]` → `Array(2, Array(4, Bit))`.
    #[test]
    fn nested_array() {
        let inner = Type::Array {
            loc: Location::default(),
            element_type: Box::new(Type::Bit),
            element_count: 4,
        };
        let outer = Type::Array {
            loc: Location::default(),
            element_type: Box::new(inner),
            element_count: 2,
        };
        assert_eq!(
            construct_type(Some(outer), &empty_map()).unwrap(),
            TypeNode::Array(2, Box::new(TypeNode::Array(4, Box::new(TypeNode::Bit))))
        );
    }

    /// `Type::Function { .. }` → `TypeNode::Unsupported`.
    #[test]
    fn function_type_is_unsupported() {
        use crate::parser::ast::ParameterList;
        let ty = Type::Function {
            params: ParameterList::default(),
            returns: None,
        };
        assert_eq!(
            construct_type(Some(ty), &empty_map()).unwrap(),
            TypeNode::Unsupported
        );
    }
}
