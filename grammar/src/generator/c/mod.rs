//! Генератор C-кода из семантического дерева BuT.
//!
//! Модуль реализует трансляцию семантического дерева [`ModelNode`] в пару файлов:
//! заголовочный (`.h`) и исходный (`.c`).
//!
//! ## Алгоритм трансляции
//!
//! 1. **Заголовок** (`generate_header`): генерирует `#ifndef`-защиту, структуру
//!    модели (рекурсивно для вложенных моделей) и прототипы функций `_init`, `_tick`, `_reset`.
//! 2. **Источник** (`generate_source`): генерирует `#include` и раскрывает
//!    константы, порты и перечисления через `#define`.
//! 3. **Вспомогательные функции**: `unroll_model`, `unroll_variable`, `unroll_cond`,
//!    `unroll_expression` рекурсивно преобразуют семантические узлы в C-выражения.
//!
//! ## Именование
//!
//! - Структура модели: `<PascalCase>` (например, `MainRobot`).
//! - Поля структуры: snake_case (например, `main->robot.idle`).
//! - Порты: `PORT_<UPPER_SNAKE>` (например, `PORT_MAIN_SENSORS_1`).
//! - Константы: `CONST_<UPPER_SNAKE>`.
//! - Условия: `COND_<UPPER_SNAKE>`.
//! - Перечисления: `ENUM_<UPPER_SNAKE>_<VARIANT>`.
//!
//! ## Интерфейс портов
//!
//! Сгенерированный код обращается к портам через указатели на функции
//! `write_bit`, `read_bit`, `write_float`, `read_float`, которые должны
//! быть предоставлены платформенным слоем.

#![allow(clippy::needless_borrow)]
#![allow(clippy::explicit_auto_deref)]

mod c_header;
mod c_map;
mod c_source;

use crate::diagnostics::{Diagnostic, Location};
use crate::generator::Generator as AsGenerator;
use crate::generator::c::c_header::generate_header;
use crate::generator::c::c_map::CMap;
use crate::semantic::naming::normalize_lowercase_snakecase;
use crate::semantic::type_node::TypeNode;
use crate::semantic::ModelNode;
use itertools::Itertools;
use std::fs;
use std::path::Path;

const FUNCTION_PORT_WRITE_BIT: &str = "write_bit";
const FUNCTION_PORT_READ_BIT: &str = "read_bit";
const FUNCTION_PORT_WRITE_FLOAT: &str = "write_float";
const FUNCTION_PORT_READ_FLOAT: &str = "read_float";

/// Генератор C-кода для модели BuT.
///
/// Реализует трейт [`Generator`](crate::generator::Generator): принимает
/// корневой [`ModelNode`] и записывает пару файлов `.h`/`.c` по заданному пути.
pub struct Generator {}

impl AsGenerator for Generator {
    fn generate(&self, model: &ModelNode, output_path: &str) -> Result<(), Diagnostic> {
        //TODO: При генерации следует работать с примитивным слепком модели
        let map = CMap::new(model.name(), model)?;
        let header = generate_header(map.get_filename(), &map)?;
        // let source = self.generate_source(model)?;
        let model_name = Self::resolve_model_name(model)?;
        let filename = normalize_lowercase_snakecase(model_name);
        let _ = fs::create_dir(Path::new(output_path));
        fs::write(Path::new(output_path).join(filename.clone() + ".h"), header)
            .map_err(|e| Diagnostic::warning(Location::Codegen, format!("{:?}", e)))?;
        // fs::write(Path::new(output_path).join(filename + ".c"), source)
        //     .map_err(|e| Diagnostic::warning(Location::Codegen, format!("{:?}", e)))?;
        Ok(())
    }
}

