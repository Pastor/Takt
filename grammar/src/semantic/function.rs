use crate::diagnostics::Diagnostic;
use crate::parser::ast;
use crate::semantic::statement::resolve_statement;
use crate::semantic::type_::construct_type;
use crate::semantic::{FunctionNode, ModelNode, Statement, TypeNode};
use std::cell::RefCell;
use std::rc::Rc;

pub fn construct_function(
    func: FunctionNode,
    model: Rc<RefCell<ModelNode>>,
) -> Result<FunctionNode, Diagnostic> {
    if let FunctionNode::Unresolved(def) = func {
        let name = def
            .clone()
            .name
            .ok_or_else(|| "При определении функция должна иметь имя".into())?
            .name
            .clone();
        if model.borrow().functions.contains_key(&name) {
            Err(format!("Функция с именем '{}' уже определена", name)
                .as_str()
                .into())
        } else {
            let mut params = Vec::new();
            for (_, param) in def.params.iter() {
                if let Some(param) = param {
                    if let ast::Expression::Type(_, typ) = param.clone().ty {
                        params.push((
                            param
                                .clone()
                                .name
                                .map(|t| t.name.clone())
                                .unwrap_or_default(),
                            construct_type(Some(typ), &model.borrow().types)?,
                        ));
                    } else {
                        return Err("Параметр функции должен иметь тип".into());
                    }
                }
            }
            let rett = match def.return_type {
                Some(t) => construct_type(Some(t), &model.borrow().types).map_err(|e| e)?,
                None => TypeNode::Unit,
            };
            if def.external {
                Ok(FunctionNode::External(name.clone(), params, rett))
            } else {
                let statement = if let Some(body) = def.body {
                    resolve_statement(&Statement::Unresolved(body), model.clone())?
                } else {
                    return Err("Локальная функция должна иметь тело".into());
                };
                Ok(FunctionNode::Local(name.clone(), params, rett, statement))
            }
        }
    } else if let FunctionNode::None = func {
        Err("Функция не определена".into())
    } else {
        Ok(func)
    }
}
