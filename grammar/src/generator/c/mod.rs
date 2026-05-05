//! Генератор C-кода из семантического дерева Lam.
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
//! - Порты: варианты `BitPort`, `RationalPort`, `NumericPort` (например, `BitPort_MAIN_SENSORS_1`).
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

mod c_decl;
mod c_expr;
mod c_header;
mod c_map;
mod c_model;
mod c_source;

use crate::diagnostics::{Diagnostic, Location};
use crate::generator::Generator as AsGenerator;
use crate::generator::c::c_header::generate_header;
use crate::generator::c::c_map::CMap;
use crate::generator::c::c_source::generate_source;
use crate::semantic::ModelNode;
use crate::semantic::PortDirection;
use crate::semantic::minimap::{Element, StateExtend};
use crate::semantic::naming::normalize_lowercase_snakecase;
use crate::semantic::type_node::TypeNode;
use itertools::Itertools;
use std::collections::{HashMap, HashSet};
use std::fs;
use std::path::Path;

pub(super) const FUNCTION_PORT_WRITE_BIT: &str = "write_bit";
pub(super) const FUNCTION_PORT_READ_BIT: &str = "read_bit";
pub(super) const FUNCTION_PORT_WRITE_FLOAT: &str = "write_float";
pub(super) const FUNCTION_PORT_READ_FLOAT: &str = "read_float";
pub(super) const FUNCTION_PORT_WRITE_NUMERIC: &str = "write_numeric";
pub(super) const FUNCTION_PORT_READ_NUMERIC: &str = "read_numeric";

/// Категория типа порта — определяет имя перечисления и набор функций.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub(super) enum PortClass {
    /// Однобитовый порт (`bit`, `bool`).
    Bit,
    /// Порт с плавающей точкой (`rational`).
    Rational,
    /// Числовой порт (`u8`, `u16`, массив битов и т. п.).
    Numeric,
}

impl PortClass {
    /// Суффикс C-перечисления для этой категории.
    pub(super) fn enum_name(self) -> &'static str {
        match self {
            PortClass::Bit => "BitPort",
            PortClass::Rational => "RationalPort",
            PortClass::Numeric => "NumericPort",
        }
    }

    /// Полное имя C-перечисления с направлением порта.
    /// Например: `ElevatorMini_In_BitPort`, `ElevatorMini_Out_NumericPort`.
    pub(super) fn qualified_enum_name_with_dir(
        self,
        root_camelcase: &str,
        dir: PortDirection,
    ) -> String {
        let dir_str = match dir {
            PortDirection::In => "In",
            PortDirection::Out => "Out",
            PortDirection::InOut => "InOut",
        };
        format!("{}_{}_{}", root_camelcase, dir_str, self.enum_name())
    }

    /// Определяет категорию по [`TypeNode`].
    pub(super) fn from_type(ty: &TypeNode) -> Self {
        match ty {
            TypeNode::Bit | TypeNode::Bool => PortClass::Bit,
            TypeNode::Rational => PortClass::Rational,
            _ => PortClass::Numeric,
        }
    }
}

/// Генератор C-кода для модели Lam.
///
/// Реализует трейт [`Generator`](crate::generator::Generator): принимает
/// корневой [`ModelNode`] и записывает пару файлов `.h`/`.c` по заданному пути.
pub struct Generator {}

impl AsGenerator for Generator {
    fn generate(
        &self,
        model: &ModelNode,
        output_path: &str,
        guard_enable: bool,
    ) -> Result<(), Diagnostic> {
        //TODO: При генерации следует работать с примитивным слепком модели
        let map = CMap::new(
            &*normalize_lowercase_snakecase(model.name().to_string()),
            model,
            guard_enable,
        )?;
        let header = generate_header(map.get_filename(), &map)?;
        let source = generate_source(map.get_filename(), &map)?;
        let filename = map.get_filename();
        let _ = fs::create_dir(Path::new(output_path));
        fs::write(
            Path::new(output_path).join(filename.to_owned() + ".h"),
            header,
        )
        .map_err(|e| {
            Diagnostic::warning(Location::Codegen, format!("{:?}", e)).with_code("CC-010")
        })?;
        fs::write(
            Path::new(output_path).join(filename.to_owned() + ".c"),
            source,
        )
        .map_err(|e| {
            Diagnostic::warning(Location::Codegen, format!("{:?}", e)).with_code("CC-010")
        })?;
        Ok(())
    }
}

pub fn get_c_type(typ: &TypeNode, model: &ModelNode) -> Option<String> {
    match typ {
        TypeNode::Bit => Some("int".to_string()),
        TypeNode::Bool => Some("bool".to_string()),
        TypeNode::Rational => Some("float".to_string()),
        TypeNode::Array(size, typ) => {
            if let TypeNode::Rational = **typ {
                Some("float *".to_string())
            } else {
                Some(format!("uint{}_t", *size))
            }
        }
        TypeNode::Unit => Some("void".to_string()),
        TypeNode::BuiltinString => Some("char *".to_string()),
        TypeNode::Struct(struct_name) => Some(struct_name.to_string()),
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
            let bits: u8 = if max > u32::MAX as i64 {
                64
            } else if max > u16::MAX as i64 {
                32
            } else if max > u8::MAX as i64 {
                16
            } else {
                8
            };
            Some(format!("uint{}_t", bits))
        }
        TypeNode::BuiltinModel
        | TypeNode::BuiltinState
        | TypeNode::BuiltinNumeric
        | TypeNode::Unsupported
        | TypeNode::Inference
        | TypeNode::Address(_, _) => None,
    }
}

