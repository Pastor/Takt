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

use super::{Generator, get_c_type};
use crate::diagnostics::Diagnostic;
use crate::generator::c::c_map::CMap;
use crate::generator::indent::Printer;
use crate::semantic::minimap::{Element, Name};
use crate::semantic::naming::normalize_lowercase_snakecase;
use crate::semantic::type_node::TypeNode;
use crate::semantic::{ExpressionNode, FunctionDefinitionNode, StatementNode, VariableNode};

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
    generate_functions(&mut printer, map)?;
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
                VariableNode::Const { name, ref expr, .. } => {
                    let name = model_name.unique_uppercase_snakecase()
                        + "_"
                        + normalize_lowercase_snakecase(name.clone())
                            .to_uppercase()
                            .as_str();
                    let value = const_expr_string(expr, &name)?;
                    lines.push(format!("#define CONST_{} {}", name, value));
                }
            }
        }
        if !lines.is_empty() {
            printer
                .print(format!("/// Константы и порты модели {}", model_name).as_str())
                .nl();
            lines.sort();
            printer.print(lines.join("\n").as_str()).nl();
        }

        let enums = model.enums.clone().into_values();
        let mut lines = Vec::new();
        for en in enums {
            let prefix = format!("#define ENUM_{}_", model_name.unique_uppercase_snakecase());
            for (name, value) in en.variants {
                lines.push(format!(
                    "{}{} {}",
                    prefix,
                    normalize_lowercase_snakecase(name.clone()).to_uppercase(),
                    value
                ));
            }
        }
        if !lines.is_empty() {
            printer
                .print(format!("/// Перечисления модели {}", model_name).as_str())
                .nl();
            lines.sort();
            printer.print(lines.join("\n").as_str()).nl();
        }
    }
    Ok(())
}

fn get_function_name(fun: &FunctionDefinitionNode) -> String {
    match fun {
        FunctionDefinitionNode::Local { upper, name, .. } => {
            let model_name = Name::from(upper.clone().unwrap().upgrade().unwrap());
            format!("{}_{}", model_name.unique_camelcase(), name)
        }
        FunctionDefinitionNode::External { name, .. } => name.clone(),
        _ => {
            unreachable!("Unresolved function definition");
        }
    }
}

fn generate_stmt_expression(
    printer: &mut Printer,
    map: &CMap,
    owner: &Element,
    params: Vec<(String, TypeNode)>,
    expr: &ExpressionNode,
) -> Result<(), Diagnostic> {
    match expr {
        ExpressionNode::None | ExpressionNode::Unresolved(_) => {
            return Err("Unresolved expression".into());
        }
        ExpressionNode::ArraySubscript(_, _) => {}
        ExpressionNode::ArraySlice(_, _, _) => {}
        ExpressionNode::Parenthesis(_) => {}
        ExpressionNode::BitAccess(_, _) => {}
        ExpressionNode::Function(_, _) => {}
        ExpressionNode::CodeBlock(_, _) => {}
        ExpressionNode::NamedFunctionBox(_, _) => {}
        ExpressionNode::Not(_) => {}
        ExpressionNode::BitwiseNot(_) => {}
        ExpressionNode::UnaryPlus(_) => {}
        ExpressionNode::Negate(_) => {}
        ExpressionNode::Power(_, _) => {}
        ExpressionNode::Multiply(_, _) => {}
        ExpressionNode::Divide(_, _) => {}
        ExpressionNode::Modulo(_, _) => {}
        ExpressionNode::Add(_, _) => {}
        ExpressionNode::Subtract(_, _) => {}
        ExpressionNode::ShiftLeft(_, _) => {}
        ExpressionNode::ShiftRight(_, _) => {}
        ExpressionNode::BitwiseAnd(_, _) => {}
        ExpressionNode::BitwiseXor(_, _) => {}
        ExpressionNode::BitwiseOr(_, _) => {}
        ExpressionNode::Less(_, _) => {}
        ExpressionNode::More(_, _) => {}
        ExpressionNode::LessEqual(_, _) => {}
        ExpressionNode::MoreEqual(_, _) => {}
        ExpressionNode::Equal(_, _) => {}
        ExpressionNode::NotEqual(_, _) => {}
        ExpressionNode::And(_, _) => {}
        ExpressionNode::Or(_, _) => {}
        ExpressionNode::ConditionalOperator(_, _, _) => {}
        ExpressionNode::Assign(_, _) => {}
        ExpressionNode::Number(n) => {
            printer.print(format!("{}", n).as_str());
        }
        ExpressionNode::Rational(_, _) => {}
        ExpressionNode::String(v) => {
            printer.print("\"").print(v.join("").as_str()).print("\"");
        }
        ExpressionNode::Type(_) => {}
        ExpressionNode::Address(_, _) => {}
        ExpressionNode::Bool(value) => {
            if *value {
                printer.print("true");
            } else {
                printer.print("false");
            }
        }
        ExpressionNode::Variable(_) => {}
        ExpressionNode::Model(_) => {}
        ExpressionNode::Condition(_) => {}
        ExpressionNode::List(_) => {}
        ExpressionNode::Array(_) => {}
        ExpressionNode::Initializer(_) => {}
        ExpressionNode::Cast(expr, typ) => {
            let model = map.raw_model_at(owner.name())?;
            let model = &*model.borrow();
            let type_c = get_c_type(typ, model).unwrap();
            printer.print("(").print(&*type_c).print(")(");
            generate_stmt_expression(printer, map, owner, params.clone(), expr)?;
            printer.print(")");
        }
    }
    Ok(())
}

