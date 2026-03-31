use crate::diagnostics::{Diagnostic, Location};
use crate::generator::Generator as AsGenerator;
use crate::generator::indent::Printer;
use crate::semantic::naming::{normalize_lowercase_snakecase, normalize_model_name};
use crate::semantic::type_node::TypeNode;
use crate::semantic::{
    ConditionDefinitionNode, ExpressionNode, Implement, ModelNode, StateNode, VariableNode,
};
use itertools::Itertools;
use std::fs;
use std::path::Path;

pub struct Generator {}

impl AsGenerator for Generator {
    fn generate(&self, model: &ModelNode, output_path: &str) -> Result<(), Diagnostic> {
        let header = self.generate_header(model)?;
        let source = self.generate_source(model)?;
        let model_name = Self::determinate_model_name(model)?;
        let filename = normalize_lowercase_snakecase(model_name);
        let _ = fs::create_dir(Path::new(output_path));
        fs::write(Path::new(output_path).join(filename.clone() + ".h"), header)
            .map_err(|e| Diagnostic::warning(Location::Codegen, format!("{:?}", e)))?;
        fs::write(Path::new(output_path).join(filename + ".c"), source)
            .map_err(|e| Diagnostic::warning(Location::Codegen, format!("{:?}", e)))?;
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
            + &*normalize_lowercase_snakecase(Self::determinate_model_name(model).unwrap())
                .to_uppercase()
    }

    #[inline]
    fn get_model_name_struct(model: &ModelNode) -> String {
        normalize_model_name(Self::determinate_model_name(model).unwrap().as_str())
    }

    fn get_typed_variable(typ: &TypeNode, name: Option<String>) -> String {
        match typ {
            TypeNode::Bit => format!("int {}", name.clone().unwrap_or_default()),
            TypeNode::Bool => format!("bool {}", name.clone().unwrap_or_default()),
            TypeNode::Rational => format!("float {}", name.clone().unwrap_or_default()),
            TypeNode::Array(size, typ) => {
                if let TypeNode::Rational = **typ {
                    format!("float {}[{}]", name.clone().unwrap().as_str(), *size)
                } else {
                    format!("u_int{}_t {}", *size, name.clone().unwrap().as_str())
                }
            }
            TypeNode::Unit => "void".to_string(),
            TypeNode::BuiltinString => "char *".to_string(),
            // Ce4: перечисление представляется как uint32_t в C
            TypeNode::Enum(enum_name) => {
                format!(
                    "uint32_t /*enum {}*/ {}",
                    enum_name,
                    name.clone().unwrap_or_default()
                )
            }
            // NI3: структурный тип представляется как struct в C
            TypeNode::Struct(struct_name) => {
                format!(
                    "struct {} {}",
                    struct_name,
                    name.clone().unwrap_or_default()
                )
            }
            TypeNode::BuiltinModel
            | TypeNode::BuiltinState
            | TypeNode::BuiltinNumeric
            | TypeNode::Unsupported
            | TypeNode::Inference
            | TypeNode::Address(_, _) => {
                panic!("Unsupported type")
            }
        }
    }

    fn generate_model_states(
        #[allow(unused_mut)] mut printer: &mut Printer,
        model: &ModelNode,
    ) -> Result<(), Diagnostic> {
        printer.ident("enum {").nl().up();
        let upper = Self::get_upper_name(model);
        printer.ident(&upper).print("_INIT");
        for name in model.states.keys().clone() {
            let name = normalize_lowercase_snakecase(name.clone()).to_uppercase();
            let name = format!("{}_{}", &upper, name);
            printer.print(",").nl().ident(&name);
        }
        printer.down().nl().ident("} state;").nl();
        Ok(())
    }