pub(super) fn get_typed_variable(
    typ: &TypeNode,
    name: Option<String>,
    model: &ModelNode,
) -> Option<String> {
    match typ {
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
        _t => get_c_type(typ, model).map(|c_type| format!("{} {}", c_type, name.clone().unwrap())),
    }
}

#[cfg(test)]
mod tests {
    use crate::generator::c::c_map::CMap;
    use crate::generator::c::c_source::generate_source;
    use crate::{parse, semantic};

    const SRC: &str = r#"
type u8 = [bit;8];

in sensors_1: u8 = 0x100000000;
in sensors_2: u8 = 0x200000000;
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
        let _model = semantic::tree::construct_model(&model_ast, None, &[]).unwrap();
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
        let name = inner.name();
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
        let name = inner.name();
        assert!(!name.is_empty(), "структурное имя не должно быть пустым");
    }

    // ── Тесты имён констант и портов ─────────────────────────────────────────

    /// Константы и порты с ALL_CAPS-именами не разбиваются посимвольно.
    ///
    /// Регрессия: `normalize_lowercase_snakecase("MATRIX")` ранее давала
    /// `m_a_t_r_i_x`, что приводило к `CONST_..._M_A_T_R_I_X` вместо `CONST_..._MATRIX`.
    #[test]
    fn const_port_names_are_not_char_split() {
        // Константы используются в always, чтобы попасть в UsageSet.
        let src = r#"
type u8 = [bit;8];
const MATRIX: u8 = 0;
const NUMB: u8 = 255;
in SENSOR: u8 = 0x100000;
var v: u8 = 0;
start Main { always { v = MATRIX; v = NUMB; } }
        "#;
        let (model_ast, _) = parse(src, 0).unwrap();
        let model = semantic::tree::construct_model(&model_ast, None, &[]).unwrap();
        model.borrow_mut().name = Some("Main".to_string());
        let model = model.borrow();
        let map = CMap::new(model.name(), &*model, true).unwrap();
        let source = generate_source(map.get_filename(), &map).unwrap();
        assert!(
            source.contains("CONST_MAIN_MATRIX"),
            "ожидалось CONST_MAIN_MATRIX, получено:\n{source}"
        );
        assert!(
            source.contains("CONST_MAIN_NUMB"),
            "ожидалось CONST_MAIN_NUMB, получено:\n{source}"
        );
        // Порт теперь генерируется как enum в заголовочном файле — в .c его нет.
        assert!(
            !source.contains("PORT_MAIN_SENSOR"),
            "PORT_MAIN_SENSOR не должен присутствовать в .c-файле:\n{source}"
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
        let map = CMap::new(model.name(), &*model, true).unwrap();
        let source = generate_source(map.get_filename(), &map).unwrap();
        assert!(
            !source.contains(".h\" "),
            "#include не должен содержать пробел после кавычки:\n{source}"
        );
    }

    /// Вызов extern функции в блоке always генерируется в C-коде.
    #[test]
    fn test_extern_fn_call_in_always() {
        let src = r#"
type u8 = [bit;8];
extern fn log_val(v: u8);
model Counter {
    var x: u8 = 0;
    start Running {
        always { x = x + 1; log_val(x); }
    }
}
start Root = Counter;
"#;
        let (model_ast, _) = parse(src, 0).unwrap();
        let model_rc = semantic::tree::construct_model(&model_ast, None, &[]).unwrap();
        model_rc.borrow_mut().name = Some("main".to_string());
        let model = model_rc.borrow();
        let map = CMap::new(model.name(), &*model, true).unwrap();
        let source = generate_source(map.get_filename(), &map).unwrap();
        // Ищем именно вызов, а не декларацию — вызов не содержит "extern"
        let call_present = source
            .lines()
            .filter(|l| !l.contains("extern "))
            .any(|l| l.contains("log_val("));
        assert!(
            call_present,
            "вызов extern функции должен быть в генерированном коде (без 'extern'):\n{source}"
        );
    }

    /// Вызов extern функции после локальной переменной в блоке always генерируется.
    #[test]
    fn test_extern_fn_call_after_local_var_in_always() {
        let src = r#"
type u8 = [bit;8];
extern fn log_val(v: u8);
model Counter {
    var x: u8 = 0;
    start Running {
        always {
            var delta: u8 = 1;
            x = x + delta;
            log_val(x);
        }
    }
}
start Root = Counter;
"#;
        let (model_ast, _) = parse(src, 0).unwrap();
        let model_rc = semantic::tree::construct_model(&model_ast, None, &[]).unwrap();
        model_rc.borrow_mut().name = Some("main".to_string());
        let model = model_rc.borrow();
        let map = CMap::new(model.name(), &*model, true).unwrap();
        let source = generate_source(map.get_filename(), &map).unwrap();
        eprintln!("=== GENERATED ===\n{source}\n=== END ===");
        let call_present = source
            .lines()
            .filter(|l| !l.contains("extern "))
            .any(|l| l.contains("log_val("));
        assert!(
            call_present,
            "вызов extern функции после local var должен быть:\n{source}"
        );
    }

    #[test]
    fn test_guard_formula_codegen() {
        let src = r#"
            type u8 = [bit;8];
            var x: u8 = 0;
            :[Guard] x < 100;
            start Running {
                :[Guard] x >= 0;
                always {
                    x = x + 1;
                    :[Guard] x > 0;
                }
            }
        "#;
        let (model_ast, _) = parse(src, 0).unwrap();
        let model_rc = semantic::tree::construct_model(&model_ast, None, &[]).unwrap();
        model_rc.borrow_mut().name = Some("Main".to_string());
        let model = model_rc.borrow();

        // С включенными Guard-проверками
        let map_enabled = CMap::new(model.name(), &*model, true).unwrap();
        let source_enabled = generate_source(map_enabled.get_filename(), &map_enabled).unwrap();

        assert!(
            source_enabled.contains("assert(model->x < 100);"),
            "Отсутствует проверка формулы модели:\n{}",
            source_enabled
        );
        assert!(
            source_enabled.contains("assert(model->x >= 0);"),
            "Отсутствует проверка формулы состояния:\n{}",
            source_enabled
        );
        assert!(
            source_enabled.contains("assert(model->x > 0);"),
            "Отсутствует проверка встроенной формулы:\n{}",
            source_enabled
        );

        // С выключенными Guard-проверками
        let map_disabled = CMap::new(model.name(), &*model, false).unwrap();
        let source_disabled = generate_source(map_disabled.get_filename(), &map_disabled).unwrap();

        assert!(!source_disabled.contains("assert(model->x < 100);"));
        assert!(!source_disabled.contains("assert(model->x >= 0);"));
        assert!(!source_disabled.contains("assert(model->x > 0);"));
    }
}