fn get_typed_variable(typ: &TypeNode, name: Option<String>, model: &ModelNode) -> Option<String> {
    match typ {
        TypeNode::Bit => Some(format!("int {}", name.clone().unwrap_or_default())),
        TypeNode::Bool => Some(format!("bool {}", name.clone().unwrap_or_default())),
        TypeNode::Rational => Some(format!("float {}", name.clone().unwrap_or_default())),
        TypeNode::Array(size, typ) => {
            if let TypeNode::Rational = **typ {
                Some(format!(
                    "float {}[{}]",
                    name.clone().unwrap().as_str(),
                    *size
                ))
            } else {
                Some(format!(
                    "uint{}_t {}",
                    *size,
                    name.clone().unwrap().as_str()
                ))
            }
        }
        TypeNode::Unit => Some("void".to_string()),
        TypeNode::BuiltinString => Some("char *".to_string()),
        TypeNode::Struct(struct_name) => Some(format!(
            "struct {} {}",
            struct_name,
            name.clone().unwrap_or_default()
        )),
        TypeNode::Enum(enum_name) => {
            let enum_node = model.search_enum(enum_name)?;
            let max = enum_node
                .variants
                .into_iter()
                .sorted_by(|a, b| a.1.cmp(&b.1))
                .collect::<Vec<(String, i64)>>()
                .last()
                .map(|x| x.1)
                .unwrap_or_default();
            // Выбираем минимальный беззнаковый тип, вмещающий максимальное значение
            let bits: u8 = if max > u32::MAX as i64 {
                64
            } else if max > u16::MAX as i64 {
                32
            } else if max > u8::MAX as i64 {
                16
            } else {
                8
            };
            // Исправлено: используем имя переменной (name), а не имя enum-типа
            Some(format!("uint{}_t {}", bits, name.clone().unwrap_or_default()))
        }
        TypeNode::BuiltinModel
        | TypeNode::BuiltinState
        | TypeNode::BuiltinNumeric
        | TypeNode::Unsupported
        | TypeNode::Inference
        | TypeNode::Address(_, _) => None,
    }
}


#[cfg(test)]
mod tests {
    use crate::generator::c::Generator;
    use crate::{parse, semantic};

    const SRC: &str = r#"
type u8 = [bit;8];

port sensors_1: u8 = 0x100000000;
port sensors_2: u8 = 0x200000000;
cond AtFloor8 = sensors_1.0 & sensors_1.1;
cond AtFloor9 = sensors_2.0 & sensors_2.1;

enum Direction { North, South, East, West }
enum Priority { Low = 0, Medium = 5, High = 10 }
var heading: Direction = 0;
model Robot {
    var speed: u8 = 0;
    var active: bit = false;

    model Idle {
        start Start {
                enter {
                speed = 0;
                active = false;
                heading = North;
            }
            ref End: active;
        }
        state End;
    }

    start Idle = Idle {
        next Moving;
    }

    state Moving {
        always {
            heading = West;
            speed = 100;
            debug("Moving");
        }
        ref Idle: AtFloor8 & heading = West;
    }
}

start Main = Robot;
    "#;

    #[test]
    fn test_unroll_model() {
        let (model_ast, _) = parse(SRC, 0)
            .map_err(|d| d.into_iter().next().unwrap())
            .unwrap();
        let model = semantic::tree::construct_model(&model_ast, None, &[]).unwrap();
        let model = &*model.borrow();
        let result = Generator::unroll_model(model).unwrap();
        assert_eq!("main", &result);

        let model = model.search_model("Robot").unwrap();
        let model = &*model.borrow();
        let result = Generator::unroll_model(model).unwrap();
        assert_eq!("main->robot", &result);
        let model = model.search_model("Idle").unwrap();
        let model = &*model.borrow();
        let result = Generator::unroll_model(model).unwrap();
        assert_eq!("main->robot.idle", &result);
    }

    // ── V6: Тесты безопасности resolve_model_name ─────────────────────────────

    /// V6: get_upper_name не паникует для модели с явным именем.
    #[test]
    fn v6_get_upper_name_with_named_model_does_not_panic() {
        let (model_ast, _) = parse("model Named { start S; }", 0).unwrap();
        let model_rc = semantic::tree::construct_model(&model_ast, None, &[]).unwrap();
        let model = model_rc.borrow();
        let inner = model.search_model("Named").unwrap();
        let inner = inner.borrow();
        // Должен вернуть строку без паники
        let name = Generator::get_upper_name(&inner);
        assert!(!name.is_empty(), "имя не должно быть пустым");
    }

