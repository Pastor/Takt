//! Генерация исходного C-файла (`.c`) из семантического дерева BuT.
//!
//! Содержит все функции генерации `.c`-исходника:
//! [`Generator::generate_source`], вспомогательные `unroll_*`, `resolve_*`,
//! `generate_model_*` и утилиты именования.
//!
//! ## Состояние реализации
//!
//! Генерация `.c`-файлов отложена до реализации задач I1–I4 (тела функций
//! `_init`, `_tick`, `_reset`). Код перенесён сюда из `mod.rs` и готов
//! к дальнейшей доработке.

use super::Generator;
use crate::diagnostics::Diagnostic;
use crate::generator::c::c_map::CMap;
use crate::generator::indent::Printer;
use crate::semantic::minimap::Element;
use crate::semantic::naming::normalize_lowercase_snakecase;
use crate::semantic::{ConditionDefinitionNode, EnumDefinitionNode, ExpressionNode, VariableNode};
use log::error;

/// Генерирует содержимое `.c`-файла для модели.
///
/// Не вызывается напрямую: генерация `.c`-файлов отложена до реализации I1–I4.
/// Код сохранён для будущей доработки.
#[allow(dead_code, deprecated)]
pub(super) fn generate_source(filename: &str, map: &CMap) -> Result<String, Diagnostic> {
    let mut source = String::new();
    let mut printer = Printer::new(4, &mut source);
    printer
        .print(format!("#include \"{}.h\"", filename).as_str())
        .nl();
    generate_constants_and_ports_and_enums(&mut printer, map)?;
    // Self::generate_model_source(&printer, model, true)?;
    printer.nl();
    let struct_name = map.root_name().unique_camelcase();
    printer
        .print("void ")
        .print(&struct_name)
        .print("_init(")
        .print(&struct_name)
        .print(" *main) {")
        .nl();
    {
        printer
            .up()
            .ident("main->state = ")
            .print(&map.root_name().unique_uppercase_snakecase())
            .print("_INIT;")
            .down()
            .nl();
    }
    printer.print("}").nl().nl();
    printer
        .print("void ")
        .print(&struct_name)
        .print("_tick(")
        .print(&struct_name)
        .print(" *main) {")
        .nl();
    printer.up();
    // Self::generate_model_tick_source(&printer, model, true)?;
    printer.down();
    printer.print("}").nl().nl();
    printer
        .print("void ")
        .print(&struct_name)
        .print("_reset(")
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
        .print("_is_done(const ")
        .print(&struct_name)
        .print(" *main) {")
        .nl();
    let mut cond = String::new();
    for state_name in map.states().iter() {
        let state = map.raw_state_at(state_name.clone())?;
        let state = &*state.borrow();
        if !state.is_terminated() {
            continue;
        }
        if !cond.is_empty() {
            cond.push_str(" || ");
        }
        cond.push_str("main->state == ");
        cond.push_str(&state_name.unique_uppercase_snakecase());
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

fn generate_constants_and_ports_and_enums(
    printer: &mut Printer,
    map: &CMap,
) -> Result<(), Diagnostic> {
    let mut models = map.using_models();
    models.insert(
        0,
        Element::Model {
            name: map.root_name().clone(),
            states: map.states().clone(),
            start: map.start().clone(),
        },
    );
    for model in models {
        let model_name = model.name();
        let model = map.raw_model_at(model.name())?;
        let model = &*model.borrow();
        let variables = model.variables.clone().into_values();
        let mut lines = Vec::new();
        for var in variables {
            match var {
                VariableNode::Unresolved | VariableNode::Simple { .. } => {}
                VariableNode::Port { name, expr, .. } => {
                    let name = model_name.unique_uppercase_snakecase()
                        + "_"
                        + normalize_lowercase_snakecase(name.clone())
                            .to_uppercase()
                            .as_str();
                    let (address, _bit) = if let ExpressionNode::Address(address, bit) = expr {
                        (address, bit)
                    } else if let ExpressionNode::Number(address) = expr {
                        (address, 0)
                    } else {
                        return Err("Unresolved address".into());
                    };
                    lines.push(format!("#define PORT_{} 0x{:x}", name, address));
                }
                VariableNode::Const { name, expr, .. } => {
                    let name = model_name.unique_uppercase_snakecase()
                        + "_"
                        + normalize_lowercase_snakecase(name.clone())
                            .to_uppercase()
                            .as_str();
                    let value = if let ExpressionNode::Number(value) = expr {
                        value
                    } else {
                        error!("Unresolved constant value: {:?}", expr);
                        //return Err("Unresolved constant value".into());
                        continue;
                    };
                    lines.push(format!("#define CONST_{} 0x{:x}", name, value));
                }
            }
        }
        if lines.is_empty() {
            continue;
        }
        printer
            .print(format!("/// Константы, перечисления и порты модели {}", model_name).as_str())
            .nl();
        lines.sort();
        printer.print(lines.join("\n").as_str()).nl();
    }

    // let upper_name = Self::get_upper_name(model);
    //
    // let variables = model.variables.clone();
    // for var in variables
    //     .into_values()
    //     .sorted_by(|a, b| a.name().cmp(b.name()))
    // {
    //     match var.clone() {
    //         VariableNode::Unresolved => {}
    //         VariableNode::Simple { .. } => {}
    //         VariableNode::Port { name, expr, .. } => {
    //             let name = Self::resolve_raw_name(upper_name.clone(), name)?;
    //
    //             let (address, _bit) = if let ExpressionNode::Address(address, bit) = expr {
    //                 (address, bit)
    //             } else if let ExpressionNode::Number(address) = expr {
    //                 (address, 0)
    //             } else {
    //                 return Err("Unresolved address".into());
    //             };
    //             printer.print("#define PORT_").print(&name).print(" ");
    //             printer.print(format!("0x{:x}", address).as_str());
    //             printer.nl();
    //         }
    //         VariableNode::Const { name, expr, .. } => {
    //             let name = Self::resolve_raw_name(upper_name.clone(), name)?;
    //             let unrolled = Self::unroll_expression(&expr)?;
    //             printer
    //                 .print("#define CONST_")
    //                 .print(&name)
    //                 .print(" (")
    //                 .print(unrolled.as_str())
    //                 .print(")")
    //                 .nl();
    //         }
    //     }
    // }
    // let conditions = model.conditions.clone();
    // for cond in conditions
    //     .into_values()
    //     .sorted_by(|a, b| a.name().cmp(b.name()))
    // {
    //     let unrolled = Self::unroll_cond(&cond.value)?;
    //     printer
    //         .print("#define COND_")
    //         .print(&Self::resolve_cond_name(upper_name.clone(), &cond)?)
    //         .print(" (")
    //         .print(unrolled.as_str())
    //         .print(")")
    //         .nl();
    // }
    // let enums = model.enums.clone();
    // for en in enums.into_values().sorted_by(|a, b| a.name().cmp(b.name())) {
    //     printer
    //         .print(format!("/* Enum  {}*/", en.name()).as_str())
    //         .nl();
    //     let prefix =
    //         "#define ENUM_".to_string() + &*Self::resolve_enum_name(upper_name.clone(), &en)?;
    //     for (name, value) in en.variants {
    //         printer
    //             .print(prefix.clone().as_str())
    //             .print("_")
    //             .print(normalize_lowercase_snakecase(name).to_uppercase().as_str())
    //             .print(format!(" {}", value).as_str())
    //             .nl();
    //     }
    // }
    Ok(())
}

impl Generator {
    /*
        /// Разворачивает именованное условие в C-выражение.
        #[allow(dead_code)]
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

        /// Разворачивает выражение в C-выражение.
        pub(super) fn unroll_expression(expr: &ExpressionNode) -> Result<String, Diagnostic> {
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
    */
    /// Формирует имя константы/порта в UPPER_SNAKE_CASE из пространства имён модели.
    #[inline]
    fn resolve_raw_name(upper_name: String, name: String) -> Result<String, Diagnostic> {
        Ok(
            (upper_name.to_owned() + "_" + normalize_lowercase_snakecase(name.clone()).as_str())
                .to_uppercase(),
        )
    }

    /// Формирует имя именованного условия в UPPER_SNAKE_CASE.
    #[inline]
    fn resolve_cond_name(
        upper_name: String,
        cond: &ConditionDefinitionNode,
    ) -> Result<String, Diagnostic> {
        Self::resolve_raw_name(upper_name, cond.name.clone())
    }

    /// Формирует имя перечисления в UPPER_SNAKE_CASE.
    #[allow(dead_code)]
    #[inline]
    fn resolve_enum_name(
        upper_name: String,
        en: &EnumDefinitionNode,
    ) -> Result<String, Diagnostic> {
        Self::resolve_raw_name(upper_name, en.name.clone())
    }
}
