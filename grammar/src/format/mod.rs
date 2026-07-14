//! Канонический форматтер `.lam` — ядро печати (фича 0024, ADR Option A).
//!
//! # Откуда печатаем
//!
//! Печать идёт **от АСД** (`ast::Model`), а не от семантического дерева:
//! `ModelNode` нормализует имена (`Root`, snake_case) и теряет исходную
//! раскладку — для форматтера непригоден. Семантический проход не запускается
//! вовсе, поэтому форматтер работает и на семантически некорректных, но
//! синтаксически валидных файлах (требование R1).
//!
//! # Почему скобки не пересчитываются
//!
//! `ast::Expression::Parenthesis` и `ast::Condition::Parenthesis` — **явные узлы**
//! АСД. Значит скобки, написанные автором, сохраняются как есть, а расставлять их
//! по приоритетам не нужно: это и делает печать семантически нейтральной
//! (требование R4) почти бесплатно.
//!
//! # Ошибка вместо тихой потери
//!
//! Непокрытый узел даёт [`FormatError`], а не пустую строку. Урок фичи 0025:
//! молчаливый пропуск неотличим от корректной работы. Там, где перечисление
//! помечено `#[non_exhaustive]` (например, `ast::Statement`), ветка `_`
//! вынужденная — она **отказывает**, а не печатает пустоту.

mod comments;
mod expr;
mod stmt;

use crate::parser::ast;

/// Единица отступа канонического стиля.
pub(crate) const INDENT: &str = "    ";

/// Ошибка форматирования.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum FormatError {
    /// Исходник не разобран (форматировать нечего).
    Parse(String),
    /// Узел АСД пока не поддержан печатью.
    ///
    /// Явный отказ: печатать «что-нибудь» вместо неизвестного узла означало бы
    /// молча портить исходник пользователя.
    Unsupported(String),
}

impl std::fmt::Display for FormatError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            FormatError::Parse(msg) => write!(f, "не удалось разобрать исходник: {msg}"),
            FormatError::Unsupported(node) => {
                write!(f, "печать узла '{node}' пока не поддерживается форматтером")
            }
        }
    }
}

impl std::error::Error for FormatError {}

/// Накопитель канонического текста с отступами и курсором комментариев.
pub(crate) struct Out<'a> {
    buf: String,
    depth: usize,
    comments: comments::Comments<'a>,
}

impl<'a> Out<'a> {
    fn new(source: &'a str, items: &[ast::Comment]) -> Self {
        Self {
            buf: String::with_capacity(source.len() + 64),
            depth: 0,
            comments: comments::Comments::new(source, items),
        }
    }

    /// Печатает строку, **привязанную к узлу**: сперва ведущие комментарии узла,
    /// затем сама строка, затем хвостовой комментарий той же строки исходника.
    ///
    /// Это единственный путь печати содержательных строк — так комментарий не
    /// может «потеряться» из-за того, что о нём забыли в конкретной ветке.
    pub(crate) fn node_line(&mut self, loc: &crate::diagnostics::Location, text: &str) {
        let Some((start, end)) = comments::span(loc) else {
            self.line(text);
            return;
        };
        for c in self.comments.leading(start) {
            self.line(&c);
        }
        match self.comments.trailing(end) {
            Some(trailing) => self.line(&format!("{text} {trailing}")),
            None => self.line(text),
        }
    }

    /// Ведущие комментарии перед узлом (для блоков, где строка печатается сама).
    pub(crate) fn leading_for(&mut self, loc: &crate::diagnostics::Location) {
        let Some((start, _)) = comments::span(loc) else {
            return;
        };
        for c in self.comments.leading(start) {
            self.line(&c);
        }
    }

    pub(crate) fn up(&mut self) {
        self.depth += 1;
    }

    pub(crate) fn down(&mut self) {
        self.depth = self.depth.saturating_sub(1);
    }