    fn generate_model_struct(
        #[allow(unused_mut)] mut printer: &mut Printer,
        model: &ModelNode,
        first_model: bool,
    ) -> Result<(), Diagnostic> {
        printer.ident("struct ");
        if first_model {
            printer
                .print(Self::get_model_name_struct(model).as_str())
                .print(" ");
        }
        printer.print("{").nl().up();
        //Models
        {
            for (_, model) in model.models.clone().iter() {
                Self::generate_model_struct(&mut printer, &*model.borrow(), false)?;
            }
        }
        //States
        {
            let _ = Self::generate_model_states(&mut printer, &*model);
        }
        //Variables
        {
            for (name, var) in model.variables.clone().iter() {
                match var {
                    VariableNode::Unresolved => {}
                    VariableNode::Simple { ty, expr, .. } => {
                        printer.ident(Self::get_typed_variable(ty, Some(name.clone())).as_str());
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
        }
        printer.down().ident("}");
        if first_model {
            printer.print(";");
        } else {
            printer
                .print(" ")
                .print(normalize_lowercase_snakecase(Self::determinate_model_name(model)?).as_str())
                .print(";");
        }
        printer.nl();
        Ok(())
    }

    fn generate_constants_and_ports(
        printer: &mut Printer,
        model: &ModelNode,
    ) -> Result<(), Diagnostic> {
        let upper_name = Self::get_upper_name(model);
        for (name, model) in model.models.iter() {
            Self::generate_constants_and_ports(printer, &model.borrow())?
        }

        let variables = model.variables.clone();
        if !variables.is_empty() {
            printer.print("/* Constant & Ports */").nl();
        }
        for var in variables
            .into_values()
            .sorted_by(|a, b| a.name().cmp(b.name()))
        {
            match var.clone() {
                VariableNode::Unresolved => {}
                VariableNode::Simple { .. } => {}
                VariableNode::Port { name, expr, .. } => {
                    let name = Self::determinate_raw_name(upper_name.clone(), name)?;

                    let (address, bit) = if let ExpressionNode::Address(address, bit) = expr {
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
                    let name = Self::determinate_raw_name(upper_name.clone(), name)?;
                    printer.print("#define CONST_").print(&name).nl();
                }
            }
        }
        for state in model
            .states
            .clone()
            .into_values()
            .sorted_by(|a, b| a.name().cmp(b.name()))
        {}

        let conditions = model.conditions.clone();
        if !conditions.is_empty() {
            printer.print("/* Conditions */").nl();
        }
        for cond in conditions
            .into_values()
            .sorted_by(|a, b| a.name().cmp(b.name()))
        {
            printer
                .print("#define COND_")
                .print(&Self::determinate_cond_name(upper_name.clone(), &cond)?)
                .nl();
        }
        Ok(())
    }

    fn generate_source(&self, model: &ModelNode) -> Result<String, Diagnostic> {
        let mut source = String::new();
        let mut printer = Printer::new(4, &mut source);
        let filename = normalize_lowercase_snakecase(Self::determinate_model_name(model)?);
        printer
            .print(format!("#include \"{}.h\" ", filename).as_str())
            .nl();
        Self::generate_constants_and_ports(&mut printer, model)?;
        printer.nl();
        Ok(source)
    }

    fn generate_header(&self, model: &ModelNode) -> Result<String, Diagnostic> {
        let mut header = String::new();
        let mut printer = Printer::new(4, &mut header);
        let id = normalize_lowercase_snakecase(Self::determinate_model_name(model)?).to_uppercase()
            + "__";
        printer.print("#ifndef ").print(&id).nl();
        printer.print("#define ").print(&id).nl();
        printer.print("#include <stdint.h>").nl();
        printer.nl();
        Self::generate_model_struct(&mut printer, model, true)?;
        let struct_name = Self::get_model_name_struct(model);
        printer
            .print("void ")
            .print(&struct_name)
            .print("_init(struct ")
            .print(&struct_name)
            .print(" *main);")
            .nl();
        printer
            .print("void ")
            .print(&struct_name)
            .print("_tick(struct ")
            .print(&struct_name)
            .print(" *main);")
            .nl();
        printer
            .print("void ")
            .print(&struct_name)
            .print("_reset(struct ")
            .print(&struct_name)
            .print(" *main);")
            .nl();
        printer.print("#endif").nl();
        Ok(header)
    }

    #[inline]
    fn determinate_raw_name(upper_name: String, name: String) -> Result<String, Diagnostic> {
        Ok(
            (upper_name.to_owned() + "_" + normalize_lowercase_snakecase(name.clone()).as_str())
                .to_uppercase(),
        )
    }

    #[inline]
    fn determinate_cond_name(
        upper_name: String,
        cond: &ConditionDefinitionNode,
    ) -> Result<String, Diagnostic> {
        Self::determinate_raw_name(upper_name, cond.name.clone())
    }

    #[inline]
    fn determinate_model_name(model: &ModelNode) -> Result<String, Diagnostic> {
        let name = model.name.clone().unwrap_or("Unknown".to_string());
        if !name.eq("Unknown") {
            Ok(name)
        } else if model.has_states()
            && let Some(state) = model.get_start_state()
        {
            match state.clone() {
                StateNode::Unresolved | StateNode::Simple { .. } => Ok(state.name().to_string()),
                StateNode::Implement { implements, .. } => {
                    Self::determinate_implement_name(implements)
                }
            }
        } else {
            Ok(name)
        }
    }

    #[inline]
    fn determinate_implement_name(implement: Implement) -> Result<String, Diagnostic> {
        match implement {
            Implement::None => Ok("".to_string()),
            Implement::Unresolved(_) => Ok("Unresolved".to_string()),
            Implement::Model(model) => Self::determinate_model_name(&model.borrow()),
            Implement::Parentless(i) => Self::determinate_implement_name(*i),
            Implement::Add(left, right) | Implement::Or(left, right) => {
                Ok(Self::determinate_implement_name(*left)?
                    + "_"
                    + &*Self::determinate_implement_name(*right)?)
            }
        }
    }
}
