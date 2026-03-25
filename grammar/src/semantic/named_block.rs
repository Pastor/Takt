use crate::diagnostics::Diagnostic;
use crate::semantic::statement::resolve_statement;
use crate::semantic::{ModelNode, NamedCodeBlock, Statement};
use std::cell::RefCell;
use std::rc::Rc;

pub fn resolve_named_blocks(
    named_blocks: Vec<NamedCodeBlock>,
    model: Rc<RefCell<ModelNode>>,
) -> Result<Vec<NamedCodeBlock>, Diagnostic> {
    let mut blocks = Vec::with_capacity(named_blocks.len());
    for nb in named_blocks {
        let block = match nb {
            NamedCodeBlock::None => return Err("Statement должен быть определен".into()),
            NamedCodeBlock::Unresolved(name, stmt) => {
                let stmt = resolve_statement(&Statement::Unresolved(stmt), model.clone())?;
                match name.as_str() {
                    "enter" => NamedCodeBlock::Enter(stmt),
                    "exit" => NamedCodeBlock::Exit(stmt),
                    "always" => NamedCodeBlock::Always(stmt),
                    name => NamedCodeBlock::Unknown(name.to_string(), stmt),
                }
            }
            NamedCodeBlock::Enter(stmt) => {
                NamedCodeBlock::Enter(resolve_statement(&stmt, model.clone())?)
            }
            NamedCodeBlock::Exit(stmt) => {
                NamedCodeBlock::Exit(resolve_statement(&stmt, model.clone())?)
            }
            NamedCodeBlock::Always(stmt) => {
                NamedCodeBlock::Always(resolve_statement(&stmt, model.clone())?)
            }
            NamedCodeBlock::Unknown(name, stmt) => {
                NamedCodeBlock::Unknown(name.clone(), resolve_statement(&stmt, model.clone())?)
            }
        };
        blocks.push(block);
    }
    Ok(blocks)
}
