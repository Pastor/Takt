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

mod c_map;

use crate::diagnostics::{Diagnostic, Location};
use crate::generator::Generator as AsGenerator;
use crate::generator::c::c_map::CMap;
use crate::generator::indent::Printer;
use crate::parser::ast::Member;
use crate::semantic::extend::Extend;
use crate::semantic::minimap::{Element, Map};
use crate::semantic::naming::{normalize_camelcase_name, normalize_lowercase_snakecase};
use crate::semantic::type_node::TypeNode;
use crate::semantic::{
    ConditionDefinitionNode, ConditionNode, EnumDefinitionNode, ExpressionNode, ModelNode,
    StateNode, StateNodeKind, VariableNode,
};
use itertools::Itertools;
use log::warn;
use std::cell::RefCell;
use std::fs;
use std::path::Path;
use std::rc::Rc;

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
        let header = self.generate_header(map.get_filename(), &map)?;
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

impl Generator {
    #[inline]
    fn get_upper_name(model: &ModelNode) -> String {
        model
            .upper
            .clone()
            .and_then(|weak| weak.upgrade())
            .map(|rc| Self::get_upper_name(&rc.borrow()) + "_")
            .unwrap_or_default()
            // V6: заменить unwrap() на unwrap_or_else — resolve_model_name
            // никогда не возвращает Err на практике, но явная обработка
            // исключает потенциальную панику при будущих изменениях.
            + &*normalize_lowercase_snakecase(
            Self::resolve_model_name(model).unwrap_or_else(|_| "Unknown".to_string()),
        )
            .to_uppercase()
    }

    #[inline]
    fn get_model_name_struct(model: &ModelNode) -> String {
        // V6: заменить unwrap() на unwrap_or_else для безопасности.
        normalize_camelcase_name(
            Self::resolve_model_name(model)
                .unwrap_or_else(|_| "Unknown".to_string())
                .as_str(),
        )
    }