/// Собирает имена моделей-зависимостей из элемента StateExtend.
pub fn collect_extend_model_deps(extend: &StateExtend, deps: &mut Vec<String>) {
    match extend {
        StateExtend::None => {}
        StateExtend::Model(name) => deps.push(name.unique().to_string()),
        StateExtend::Concatenation(items) | StateExtend::Parallel(items) => {
            for item in items {
                collect_extend_model_deps(item, deps);
            }
        }
    }
}

/// Рекурсивный DFS для топологической сортировки моделей.
pub fn topo_dfs(
    key: &str,
    by_name: &HashMap<String, Element>,
    deps_map: &HashMap<String, Vec<String>>,
    visited: &mut HashSet<String>,
    result: &mut Vec<Element>,
) {
    if visited.contains(key) {
        return;
    }
    visited.insert(key.to_string());
    // Сначала рекурсивно обрабатываем зависимости
    if let Some(deps) = deps_map.get(key) {
        for dep in deps.clone() {
            topo_dfs(&dep, by_name, deps_map, visited, result);
        }
    }
    // Затем добавляем текущую модель
    if let Some(elem) = by_name.get(key) {
        result.push(elem.clone());
    }
}

/// Топологически сортирует список моделей так, чтобы зависимости шли первыми.
///
/// Модель A зависит от B, если одно из её состояний расширяет B (`StateExtend::Model`).
/// Алгоритм: обход в глубину (DFS) с постфиксным добавлением в результат.
/// Нет гарантий порядка одноуровневых вершин — нужен только частичный порядок.
pub fn topological_sort_models(map: &CMap, models: Vec<Element>) -> Vec<Element> {
    // Фаза 1: строим карту unique_name → Element
    let mut by_name: HashMap<String, Element> = HashMap::new();
    for elem in models {
        if let Element::Model { name, .. } = &elem {
            by_name.insert(name.unique().to_string(), elem);
        }
    }

    // Фаза 2: строим граф зависимостей (только зависимости из нашего набора моделей)
    let mut deps_map: HashMap<String, Vec<String>> = HashMap::new();
    let keys: Vec<String> = by_name.keys().cloned().collect();
    for key in &keys {
        if let Some(Element::Model { states, .. }) = by_name.get(key) {
            let mut deps = Vec::new();
            for state_name in states.clone() {
                if let Some(Element::StateExtend { extend, .. }) = map.state_at(state_name) {
                    collect_extend_model_deps(&extend, &mut deps);
                }
            }
            // Отбрасываем зависимости, которых нет в нашем наборе
            deps.retain(|d| by_name.contains_key(d.as_str()));
            deps_map.insert(key.clone(), deps);
        }
    }

    // Фаза 3: топологический обход (DFS)
    let mut visited = HashSet::new();
    let mut result = Vec::new();
    for key in &keys {
        topo_dfs(key, &by_name, &deps_map, &mut visited, &mut result);
    }
    result
}
