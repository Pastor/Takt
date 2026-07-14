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

/// Накопитель канонического текста с отступами.
pub(crate) struct Out {
    buf: String,
    depth: usize,
}

impl Out {
    fn new() -> Self {
        Self {
            buf: String::with_capacity(1024),
            depth: 0,
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
/// Комментарии на этом шаге **не печатаются** — их переассоциация по `Location`
/// вынесена в задачу `0024-02` (см. анализ фичи). До неё функция отказывает на
/// входе с комментариями, а не молча их теряет.
pub fn format_source(source: &str) -> Result<String, FormatError> {
    let (model, comments) =
        crate::parse(source, 0).map_err(|d| FormatError::Parse(format!("{d:?}")))?;
    if !comments.is_empty() {
        return Err(FormatError::Unsupported(
            "комментарии (печать — задача 0024-02)".to_string(),
        ));
    }
    let mut out = Out::new();
    print_model_body(&mut out, &model)?;
    Ok(out.finish())
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
    match element {
        ast::ModelElement::StraySemicolon(_) => {
            // Одиночная `;` — синтаксический шум; канон её опускает.
            Ok(())
        }
        ast::ModelElement::Variable(v) => {
            out.line(&format!("{};", expr::variable_define(v)?));
            Ok(())
        }
        ast::ModelElement::Type(t) => {
            out.line(&format!("type {} = {};", t.name.name, expr::ty(&t.ty)?));
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
            out.line(&format!("enum {name} {{ {variants} }}"));
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
            out.line(&expr::import(i)?);
            Ok(())
        }
        ast::ModelElement::Function(f) => print_function(out, f),
        // Узлы, печать которых относится к последующим задачам фичи.
        ast::ModelElement::Formula(_) => Err(FormatError::Unsupported("Formula".to_string())),
        ast::ModelElement::Condition(_) => {
            Err(FormatError::Unsupported("ConditionDefine".to_string()))
        }
        ast::ModelElement::InlineFormula(_) => {
            Err(FormatError::Unsupported("InlineFormula".to_string()))
        }
        ast::ModelElement::Address(_) => Err(FormatError::Unsupported("Address".to_string())),
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
    fn comments_are_refused_not_dropped() {
        // Задача 0024-02: пока комментарии не печатаются — отказываем явно,
        // потому что молча потерять комментарий пользователя недопустимо.
        let err = format_source("// заметка\nstart S;").unwrap_err();
        assert!(matches!(err, FormatError::Unsupported(_)), "{err:?}");
    }

    #[test]
    fn file_ends_with_single_newline() {
        let out = format_source("start S;").unwrap();
        assert!(out.ends_with(";\n"));
        assert!(!out.ends_with("\n\n"));
    }
}
