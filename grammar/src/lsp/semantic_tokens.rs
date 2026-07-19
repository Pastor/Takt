//! Семантическая подсветка.
//!
//! Часть модуля `lsp` (фича 0027: деление по логике).

use super::*;
use crate::semantic::ModelNode;
use std::cell::RefCell;
use std::collections::HashSet;
use std::rc::Rc;

/// Имена, объявленные во **всём** дереве моделей файла, сгруппированные по
/// категории подсветки (фикс [0038-01]: классификация обязана видеть члены
/// под-моделей, а не только корня).
///
/// Функции `ModelNode::search_*` ищут в текущей модели и идут **вверх** по
/// `upper` — это верно для разрешения области видимости из вложенной точки, но
/// `semantic_tokens` строит **корневую** модель файла, и «вверх» от неё ничего
/// не находит. Поэтому здесь собираем имена спуском **вниз** по `models`.
#[derive(Default)]
struct DeclNames {
    /// Имена функций (`fn`).
    functions: HashSet<String>,
    /// Имена типов: псевдонимы (`type`) и перечисления (`enum`).
    types: HashSet<String>,
    /// Имена вариантов перечислений.
    variants: HashSet<String>,
    /// Имена состояний и моделей (категория `CLASS`).
    classes: HashSet<String>,
}

/// Собирает имена деклараций всего дерева моделей (корень + все под-модели).
/// Разделяемые под-модели (композиция) обходятся однократно (набор посещённых
/// по указателю) — защита от повторной работы и от циклов.
fn collect_decl_names(root: &Rc<RefCell<ModelNode>>) -> DeclNames {
    let mut names = DeclNames::default();
    let mut visited: HashSet<*const RefCell<ModelNode>> = HashSet::new();
    walk_decls(root, &mut names, &mut visited);
    names
}

fn walk_decls(
    model: &Rc<RefCell<ModelNode>>,
    names: &mut DeclNames,
    visited: &mut HashSet<*const RefCell<ModelNode>>,
) {
    if !visited.insert(Rc::as_ptr(model)) {
        return;
    }
    let m = model.borrow();
    for k in m.functions.keys() {
        names.functions.insert(k.clone());
    }
    for k in m.types.keys() {
        names.types.insert(k.clone());
    }
    for (k, e) in &m.enums {
        names.types.insert(k.clone());
        for (variant, _) in &e.variants {
            names.variants.insert(variant.clone());
        }
    }
    for k in m.states.keys() {
        names.classes.insert(k.clone());
    }
    for (k, child) in &m.models {
        names.classes.insert(k.clone());
        walk_decls(child, names, visited);
    }
}

