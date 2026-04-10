//! Генерация исходного C-файла (`.c`) из семантического дерева BuT.
//!
//! Содержит все функции генерации `.c`-исходника:
//! [`generate_source`], вспомогательные `resolve_variable_c_expr`,
//! `generate_function_call`, `generate_stmt_expression`,
//! `generate_code_block` и утилиты именования.
//!
//! ## Состояние реализации
//!
//! Генерация `.c`-файлов поддерживает:
//! - Все унарные и бинарные операторы (включая `pow()` для `**`).
//! - Чтение/запись портов через указатели `read_bit`/`write_bit`/`read_float`/`write_float`.
//! - Присваивание, вызовы локальных и внешних функций.
//! - Встроенные функции: `min`, `max`, `abs`, `clamp` (раскрываются как тернарные выражения).
//! - Условные операторы (`if`/`else`), циклы (`loop`, `for`), объявления переменных.

use super::{get_c_type, get_typed_variable};
use crate::diagnostics::{Diagnostic, Location};
use crate::generator::c::c_map::CMap;
use crate::generator::indent::Printer;
use crate::semantic::minimap::{Element, Name};
use crate::semantic::naming::normalize_lowercase_snakecase;
use crate::semantic::type_node::TypeNode;
use crate::semantic::{
    ConditionDefinitionNode, ExpressionNode, FunctionDefinitionNode, StatementNode, VariableNode,
};

