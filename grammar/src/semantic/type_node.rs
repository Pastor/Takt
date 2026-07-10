//! Построение семантических узлов типов языка Lam.
//!
//! Основная функция [`construct_type`] преобразует АСД-тип [`Type`]
//! в семантический [`TypeNode`].
//!
//! Вместо `&HashMap<String, TypeNode>` функция принимает `Rc<RefCell<ModelNode>>`,
//! что позволяет искать типы через цепочку родительских моделей.
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
//! | `Type::Enum("Color")`         | `TypeNode::Enum("Color")`      |
//! | `Type::Alias("bit")`          | `TypeNode::Bit`                |
//! | `Type::Alias("bool")`         | `TypeNode::Bool`               |
//! | `Type::Alias("float")`        | `TypeNode::Rational`           |
//! | `Type::Alias("unit")`         | `TypeNode::Unit`               |
//! | `Type::Alias(local)`          | значение из таблицы типов модели |
//! | `Type::Function { .. }`       | `TypeNode::Unsupported`        |
//! | `None`                        | `TypeNode::Inference`          |
//!
//! **Примечание о `Type::Enum`:** `construct_type` не проверяет, объявлено ли
//! перечисление; эта проверка выполняется в `validate_model` через
//! `validate_enum_type_declarations`. Данное разделение позволяет обрабатывать
//! взаимные ссылки между перечислениями и переменными.

use crate::diagnostics::Diagnostic;
use crate::parser::ast::Type;
use crate::semantic::ModelNode;
use std::cell::RefCell;
use std::fmt;
use std::rc::Rc;

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
    model: Rc<RefCell<ModelNode>>,
) -> Result<TypeNode, Diagnostic> {
    if typ.is_none() {
        return Ok(TypeNode::Inference);
    }
    match typ.unwrap() {
        Type::Address { address, bit } => Ok(TypeNode::Address(address, bit)),
        Type::Bit => Ok(TypeNode::Bit),
        Type::Bool => Ok(TypeNode::Bool),
        Type::Rational => Ok(TypeNode::Rational),
        Type::Alias(def) => {
            // Примитивные типы: «bit», «bool», «float», «unit» — жёстко связаны.
            match def.name.as_str() {
                "bit" => return Ok(TypeNode::Bit),
                "bool" => return Ok(TypeNode::Bool),
                "float" => return Ok(TypeNode::Rational),
                "unit" => return Ok(TypeNode::Unit),
                _ => {}
            }
            // Пользовательский псевдоним в таблице типов модели берёт приоритет
            // над встроенными именами (u8, i32 и пр.), что позволяет переопределять
            // встроенные типы на уровне модели для обратной совместимости.
            if let Some(rc) = model.borrow().search_type(&def.name) {
                return Ok(rc.borrow().clone());
            }
            // Встроенные целочисленные типы
            match def.name.as_str() {
                "u8" => Ok(TypeNode::Integer {
                    bits: 8,
                    signed: false,
                }),
                "u16" => Ok(TypeNode::Integer {
                    bits: 16,
                    signed: false,
                }),
                "u32" => Ok(TypeNode::Integer {
                    bits: 32,
                    signed: false,
                }),
                "u64" => Ok(TypeNode::Integer {
                    bits: 64,
                    signed: false,
                }),
                "i8" => Ok(TypeNode::Integer {
                    bits: 8,
                    signed: true,
                }),
                "i16" => Ok(TypeNode::Integer {
                    bits: 16,
                    signed: true,
                }),
                "i32" => Ok(TypeNode::Integer {
                    bits: 32,
                    signed: true,
                }),
                "i64" => Ok(TypeNode::Integer {
                    bits: 64,
                    signed: true,
                }),
                local => Err(Diagnostic::declaration_error(
                    def.loc,
                    format!("Локальный тип '{}' не найден", local),
                )
                .with_code("SE-034")),
            }
        }
        Type::Array {
            element_type,
            element_count,
            ..
        } => Ok(TypeNode::Array(
            element_count,
            Box::new(construct_type(Some(*element_type), model)?),
        )),
        Type::Function { .. } => Ok(TypeNode::Unsupported),
        Type::Unit => Ok(TypeNode::Unit),
        // Type::Enum используется только в парсере как узел грамматики;
        // в качестве типа переменной не поддерживается на уровне семантики.
        Type::Enum(name) => Ok(TypeNode::Enum(name.clone())),
        // Type::Struct — ссылка на объявленный структурный тип.
        Type::Struct(name) => Ok(TypeNode::Struct(name.clone())),
    }
}

