//! Проверки АСД, которым семантика не нужна: лишняя `;` (`SE-044`) и именованный
//! блок с неизвестным именем (`SE-045`).
//!
//! Обе смотрят на дерево разбора, а не на построенную модель: опечатку в имени блока
//! семантика отбрасывает раньше, чем её можно было бы назвать.

use crate::diagnostics::Diagnostic;
use crate::diagnostics::lang::keys;
use crate::msg;
use crate::parser::ast;

/// SE-044: предупреждения о лишних точках с запятой в АСД модели.
///
/// Обходит все элементы модели и состояний (рекурсивно), генерируя предупреждение для
/// каждого [`ast::ModelElement::StraySemicolon`] и
/// [`ast::StateElement::StraySemicolon`].
pub fn stray_semicolon_warnings(model: &ast::Model) -> Vec<Diagnostic> {
    let mut diags = Vec::new();
    collect_stray_semicolons_model(model, &mut diags);
    diags
}

/// SE-045: предупреждения об именованных блоках с неизвестным именем.
///
/// Допустимые имена: `enter`, `exit`, `always`. Любое другое имя генерирует
/// предупреждение - вероятнее всего это опечатка.
pub fn unknown_named_block_warnings(model: &ast::Model) -> Vec<Diagnostic> {
    let mut diags = Vec::new();
    collect_unknown_named_blocks_model(model, &mut diags);
    diags
}

const KNOWN_NAMED_BLOCKS: &[&str] = &["enter", "exit", "always"];

fn collect_unknown_named_blocks_model(model: &ast::Model, out: &mut Vec<Diagnostic>) {
    for element in &model.elements {
        match element {
            ast::ModelElement::NamedBlockCode(def) => {
                check_named_block_def(def, out);
            }
            ast::ModelElement::State(state) => {
                for se in &state.elements {
                    if let ast::StateElement::NamedBlockCode(def) = se {
                        check_named_block_def(def, out);
                    }
                }
            }
            ast::ModelElement::Model(nested) => {
                collect_unknown_named_blocks_model(nested, out);
            }
            _ => {}
        }
    }
}

fn check_named_block_def(def: &ast::NamedBlockCodeDefine, out: &mut Vec<Diagnostic>) {
    if let Some(name_id) = &def.name
        && !KNOWN_NAMED_BLOCKS.contains(&name_id.name.as_str())
    {
        out.push(
            Diagnostic::warning(
                name_id.loc,
                msg!(keys::SE_045_UNKNOWN_NAMED_BLOCK, name = name_id.name),
            )
            .with_code("SE-045"),
        );
    }
}

fn collect_stray_semicolons_model(model: &ast::Model, out: &mut Vec<Diagnostic>) {
    for element in &model.elements {
        match element {
            ast::ModelElement::StraySemicolon(loc) => {
                out.push(
                    Diagnostic::warning(*loc, msg!(keys::SE_044_STRAY_SEMICOLON))
                        .with_code("SE-044"),
                );
            }
            ast::ModelElement::State(state) => {
                for se in &state.elements {
                    if let ast::StateElement::StraySemicolon(loc) = se {
                        out.push(
                            Diagnostic::warning(*loc, msg!(keys::SE_044_STRAY_SEMICOLON))
                                .with_code("SE-044"),
                        );
                    }
                }
            }
            ast::ModelElement::Model(nested) => {
                collect_stray_semicolons_model(nested, out);
            }
            _ => {}
        }
    }
}