fn generate_code_block(
    printer: &mut Printer,
    map: &CMap,
    owner: &Element,
    params: Vec<(String, TypeNode)>,
    body: &StatementNode,
) -> Result<(), Diagnostic> {
    match body {
        StatementNode::None => {}
        StatementNode::Unresolved(_) => {}
        StatementNode::Block(block) => {
            printer.print("{").nl().up();
            for stmt in block {
                generate_code_block(printer, map, owner, params.clone(), stmt)?;
            }
            printer.down().print("}");
        }
        StatementNode::Expression(expr) => {
            generate_stmt_expression(printer, map, owner, params.clone(), expr)?;
        }
        StatementNode::If { .. } => {}
        StatementNode::Loop { .. } => {}
        StatementNode::For { .. } => {}
        StatementNode::Variable(var, ty, init) => {}
        StatementNode::Return(ret) => {
            printer.ident("return");
            if let Some(expr) = ret {
                printer.print(" ( ");
                generate_stmt_expression(printer, map, owner, params.clone(), expr)?;
                printer.print(" )");
            }
            printer.print(";").nl();
        }
        StatementNode::Continue => {
            printer.ident("continue");
        }
        StatementNode::Break => {
            printer.ident("break");
        }
    }
    Ok(())
}

fn generate_functions(printer: &mut Printer, map: &CMap) -> Result<(), Diagnostic> {
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
        let element = model.clone();
        let model = map.raw_model_at(model.name())?;
        let model = &*model.borrow();
        let mut external_funcs = Vec::new();
        let mut local_funcs = Vec::new();
        for ref fun in model.functions.clone().into_values() {
            match fun {
                FunctionDefinitionNode::Local {
                    upper,
                    name,
                    params,
                    body,
                    ret,
                    ..
                } => {
                    let mut definition = String::new();
                    let mut tiny_params = params
                        .into_iter()
                        .map(|(name, typ)| {
                            get_c_type(&typ, model)
                                .map(|c_type| format!("{} {}", c_type, name.clone()))
                                .unwrap()
                        })
                        .collect::<Vec<String>>();
                    tiny_params.insert(0, format!("{} *main", map.root_name().unique_camelcase()));
                    definition.push_str(
                        format!(
                            "static {} {}({}) ",
                            get_c_type(&ret, model).unwrap().as_str(),
                            get_function_name(&fun),
                            tiny_params.join(", ")
                        )
                        .as_str(),
                    );
                    let mut code_block = String::new();
                    {
                        let mut tmp_printer = Printer::new(4, &mut code_block);
                        tmp_printer.up();
                        generate_code_block(&mut tmp_printer, map, &element, params.clone(), body)?;
                        tmp_printer.down().nl();
                    }
                    definition.push_str(&code_block);
                    local_funcs.push(definition);
                }
                FunctionDefinitionNode::External {
                    name, params, ret, ..
                } => {
                    let params = params
                        .into_iter()
                        .map(|(name, typ)| {
                            get_c_type(&typ, model)
                                .map(|c_type| format!("{} {}", c_type, name.clone()))
                                .unwrap()
                        })
                        .collect::<Vec<String>>();
                    external_funcs.push(format!(
                        "extern {} {}({});",
                        get_c_type(&ret, model).unwrap().as_str(),
                        get_function_name(&fun),
                        params.join(", ").as_str()
                    ));
                }
                _ => {
                    return Err(format!("Unresolved function '{}'", fun.name())
                        .as_str()
                        .into());
                }
            }
        }

        if !external_funcs.is_empty() {
            printer.print("///Внешние функции").nl();
            external_funcs.sort();
            for func in external_funcs {
                printer.print(func.as_str()).nl();
            }
        }
        if !local_funcs.is_empty() {
            printer.print("///Функции моделей").nl();
            local_funcs.sort();
            for func in local_funcs {
                printer.print(func.as_str()).nl();
            }
        }
    }
    Ok(())
}

fn const_expr_string(expr: &ExpressionNode, name: &String) -> Result<String, Diagnostic> {
    Ok(if let ExpressionNode::Number(value) = expr {
        value.to_string()
    } else if let ExpressionNode::Bool(value) = expr {
        if *value {
            "true".to_string()
        } else {
            "false".to_string()
        }
    } else if let ExpressionNode::String(value) = expr {
        format!("\"{}\"", value.join(""))
    } else if let ExpressionNode::Rational(value, _) = expr {
        value.clone()
    } else if let ExpressionNode::Initializer(value) = expr {
        let mut parts = Vec::new();
        for v in value.iter() {
            parts.push(const_expr_string(v, name)?);
        }
        format!("{{{}}}", parts.join(", "))
    } else {
        return Err(format!("Unresolved constant '{}' value: {:?}", name, expr)
            .as_str()
            .into());
    })
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
}