    /// Печатает строку с текущим отступом и переводом строки.
    pub(crate) fn line(&mut self, text: &str) {
        if !text.is_empty() {
            for _ in 0..self.depth {
                self.buf.push_str(INDENT);
            }
            self.buf.push_str(text);
        }
        self.buf.push('\n');
    }

    fn finish(mut self) -> String {
        // Канон: ровно один перевод строки в конце файла.
        while self.buf.ends_with("\n\n") {
            self.buf.pop();
        }
        if !self.buf.is_empty() && !self.buf.ends_with('\n') {
            self.buf.push('\n');
        }
        self.buf
    }
}

/// Форматирует исходник `.lam` в канонический вид.
///
/// Комментарии сохраняются: они переассоциируются по `Location` (см.
/// [`comments`]) и печатаются как ведущие/хвостовые. Ни один не теряется —
/// требование R2.
pub fn format_source(source: &str) -> Result<String, FormatError> {
    let (model, items) =
        crate::parse(source, 0).map_err(|d| FormatError::Parse(format!("{d:?}")))?;
    let mut out = Out::new(source, &items);
    print_model_body(&mut out, &model)?;
    // Хвост файла: комментарии после последнего узла.
    for c in out.comments.rest() {
        out.line(&c);
    }
    // Страховка R2: ни один комментарий не должен остаться непогашенным.
    debug_assert!(
        out.comments.is_exhausted(),
        "комментарий потерян при печати — нарушено требование R2"
    );
    Ok(out.finish())
}

/// Позиция элемента модели.
///
/// У `ModelElement` нет `loc()` (в отличие от `Expression`/`Condition`),
/// поэтому извлекаем из вложенного узла — иначе комментарии не к чему привязать.
fn element_loc(element: &ast::ModelElement) -> crate::diagnostics::Location {
    use ast::ModelElement as M;
    match element {
        M::StraySemicolon(loc) => *loc,
        M::Variable(v) => variable_loc(v),
        M::Type(t) => t.loc,
        M::State(s) => s.loc,
        M::Model(m) => m.loc,
        M::NamedBlockCode(b) => b.loc,
        M::Enum(e) => e.loc,
        M::Struct(s) => s.loc,
        M::Function(f) => f.loc,
        M::Import(i) => import_loc(i),
        M::Formula(f) => f.loc,
        M::Condition(c) => c.loc,
        M::InlineFormula(f) => inline_formula_loc(f),
        M::Address(a) => a.loc,
    }
}

fn variable_loc(v: &ast::VariableDefine) -> crate::diagnostics::Location {
    use ast::VariableDefine as V;
    match v {
        V::Variable { loc, .. } | V::Port { loc, .. } | V::Constant { loc, .. } => *loc,
    }
}

fn import_loc(i: &ast::ImportDefine) -> crate::diagnostics::Location {
    use ast::ImportDefine as I;
    match i {
        I::Plain(_, loc) | I::GlobalSymbol(_, _, loc) | I::Rename(_, _, loc) => *loc,
    }
}

#[allow(clippy::wildcard_enum_match_arm)]
fn inline_formula_loc(f: &ast::InlineFormulaDefine) -> crate::diagnostics::Location {
    // `InlineFormulaDefine` — перечисление; печать его пока не поддержана,
    // позиция нужна лишь для сообщения об отказе.
    let _ = f;
    crate::diagnostics::Location::Builtin
}

/// Печатает элементы модели верхнего уровня (без обёртки `model … { }`).
fn print_model_body(out: &mut Out, model: &ast::Model) -> Result<(), FormatError> {
    for element in &model.elements {
        print_element(out, element)?;
    }
    if let Some(implements) = &model.implements {
        out.line(&format!("= {};", expr::expression(implements)?));
    }
    Ok(())
}