/// Генерирует семантические токены для подсветки синтаксиса документа.
///
/// Использует лексер Lam для токенизации и семантическую модель для уточнения
/// типов идентификаторов (функции, типы, состояния, варианты перечислений и т.д.).
/// Результат передаётся редактору в ответ на `textDocument/semanticTokens/full`.
pub fn semantic_tokens(source: &str) -> SemanticTokens {
    use crate::ast::Comment;
    use crate::diagnostics::Location;
    use crate::parser::lexer::{Lexer, Token};

    // Строим семантическую модель для обогащения идентификаторов и собираем
    // имена деклараций всего дерева (корень + под-модели, фикс 0038-01).
    let model_opt = crate::parse(source, 0)
        .ok()
        .and_then(|(ast, _)| semantic::tree::construct_model(&ast, None, &[]).ok());
    let decl_names = model_opt.as_ref().map(collect_decl_names);

    // Собираем токены и комментарии через лексер
    let mut comments: Vec<Comment> = Vec::new();
    let mut lex_errors = Vec::new();
    let token_results: Vec<_> = Lexer::new(source, 0, &mut comments, &mut lex_errors).collect();

    let mut raw: Vec<(usize, usize, u32)> = Vec::new();

    for (start, token, end) in token_results {
        let tt = match token {
            Token::Identifier(name) => {
                // Встроенные типы имеют приоритет над пользовательскими именами
                // Встроенные типы имеют приоритет над пользовательскими именами.
                // Далее — членство в декларациях всего дерева (фикс 0038-01), в том
                // же порядке приоритета, что и прежний поиск по корню:
                // функция → тип → вариант enum → состояние/модель → иначе переменная.
                if BUT_BUILTIN_TYPES.iter().any(|(t, _)| *t == name) {
                    TT_TYPE
                } else if let Some(ref n) = decl_names {
                    if n.functions.contains(name) {
                        TT_FUNCTION
                    } else if n.types.contains(name) {
                        TT_TYPE
                    } else if n.variants.contains(name) {
                        TT_ENUM_MEMBER
                    } else if n.classes.contains(name) {
                        TT_CLASS
                    } else {
                        TT_VARIABLE
                    }
                } else {
                    TT_VARIABLE
                }
            }
            Token::Model
            | Token::State
            | Token::Start
            | Token::Variable
            | Token::Constant
            | Token::PortIn
            | Token::PortOut
            | Token::PortInOut
            | Token::Address
            | Token::Function
            | Token::Extern
            | Token::Enum
            | Token::Struct
            | Token::Type
            | Token::Loop
            | Token::While
            | Token::Match
            | Token::Wildcard
            | Token::Continue
            | Token::Break
            | Token::Return
            | Token::If
            | Token::Else
            | Token::For
            | Token::Import
            | Token::As
            | Token::From
            | Token::Assembly
            | Token::Formula
            | Token::Condition
            | Token::Next
            | Token::Reference
            | Token::Template
            | Token::Pragma
            | Token::True
            | Token::False
            | Token::String => TT_KEYWORD,
            Token::Number(_) | Token::RationalNumber(..) | Token::AddressLiteral(_) => TT_NUMBER,
            Token::StringLiteral(..) => TT_STRING,
            Token::Equal
            | Token::NotEqual
            | Token::Assign
            | Token::ColonAssign
            | Token::Add
            | Token::Subtract
            | Token::Mul
            | Token::Divide
            | Token::Modulo
            | Token::Power
            | Token::And
            | Token::Or
            | Token::Not
            | Token::BitwiseAnd
            | Token::BitwiseOr
            | Token::BitwiseXor
            | Token::BitwiseNot
            | Token::ShiftLeft
            | Token::ShiftRight
            | Token::Less
            | Token::LessEqual
            | Token::More
            | Token::MoreEqual
            | Token::PeirceArrow
            | Token::Member => TT_OPERATOR,
            // Пунктуация и прочее — не подсвечиваем
            _ => continue,
        };
        raw.push((start, end, tt));
    }

    // Добавляем комментарии (лексер накапливает их отдельно, не как токены)
    for comment in &comments {
        let loc = match comment {
            Comment::Line(loc, _) | Comment::DocLine(loc, _) | Comment::Block(loc, _) => loc,
        };
        if let Location::Source(_, start, end) = loc {
            raw.push((*start as usize, *end as usize, TT_COMMENT));
        }
    }

    // Сортируем по байтовому смещению
    raw.sort_unstable_by_key(|&(s, _, _)| s);

    // Кодируем в дельта-формат LSP SemanticTokens
    let mut data = Vec::with_capacity(raw.len());
    let mut prev_line = 0u32;
    let mut prev_start = 0u32;

    for (start, end, tt) in raw {
        // LSP требует длину токена в кодовых единицах UTF-16, а не в байтах.
        // Для ASCII (большинство идентификаторов Lam) оба значения совпадают;
        // различие возникает для кириллицы, CJK и прочих многобайтовых символов.
        let length: u32 = if end > start
            && end <= source.len()
            && source.is_char_boundary(start)
            && source.is_char_boundary(end)
        {
            source[start..end]
                .chars()
                .map(|c| c.len_utf16() as u32)
                .sum()
        } else {
            end.saturating_sub(start) as u32
        };
        if length == 0 {
            continue;
        }
        let pos = offset_to_position(source, start);
        let delta_line = pos.line - prev_line;
        let delta_start = if delta_line == 0 {
            pos.character.saturating_sub(prev_start)
        } else {
            pos.character
        };
        data.push(SemanticToken {
            delta_line,
            delta_start,
            length,
            token_type: tt,
            token_modifiers_bitset: 0,
        });
        prev_line = pos.line;
        prev_start = pos.character;
    }

    SemanticTokens {
        result_id: None,
        data,
    }
}
