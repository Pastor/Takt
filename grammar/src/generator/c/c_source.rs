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

use super::{
    Generator, FUNCTION_PORT_READ_BIT, FUNCTION_PORT_READ_FLOAT, FUNCTION_PORT_WRITE_BIT,
    FUNCTION_PORT_WRITE_FLOAT, get_typed_variable,
};
use crate::diagnostics::{Diagnostic, Location};
use crate::generator::indent::Printer;
use crate::parser::ast::Member;
use crate::semantic::extend::Extend;
use crate::semantic::naming::{normalize_camelcase_name, normalize_lowercase_snakecase};
use crate::semantic::type_node::TypeNode;
use crate::semantic::{
    ConditionDefinitionNode, ConditionNode, EnumDefinitionNode, ExpressionNode, ModelNode,
    StateNode, StateNodeKind, VariableNode,
};
use itertools::Itertools;
use log::warn;

impl Generator {
    /// Возвращает префикс имени модели в UPPER_SNAKE_CASE, включая цепочку родителей.
    ///
    /// Используется при построении имён констант, портов и перечислений.
    /// Пример: `Robot::Idle` → `"ROBOT_IDLE_"`.
    #[inline]
    pub(super) fn get_upper_name(model: &ModelNode) -> String {
        model
            .upper
            .clone()
            .and_then(|weak| weak.upgrade())
            .map(|rc| Self::get_upper_name(&rc.borrow()) + "_")
            .unwrap_or_default()
            + &*normalize_lowercase_snakecase(
                Self::resolve_model_name(model).unwrap_or_else(|_| "Unknown".to_string()),
            )
            .to_uppercase()
    }

    /// Возвращает имя модели в CamelCase для использования как имя C-структуры.
    #[allow(dead_code)]
    #[inline]
    pub(super) fn get_model_name_struct(model: &ModelNode) -> String {
        normalize_camelcase_name(
            Self::resolve_model_name(model)
                .unwrap_or_else(|_| "Unknown".to_string())
                .as_str(),
        )
    }

    /// Генерирует enum-список состояний модели в C.
    #[allow(dead_code)]
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

    /// Генерирует вложенные struct-поля для каждого implement-состояния модели.
    #[allow(dead_code)]
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

    /// Генерирует typedef struct для модели, рекурсивно обходя вложенные модели.
    #[allow(dead_code)]
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
        // Состояния
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
        // Переменные
        {
            for (name, var) in model.variables.clone().iter() {
                match var {
                    VariableNode::Unresolved => {}
                    VariableNode::Simple { ty, expr, .. } => {
                        let typed_variable = get_typed_variable(ty, Some(name.clone()), model);
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

    /// Генерирует `#define`-константы для портов, констант и перечислений модели.
    ///
    /// Устарело: логика перенесена в заголовочный файл; оставлено для совместимости.
    #[deprecated]
    #[allow(dead_code)]
    fn generate_constants_and_ports_and_enums(
        printer: &mut Printer,
        model: &ModelNode,
    ) -> Result<(), Diagnostic> {
        let upper_name = Self::get_upper_name(model);

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

    /// Генерирует вызов реализации для implement-состояния.
    #[allow(dead_code)]
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

    /// Заготовка генерации тела функции `_tick` (задача I1).
    #[allow(dead_code)]
    fn generate_model_tick_source(
        _printer: &Printer,
        _model: &ModelNode,
        _main: bool,
    ) -> Result<(), Diagnostic> {
        //TODO: реализовать генерацию тела _tick (I1)
        Ok(())
    }

    /// Генерирует источник (`.c`) для одной модели.
    #[allow(dead_code)]
    fn generate_model_source(
        printer: &Printer,
        model: &ModelNode,
        main: bool,
    ) -> Result<(), Diagnostic> {
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

    /// Генерирует содержимое `.c`-файла для модели.
    ///
    /// Не вызывается напрямую: генерация `.c`-файлов отложена до реализации I1–I4.
    /// Код сохранён для будущей доработки.
    #[allow(dead_code, deprecated)]
    pub(super) fn generate_source(&self, model: &ModelNode) -> Result<String, Diagnostic> {
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

    /// Разворачивает путь доступа к модели в C-выражение.
    ///
    /// Корневая модель → `"main"`, вложенная → `"main->robot.idle"`.
    pub(super) fn unroll_model(model: &ModelNode) -> Result<String, Diagnostic> {
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

    /// Разворачивает переменную в C-выражение.
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

    /// Определяет строковое имя модели: использует `model.name`, иначе — имя стартового состояния.
    #[inline]
    pub(super) fn resolve_model_name(model: &ModelNode) -> Result<String, Diagnostic> {
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

    /// Разворачивает имя implement-цепочки в строку через `_`.
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