fn print_element(out: &mut Out, element: &ast::ModelElement) -> Result<(), FormatError> {
    let loc = element_loc(element);
    // Ведущие комментарии печатаются для ЛЮБОГО элемента — единая точка, чтобы
    // ни одна ветка не забыла про них (требование R2).
    out.leading_for(&loc);
    match element {
        ast::ModelElement::StraySemicolon(_) => {
            // Одиночная `;` — синтаксический шум; канон её опускает.
            Ok(())
        }
        ast::ModelElement::Variable(v) => {
            out.node_line(&loc, &format!("{};", expr::variable_define(v)?));
            Ok(())
        }
        ast::ModelElement::Type(t) => {
            out.node_line(
                &loc,
                &format!("type {} = {};", t.name.name, expr::ty(&t.ty)?),
            );
            Ok(())
        }
        ast::ModelElement::State(s) => print_state(out, s),
        ast::ModelElement::Model(m) => print_nested_model(out, m),
        ast::ModelElement::NamedBlockCode(b) => print_named_block(out, b),
        ast::ModelElement::Enum(e) => {
            let variants = e
                .variants
                .iter()
                .map(|v| match v.value {
                    Some(value) => format!("{} = {value}", v.name.name),
                    None => v.name.name.clone(),
                })
                .collect::<Vec<_>>()
                .join(", ");
            let name = e.name.as_ref().map(|n| n.name.as_str()).unwrap_or("");
            out.node_line(&loc, &format!("enum {name} {{ {variants} }}"));
            Ok(())
        }
        ast::ModelElement::Struct(s) => {
            let name = s.name.as_ref().map(|n| n.name.as_str()).unwrap_or("");
            out.line(&format!("struct {name} {{"));
            out.up();
            for (i, field) in s.fields.iter().enumerate() {
                let comma = if i + 1 < s.fields.len() { "," } else { "" };
                out.line(&format!(
                    "{}: {}{comma}",
                    field.name.name,
                    expr::ty(&field.ty)?
                ));
            }
            out.down();
            out.line("}");
            Ok(())
        }
        ast::ModelElement::Import(i) => {
            out.node_line(&loc, &expr::import(i)?);
            Ok(())
        }
        ast::ModelElement::Function(f) => print_function(out, f),
        // Узлы, печать которых относится к последующим задачам фичи.
        ast::ModelElement::Formula(_) => Err(FormatError::Unsupported("Formula".to_string())),
        ast::ModelElement::Condition(c) => {
            // `cond Имя = условие;` — печатается ПЕЧАТЬЮ УСЛОВИЙ, а не выражений:
            // `=` здесь равенство (инвариант ADR 0019).
            let name = c.name.as_ref().map(|n| n.name.as_str()).unwrap_or("");
            out.node_line(
                &loc,
                &format!("cond {name} = {};", expr::condition(&c.value)?),
            );
            Ok(())
        }
        ast::ModelElement::InlineFormula(_) => {
            Err(FormatError::Unsupported("InlineFormula".to_string()))
        }
        ast::ModelElement::Address(a) => {
            // `address ИМЯ = <выражение>;` (фича 0020).
            let name = a.name.as_ref().map(|n| n.name.as_str()).unwrap_or("");
            out.node_line(
                &loc,
                &format!("address {name} = {};", expr::expression(&a.value)?),
            );
            Ok(())
        }
    }
}

fn print_nested_model(out: &mut Out, model: &ast::Model) -> Result<(), FormatError> {
    let name = model.name.as_ref().map(|n| n.name.as_str()).unwrap_or("");
    match &model.implements {
        // `model M = выражение;` — компоновка без тела.
        Some(implements) if model.elements.is_empty() => {
            out.line(&format!(
                "model {name} = {};",
                expr::expression(implements)?
            ));
            Ok(())
        }
        _ => {
            out.line(&format!("model {name} {{"));
            out.up();
            for element in &model.elements {
                print_element(out, element)?;
            }
            out.down();
            out.line("}");
            Ok(())
        }
    }
}