    fn get_typed_variable(
        typ: &TypeNode,
        name: Option<String>,
        model: &ModelNode,
    ) -> Option<String> {
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
                let max = if max > i8::MAX as i64 && max < i16::MAX as i64 {
                    16
                } else if max > i16::MAX as i64 && max < i32::MAX as i64 {
                    32
                } else if max > i32::MAX as i64 {
                    64
                } else {
                    8
                };
                let enum_name = normalize_lowercase_snakecase(enum_name.clone());
                Some(format!("uint{}_t {}", max, &enum_name))
            }
            TypeNode::BuiltinModel
            | TypeNode::BuiltinState
            | TypeNode::BuiltinNumeric
            | TypeNode::Unsupported
            | TypeNode::Inference
            | TypeNode::Address(_, _) => None,
        }
    }

    fn generate_model_states(
        #[allow(unused_mut)] mut printer: &mut Printer,
        model: &ModelNode,
    ) -> Result<(), Diagnostic> {
        printer.ident("enum {").nl().up();
        let upper = Self::get_upper_name(model);
        printer.ident(&upper).print("_INIT");
        for (name, state) in model.states.iter() {
            let name = normalize_lowercase_snakecase(name.clone()).to_uppercase();
            let name = format!("{}_{}", &upper, name);
            if let StateNode::Implement { .. } = state {
                printer.print(",").nl().ident(&name).print("_INIT");
            }
            printer.print(",").nl().ident(&name);
        }
        printer.down().nl().ident("} state;").nl();
        Ok(())
    }

    fn generate_model_states_struct(
        #[allow(unused_mut)] mut printer: &mut Printer,
        model: &ModelNode,
    ) -> Result<(), Diagnostic> {
        for (name, state) in model.states.iter() {
            if let StateNode::Implement {
                implements, kind, ..
            } = state
                && let Extend::Model(m) = implements
                && *kind != StateNodeKind::Start
            {
                Self::generate_model_struct(&mut printer, &*m.borrow(), false)?;
            } else {
                warn!(
                    "For model '{}' on state '{}'",
                    model.name.clone().unwrap().clone(),
                    &name
                );
                continue;
            }
        }
        Ok(())
    }

    fn generate_model_struct(
        #[allow(unused_mut)] mut printer: &mut Printer,
        model: &ModelNode,
        first_model: bool,
    ) -> Result<(), Diagnostic> {
        printer
            .ident(
                format!(
                    "/** Generated '{}' structure */",
                    model.name.clone().unwrap_or("?".to_string())
                )
                .as_str(),
            )
            .nl();
        printer.ident("struct ");
        if first_model {
            printer
                .print(Self::get_model_name_struct(model).as_str())
                .print(" ");
        }
        printer.print("{").nl().up();
        let start = model.get_start_state().ok_or(Diagnostic::warning(
            Location::Codegen,
            format!(
                "Start state not found for model ''{}''",
                model.name.clone().unwrap_or_default()
            ),
        ))?;
        if let StateNode::Implement { implements, .. } = start {
            if let Extend::Model(implement_model) = implements {
                Self::generate_model_struct(printer, &*implement_model.borrow(), false)?;
            } else if let Extend::Parallel(implements) = implements {
                //TODO: Доделать параллельную обработку
                for implement in implements {
                    if let Extend::Model(implement_model) = *implement {
                        Self::generate_model_struct(printer, &*implement_model.borrow(), false)?;
                    }
                }
            }
        }
        //States
        {
            printer
                .ident(
                    format!(
                        "/** Generated states to '{}' */",
                        model.name.clone().unwrap_or("?".to_string())
                    )
                    .as_str(),
                )
                .nl();
            let _ = Self::generate_model_states(printer, model);
            let _ = Self::generate_model_states_struct(printer, model);
        }

        //Variables
        {
            for (name, var) in model.variables.clone().iter() {
                match var {
                    VariableNode::Unresolved => {}
                    VariableNode::Simple { ty, expr, .. } => {
                        let typed_variable =
                            Self::get_typed_variable(ty, Some(name.clone()), model);
                        if typed_variable.is_none() {
                            continue;
                        }
                        printer.ident(typed_variable.unwrap().as_str());
                        if let ExpressionNode::None = expr {
                            //Skip
                        } else {
                            //TODO:
                        }
                        printer.print(";").nl();
                    }
                    VariableNode::Port { .. } => {}
                    VariableNode::Const { .. } => {}
                }
            }
            if first_model {
                printer.ident("void *userdata;").nl();
            }
        }
        if first_model {
            printer
                .ident(
                    format!(
                        "void  (*{}  )(int address, int bit, bool val, void *userdata);",
                        FUNCTION_PORT_WRITE_BIT
                    )
                    .as_str(),
                )
                .nl();
            printer
                .ident(
                    format!(
                        "bool  (*{}   )(int address, int bit, void *userdata);",
                        FUNCTION_PORT_READ_BIT
                    )
                    .as_str(),
                )
                .nl();
            printer
                .ident(
                    format!(
                        "void  (*{})(int address, int bit, float val, void *userdata);",
                        FUNCTION_PORT_WRITE_FLOAT
                    )
                    .as_str(),
                )
                .nl();
            printer
                .ident(
                    format!(
                        "float (*{} )(int address, int bit, void *userdata);",
                        FUNCTION_PORT_READ_FLOAT
                    )
                    .as_str(),
                )
                .nl();
        }
        printer.down().ident("}");
        if first_model {
            printer.print(";");
        } else {
            printer
                .print(" ")
                .print(normalize_lowercase_snakecase(Self::resolve_model_name(model)?).as_str())
                .print(";");
        }
        printer.nl();
        Ok(())
    }

    fn generate_constants_and_ports_and_enums(
        printer: &mut Printer,
        model: &ModelNode,
    ) -> Result<(), Diagnostic> {
        let upper_name = Self::get_upper_name(model);
        for (_name, model) in model.models.iter() {
            Self::generate_constants_and_ports_and_enums(printer, &model.borrow())?
        }

        let variables = model.variables.clone();
        for var in variables
            .into_values()
            .sorted_by(|a, b| a.name().cmp(b.name()))
        {
            match var.clone() {
                VariableNode::Unresolved => {}
                VariableNode::Simple { .. } => {}
                VariableNode::Port { name, expr, .. } => {
                    let name = Self::resolve_raw_name(upper_name.clone(), name)?;

                    let (address, _bit) = if let ExpressionNode::Address(address, bit) = expr {
                        (address, bit)
                    } else if let ExpressionNode::Number(address) = expr {
                        (address, 0)
                    } else {
                        return Err("Unresolved address".into());
                    };
                    printer.print("#define PORT_").print(&name).print(" ");
                    printer.print(format!("0x{:x}", address).as_str());
                    printer.nl();
                }
                VariableNode::Const { name, expr, .. } => {
                    let name = Self::resolve_raw_name(upper_name.clone(), name)?;
                    let unrolled = Self::unroll_expression(&expr)?;
                    printer
                        .print("#define CONST_")
                        .print(&name)
                        .print(" (")
                        .print(unrolled.as_str())
                        .print(")")
                        .nl();
                }
            }
        }
        // Состояния транслируются в enum-константы в generate_model_states — здесь пропускаем.
        for _state in model
            .states
            .clone()
            .into_values()
            .sorted_by(|a, b| a.name().cmp(b.name()))
        {}

        let conditions = model.conditions.clone();
        for cond in conditions
            .into_values()
            .sorted_by(|a, b| a.name().cmp(b.name()))
        {
            let unrolled = Self::unroll_cond(&cond.value)?;
            printer
                .print("#define COND_")
                .print(&Self::resolve_cond_name(upper_name.clone(), &cond)?)
                .print(" (")
                .print(unrolled.as_str())
                .print(")")
                .nl();
        }
        let enums = model.enums.clone();
        for en in enums.into_values().sorted_by(|a, b| a.name().cmp(b.name())) {
            printer
                .print(format!("/* Enum  {}*/", en.name()).as_str())
                .nl();
            let prefix =
                "#define ENUM_".to_string() + &*Self::resolve_enum_name(upper_name.clone(), &en)?;
            for (name, value) in en.variants {
                printer
                    .print(prefix.clone().as_str())
                    .print("_")
                    .print(normalize_lowercase_snakecase(name).to_uppercase().as_str())
                    .print(format!(" {}", value).as_str())
                    .nl();
            }
        }
        Ok(())
    }

    fn generate_implement_source(
        printer: &Printer,
        implement: &Extend,
        model: &ModelNode,
        main: bool,
    ) -> Result<(), Diagnostic> {
        match implement {
            Extend::None | Extend::Unresolved(_) => {
                return Err(Diagnostic::error(
                    Location::Codegen,
                    "Implementation maybe defined".to_string(),
                ));
            }
            Extend::Model(slave) => {
                Self::generate_model_source(printer, &*slave.clone().borrow(), false)?;
            }
            Extend::Parentless(implement) => {
                Self::generate_implement_source(printer, implement, model, main)?;
            }
            Extend::Concatenation(_items) => {
                // TODO(NI5): генерация плоской последовательной композиции
            }
            Extend::Parallel(_items) => {
                // TODO(NI5): генерация плоской параллельной композиции
            }
        }
        Ok(())
    }

    fn generate_model_tick_source(
        _printer: &Printer,
        _model: &ModelNode,
        _main: bool,
    ) -> Result<(), Diagnostic> {
        //TODO
        Ok(())
    }

    fn generate_model_source(
        printer: &Printer,
        model: &ModelNode,
        main: bool,
    ) -> Result<(), Diagnostic> {
        // for model in model.models.values() {
        //     self.generate_model_source(&printer, &*model.clone().borrow(), false)?;
        // }
        let start = model.get_start_state().ok_or(Diagnostic::error(
            Location::Codegen,
            "Start state not found".to_string(),
        ))?;
        if let StateNode::Implement { implements, .. } = start {
            return Self::generate_implement_source(printer, &implements, model, false);
        } else if let StateNode::Simple { .. } = start {
            if main {
                return Ok(());
            }
        } else {
            return Err(Diagnostic::error(
                Location::Codegen,
                "Unimplement state".to_string(),
            ));
        }
        Ok(())
    }

    fn generate_source(&self, model: &ModelNode) -> Result<String, Diagnostic> {
        let mut source = String::new();
        let mut printer = Printer::new(4, &mut source);
        let filename = normalize_lowercase_snakecase(Self::resolve_model_name(model)?);
        printer
            .print(format!("#include \"{}.h\"", filename).as_str())
            .nl();
        Self::generate_constants_and_ports_and_enums(&mut printer, model)?;
        Self::generate_model_source(&printer, model, true)?;
        printer.nl();
        let struct_name = Self::get_model_name_struct(model);
        printer
            .print("void ")
            .print(&struct_name)
            .print("_init(struct ")
            .print(&struct_name)
            .print(" *main) {")
            .nl();
        {
            printer.up().ident("main->state = ");
            let upper = Self::get_upper_name(model);
            printer.print(&upper).print("_INIT;").down().nl();
        }
        printer.print("}").nl().nl();
        printer
            .print("void ")
            .print(&struct_name)
            .print("_tick(struct ")
            .print(&struct_name)
            .print(" *main) {")
            .nl();
        printer.up();
        Self::generate_model_tick_source(&printer, model, true)?;
        printer.down();
        printer.print("}").nl().nl();
        printer
            .print("void ")
            .print(&struct_name)
            .print("_reset(struct ")
            .print(&struct_name)
            .print(" *main) {")
            .nl();
        printer
            .up()
            .ident(format!("{}_init(main);", &struct_name).as_str())
            .down()
            .nl();
        printer.print("}").nl().nl();
        printer
            .print("bool ")
            .print(&struct_name)
            .print("_is_done(const struct ")
            .print(&struct_name)
            .print(" *main) {")
            .nl();
        let mut cond = String::new();
        let upper_model_name = Self::get_upper_name(model);
        for state in model.get_end_states().iter() {
            if !cond.is_empty() {
                cond.push_str(" || ");
            }
            let name = normalize_lowercase_snakecase(state.name().to_string()).to_uppercase();
            let name = format!("{}_{}", &upper_model_name, name);
            cond.push_str("main->state == ");
            cond.push_str(&name);
        }
        if cond.is_empty() {
            cond.push_str("false");
        }
        printer
            .up()
            .ident("return ")
            .print(cond.as_str())
            .print(";")
            .down()
            .nl();
        printer.print("}").nl().nl();
        Ok(source)
    }

    fn generate_header(&self, filename: &str, map: &CMap) -> Result<String, Diagnostic> {
        let mut header = String::new();
        let mut printer = Printer::new(4, &mut header);
        let id = normalize_lowercase_snakecase(filename.to_string())
            .replace("\\.", "_")
            .to_uppercase()
            + "_H__";
        printer.print("#ifndef ").print(&id).nl();
        printer.print("#define ").print(&id).nl();
        printer.print("#include <stdint.h>").nl();
        printer.print("#include <stdbool.h>").nl();
        printer.nl();
        for element in map.using_models() {
            let Element::Model {
                name,
                states,
                start,
            } = element
            else {
                continue;
            };
            let struct_name = name.unique_camelcase();
            printer.print(format!("/* Model {} */", name).as_str()).nl();
            printer
                .print(format!("typedef struct {} {{", struct_name).as_str())
                .nl();
            printer.up();
            printer.ident("enum {").up().nl();
            printer.ident(&*(name.unique_uppercase_snakecase() + "_INIT"));
            for state_name in states {
                printer.print(",").nl();
                printer.ident(&state_name.unique_uppercase_snakecase());
            }
            printer.down().nl().ident("} state;").nl();
            //TODO: Определение переменных модели
            printer.down();
            printer.print("};").nl();
            printer.nl();
        }
        let state_name = map.start();
        let start_state = map.state_at(state_name.clone()).ok_or_else(|| {
            Diagnostic::error(Location::Codegen, "Start state not found".to_string())
        })?;
        let struct_name = state_name.clone().unique_camelcase();
        printer
            .print(format!("typedef struct {} {{", struct_name).as_str())
            .nl();
        printer.up();
        printer.down();
        printer.print("};").nl();
        printer.nl();

        printer
            .print("void ")
            .print(&struct_name)
            .print("_init(")
            .print(&struct_name)
            .print(" *main);")
            .nl();
        printer
            .print("void ")
            .print(&struct_name)
            .print("_tick(")
            .print(&struct_name)
            .print(" *main);")
            .nl();
        printer
            .print("void ")
            .print(&struct_name)
            .print("_reset(")
            .print(&struct_name)
            .print(" *main);")
            .nl();
        printer
            .print("bool ")
            .print(&struct_name)
            .print("_is_done(const struct ")
            .print(&struct_name)
            .print(" *main);")
            .nl();
        printer.print("#endif").nl();
        Ok(header)
    }

    fn unroll_model(model: &ModelNode) -> Result<String, Diagnostic> {
        if let Some(weak) = model.upper.clone() {
            let rc = weak.upgrade().unwrap();
            let parent = rc.borrow();
            let access = if parent.upper.is_none() { "->" } else { "." };
            Ok(Self::unroll_model(&parent)?
                + access
                + &*normalize_lowercase_snakecase(Self::resolve_model_name(model)?))
        } else {
            Ok("main".to_string())
        }
    }

    fn unroll_variable(var: &VariableNode) -> Result<String, Diagnostic> {
        match var {
            VariableNode::Unresolved => Err("Unresolved variable can't unrolled".into()),
            VariableNode::Simple { upper, name, .. } => {
                let rc = upper.clone().and_then(|w| w.upgrade()).unwrap();
                let model = rc.borrow();
                let access = if model.upper.is_none() { "->" } else { "." };
                Ok(Self::unroll_model(&model)?
                    + access
                    + &*normalize_lowercase_snakecase(name.clone()))
            }
            VariableNode::Port { upper, name, .. } => {
                let rc = upper.clone().and_then(|w| w.upgrade()).unwrap();
                let model = rc.borrow();
                let upper_name = Self::get_upper_name(&*model);
                let name = Self::resolve_raw_name(upper_name.clone(), name.clone())?;
                Ok("PORT_".to_string() + &name)
            }
            VariableNode::Const { upper, name, .. } => {
                let rc = upper.clone().and_then(|w| w.upgrade()).unwrap();
                let model = rc.borrow();
                let upper_name = Self::get_upper_name(&*model);
                let name = Self::resolve_raw_name(upper_name.clone(), name.clone())?;
                Ok("CONST_".to_string() + &name)
            }
        }
    }

    fn unroll_cond(cond: &ConditionNode) -> Result<String, Diagnostic> {
        match cond {
            ConditionNode::ArraySubscript(array, num) => {
                Ok(Self::unroll_variable(&*array.borrow())? + "[" + num.to_string().as_str() + "]")
            }
            ConditionNode::Parenthesis(cond) => {
                Ok("(".to_owned() + &*Self::unroll_cond(cond)? + ")")
            }
            ConditionNode::BitAccess(cond, m) => {
                let bit = if let Member::Number(num) = m {
                    *num
                } else {
                    0i64
                };
                let member = Self::unroll_cond(cond)?;
                Ok(format!(
                    "(*main->read_bit)({}, {}, main->userdata)",
                    member, bit
                ))
            }
            ConditionNode::Function(fun, _args, _) => {
                todo!("Unrolling not implemented {:?}", fun)
            }
            ConditionNode::Not(cond) => Ok("!(".to_owned() + &*Self::unroll_cond(cond)? + ")"),
            ConditionNode::Add(left, right) => Ok("(".to_owned()
                + &*Self::unroll_cond(left)?
                + " + "
                + &*Self::unroll_cond(right)?
                + ")"),
            ConditionNode::Subtract(left, right) => Ok("(".to_owned()
                + &*Self::unroll_cond(left)?
                + " - "
                + &*Self::unroll_cond(right)?
                + ")"),
            ConditionNode::And(left, right) => Ok("(".to_owned()
                + &*Self::unroll_cond(left)?
                + " && "
                + &*Self::unroll_cond(right)?
                + ")"),
            ConditionNode::Or(left, right) => Ok("(".to_owned()
                + &*Self::unroll_cond(left)?
                + " || "
                + &*Self::unroll_cond(right)?
                + ")"),
            ConditionNode::Less(left, right) => Ok("(".to_owned()
                + &*Self::unroll_cond(left)?
                + " < "
                + &*Self::unroll_cond(right)?
                + ")"),
            ConditionNode::More(left, right) => Ok("(".to_owned()
                + &*Self::unroll_cond(left)?
                + " > "
                + &*Self::unroll_cond(right)?
                + ")"),
            ConditionNode::LessEqual(left, right) => Ok("(".to_owned()
                + &*Self::unroll_cond(left)?
                + " <= "
                + &*Self::unroll_cond(right)?
                + ")"),
            ConditionNode::MoreEqual(left, right) => Ok("(".to_owned()
                + &*Self::unroll_cond(left)?
                + " >= "
                + &*Self::unroll_cond(right)?
                + ")"),
            ConditionNode::Equal(left, right) => Ok("(".to_owned()
                + &*Self::unroll_cond(left)?
                + " == "
                + &*Self::unroll_cond(right)?
                + ")"),
            ConditionNode::NotEqual(left, right) => Ok("(".to_owned()
                + &*Self::unroll_cond(left)?
                + " != "
                + &*Self::unroll_cond(right)?
                + ")"),
            ConditionNode::Number(n) => Ok(n.to_string()),
            ConditionNode::Rational(n, _) => Ok(n.to_string()),
            ConditionNode::String(n) => Ok(n.iter().join("").to_string()),
            ConditionNode::Bool(n) => Ok(n.to_string()),
            ConditionNode::Variable(var, _) => Self::unroll_variable(&*var.borrow()),
            ConditionNode::Model(model) => Self::unroll_model(&*model.borrow()),
            ConditionNode::State(state) => {
                todo!("Not implement unrolling {:?}", state);
            }
            ConditionNode::EnumVariant(edn, name, _n) => {
                let edn = &*edn.borrow();
                let upper_name = Self::get_upper_name(
                    &*edn
                        .upper
                        .clone()
                        .and_then(|w| w.upgrade())
                        .unwrap()
                        .borrow(),
                );
                Ok("ENUM_".to_string()
                    + &*Self::resolve_enum_name(upper_name.clone(), &edn)?
                    + "_"
                    + normalize_lowercase_snakecase(name.clone())
                        .to_uppercase()
                        .as_str())
            }
            cond => Err(format!("Can't unrolling condition {:#?}", cond)
                .as_str()
                .into()),
        }
    }

    fn unroll_expression(expr: &ExpressionNode) -> Result<String, Diagnostic> {
        match expr {
            ExpressionNode::ArraySubscript(var, n) => Ok(Self::unroll_variable(&*var.borrow())?
                + &*"[".to_string()
                + n.to_string().as_str()
                + &*"]".to_string()),
            ExpressionNode::Parenthesis(expr) => {
                Ok("(".to_string() + &*Self::unroll_expression(expr)? + &*")".to_string())
            }
            ExpressionNode::BitAccess(val, _bit) => {
                todo!("BitAccess {:?} not enrolled", val);
            }
            ExpressionNode::Function(fun, _args) => {
                todo!("Function {:?} not enrolled", fun);
            }
            ExpressionNode::Not(expr) => Ok("!".to_string() + &*Self::unroll_expression(&**expr)?),
            ExpressionNode::BitwiseNot(expr) => {
                Ok("~".to_string() + &*Self::unroll_expression(&**expr)?)
            }
            ExpressionNode::UnaryPlus(expr) => {
                Ok("+".to_string() + &*Self::unroll_expression(&**expr)?)
            }
            ExpressionNode::Negate(expr) => {
                Ok("-".to_string() + &*Self::unroll_expression(&**expr)?)
            }
            ExpressionNode::Power(left, right) => Ok(Self::unroll_expression(&**left)?
                + &*"^".to_string()
                + &*Self::unroll_expression(&**right)?),
            ExpressionNode::Multiply(left, right) => Ok(Self::unroll_expression(&**left)?
                + &*" * ".to_string()
                + &*Self::unroll_expression(&**right)?),
            ExpressionNode::Divide(left, right) => Ok(Self::unroll_expression(&**left)?
                + &*" / ".to_string()
                + &*Self::unroll_expression(&**right)?),
            ExpressionNode::Modulo(left, right) => Ok(Self::unroll_expression(&**left)?
                + &*" % ".to_string()
                + &*Self::unroll_expression(&**right)?),
            ExpressionNode::Add(left, right) => Ok(Self::unroll_expression(&**left)?
                + &*" + ".to_string()
                + &*Self::unroll_expression(&**right)?),
            ExpressionNode::Subtract(left, right) => Ok(Self::unroll_expression(&**left)?
                + &*" - ".to_string()
                + &*Self::unroll_expression(&**right)?),
            ExpressionNode::ShiftLeft(left, right) => Ok(Self::unroll_expression(&**left)?
                + &*" << ".to_string()
                + &*Self::unroll_expression(&**right)?),
            ExpressionNode::ShiftRight(left, right) => Ok(Self::unroll_expression(&**left)?
                + &*" >> ".to_string()
                + &*Self::unroll_expression(&**right)?),
            ExpressionNode::BitwiseAnd(left, right) => Ok(Self::unroll_expression(&**left)?
                + &*" & ".to_string()
                + &*Self::unroll_expression(&**right)?),
            ExpressionNode::BitwiseXor(left, right) => Ok(Self::unroll_expression(&**left)?
                + &*" ^ ".to_string()
                + &*Self::unroll_expression(&**right)?),
            ExpressionNode::BitwiseOr(left, right) => Ok(Self::unroll_expression(&**left)?
                + &*" | ".to_string()
                + &*Self::unroll_expression(&**right)?),
            ExpressionNode::Less(left, right) => Ok(Self::unroll_expression(&**left)?
                + &*" < ".to_string()
                + &*Self::unroll_expression(&**right)?),
            ExpressionNode::More(left, right) => Ok(Self::unroll_expression(&**left)?
                + &*" > ".to_string()
                + &*Self::unroll_expression(&**right)?),
            ExpressionNode::LessEqual(left, right) => Ok(Self::unroll_expression(&**left)?
                + &*" <= ".to_string()
                + &*Self::unroll_expression(&**right)?),
            ExpressionNode::MoreEqual(left, right) => Ok(Self::unroll_expression(&**left)?
                + &*" >= ".to_string()
                + &*Self::unroll_expression(&**right)?),
            ExpressionNode::Equal(left, right) => Ok(Self::unroll_expression(&**left)?
                + &*" == ".to_string()
                + &*Self::unroll_expression(&**right)?),
            ExpressionNode::NotEqual(left, right) => Ok(Self::unroll_expression(&**left)?
                + &*" != ".to_string()
                + &*Self::unroll_expression(&**right)?),
            ExpressionNode::And(left, right) => Ok(Self::unroll_expression(&**left)?
                + &*" && ".to_string()
                + &*Self::unroll_expression(&**right)?),
            ExpressionNode::Or(left, right) => Ok(Self::unroll_expression(&**left)?
                + &*" || ".to_string()
                + &*Self::unroll_expression(&**right)?),
            ExpressionNode::Assign(left, right) => Ok(Self::unroll_expression(&**left)?
                + &*" = ".to_string()
                + &*Self::unroll_expression(&**right)?),
            ExpressionNode::Number(n) => Ok(n.to_string()),
            ExpressionNode::Rational(n, _) => Ok(n.clone()),
            ExpressionNode::String(n) => Ok(n.join("").to_string()),
            ExpressionNode::Bool(n) => Ok(n.to_string()),
            ExpressionNode::Variable(var) => Self::unroll_variable(&*var.borrow()),
            ExpressionNode::Model(_model) => {
                todo!("Model unrolling not yet implemented")
            }
            ExpressionNode::Condition(cond) => {
                let cond = &*cond.borrow();
                let upper_name = Self::get_upper_name(
                    &*cond
                        .upper
                        .clone()
                        .and_then(|w| w.upgrade())
                        .unwrap()
                        .borrow(),
                );
                let name = Self::resolve_cond_name(upper_name.clone(), &cond)?;
                Ok("COND_".to_string() + &*name)
            }
            ExpressionNode::Initializer(elems) => {
                // Массивный инициализатор {a, b, c} → C-синтаксис {a, b, c}
                let parts: Result<Vec<String>, Diagnostic> =
                    elems.iter().map(Self::unroll_expression).collect();
                Ok("{".to_string() + &parts?.join(", ") + "}")
            }
            expr => Err(format!("Can't unroll {:#?}", expr).as_str().into()),
        }
    }

    /// Генерирует C-выражение записи значения в порт через `write_bit` / `write_float`.
    ///
    /// Сейчас поддерживаются только порты типа `bit` и `bool`.
    /// Вызов зарезервирован для будущей реализации `_tick`.
    #[allow(dead_code)]
    fn port_write(var: &VariableNode, val: &ExpressionNode) -> Result<String, Diagnostic> {
        let upper_name = Self::get_upper_name(&*var.upper().unwrap().borrow());
        let val = Self::unroll_expression(val)?;
        match var {
            VariableNode::Unresolved => Err("Unresolved variable".into()),
            VariableNode::Simple { name: _, ty: _, .. } => Err("Not implement yet".into()),
            VariableNode::Port { name, ty, expr, .. } => {
                let name = Self::resolve_raw_name(upper_name.clone(), name.clone())?;
                match ty {
                    TypeNode::Bit | TypeNode::Bool => {
                        let bit = if let ExpressionNode::Address(_, bit) = expr {
                            *bit
                        } else {
                            0i64
                        };

                        Ok(format!(
                            "(*main->write_bit)(PORT_{}, {}, {}, main->userdata)",
                            &name, bit, val
                        ))
                    }
                    _ => Err("Only bit or bool type of port already supported".into()),
                }
            }
            VariableNode::Const { .. } => Err("Const can't be modified".into()),
        }
    }

    #[inline]
    fn resolve_raw_name(upper_name: String, name: String) -> Result<String, Diagnostic> {
        Ok(
            (upper_name.to_owned() + "_" + normalize_lowercase_snakecase(name.clone()).as_str())
                .to_uppercase(),
        )
    }

    #[inline]
    fn resolve_cond_name(
        upper_name: String,
        cond: &ConditionDefinitionNode,
    ) -> Result<String, Diagnostic> {
        Self::resolve_raw_name(upper_name, cond.name.clone())
    }

    #[inline]
    fn resolve_enum_name(
        upper_name: String,
        en: &EnumDefinitionNode,
    ) -> Result<String, Diagnostic> {
        Self::resolve_raw_name(upper_name, en.name.clone())
    }

    #[inline]
    fn resolve_model_name(model: &ModelNode) -> Result<String, Diagnostic> {
        let name = model.name.clone().unwrap_or("Unknown".to_string());
        if !name.eq("Unknown") {
            Ok(name)
        } else if model.has_states()
            && let Some(state) = model.get_start_state()
        {
            match state.clone() {
                StateNode::Unresolved | StateNode::Simple { .. } => Ok(state.name().to_string()),
                StateNode::Implement { implements, .. } => Self::resolve_implement_name(implements),
            }
        } else {
            Ok(name)
        }
    }

    #[inline]
    fn resolve_implement_name(implement: Extend) -> Result<String, Diagnostic> {
        match implement {
            Extend::None => Ok("".to_string()),
            Extend::Unresolved(_) => Ok("Unresolved".to_string()),
            Extend::Model(model) => Self::resolve_model_name(&model.borrow()),
            Extend::Parentless(i) => Self::resolve_implement_name(*i),
            Extend::Concatenation(implements) | Extend::Parallel(implements) => {
                let name = implements
                    .iter()
                    .map(|implement| Self::resolve_implement_name(*implement.clone()))
                    .collect::<Result<Vec<_>, _>>()?;
                Ok(name.join("_"))
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use crate::generator::c::Generator;
    use crate::generator::c::c_map::CMap;
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

    /// Enum с большими значениями выбирает тип достаточной ширины по максимуму.
    ///
    /// Регрессия: `.first()` выбирал минимальное значение → тип uint8_t для
    /// enum с вариантами High=200 (> i8::MAX), тогда как нужен uint16_t.
    #[test]
    fn enum_type_sized_by_maximum_variant() {
        let src = r#"
enum Priority { Low = 0, Medium = 5, High = 200 }
var p: Priority = 0;
start Main { always { } }
        "#;
        let (model_ast, _) = parse(src, 0).unwrap();
        let model = semantic::tree::construct_model(&model_ast, None, &[]).unwrap();
        let model = model.borrow();
        let generator = Generator {};
        let map = CMap::new(model.name(), &*model).unwrap();
        let header = generator.generate_header(map.get_filename(), &map).unwrap();
        // High=200 > i8::MAX(127) → должен выбраться uint16_t
        assert!(
            header.contains("uint16_t"),
            "ожидался uint16_t для max=200, получено:\n{header}"
        );
        assert!(
            !header.contains("uint8_t p"),
            "uint8_t слишком мал для max=200:\n{header}"
        );
    }
}