/// Генерирует содержимое `.c`-файла для модели.
pub(super) fn generate_source(filename: &str, map: &CMap) -> Result<String, Diagnostic> {
    let mut source = String::new();
    let mut printer = Printer::new(4, &mut source);
    printer
        .print(format!("#include \"{}.h\"", filename).as_str())
        .nl();
    printer.print("#include <math.h>").nl();
    generate_constants_and_ports_and_enums(&mut printer, map)?;
    generate_functions(&mut printer, map)?;
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

/// Преобразует [`VariableNode`] в C-выражение для чтения.
///
/// - `Simple` с `loc == Implicit` — локальная переменная (stack), доступ по имени.
/// - `Simple` — переменная модели, доступ `main->field` или `main->model.field`.
/// - `Const` — `CONST_{MODEL}_{NAME}`.
/// - `Port` — вызов `(*main->read_bit)(PORT_..., bit, main->userdata)`.
fn resolve_variable_c_expr(
    var: &VariableNode,
    params: &[(String, TypeNode)],
) -> Result<String, Diagnostic> {
    match var {
        VariableNode::Simple { name, upper, loc, .. } => {
            // Локальная переменная (объявлена через register_local_var) имеет loc == Implicit
            if matches!(loc, Location::Implicit) {
                return Ok(normalize_lowercase_snakecase(name.clone()));
            }
            // Параметр функции — тоже доступ по имени
            if params.iter().any(|(p, _)| p == name) {
                return Ok(normalize_lowercase_snakecase(name.clone()));
            }
            // Переменная уровня модели → main->field
            if let Some(model_rc) = upper.as_ref().and_then(|w| w.upgrade()) {
                let model = model_rc.borrow();
                if let Some(model_name) = &model.name {
                    // Вложенная модель — через имя подструктуры
                    Ok(format!(
                        "main->{}.{}",
                        normalize_lowercase_snakecase(model_name.clone()),
                        normalize_lowercase_snakecase(name.clone())
                    ))
                } else {
                    // Корневая модель — поле напрямую
                    Ok(format!("main->{}", normalize_lowercase_snakecase(name.clone())))
                }
            } else {
                Ok(format!("main->{}", normalize_lowercase_snakecase(name.clone())))
            }
        }
        VariableNode::Const { name, upper, .. } => {
            // CONST_{MODEL_UPPERCASE}_{CONST_UPPERCASE}
            if let Some(model_rc) = upper.as_ref().and_then(|w| w.upgrade()) {
                let model_name = Name::from(model_rc);
                Ok(format!(
                    "CONST_{}_{}",
                    model_name.unique_uppercase_snakecase(),
                    normalize_lowercase_snakecase(name.clone()).to_uppercase()
                ))
            } else {
                Ok(format!(
                    "CONST_{}",
                    normalize_lowercase_snakecase(name.clone()).to_uppercase()
                ))
            }
        }
        VariableNode::Port { name, ty, expr, upper, .. } => {
            // Чтение порта через read_bit или read_float
            let model_name = if let Some(model_rc) = upper.as_ref().and_then(|w| w.upgrade()) {
                Name::from(model_rc)
            } else {
                return Err("Неразрешённый owner порта".into());
            };
            let port_name = format!(
                "PORT_{}_{}",
                model_name.unique_uppercase_snakecase(),
                normalize_lowercase_snakecase(name.clone()).to_uppercase()
            );
            let bit = if let ExpressionNode::Address(_, bit) = expr {
                *bit
            } else {
                0i64
            };
            match ty {
                TypeNode::Rational => {
                    Ok(format!("(*main->read_float)({}, main->userdata)", port_name))
                }
                _ => {
                    Ok(format!(
                        "(*main->read_bit)({}, {}, main->userdata)",
                        port_name, bit
                    ))
                }
            }
        }
        VariableNode::Unresolved => Err("Неразрешённая переменная".into()),
    }
}

/// Генерирует список C-аргументов для вызова функции.
fn generate_args(
    map: &CMap,
    owner: &Element,
    params: &[(String, TypeNode)],
    args: &[ExpressionNode],
) -> Result<Vec<String>, Diagnostic> {
    let mut result = Vec::new();
    for arg in args {
        let mut s = String::new();
        let mut tmp = Printer::new(4, &mut s);
        generate_stmt_expression(&mut tmp, map, owner, params.to_vec(), arg)?;
        result.push(s);
    }
    Ok(result)
}

/// Генерирует C-вызов функции.
///
/// - `Local` → `{ModelCamelCase}_{name}(main, args...)`
/// - `External` → `{name}(args...)`
/// - `Builtin("min"|"max"|"abs"|"clamp")` → раскрывается как тернарное выражение
/// - `Builtin("debug"|"S")` → возвращает ошибку (не транслируется в C)
fn generate_function_call(
    printer: &mut Printer,
    map: &CMap,
    owner: &Element,
    params: Vec<(String, TypeNode)>,
    fun_def: &FunctionDefinitionNode,
    args: &[ExpressionNode],
) -> Result<(), Diagnostic> {
    match fun_def {
        FunctionDefinitionNode::Local { upper, name, .. } => {
            let model_rc = upper
                .as_ref()
                .and_then(|w| w.upgrade())
                .ok_or_else(|| -> Diagnostic { "Неразрешённый owner функции".into() })?;
            let model_name = Name::from(model_rc);
            let func_name = format!("{}_{}", model_name.unique_camelcase(), name);
            let arg_strs = generate_args(map, owner, &params, args)?;
            let mut all_args = vec!["main".to_string()];
            all_args.extend(arg_strs);
            printer.print(&format!("{}({})", func_name, all_args.join(", ")));
        }
        FunctionDefinitionNode::External { name, .. } => {
            let arg_strs = generate_args(map, owner, &params, args)?;
            printer.print(&format!("{}({})", name, arg_strs.join(", ")));
        }
        FunctionDefinitionNode::Builtin(builtin_name, _, _) => {
            match *builtin_name {
                "min" => {
                    let arg_strs = generate_args(map, owner, &params, args)?;
                    if arg_strs.len() >= 2 {
                        printer.print(&format!(
                            "((({a}) < ({b})) ? ({a}) : ({b}))",
                            a = arg_strs[0],
                            b = arg_strs[1]
                        ));
                    }
                }
                "max" => {
                    let arg_strs = generate_args(map, owner, &params, args)?;
                    if arg_strs.len() >= 2 {
                        printer.print(&format!(
                            "((({a}) > ({b})) ? ({a}) : ({b}))",
                            a = arg_strs[0],
                            b = arg_strs[1]
                        ));
                    }
                }
                "abs" => {
                    let arg_strs = generate_args(map, owner, &params, args)?;
                    if !arg_strs.is_empty() {
                        printer.print(&format!(
                            "((({x}) < 0) ? -({x}) : ({x}))",
                            x = arg_strs[0]
                        ));
                    }
                }
                "clamp" => {
                    let arg_strs = generate_args(map, owner, &params, args)?;
                    if arg_strs.len() >= 3 {
                        printer.print(&format!(
                            "((({x}) < ({lo})) ? ({lo}) : ((({x}) > ({hi})) ? ({hi}) : ({x})))",
                            x = arg_strs[0],
                            lo = arg_strs[1],
                            hi = arg_strs[2]
                        ));
                    }
                }
                "debug" | "S" => {
                    return Err(format!(
                        "Встроенная функция '{}' не поддерживается в C генераторе",
                        builtin_name
                    )
                    .as_str()
                    .into());
                }
                other => {
                    return Err(format!(
                        "Неизвестная встроенная функция '{}'",
                        other
                    )
                    .as_str()
                    .into());
                }
            }
        }
        _ => {
            return Err("Неразрешённое определение функции".into());
        }
    }
    Ok(())
}

/// Генерирует C-выражение из семантического узла выражения.
///
/// Функция пишет в `printer` без начального отступа и без завершающего `;\n`.
/// Отступ и разделители добавляет вызывающий код.
fn generate_stmt_expression(
    printer: &mut Printer,
    map: &CMap,
    owner: &Element,
    params: Vec<(String, TypeNode)>,
    expr: &ExpressionNode,
) -> Result<(), Diagnostic> {
    match expr {
        ExpressionNode::None | ExpressionNode::Unresolved(_) => {
            return Err("Неразрешённое выражение".into());
        }

        // ── Литералы ──────────────────────────────────────────────────────────

        ExpressionNode::Number(n) => {
            printer.print(&n.to_string());
        }
        ExpressionNode::Bool(value) => {
            printer.print(if *value { "true" } else { "false" });
        }
        ExpressionNode::String(v) => {
            printer.print("\"").print(&v.join("")).print("\"");
        }
        ExpressionNode::Rational(s, neg) => {
            if *neg {
                printer.print("-");
            }
            printer.print(s);
        }

        // ── Унарные операторы ──────────────────────────────────────────────────

        ExpressionNode::Not(e) => {
            printer.print("!(");
            generate_stmt_expression(printer, map, owner, params, e)?;
            printer.print(")");
        }
        ExpressionNode::BitwiseNot(e) => {
            printer.print("~(");
            generate_stmt_expression(printer, map, owner, params, e)?;
            printer.print(")");
        }
        ExpressionNode::UnaryPlus(e) => {
            printer.print("+(");
            generate_stmt_expression(printer, map, owner, params, e)?;
            printer.print(")");
        }
        ExpressionNode::Negate(e) => {
            printer.print("-(");
            generate_stmt_expression(printer, map, owner, params, e)?;
            printer.print(")");
        }

        // ── Степень → pow() ────────────────────────────────────────────────────

        ExpressionNode::Power(l, r) => {
            printer.print("pow((double)(");
            generate_stmt_expression(printer, map, owner, params.clone(), l)?;
            printer.print("), (double)(");
            generate_stmt_expression(printer, map, owner, params, r)?;
            printer.print("))");
        }

        // ── Бинарные арифметические ────────────────────────────────────────────

        ExpressionNode::Multiply(l, r) => {
            printer.print("(");
            generate_stmt_expression(printer, map, owner, params.clone(), l)?;
            printer.print(") * (");
            generate_stmt_expression(printer, map, owner, params, r)?;
            printer.print(")");
        }
        ExpressionNode::Divide(l, r) => {
            printer.print("(");
            generate_stmt_expression(printer, map, owner, params.clone(), l)?;
            printer.print(") / (");
            generate_stmt_expression(printer, map, owner, params, r)?;
            printer.print(")");
        }
        ExpressionNode::Modulo(l, r) => {
            printer.print("(");
            generate_stmt_expression(printer, map, owner, params.clone(), l)?;
            printer.print(") % (");
            generate_stmt_expression(printer, map, owner, params, r)?;
            printer.print(")");
        }
        ExpressionNode::Add(l, r) => {
            printer.print("(");
            generate_stmt_expression(printer, map, owner, params.clone(), l)?;
            printer.print(") + (");
            generate_stmt_expression(printer, map, owner, params, r)?;
            printer.print(")");
        }
        ExpressionNode::Subtract(l, r) => {
            printer.print("(");
            generate_stmt_expression(printer, map, owner, params.clone(), l)?;
            printer.print(") - (");
            generate_stmt_expression(printer, map, owner, params, r)?;
            printer.print(")");
        }

        // ── Битовые сдвиги ────────────────────────────────────────────────────

        ExpressionNode::ShiftLeft(l, r) => {
            printer.print("(");
            generate_stmt_expression(printer, map, owner, params.clone(), l)?;
            printer.print(") << (");
            generate_stmt_expression(printer, map, owner, params, r)?;
            printer.print(")");
        }
        ExpressionNode::ShiftRight(l, r) => {
            printer.print("(");
            generate_stmt_expression(printer, map, owner, params.clone(), l)?;
            printer.print(") >> (");
            generate_stmt_expression(printer, map, owner, params, r)?;
            printer.print(")");
        }

        // ── Побитовые операторы ────────────────────────────────────────────────

        ExpressionNode::BitwiseAnd(l, r) => {
            printer.print("(");
            generate_stmt_expression(printer, map, owner, params.clone(), l)?;
            printer.print(") & (");
            generate_stmt_expression(printer, map, owner, params, r)?;
            printer.print(")");
        }
        ExpressionNode::BitwiseXor(l, r) => {
            printer.print("(");
            generate_stmt_expression(printer, map, owner, params.clone(), l)?;
            printer.print(") ^ (");
            generate_stmt_expression(printer, map, owner, params, r)?;
            printer.print(")");
        }
        ExpressionNode::BitwiseOr(l, r) => {
            printer.print("(");
            generate_stmt_expression(printer, map, owner, params.clone(), l)?;
            printer.print(") | (");
            generate_stmt_expression(printer, map, owner, params, r)?;
            printer.print(")");
        }

        // ── Сравнение ─────────────────────────────────────────────────────────

        ExpressionNode::Less(l, r) => {
            printer.print("(");
            generate_stmt_expression(printer, map, owner, params.clone(), l)?;
            printer.print(") < (");
            generate_stmt_expression(printer, map, owner, params, r)?;
            printer.print(")");
        }
        ExpressionNode::More(l, r) => {
            printer.print("(");
            generate_stmt_expression(printer, map, owner, params.clone(), l)?;
            printer.print(") > (");
            generate_stmt_expression(printer, map, owner, params, r)?;
            printer.print(")");
        }
        ExpressionNode::LessEqual(l, r) => {
            printer.print("(");
            generate_stmt_expression(printer, map, owner, params.clone(), l)?;
            printer.print(") <= (");
            generate_stmt_expression(printer, map, owner, params, r)?;
            printer.print(")");
        }
        ExpressionNode::MoreEqual(l, r) => {
            printer.print("(");
            generate_stmt_expression(printer, map, owner, params.clone(), l)?;
            printer.print(") >= (");
            generate_stmt_expression(printer, map, owner, params, r)?;
            printer.print(")");
        }
        ExpressionNode::Equal(l, r) => {
            printer.print("(");
            generate_stmt_expression(printer, map, owner, params.clone(), l)?;
            printer.print(") == (");
            generate_stmt_expression(printer, map, owner, params, r)?;
            printer.print(")");
        }
        ExpressionNode::NotEqual(l, r) => {
            printer.print("(");
            generate_stmt_expression(printer, map, owner, params.clone(), l)?;
            printer.print(") != (");
            generate_stmt_expression(printer, map, owner, params, r)?;
            printer.print(")");
        }

        // ── Логические ────────────────────────────────────────────────────────

        ExpressionNode::And(l, r) => {
            printer.print("(");
            generate_stmt_expression(printer, map, owner, params.clone(), l)?;
            printer.print(") && (");
            generate_stmt_expression(printer, map, owner, params, r)?;
            printer.print(")");
        }
        ExpressionNode::Or(l, r) => {
            printer.print("(");
            generate_stmt_expression(printer, map, owner, params.clone(), l)?;
            printer.print(") || (");
            generate_stmt_expression(printer, map, owner, params, r)?;
            printer.print(")");
        }

        // ── Специальные ───────────────────────────────────────────────────────

        ExpressionNode::Parenthesis(e) => {
            printer.print("(");
            generate_stmt_expression(printer, map, owner, params, e)?;
            printer.print(")");
        }

        ExpressionNode::ConditionalOperator(cond, then_, else_) => {
            printer.print("(");
            generate_stmt_expression(printer, map, owner, params.clone(), cond)?;
            printer.print(") ? (");
            generate_stmt_expression(printer, map, owner, params.clone(), then_)?;
            printer.print(") : (");
            generate_stmt_expression(printer, map, owner, params, else_)?;
            printer.print(")");
        }

        ExpressionNode::Assign(l, r) => {
            // Запись в порт → write_bit / write_float
            if let ExpressionNode::Variable(var_rc) = l.as_ref() {
                let var = var_rc.borrow();
                if let VariableNode::Port { name, ty, expr: addr_expr, upper, .. } = &*var {
                    let model_name =
                        if let Some(model_rc) = upper.as_ref().and_then(|w| w.upgrade()) {
                            Name::from(model_rc)
                        } else {
                            return Err("Неразрешённый owner порта при записи".into());
                        };
                    let port_name = format!(
                        "PORT_{}_{}",
                        model_name.unique_uppercase_snakecase(),
                        normalize_lowercase_snakecase(name.clone()).to_uppercase()
                    );
                    let bit =
                        if let ExpressionNode::Address(_, bit) = addr_expr { *bit } else { 0i64 };
                    let mut rhs_str = String::new();
                    {
                        let mut tmp = Printer::new(4, &mut rhs_str);
                        generate_stmt_expression(&mut tmp, map, owner, params, r)?;
                    }
                    match ty {
                        TypeNode::Rational => {
                            printer.print(&format!(
                                "(*main->write_float)({}, {}, main->userdata)",
                                port_name, rhs_str
                            ));
                        }
                        _ => {
                            printer.print(&format!(
                                "(*main->write_bit)({}, {}, {}, main->userdata)",
                                port_name, bit, rhs_str
                            ));
                        }
                    }
                    return Ok(());
                }
            }
            // Обычное присваивание
            printer.print("(");
            generate_stmt_expression(printer, map, owner, params.clone(), l)?;
            printer.print(") = (");
            generate_stmt_expression(printer, map, owner, params, r)?;
            printer.print(")");
        }

        ExpressionNode::ArraySubscript(var_rc, idx) => {
            let var = var_rc.borrow();
            let var_expr = resolve_variable_c_expr(&*var, &params)?;
            printer.print(&format!("{}[{}]", var_expr, idx));
        }

        ExpressionNode::Variable(var_rc) => {
            let var = var_rc.borrow();
            printer.print(&resolve_variable_c_expr(&*var, &params)?);
        }

        ExpressionNode::Condition(cond_rc) => {
            let cond = cond_rc.borrow();
            let cond_str = condition_macro_name(&*cond);
            printer.print(&cond_str);
        }

        ExpressionNode::Function(fun_rc, args) => {
            let fun = fun_rc.borrow();
            generate_function_call(printer, map, owner, params, &*fun, args)?;
        }

        ExpressionNode::Initializer(elems) => {
            printer.print("{");
            for (i, elem) in elems.iter().enumerate() {
                if i > 0 {
                    printer.print(", ");
                }
                generate_stmt_expression(printer, map, owner, params.clone(), elem)?;
            }
            printer.print("}");
        }

        ExpressionNode::Array(elems) => {
            printer.print("{");
            for (i, elem) in elems.iter().enumerate() {
                if i > 0 {
                    printer.print(", ");
                }
                generate_stmt_expression(printer, map, owner, params.clone(), elem)?;
            }
            printer.print("}");
        }

        ExpressionNode::Cast(expr, typ) => {
            let model = map.raw_model_at(owner.name())?;
            let model = &*model.borrow();
            let type_c = get_c_type(typ, model).unwrap_or_else(|| "int".to_string());
            printer.print("(").print(&type_c).print(")(");
            generate_stmt_expression(printer, map, owner, params, expr)?;
            printer.print(")");
        }

        // ── Неподдерживаемые ──────────────────────────────────────────────────

        ExpressionNode::ArraySlice(_, _, _) => {
            return Err("ArraySlice не поддерживается в C генераторе".into());
        }
        ExpressionNode::BitAccess(_, _) => {
            return Err("BitAccess не поддерживается в C генераторе".into());
        }
        ExpressionNode::CodeBlock(_, _) => {
            return Err("CodeBlock не поддерживается как выражение в C генераторе".into());
        }
        ExpressionNode::NamedFunctionBox(_, _) => {
            return Err("NamedFunctionBox не поддерживается в C генераторе".into());
        }
        ExpressionNode::List(_) => {
            return Err("List не поддерживается в C генераторе".into());
        }
        ExpressionNode::Type(_) => {
            return Err("Type не поддерживается как выражение в C генераторе".into());
        }
        ExpressionNode::Address(_, _) => {
            return Err("Address не поддерживается как выражение в C генераторе".into());
        }
        ExpressionNode::Model(_) => {
            return Err("Model не поддерживается как выражение в C генераторе".into());
        }
    }
    Ok(())
}

/// Генерирует имя макроса условия вида `COND_{MODEL}_{NAME}`.
fn condition_macro_name(cond: &ConditionDefinitionNode) -> String {
    if let Some(model_rc) = cond.upper.as_ref().and_then(|w| w.upgrade()) {
        let model_name = Name::from(model_rc);
        format!(
            "COND_{}_{}",
            model_name.unique_uppercase_snakecase(),
            normalize_lowercase_snakecase(cond.name.clone()).to_uppercase()
        )
    } else {
        format!("COND_{}", normalize_lowercase_snakecase(cond.name.clone()).to_uppercase())
    }
}

/// Генерирует C-оператор из семантического узла.
///
/// Для `Block` рекурсивно генерирует все вложенные операторы.
/// Для `Expression` генерирует выражение с отступом и `;`.
/// Поддерживает `If`, `Loop`, `For`, `Variable`, `Return`, `Continue`, `Break`.
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
            // Генерируем во временный буфер, чтобы безопасно пропустить
            // неподдерживаемые встроенные функции (debug, S) без порчи вывода
            let mut expr_buf = String::new();
            let result = {
                let mut tmp = Printer::new(4, &mut expr_buf);
                generate_stmt_expression(&mut tmp, map, owner, params, expr)
            };
            match result {
                Ok(()) if !expr_buf.is_empty() => {
                    printer.ident(&expr_buf).print(";").nl();
                }
                Ok(()) => {}
                Err(_) => {
                    // Пропускаем неподдерживаемые выражения (debug, S и т.п.)
                }
            }
        }

        StatementNode::If { cond, then_, else_ } => {
            printer.ident("if ((");
            generate_stmt_expression(printer, map, owner, params.clone(), cond)?;
            printer.print(")) ");
            generate_code_block(printer, map, owner, params.clone(), then_)?;
            if let Some(else_) = else_ {
                printer.print(" else ");
                generate_code_block(printer, map, owner, params.clone(), else_)?;
            }
            printer.nl();
        }

        StatementNode::Loop { cond, body } => {
            match cond {
                None => {
                    printer.ident("while (1) ");
                }
                Some(cond_expr) => {
                    printer.ident("while ((");
                    generate_stmt_expression(printer, map, owner, params.clone(), cond_expr)?;
                    printer.print(")) ");
                }
            }
            generate_code_block(printer, map, owner, params.clone(), body)?;
            printer.nl();
        }

        StatementNode::For { init, cond, step, body } => {
            let has_var_init =
                matches!(init.as_ref().map(|b| b.as_ref()), Some(StatementNode::Variable(..)));

            if has_var_init {
                // Объявление переменной выносим перед `for` в обёртку `{}`
                printer.ident("{").nl();
                printer.up();
                if let Some(init_stmt) = init {
                    generate_code_block(printer, map, owner, params.clone(), init_stmt)?;
                }
                printer.ident("for (;");
                if let Some(cond_expr) = cond {
                    printer.print(" ");
                    generate_stmt_expression(printer, map, owner, params.clone(), cond_expr)?;
                }
                printer.print(";");
                if let Some(step_expr) = step {
                    printer.print(" ");
                    generate_stmt_expression(printer, map, owner, params.clone(), step_expr)?;
                }
                printer.print(") ");
                generate_code_block(printer, map, owner, params.clone(), body)?;
                printer.nl();
                printer.down();
                printer.ident("}").nl();
            } else {
                printer.ident("for (");
                if let Some(init_stmt) = init {
                    // Инициализация — только выражение (без отступа и точки с запятой)
                    if let StatementNode::Expression(expr) = init_stmt.as_ref() {
                        generate_stmt_expression(printer, map, owner, params.clone(), expr)?;
                    }
                }
                printer.print(";");
                if let Some(cond_expr) = cond {
                    printer.print(" ");
                    generate_stmt_expression(printer, map, owner, params.clone(), cond_expr)?;
                }
                printer.print(";");
                if let Some(step_expr) = step {
                    printer.print(" ");
                    generate_stmt_expression(printer, map, owner, params.clone(), step_expr)?;
                }
                printer.print(") ");
                generate_code_block(printer, map, owner, params.clone(), body)?;
                printer.nl();
            }
        }

        StatementNode::Variable(name, ty, init) => {
            let model = map.raw_model_at(owner.name())?;
            let model_ref = model.borrow();
            let snake_name = normalize_lowercase_snakecase(name.clone());
            let decl = get_typed_variable(ty, Some(snake_name.clone()), &*model_ref)
                .unwrap_or_else(|| format!("int {}", snake_name));
            printer.ident(&decl);
            if let Some(init_expr) = init {
                printer.print(" = ");
                generate_stmt_expression(printer, map, owner, params, init_expr)?;
            }
            printer.print(";").nl();
        }

        StatementNode::Return(ret) => {
            printer.ident("return");
            if let Some(expr) = ret {
                printer.print(" (");
                generate_stmt_expression(printer, map, owner, params, expr)?;
                printer.print(")");
            }
            printer.print(";").nl();
        }

        StatementNode::Continue => {
            printer.ident("continue;").nl();
        }

        StatementNode::Break => {
            printer.ident("break;").nl();
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
        let element = model.clone();
        let model = map.raw_model_at(model.name())?;
        let model = &*model.borrow();
        let mut external_funcs = Vec::new();
        let mut local_funcs = Vec::new();
        for ref fun in model.functions.clone().into_values() {
            match fun {
                FunctionDefinitionNode::Local {
                    params,
                    body,
                    ret,
                    ..
                } => {
                    let mut definition = String::new();
                    let mut tiny_params = params
                        .iter()
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
                    params, ret, ..
                } => {
                    let params = params
                        .iter()
                        .map(|(name, typ)| {
                            get_c_type(typ, model)
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

#[cfg(test)]
mod tests {
    use crate::generator::c::c_map::CMap;
    use crate::generator::c::c_source::generate_source;
    use crate::semantic::ExpressionNode;
    use crate::{parse, semantic};

    use super::*;

    // ── Вспомогательная функция ────────────────────────────────────────────────

    /// Создаёт минимальный CMap для тестов генерации выражений.
    fn make_map_and_owner(src: &str) -> (CMap, Element) {
        let (ast, _) = parse(src, 0).expect("ошибка разбора");
        let model_rc = semantic::tree::construct_model(&ast, None, &[]).unwrap();
        model_rc.borrow_mut().name = Some("Main".to_string());
        let model = model_rc.borrow();
        let map = CMap::new(model.name(), &*model).unwrap();
        let owner = Element::Model {
            name: map.root_name().clone(),
            states: map.states().clone(),
            start: map.start().clone(),
        };
        (map, owner)
    }

    /// Генерирует C-строку из выражения.
    fn expr_to_str(map: &CMap, owner: &Element, expr: &ExpressionNode) -> String {
        let mut s = String::new();
        let mut printer = Printer::new(4, &mut s);
        generate_stmt_expression(&mut printer, map, owner, vec![], expr).unwrap();
        s
    }

    // ── Тесты литералов ────────────────────────────────────────────────────────

    #[test]
    fn test_expr_number() {
        let (map, owner) = make_map_and_owner("start Main { always { } }");
        let expr = ExpressionNode::Number(42);
        assert_eq!(expr_to_str(&map, &owner, &expr), "42");
    }

    #[test]
    fn test_expr_bool_true() {
        let (map, owner) = make_map_and_owner("start Main { always { } }");
        assert_eq!(expr_to_str(&map, &owner, &ExpressionNode::Bool(true)), "true");
    }

    #[test]
    fn test_expr_bool_false() {
        let (map, owner) = make_map_and_owner("start Main { always { } }");
        assert_eq!(expr_to_str(&map, &owner, &ExpressionNode::Bool(false)), "false");
    }

    #[test]
    fn test_expr_rational_positive() {
        let (map, owner) = make_map_and_owner("start Main { always { } }");
        let expr = ExpressionNode::Rational("3.14".to_string(), false);
        assert_eq!(expr_to_str(&map, &owner, &expr), "3.14");
    }

    #[test]
    fn test_expr_rational_negative() {
        let (map, owner) = make_map_and_owner("start Main { always { } }");
        let expr = ExpressionNode::Rational("3.14".to_string(), true);
        assert_eq!(expr_to_str(&map, &owner, &expr), "-3.14");
    }

    #[test]
    fn test_expr_string() {
        let (map, owner) = make_map_and_owner("start Main { always { } }");
        let expr = ExpressionNode::String(vec!["hello".to_string()]);
        assert_eq!(expr_to_str(&map, &owner, &expr), "\"hello\"");
    }

    // ── Тесты унарных операторов ───────────────────────────────────────────────

    #[test]
    fn test_expr_negate() {
        let (map, owner) = make_map_and_owner("start Main { always { } }");
        let inner = Box::new(ExpressionNode::Number(42));
        let expr = ExpressionNode::Negate(inner);
        assert_eq!(expr_to_str(&map, &owner, &expr), "-(42)");
    }

    #[test]
    fn test_expr_not() {
        let (map, owner) = make_map_and_owner("start Main { always { } }");
        let inner = Box::new(ExpressionNode::Bool(true));
        let expr = ExpressionNode::Not(inner);
        assert_eq!(expr_to_str(&map, &owner, &expr), "!(true)");
    }

    #[test]
    fn test_expr_bitwise_not() {
        let (map, owner) = make_map_and_owner("start Main { always { } }");
        let inner = Box::new(ExpressionNode::Number(0xFF));
        let expr = ExpressionNode::BitwiseNot(inner);
        assert_eq!(expr_to_str(&map, &owner, &expr), "~(255)");
    }

    #[test]
    fn test_expr_parenthesis() {
        let (map, owner) = make_map_and_owner("start Main { always { } }");
        let inner = Box::new(ExpressionNode::Number(42));
        let expr = ExpressionNode::Parenthesis(inner);
        assert_eq!(expr_to_str(&map, &owner, &expr), "(42)");
    }

    // ── Тесты бинарных операторов ──────────────────────────────────────────────

    #[test]
    fn test_expr_add() {
        let (map, owner) = make_map_and_owner("start Main { always { } }");
        let expr = ExpressionNode::Add(
            Box::new(ExpressionNode::Number(1)),
            Box::new(ExpressionNode::Number(2)),
        );
        assert_eq!(expr_to_str(&map, &owner, &expr), "(1) + (2)");
    }

    #[test]
    fn test_expr_subtract() {
        let (map, owner) = make_map_and_owner("start Main { always { } }");
        let expr = ExpressionNode::Subtract(
            Box::new(ExpressionNode::Number(5)),
            Box::new(ExpressionNode::Number(3)),
        );
        assert_eq!(expr_to_str(&map, &owner, &expr), "(5) - (3)");
    }

    #[test]
    fn test_expr_multiply() {
        let (map, owner) = make_map_and_owner("start Main { always { } }");
        let expr = ExpressionNode::Multiply(
            Box::new(ExpressionNode::Number(4)),
            Box::new(ExpressionNode::Number(5)),
        );
        assert_eq!(expr_to_str(&map, &owner, &expr), "(4) * (5)");
    }

    #[test]
    fn test_expr_divide() {
        let (map, owner) = make_map_and_owner("start Main { always { } }");
        let expr = ExpressionNode::Divide(
            Box::new(ExpressionNode::Number(10)),
            Box::new(ExpressionNode::Number(2)),
        );
        assert_eq!(expr_to_str(&map, &owner, &expr), "(10) / (2)");
    }

    #[test]
    fn test_expr_modulo() {
        let (map, owner) = make_map_and_owner("start Main { always { } }");
        let expr = ExpressionNode::Modulo(
            Box::new(ExpressionNode::Number(7)),
            Box::new(ExpressionNode::Number(3)),
        );
        assert_eq!(expr_to_str(&map, &owner, &expr), "(7) % (3)");
    }

    #[test]
    fn test_expr_shift_left() {
        let (map, owner) = make_map_and_owner("start Main { always { } }");
        let expr = ExpressionNode::ShiftLeft(
            Box::new(ExpressionNode::Number(1)),
            Box::new(ExpressionNode::Number(4)),
        );
        assert_eq!(expr_to_str(&map, &owner, &expr), "(1) << (4)");
    }

    #[test]
    fn test_expr_shift_right() {
        let (map, owner) = make_map_and_owner("start Main { always { } }");
        let expr = ExpressionNode::ShiftRight(
            Box::new(ExpressionNode::Number(16)),
            Box::new(ExpressionNode::Number(2)),
        );
        assert_eq!(expr_to_str(&map, &owner, &expr), "(16) >> (2)");
    }

    #[test]
    fn test_expr_bitwise_and() {
        let (map, owner) = make_map_and_owner("start Main { always { } }");
        let expr = ExpressionNode::BitwiseAnd(
            Box::new(ExpressionNode::Number(0xF0)),
            Box::new(ExpressionNode::Number(0xFF)),
        );
        assert_eq!(expr_to_str(&map, &owner, &expr), "(240) & (255)");
    }

    #[test]
    fn test_expr_bitwise_or() {
        let (map, owner) = make_map_and_owner("start Main { always { } }");
        let expr = ExpressionNode::BitwiseOr(
            Box::new(ExpressionNode::Number(0xF0)),
            Box::new(ExpressionNode::Number(0x0F)),
        );
        assert_eq!(expr_to_str(&map, &owner, &expr), "(240) | (15)");
    }

    #[test]
    fn test_expr_bitwise_xor() {
        let (map, owner) = make_map_and_owner("start Main { always { } }");
        let expr = ExpressionNode::BitwiseXor(
            Box::new(ExpressionNode::Number(0xAA)),
            Box::new(ExpressionNode::Number(0x55)),
        );
        assert_eq!(expr_to_str(&map, &owner, &expr), "(170) ^ (85)");
    }

    #[test]
    fn test_expr_less() {
        let (map, owner) = make_map_and_owner("start Main { always { } }");
        let expr = ExpressionNode::Less(
            Box::new(ExpressionNode::Number(1)),
            Box::new(ExpressionNode::Number(2)),
        );
        assert_eq!(expr_to_str(&map, &owner, &expr), "(1) < (2)");
    }

    #[test]
    fn test_expr_more() {
        let (map, owner) = make_map_and_owner("start Main { always { } }");
        let expr = ExpressionNode::More(
            Box::new(ExpressionNode::Number(3)),
            Box::new(ExpressionNode::Number(2)),
        );
        assert_eq!(expr_to_str(&map, &owner, &expr), "(3) > (2)");
    }

    #[test]
    fn test_expr_equal() {
        let (map, owner) = make_map_and_owner("start Main { always { } }");
        let expr = ExpressionNode::Equal(
            Box::new(ExpressionNode::Number(0)),
            Box::new(ExpressionNode::Number(0)),
        );
        assert_eq!(expr_to_str(&map, &owner, &expr), "(0) == (0)");
    }

    #[test]
    fn test_expr_not_equal() {
        let (map, owner) = make_map_and_owner("start Main { always { } }");
        let expr = ExpressionNode::NotEqual(
            Box::new(ExpressionNode::Number(1)),
            Box::new(ExpressionNode::Number(0)),
        );
        assert_eq!(expr_to_str(&map, &owner, &expr), "(1) != (0)");
    }

    #[test]
    fn test_expr_logical_and() {
        let (map, owner) = make_map_and_owner("start Main { always { } }");
        let expr = ExpressionNode::And(
            Box::new(ExpressionNode::Bool(true)),
            Box::new(ExpressionNode::Bool(false)),
        );
        assert_eq!(expr_to_str(&map, &owner, &expr), "(true) && (false)");
    }

    #[test]
    fn test_expr_logical_or() {
        let (map, owner) = make_map_and_owner("start Main { always { } }");
        let expr = ExpressionNode::Or(
            Box::new(ExpressionNode::Bool(false)),
            Box::new(ExpressionNode::Bool(true)),
        );
        assert_eq!(expr_to_str(&map, &owner, &expr), "(false) || (true)");
    }

    #[test]
    fn test_expr_conditional_operator() {
        let (map, owner) = make_map_and_owner("start Main { always { } }");
        let expr = ExpressionNode::ConditionalOperator(
            Box::new(ExpressionNode::Bool(true)),
            Box::new(ExpressionNode::Number(1)),
            Box::new(ExpressionNode::Number(0)),
        );
        assert_eq!(expr_to_str(&map, &owner, &expr), "(true) ? (1) : (0)");
    }

    #[test]
    fn test_expr_cast() {
        let (map, owner) = make_map_and_owner("start Main { always { } }");
        let expr = ExpressionNode::Cast(
            Box::new(ExpressionNode::Number(42)),
            TypeNode::Bit,
        );
        assert_eq!(expr_to_str(&map, &owner, &expr), "(int)(42)");
    }

    #[test]
    fn test_expr_initializer() {
        let (map, owner) = make_map_and_owner("start Main { always { } }");
        let expr = ExpressionNode::Initializer(vec![
            ExpressionNode::Number(1),
            ExpressionNode::Number(2),
            ExpressionNode::Number(3),
        ]);
        assert_eq!(expr_to_str(&map, &owner, &expr), "{1, 2, 3}");
    }

    // ── Интеграционные тесты generate_source ──────────────────────────────────

    #[test]
    fn test_generate_source_has_include_and_math() {
        let src = r#"start Main { always { } }"#;
        let (model_ast, _) = parse(src, 0).unwrap();
        let model_rc = semantic::tree::construct_model(&model_ast, None, &[]).unwrap();
        model_rc.borrow_mut().name = Some("Main".to_string());
        let model = model_rc.borrow();
        let map = CMap::new(model.name(), &*model).unwrap();
        let source = generate_source(map.get_filename(), &map).unwrap();
        assert!(source.contains("#include \""), "отсутствует #include header:\n{source}");
        assert!(source.contains(".h\""), "отсутствует .h в include:\n{source}");
        assert!(source.contains("#include <math.h>"), "отсутствует #include <math.h>:\n{source}");
    }

    #[test]
    fn test_generate_source_with_const_and_port() {
        let src = r#"
type u8 = [bit;8];
const LIMIT: u8 = 100;
port SENSOR: u8 = 0x100000;
start Main { always { } }
        "#;
        let (model_ast, _) = parse(src, 0).unwrap();
        let model_rc = semantic::tree::construct_model(&model_ast, None, &[]).unwrap();
        model_rc.borrow_mut().name = Some("Main".to_string());
        let model = model_rc.borrow();
        let map = CMap::new(model.name(), &*model).unwrap();
        let source = generate_source(map.get_filename(), &map).unwrap();
        assert!(source.contains("CONST_MAIN_LIMIT"), "CONST_MAIN_LIMIT отсутствует:\n{source}");
        assert!(source.contains("PORT_MAIN_SENSOR"), "PORT_MAIN_SENSOR отсутствует:\n{source}");
    }

    #[test]
    fn test_generate_source_functions() {
        let src = r#"
extern fn log_val(x: bit);
fn double_it(x: bit) -> bit { return x; }
start Main { always { } }
        "#;
        let (model_ast, _) = parse(src, 0).unwrap();
        let model_rc = semantic::tree::construct_model(&model_ast, None, &[]).unwrap();
        model_rc.borrow_mut().name = Some("Main".to_string());
        let model = model_rc.borrow();
        let map = CMap::new(model.name(), &*model).unwrap();
        let source = generate_source(map.get_filename(), &map).unwrap();
        assert!(source.contains("extern void log_val"), "extern fn отсутствует:\n{source}");
        assert!(source.contains("static int Main_double_it"), "local fn отсутствует:\n{source}");
    }
}
