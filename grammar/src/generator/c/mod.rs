use crate::diagnostics::{Diagnostic, Location};
use crate::generator::Generator as AsGenerator;
use crate::generator::indent::Printer;
use crate::semantic::naming::{normalize_lowercase_snakecase, normalize_model_name};
use crate::semantic::{Expression, ModelNode, TypeNode, VariableNode};
use std::fs;
use std::path::Path;

pub struct Generator {}

impl AsGenerator for Generator {
    fn generate(&self, model: &ModelNode, output_path: &str) -> Result<(), Diagnostic> {
        let header = self.generate_header(model)?;
        let filename =
            normalize_lowercase_snakecase(model.name.clone().unwrap_or("unknown".to_string()));
        let _ = fs::create_dir(Path::new(output_path));
        fs::write(Path::new(output_path).join(filename + ".h"), header)
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
            + &*normalize_lowercase_snakecase(model.name.clone().unwrap_or("unknown".to_string()))
                .to_uppercase()
    }

    #[inline]
    fn get_model_name_struct(model: &ModelNode) -> String {
        normalize_model_name(model.name.clone().unwrap().as_str())
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
                format!("uint32_t /*enum {}*/ {}", enum_name, name.clone().unwrap_or_default())
            }
            TypeNode::BuiltinModel
            | TypeNode::BuiltinState
            | TypeNode::Unsupported
            | TypeNode::Inference
            | TypeNode::Address(_, _) => {
                panic!("Unsupported type")
            }
        }
    }

    fn generate_model_enum(mut printer: &mut Printer, model: &ModelNode) -> Result<(), Diagnostic> {
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
        mut printer: &mut Printer,
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
            Self::generate_model_enum(&mut printer, &*model);
        }
        //Variables
        {
            for (name, var) in model.variables.clone().iter() {
                match var {
                    VariableNode::Unresolved => {}
                    VariableNode::Simple { ty, expr, .. } => {
                        printer.ident(Self::get_typed_variable(ty, Some(name.clone())).as_str());
                        if let Expression::None = expr {
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
                .print(
                    normalize_lowercase_snakecase(
                        model.name.clone().unwrap_or("unknown".to_string()),
                    )
                    .as_str(),
                )
                .print(";");
        }
        printer.nl();
        Ok(())
    }

    fn generate_header(&self, model: &ModelNode) -> Result<String, Diagnostic> {
        let mut header = String::new();
        let mut printer = Printer::new(4, &mut header);
        let mut id =
            normalize_lowercase_snakecase(model.name.clone().unwrap_or("unknown".to_string()))
                .to_uppercase()
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
}