// ── Тесты ─────────────────────────────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use super::*;
    use crate::diagnostics::Location;
    use crate::parser::ast::{Identifier, Type};
    use crate::semantic::ModelNode;
    use std::cell::RefCell;
    use std::collections::HashMap;
    use std::rc::Rc;

    /// Вспомогательная функция: создаёт пустую модель.
    fn empty_model() -> Rc<RefCell<ModelNode>> {
        Rc::new(RefCell::new(ModelNode::default()))
    }

    // ── Примитивные типы ──────────────────────────────────────────────────────

    /// `None` → `TypeNode::Inference`.
    #[test]
    fn none_gives_inference() {
        assert_eq!(
            construct_type(None, empty_model()).unwrap(),
            TypeNode::Inference
        );
    }

    /// `Type::Bit` → `TypeNode::Bit`.
    #[test]
    fn bit_gives_bit() {
        assert_eq!(
            construct_type(Some(Type::Bit), empty_model()).unwrap(),
            TypeNode::Bit
        );
    }

    /// `Type::Bool` → `TypeNode::Bool`.
    #[test]
    fn bool_gives_bool() {
        assert_eq!(
            construct_type(Some(Type::Bool), empty_model()).unwrap(),
            TypeNode::Bool
        );
    }

    /// `Type::Rational` → `TypeNode::Rational`.
    #[test]
    fn rational_gives_rational() {
        assert_eq!(
            construct_type(Some(Type::Rational), empty_model()).unwrap(),
            TypeNode::Rational
        );
    }

    /// `Type::Unit` → `TypeNode::Unit`.
    #[test]
    fn unit_gives_unit() {
        assert_eq!(
            construct_type(Some(Type::Unit), empty_model()).unwrap(),
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
            construct_type(Some(ty), empty_model()).unwrap(),
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
            construct_type(Some(ty), empty_model()).unwrap(),
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
        assert_eq!(
            construct_type(Some(alias("bit")), empty_model()).unwrap(),
            TypeNode::Bit
        );
    }

    /// `Alias("bool")` → `TypeNode::Bool`.
    #[test]
    fn alias_bool_gives_bool() {
        assert_eq!(
            construct_type(Some(alias("bool")), empty_model()).unwrap(),
            TypeNode::Bool
        );
    }

    /// `Alias("float")` → `TypeNode::Rational`.
    #[test]
    fn alias_float_gives_rational() {
        assert_eq!(
            construct_type(Some(alias("float")), empty_model()).unwrap(),
            TypeNode::Rational
        );
    }

    /// `Alias("unit")` → `TypeNode::Unit`.
    #[test]
    fn alias_unit_gives_unit() {
        assert_eq!(
            construct_type(Some(alias("unit")), empty_model()).unwrap(),
            TypeNode::Unit
        );
    }

    // ── Пользовательские псевдонимы ───────────────────────────────────────────

    /// Псевдоним из таблицы типов модели разрешается в соответствующий `TypeNode`.
    ///
    /// # Пример (Lam)
    /// ```but
    /// type byte8 = [bit;8];
    /// var x: byte8 = 0;   // alias "byte8" → Array(8, Bit)
    /// ```
    #[test]
    fn local_alias_resolves_from_map() {
        let mut map = HashMap::new();
        map.insert(
            "byte8".to_string(),
            TypeNode::Array(8, Box::new(TypeNode::Bit)),
        );
        let model = Rc::new(RefCell::new(ModelNode {
            types: map,
            ..Default::default()
        }));
        assert_eq!(
            construct_type(Some(alias("byte8")), model).unwrap(),
            TypeNode::Array(8, Box::new(TypeNode::Bit))
        );
    }

    /// Встроенный псевдоним `u8` разрешается в `Integer { bits: 8, signed: false }`.
    #[test]
    fn builtin_u8_resolves_to_integer() {
        let model = Rc::new(RefCell::new(ModelNode::default()));
        assert_eq!(
            construct_type(Some(alias("u8")), model).unwrap(),
            TypeNode::Integer {
                bits: 8,
                signed: false
            }
        );
    }

    /// Встроенный псевдоним `i32` разрешается в `Integer { bits: 32, signed: true }`.
    #[test]
    fn builtin_i32_resolves_to_integer() {
        let model = Rc::new(RefCell::new(ModelNode::default()));
        assert_eq!(
            construct_type(Some(alias("i32")), model).unwrap(),
            TypeNode::Integer {
                bits: 32,
                signed: true
            }
        );
    }

    /// Контрпример: псевдоним, отсутствующий в таблице, — ошибка.
    ///
    /// # Контрпример (Lam)
    /// ```but
    /// var x: Unknown = 0;   // ошибка: тип Unknown не объявлен
    /// ```
    #[test]
    fn unknown_alias_is_error() {
        let result = construct_type(Some(alias("Unknown")), empty_model());
        assert!(
            result.is_err(),
            "неизвестный псевдоним должен давать ошибку"
        );
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
            construct_type(Some(ty), empty_model()).unwrap(),
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
            construct_type(Some(outer), empty_model()).unwrap(),
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
            construct_type(Some(ty), empty_model()).unwrap(),
            TypeNode::Unsupported
        );
    }

    // ── Ce4: перечисления ─────────────────────────────────────────────────────

    /// `Type::Enum("Color")` → `TypeNode::Enum("Color")`.
    ///
    /// # Пример (Lam)
    /// ```text
    /// enum Color { Red = 0, Green = 1 }
    /// var c: Color = 0;   // тип аннотации → TypeNode::Enum("Color")
    /// ```
    #[test]
    fn enum_type_gives_enum_node() {
        assert_eq!(
            construct_type(Some(Type::Enum("Color".to_string())), empty_model()).unwrap(),
            TypeNode::Enum("Color".to_string())
        );
    }

    /// `Type::Enum` с пустым именем → `TypeNode::Enum("")`.
    ///
    /// Имя не проверяется в `construct_type` — валидация в `validate_model`.
    #[test]
    fn enum_type_empty_name() {
        assert_eq!(
            construct_type(Some(Type::Enum(String::new())), empty_model()).unwrap(),
            TypeNode::Enum(String::new())
        );
    }

    /// `Type::Enum` не зависит от таблицы типов (псевдонимов).
    ///
    /// # Контр-пример
    /// Наличие псевдонима "Color" в таблице не влияет на `Type::Enum("Color")` —
    /// они обрабатываются независимо.
    #[test]
    fn enum_type_ignores_type_alias_table() {
        let mut map = HashMap::new();
        // В таблице типов есть "Color" как псевдоним — но Type::Enum идёт своим путём
        map.insert(
            "Color".to_string(),
            TypeNode::Array(8, Box::new(TypeNode::Bit)),
        );
        let model = Rc::new(RefCell::new(ModelNode {
            types: map,
            ..Default::default()
        }));
        // Type::Enum("Color") всё равно → TypeNode::Enum("Color"), не Array
        assert_eq!(
            construct_type(Some(Type::Enum("Color".to_string())), model).unwrap(),
            TypeNode::Enum("Color".to_string())
        );
    }

    /// `Display` показывает читаемые имена типов, а не Rust-Debug.
    #[test]
    fn type_node_display() {
        assert_eq!(TypeNode::Bit.to_string(), "bit");
        assert_eq!(TypeNode::Bool.to_string(), "bool");
        assert_eq!(TypeNode::Rational.to_string(), "float");
        assert_eq!(TypeNode::Unit.to_string(), "unit");
        assert_eq!(
            TypeNode::Integer {
                bits: 8,
                signed: false
            }
            .to_string(),
            "u8"
        );
        assert_eq!(
            TypeNode::Integer {
                bits: 16,
                signed: true
            }
            .to_string(),
            "i16"
        );
        assert_eq!(
            TypeNode::Integer {
                bits: 64,
                signed: false
            }
            .to_string(),
            "u64"
        );
        assert_eq!(
            TypeNode::Array(8, Box::new(TypeNode::Bit)).to_string(),
            "[bit;8]"
        );
        assert_eq!(TypeNode::Enum("Color".to_string()).to_string(), "Color");
        assert_eq!(TypeNode::Struct("Packet".to_string()).to_string(), "Packet");
        assert_eq!(TypeNode::Inference.to_string(), "_");
    }
}