fn print_state(out: &mut Out, state: &ast::StateDefine) -> Result<(), FormatError> {
    let kind = match state.kind {
        Some(ast::StateKind::Start) => "start ",
        Some(ast::StateKind::End) => "end ",
        Some(ast::StateKind::Next) => "next ",
        None => "state ",
    };
    let name = state.name.as_ref().map(|n| n.name.as_str()).unwrap_or("");

    // `start S = Модель;` / `start S;` — состояние без тела.
    if state.elements.is_empty() {
        return match &state.implements {
            Some(implements) => {
                out.line(&format!(
                    "{kind}{name} = {};",
                    expr::expression(implements)?
                ));
                Ok(())
            }
            None => {
                out.line(&format!("{kind}{name};"));
                Ok(())
            }
        };
    }

    let head = match &state.implements {
        Some(implements) => format!("{kind}{name} = {} {{", expr::expression(implements)?),
        None => format!("{kind}{name} {{"),
    };
    out.line(&head);
    out.up();
    for element in &state.elements {
        print_state_element(out, element)?;
    }
    out.down();
    out.line("}");
    Ok(())
}

fn print_state_element(out: &mut Out, element: &ast::StateElement) -> Result<(), FormatError> {
    match element {
        ast::StateElement::StraySemicolon(_) => Ok(()),
        ast::StateElement::Next(id) => {
            out.line(&format!("next {};", id.name));
            Ok(())
        }
        ast::StateElement::Reference(_, id, cond) => {
            match cond {
                Some(cond) => out.line(&format!("ref {}: {};", id.name, expr::condition(cond)?)),
                None => out.line(&format!("ref {};", id.name)),
            }
            Ok(())
        }
        ast::StateElement::NamedBlockCode(b) => print_named_block(out, b),
        ast::StateElement::InlineFormula(_) => {
            Err(FormatError::Unsupported("InlineFormula".to_string()))
        }
    }
}

fn print_named_block(out: &mut Out, block: &ast::NamedBlockCodeDefine) -> Result<(), FormatError> {
    let name = block.name.as_ref().map(|n| n.name.as_str()).unwrap_or("");
    stmt::block_with_head(out, &format!("{name} "), &block.statement)
}

fn print_function(out: &mut Out, func: &ast::FunctionDefine) -> Result<(), FormatError> {
    let name = func.name.as_ref().map(|n| n.name.as_str()).unwrap_or("");
    let params = expr::parameter_list(&func.params)?;
    let ret = match &func.return_type {
        Some(ty) => format!(" -> {}", expr::ty(ty)?),
        None => String::new(),
    };
    match &func.body {
        // `extern fn f(...);` — объявление без тела.
        None => {
            out.line(&format!("extern fn {name}({params}){ret};"));
            Ok(())
        }
        Some(body) => stmt::block_with_head(out, &format!("fn {name}({params}){ret} "), body),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn formats_minimal_model() {
        let out = format_source("start   S;").unwrap();
        assert_eq!(out, "start S;\n");
    }

    #[test]
    fn normalises_variable_declaration() {
        let out = format_source("var   x :u8:=0;").unwrap();
        assert_eq!(out, "var x: u8 := 0;\n");
    }

    #[test]
    fn indents_state_body() {
        let out = format_source("state A { ref B: x > 1; }").unwrap();
        assert_eq!(out, "state A {\n    ref B: x > 1;\n}\n");
    }

    #[test]
    fn leading_comment_is_printed_before_node() {
        // Задача 0024-02: комментарии сохраняются. Раньше здесь стоял тест,
        // проверявший ОТКАЗ на комментариях (ограничение задачи 0024-01);
        // ограничение снято — тест устарел по замыслу и заменён.
        let out = format_source("// заметка\nstart S;").unwrap();
        assert_eq!(out, "// заметка\nstart S;\n");
    }

    #[test]
    fn trailing_comment_stays_on_its_line() {
        let out = format_source("var x: u8 := 0; // счётчик").unwrap();
        assert_eq!(out, "var x: u8 := 0; // счётчик\n");
    }

    #[test]
    fn file_ends_with_single_newline() {
        let out = format_source("start S;").unwrap();
        assert!(out.ends_with(";\n"));
        assert!(!out.ends_with("\n\n"));
    }
}