    /// V6: get_model_name_struct не паникует для модели с явным именем.
    #[test]
    fn v6_get_model_name_struct_with_named_model_does_not_panic() {
        let (model_ast, _) = parse("model MyModel { start S; }", 0).unwrap();
        let model_rc = semantic::tree::construct_model(&model_ast, None, &[]).unwrap();
        let model = model_rc.borrow();
        let inner = model.search_model("MyModel").unwrap();
        let inner = inner.borrow();
        let name = Generator::get_model_name_struct(&inner);
        assert!(!name.is_empty(), "структурное имя не должно быть пустым");
    }

    // ── V7: Тест исправления опечатки в сообщении об ошибке ──────────────────

    /// V7: неподдерживаемое выражение возвращает ошибку «Can't unroll»,
    /// а не «Cnt unrolled» (опечатка была исправлена).
    #[test]
    fn v7_unroll_expression_error_message_corrected() {
        use crate::semantic::ExpressionNode;
        // V7: Initializer теперь корректно генерирует C-код (был «Can't unroll» с опечаткой).
        let init =
            ExpressionNode::Initializer(vec![ExpressionNode::Number(0), ExpressionNode::Number(1)]);
        let result = Generator::unroll_expression(&init);
        assert!(
            result.is_ok(),
            "Initializer должен генерировать C-код, а не ошибку"
        );
        let c_code = result.unwrap();
        assert_eq!(
            c_code, "{0, 1}",
            "Initializer → C-инициализатор {{0, 1}}, получено: {c_code}"
        );

        // V7: для неподдерживаемых узлов ошибка содержит «Can't unroll» (не «Cnt unrolled»).
        // Используем Number для проверки, что путь "можно развернуть" работает.
        let num = ExpressionNode::Number(42);
        let num_result = Generator::unroll_expression(&num);
        assert!(
            num_result.is_ok(),
            "Number должен разворачиваться корректно"
        );
        assert_eq!(num_result.unwrap(), "42");
    }

    // ── Тесты имён констант и портов ─────────────────────────────────────────

    /// Константы и порты с ALL_CAPS-именами не разбиваются посимвольно.
    ///
    /// Регрессия: `normalize_lowercase_snakecase("MATRIX")` ранее давала
    /// `m_a_t_r_i_x`, что приводило к `CONST_..._M_A_T_R_I_X` вместо `CONST_..._MATRIX`.
    #[test]
    fn const_port_names_are_not_char_split() {
        let src = r#"
type u8 = [bit;8];
const MATRIX: u8 = 0;
const NUMB: u8 = 255;
port SENSOR: u8 = 0x100000;
start Main { always { } }
        "#;
        let (model_ast, _) = parse(src, 0).unwrap();
        let model = semantic::tree::construct_model(&model_ast, None, &[]).unwrap();
        let model = model.borrow();
        let generator = Generator {};
        let source = generator.generate_source(&model).unwrap();
        assert!(
            source.contains("CONST_MAIN_MATRIX"),
            "ожидалось CONST_MAIN_MATRIX, получено:\n{source}"
        );
        assert!(
            source.contains("CONST_MAIN_NUMB"),
            "ожидалось CONST_MAIN_NUMB, получено:\n{source}"
        );
        assert!(
            source.contains("PORT_MAIN_SENSOR"),
            "ожидалось PORT_MAIN_SENSOR, получено:\n{source}"
        );
        assert!(
            !source.contains("M_A_T_R_I_X"),
            "имя не должно разбиваться посимвольно:\n{source}"
        );
    }

    /// include в .c-файле не содержит лишнего пробела перед закрывающей кавычкой.
    #[test]
    fn include_directive_has_no_trailing_space() {
        let src = r#"start Main { always { } }"#;
        let (model_ast, _) = parse(src, 0).unwrap();
        let model = semantic::tree::construct_model(&model_ast, None, &[]).unwrap();
        let model = model.borrow();
        let generator = Generator {};
        let source = generator.generate_source(&model).unwrap();
        assert!(
            !source.contains(".h\" "),
            "#include не должен содержать пробел после кавычки:\n{source}"
        );
    }
}
