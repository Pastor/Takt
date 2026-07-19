//! Символы документа (структура файла).
//!
//! Часть модуля `lsp` (фича 0027: деление по логике).

use super::*;

/// Возвращает символы документа (outline) для отображения в панели структуры.
///
/// Заменяет функциональность `outline.scm` без использования tree-sitter.
/// Обходит AST и формирует иерархию символов: модели, состояния, функции,
/// типы, условия, перечисления и переменные.
pub fn document_symbols(source: &str) -> Vec<DocumentSymbol> {
    use crate::parser::ast::Model;
    let ast: Model = match crate::parse(source, 0) {
        Ok((ast, _)) => ast,
        Err(_) => return vec![],
    };
    symbols_from_model(&ast, source)
}

fn loc_to_range(loc: &crate::diagnostics::Location, source: &str) -> Range {
    match loc {
        crate::diagnostics::Location::Source(_, start, end) => {
            offset_to_range(source, *start as usize, *end as usize)
        }
        _ => Range {
            start: Position::new(0, 0),
            end: Position::new(0, 0),
        },
    }
}

#[allow(deprecated)]
fn make_sym(
    name: String,
    kind: SymbolKind,
    range: Range,
    selection_range: Range,
    children: Option<Vec<DocumentSymbol>>,
) -> DocumentSymbol {
    DocumentSymbol {
        name,
        detail: None,
        kind,
        tags: None,
        deprecated: None,
        range,
        selection_range,
        children,
    }
}

fn symbols_from_model(model: &crate::parser::ast::Model, source: &str) -> Vec<DocumentSymbol> {
    use crate::parser::ast::{ModelElement, StateElement, VariableDefine};

    let mut out: Vec<DocumentSymbol> = Vec::new();

    for elem in &model.elements {
        match elem {
            ModelElement::Model(m) => {
                let id = match m.name.as_ref() {
                    Some(id) => id,
                    None => continue,
                };
                let children = symbols_from_model(m, source);
                out.push(make_sym(
                    id.name.clone(),
                    SymbolKind::MODULE,
                    loc_to_range(&m.loc, source),
                    loc_to_range(&id.loc, source),
                    if children.is_empty() {
                        None
                    } else {
                        Some(children)
                    },
                ));
            }
            ModelElement::State(s) => {
                let id = match s.name.as_ref() {
                    Some(id) => id,
                    None => continue,
                };
                let children: Vec<DocumentSymbol> = s
                    .elements
                    .iter()
                    .filter_map(|e| match e {
                        StateElement::NamedBlockCode(nb) => {
                            let nb_id = nb.name.as_ref()?;
                            Some(make_sym(
                                nb_id.name.clone(),
                                SymbolKind::EVENT,
                                loc_to_range(&nb.loc, source),
                                loc_to_range(&nb_id.loc, source),
                                None,
                            ))
                        }
                        _ => None,
                    })
                    .collect();
                out.push(make_sym(
                    id.name.clone(),
                    SymbolKind::CLASS,
                    loc_to_range(&s.loc, source),
                    loc_to_range(&id.loc, source),
                    if children.is_empty() {
                        None
                    } else {
                        Some(children)
                    },
                ));
            }
            ModelElement::Function(f) => {
                let id = match f.name.as_ref() {
                    Some(id) => id,
                    None => continue,
                };
                out.push(make_sym(
                    id.name.clone(),
                    SymbolKind::FUNCTION,
                    loc_to_range(&f.loc, source),
                    loc_to_range(&id.loc, source),
                    None,
                ));
            }
            ModelElement::Type(t) => {
                out.push(make_sym(
                    t.name.name.clone(),
                    SymbolKind::TYPE_PARAMETER,
                    loc_to_range(&t.loc, source),
                    loc_to_range(&t.name.loc, source),
                    None,
                ));
            }
            ModelElement::Condition(c) => {
                let id = match c.name.as_ref() {
                    Some(id) => id,
                    None => continue,
                };
                out.push(make_sym(
                    id.name.clone(),
                    SymbolKind::CONSTANT,
                    loc_to_range(&c.loc, source),
                    loc_to_range(&id.loc, source),
                    None,
                ));
            }
            // 0044: инвариант индексируется как именованный символ (бонус к hover/goto).
            ModelElement::Invariant(i) => {
                let id = match i.name.as_ref() {
                    Some(id) => id,
                    None => continue,
                };
                out.push(make_sym(
                    id.name.clone(),
                    SymbolKind::CONSTANT,
                    loc_to_range(&i.loc, source),
                    loc_to_range(&id.loc, source),
                    None,
                ));
            }
            ModelElement::Enum(e) => {
                let id = match e.name.as_ref() {
                    Some(id) => id,
                    None => continue,
                };
                let children: Vec<DocumentSymbol> = e
                    .variants
                    .iter()
                    .map(|v| {
                        make_sym(
                            v.name.name.clone(),
                            SymbolKind::ENUM_MEMBER,
                            loc_to_range(&v.loc, source),
                            loc_to_range(&v.name.loc, source),
                            None,
                        )
                    })
                    .collect();
                out.push(make_sym(
                    id.name.clone(),
                    SymbolKind::ENUM,
                    loc_to_range(&e.loc, source),
                    loc_to_range(&id.loc, source),
                    if children.is_empty() {
                        None
                    } else {
                        Some(children)
                    },
                ));
            }
            ModelElement::Variable(v) => {
                let (loc, name_opt, kind) = match v.as_ref() {
                    VariableDefine::Variable { loc, name, .. } => (loc, name, SymbolKind::VARIABLE),
                    VariableDefine::Port { loc, name, .. } => (loc, name, SymbolKind::PROPERTY),
                    VariableDefine::Constant { loc, name, .. } => (loc, name, SymbolKind::CONSTANT),
                };
                let id = match name_opt {
                    Some(id) => id,
                    None => continue,
                };
                out.push(make_sym(
                    id.name.clone(),
                    kind,
                    loc_to_range(loc, source),
                    loc_to_range(&id.loc, source),
                    None,
                ));
            }
            ModelElement::NamedBlockCode(nb) => {
                let id = match nb.name.as_ref() {
                    Some(id) => id,
                    None => continue,
                };
                out.push(make_sym(
                    id.name.clone(),
                    SymbolKind::EVENT,
                    loc_to_range(&nb.loc, source),
                    loc_to_range(&id.loc, source),
                    None,
                ));
            }
            ModelElement::Import(_)
            | ModelElement::Formula(_)
            | ModelElement::Address(_)
            | ModelElement::StraySemicolon(_) => {}
            ModelElement::Struct(def) => {
                let children: Vec<DocumentSymbol> = def
                    .fields
                    .iter()
                    .map(|v| {
                        make_sym(
                            v.name.name.clone(),
                            SymbolKind::FIELD,
                            loc_to_range(&v.loc, source),
                            loc_to_range(&v.name.loc, source),
                            None,
                        )
                    })
                    .collect();
                out.push(make_sym(
                    def.name.clone().unwrap().name.clone(),
                    SymbolKind::STRUCT,
                    loc_to_range(&def.loc, source),
                    loc_to_range(&def.loc, source),
                    if children.is_empty() {
                        None
                    } else {
                        Some(children)
                    },
                ));
            }
            ModelElement::InlineFormula(_) => {}
        }
    }

    out
}