/// Семантический узел типа данных.
///
/// Варианты:
/// - [`Detecting`](TypeNode::Inference) — тип выводится (временная заглушка).
/// - [`Address`](TypeNode::Address) — адресный тип порта `(адрес, бит?)`.
/// - [`Bit`](TypeNode::Bit) — 1-битный примитив (`bit`).
/// - [`Bool`](TypeNode::Bool) — булев тип (`bool`).
/// - [`Rational`](TypeNode::Rational) — вещественное число (`float`).
/// - [`Array`](TypeNode::Array) — массив фиксированного размера `(N, элемент)`.
/// - [`Enum`](TypeNode::Enum) — перечисление (Ce4).
/// - [`Struct`](TypeNode::Struct) — структурный тип (NI3).
/// - [`Unsupported`](TypeNode::Unsupported) — неподдерживаемый тип (например, функциональный).
#[derive(Default, Debug, PartialEq, Eq, Clone)]
#[non_exhaustive]
pub enum TypeNode {
    /// Тип ещё не определён (вывод типа в процессе).
    #[default]
    Inference,
    /// Адресный тип порта: `(адрес, номер_бита?)`.
    Address(u64, Option<u64>),
    /// 1-битный примитив (`bit`).
    Bit,
    /// Тип `bool` — булев тип (`true`/`false`).
    Bool,
    /// Тип с плавающей точкой (`float`).
    Rational,
    /// Массив фиксированного размера: `(количество_элементов, тип_элемента)`.
    Array(u16, Box<TypeNode>),
    /// Перечисление (Ce4): именованный тип с фиксированным набором значений.
    ///
    /// Хранит имя перечисления.
    Enum(String),
    /// Неподдерживаемый тип (например, функциональный).
    Unsupported,
    /// Пустой тип.
    Unit,
    /// Встроенный строковый тип (внутренний, для встроенных функций).
    BuiltinString,
    /// Встроенный тип модели (внутренний, для встроенных функций).
    BuiltinModel,
    /// Встроенный тип состояния (внутренний, для встроенных функций).
    BuiltinState,
    /// Встроенный числовой тип (внутренний, для встроенных функций).
    ///
    /// Обозначает «любой числовой тип»: `Bit`, `Rational`, `Array(_, Bit)`.
    /// Используется для параметров и возвращаемых значений математических
    /// встроенных функций (`min`, `max`, `abs`).
    BuiltinNumeric,
    /// Структурный тип (NI3): именованная структура с полями.
    ///
    /// Хранит имя структуры.
    Struct(String),
    /// Встроенный целочисленный тип: `u8`/`i8`…`u64`/`i64`.
    ///
    /// `bits` — разрядность (8, 16, 32, 64); `signed` — знаковость.
    Integer {
        /// Ширина в битах: 8, 16, 32 или 64.
        bits: u8,
        /// `true` → знаковый (`int{bits}_t`), `false` → беззнаковый (`uint{bits}_t`).
        signed: bool,
    },
}

impl fmt::Display for TypeNode {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            TypeNode::Bit => write!(f, "bit"),
            TypeNode::Bool => write!(f, "bool"),
            TypeNode::Rational => write!(f, "float"),
            TypeNode::Unit => write!(f, "unit"),
            TypeNode::Integer { bits, signed } => {
                write!(f, "{}{}", if *signed { "i" } else { "u" }, bits)
            }
            TypeNode::Array(n, elem) => write!(f, "[{};{}]", elem, n),
            TypeNode::Enum(name) => write!(f, "{}", name),
            TypeNode::Struct(name) => write!(f, "{}", name),
            TypeNode::Address(addr, Some(bit)) => write!(f, "0x{:X}:{}", addr, bit),
            TypeNode::Address(addr, None) => write!(f, "0x{:X}", addr),
            TypeNode::BuiltinString => write!(f, "string"),
            TypeNode::BuiltinModel => write!(f, "model"),
            TypeNode::BuiltinState => write!(f, "state"),
            TypeNode::BuiltinNumeric => write!(f, "numeric"),
            TypeNode::Inference => write!(f, "_"),
            TypeNode::Unsupported => write!(f, "?"),
        }
    }
}
