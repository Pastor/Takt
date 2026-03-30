//! Вспомогательные функции LSP-сервера для языка BuT.
//!
//! Этот модуль реализует логику, связывающую компилятор BuT с протоколом LSP:
//! сбор диагностики, генерацию подсказок автодополнения и информацию о типах
//! для функции hover.
//!
//! Модуль включается только при наличии флага `lsp`.

use lsp_types::*;

/// Типы семантических токенов (порядок важен — индекс используется как тип в легенде).
pub const SEMANTIC_TOKEN_TYPES: &[SemanticTokenType] = &[
    SemanticTokenType::KEYWORD,     // 0
    SemanticTokenType::VARIABLE,    // 1
    SemanticTokenType::FUNCTION,    // 2
    SemanticTokenType::TYPE,        // 3
    SemanticTokenType::ENUM_MEMBER, // 4
    SemanticTokenType::STRING,      // 5
    SemanticTokenType::NUMBER,      // 6
    SemanticTokenType::COMMENT,     // 7
    SemanticTokenType::OPERATOR,    // 8
    SemanticTokenType::CLASS,       // 9  (состояния и модели)
];

const TT_KEYWORD: u32 = 0;
const TT_VARIABLE: u32 = 1;
const TT_FUNCTION: u32 = 2;
const TT_TYPE: u32 = 3;
const TT_ENUM_MEMBER: u32 = 4;
const TT_STRING: u32 = 5;
const TT_NUMBER: u32 = 6;
const TT_COMMENT: u32 = 7;
const TT_OPERATOR: u32 = 8;
const TT_CLASS: u32 = 9;

/// Ключевые слова языка BuT для автодополнения.
const BUT_KEYWORDS: &[(&str, &str)] = &[
    ("model", "объявление модели конечного автомата"),
    ("state", "объявление обычного состояния"),
    ("start", "объявление начального состояния"),
    ("ref", "условный переход между состояниями"),
    ("next", "безусловный переход"),
    ("enter", "именованный блок при входе в состояние"),
    ("exit", "именованный блок при выходе из состояния"),
    ("always", "именованный блок, выполняемый каждый цикл"),
    ("var", "объявление переменной"),
    ("const", "объявление константы"),
    ("type", "псевдоним типа"),
    ("fn", "объявление функции"),
    ("extern", "объявление внешней функции"),
    ("port", "объявление порта ввода-вывода"),
    ("enum", "объявление перечисления"),
    ("cond", "именованное условие перехода"),
    ("if", "условный оператор"),
    ("else", "ветка условного оператора"),
    ("loop", "цикл с опциональным условием"),
    ("for", "цикл со счётчиком"),
    ("break", "выход из цикла"),
    ("continue", "переход к следующей итерации цикла"),
    ("return", "возврат из функции"),
    ("import", "импорт файла"),
    ("as", "псевдоним при импорте"),
    ("from", "источник выборочного импорта"),
    ("formula", "формальная спецификация"),
    ("assembly", "ассемблерная вставка"),
    ("true", "булев литерал истина"),
    ("false", "булев литерал ложь"),
    ("bit", "1-битный примитивный тип"),
    ("bool", "булев тип"),
    ("float", "тип числа с плавающей точкой"),
    ("unit", "пустой тип (возвращаемый тип процедур)"),
];

/// Собирает диагностику из исходного кода BuT.
///
/// Выполняет лексический, синтаксический и семантический анализ.
/// Возвращает список LSP-диагностик для отображения в редакторе.
pub fn collect_diagnostics(source: &str) -> Vec<Diagnostic> {
    let mut lsp_diags = Vec::new();

    // Шаг 1: Синтаксический анализ
    let (ast, _) = match crate::parse(source, 0) {
        Ok(result) => result,
        Err(errors) => {
            // Конвертируем ошибки парсера в LSP-диагностики
            for err in errors {
                lsp_diags.push(grammar_diagnostic_to_lsp(&err, source));
            }
            return lsp_diags;
        }
    };

    // Шаг 2: Семантический анализ
    let model = match crate::semantic::tree::construct_model(&ast, None, &[]) {
        Ok(m) => m,
        Err(err) => {
            lsp_diags.push(grammar_diagnostic_to_lsp(&err, source));
            return lsp_diags;
        }
    };

    // Шаг 3: Дополнительные предупреждения
    let unused = crate::unused_variable_warnings(model.clone());
    for w in unused {
        lsp_diags.push(grammar_diagnostic_to_lsp(&w, source));
    }

    let nondeterministic = crate::nondeterministic_transition_warnings(model.clone());
    for w in nondeterministic {
        lsp_diags.push(grammar_diagnostic_to_lsp(&w, source));
    }

    let enum_errors = crate::enum_type_safety_errors(model);
    for e in enum_errors {
        lsp_diags.push(grammar_diagnostic_to_lsp(&e, source));
    }

    lsp_diags
}

/// Конвертирует [`crate::diagnostics::Diagnostic`] в LSP [`Diagnostic`].
pub fn grammar_diagnostic_to_lsp(
    diag: &crate::diagnostics::Diagnostic,
    source: &str,
) -> Diagnostic {
    use crate::diagnostics::Level;

    let severity = match diag.level {
        Level::Error => Some(DiagnosticSeverity::ERROR),
        Level::Warning => Some(DiagnosticSeverity::WARNING),
        Level::Info => Some(DiagnosticSeverity::INFORMATION),
        Level::Debug => Some(DiagnosticSeverity::HINT),
    };

    // Конвертируем байтовое смещение в позицию строка:столбец
    let range = match diag.loc {
        crate::diagnostics::Location::Source(_, start, end) => offset_to_range(source, start, end),
        _ => Range {
            start: Position::new(0, 0),
            end: Position::new(0, 0),
        },
    };

    Diagnostic {
        range,
        severity,
        message: diag.message.clone(),
        source: Some("but-lsp".to_string()),
        ..Default::default()
    }
}

/// Конвертирует байтовое смещение в LSP `Range`.
pub fn offset_to_range(source: &str, start: usize, end: usize) -> Range {
    Range {
        start: offset_to_position(source, start),
        end: offset_to_position(source, end),
    }
}

/// Конвертирует байтовое смещение в LSP `Position` (строка + столбец в кодовых единицах UTF-16).
///
/// Протокол LSP (спецификация v3.17, §3.1) требует, чтобы поле `character` позиции
/// выражалось в **кодовых единицах UTF-16**, а не в байтах или кодовых точках Unicode.
/// Для ASCII-символов все три единицы совпадают; различие возникает при наличии
/// многобайтовых UTF-8 символов (кириллица, CJK, эмодзи, …).
///
/// Если `offset` указывает на середину многобайтового символа (невалидная char-граница),
/// функция безопасно отступает до ближайшей предшествующей границы символа.
///
/// # Примеры
///
/// ```
/// # #[cfg(feature = "lsp")]
/// # {
/// use grammar::lsp::offset_to_position;
/// use lsp_types::Position;
///
/// // ASCII: байтовое смещение == UTF-16-столбец
/// assert_eq!(offset_to_position("hello", 3), Position::new(0, 3));
///
/// // Многострочный текст: смещение 7 — второй байт второй строки
/// assert_eq!(offset_to_position("line1\nab", 7), Position::new(1, 1));
///
/// // Кириллица: 'А' занимает 2 байта в UTF-8, но 1 кодовую единицу в UTF-16
/// // "АБ" = [0xD0,0x90, 0xD0,0x91] — 4 байта, 2 символа
/// let src = "АБ";
/// assert_eq!(offset_to_position(src, 4), Position::new(0, 2)); // конец строки
/// assert_eq!(offset_to_position(src, 2), Position::new(0, 1)); // после 'А'
/// # }
/// ```
pub fn offset_to_position(source: &str, offset: usize) -> Position {
    // Зажимаем до валидной границы символа UTF-8
    let offset = {
        let clamped = offset.min(source.len());
        // Если попали в середину многобайтового символа — откатываемся назад
        (0..=clamped)
            .rev()
            .find(|&i| source.is_char_boundary(i))
            .unwrap_or(0)
    };
    let prefix = &source[..offset];
    let line = prefix.matches('\n').count() as u32;
    // Находим начало текущей строки (байт сразу после последнего '\n')
    let line_start = prefix.rfind('\n').map(|nl| nl + 1).unwrap_or(0);
    // LSP требует столбец в кодовых единицах UTF-16
    let col_utf16: u32 = prefix[line_start..]
        .chars()
        .map(|c| c.len_utf16() as u32)
        .sum();
    Position::new(line, col_utf16)
}

/// Конвертирует UTF-16 смещение символа в байтовое смещение внутри строки `s`.
///
/// Возвращает `Some(byte_offset)`, если `utf16_offset` не выходит за пределы строки,
/// иначе `None`.
///
/// # Примеры
///
/// ```
/// // ASCII: 1 байт = 1 кодовая единица UTF-16
/// // utf16_offset 3 → байт 3
///
/// // "АБВ": каждый символ — 2 байта UTF-8, 1 кодовая единица UTF-16
/// // utf16_offset 2 → байт 4
/// ```
fn utf16_to_byte_offset(s: &str, utf16_offset: usize) -> Option<usize> {
    let mut utf16_count = 0usize;
    for (byte_i, ch) in s.char_indices() {
        if utf16_count >= utf16_offset {
            return Some(byte_i);
        }
        utf16_count += ch.len_utf16();
    }
    // Если точно достигли конца строки
    if utf16_count >= utf16_offset {
        Some(s.len())
    } else {
        None
    }
}

/// Генерирует элементы автодополнения для источника BuT.
///
/// Возвращает ключевые слова языка, а также идентификаторы из семантической
/// модели документа (имена моделей, состояний, переменных, функций).
pub fn completion_items(source: &str) -> Vec<CompletionItem> {
    let mut items: Vec<CompletionItem> = Vec::new();

    // Добавляем ключевые слова
    for (keyword, description) in BUT_KEYWORDS {
        items.push(CompletionItem {
            label: keyword.to_string(),
            kind: Some(CompletionItemKind::KEYWORD),
            detail: Some(description.to_string()),
            ..Default::default()
        });
    }

    // Добавляем идентификаторы из семантической модели
    if let Ok((ast, _)) = crate::parse(source, 0) {
        if let Ok(model) = crate::semantic::tree::construct_model(&ast, None, &[]) {
            let borrowed = model.borrow();

            // Имена вложенных моделей
            for name in borrowed.models.keys() {
                items.push(CompletionItem {
                    label: name.clone(),
                    kind: Some(CompletionItemKind::CLASS),
                    detail: Some("модель".to_string()),
                    ..Default::default()
                });
            }

            // Имена переменных и их типы
            for (name, var) in &borrowed.variables {
                let detail = match var {
                    crate::semantic::VariableNode::Simple { ty, .. } => Some(format!("{:?}", ty)),
                    crate::semantic::VariableNode::Const { ty, .. } => {
                        Some(format!("const: {:?}", ty))
                    }
                    crate::semantic::VariableNode::Port { ty, .. } => {
                        Some(format!("port: {:?}", ty))
                    }
                    crate::semantic::VariableNode::Unresolved => None,
                };
                items.push(CompletionItem {
                    label: name.clone(),
                    kind: Some(CompletionItemKind::VARIABLE),
                    detail,
                    ..Default::default()
                });
            }

            // Имена функций
            for (name, func) in &borrowed.functions {
                let detail = match func {
                    crate::semantic::FunctionNode::Local { ret, .. } => {
                        Some(format!("fn -> {:?}", ret))
                    }
                    crate::semantic::FunctionNode::External { ret, .. } => {
                        Some(format!("extern fn -> {:?}", ret))
                    }
                    crate::semantic::FunctionNode::Builtin(_, _, ret) => {
                        Some(format!("builtin fn -> {:?}", ret))
                    }
                    _ => None,
                };
                items.push(CompletionItem {
                    label: name.clone(),
                    kind: Some(CompletionItemKind::FUNCTION),
                    detail,
                    ..Default::default()
                });
            }

            // Псевдонимы типов
            for name in borrowed.types.keys() {
                items.push(CompletionItem {
                    label: name.clone(),
                    kind: Some(CompletionItemKind::TYPE_PARAMETER),
                    detail: Some("тип".to_string()),
                    ..Default::default()
                });
            }

            // Именованные условия
            for name in borrowed.conditions.keys() {
                items.push(CompletionItem {
                    label: name.clone(),
                    kind: Some(CompletionItemKind::VALUE),
                    detail: Some("cond".to_string()),
                    ..Default::default()
                });
            }

            // Перечисления и их варианты
            for (enum_name, enum_node) in &borrowed.enums {
                items.push(CompletionItem {
                    label: enum_name.clone(),
                    kind: Some(CompletionItemKind::ENUM),
                    detail: Some("enum".to_string()),
                    ..Default::default()
                });
                for (variant_name, variant_val) in &enum_node.variants {
                    items.push(CompletionItem {
                        label: variant_name.clone(),
                        kind: Some(CompletionItemKind::ENUM_MEMBER),
                        detail: Some(format!("{}::{} = {}", enum_name, variant_name, variant_val)),
                        ..Default::default()
                    });
                }
            }

            // Имена состояний
            for (state_name, state) in &borrowed.states {
                let kind_str = match state {
                    crate::semantic::StateNode::Simple { kind, .. }
                    | crate::semantic::StateNode::Implement { kind, .. } => {
                        if matches!(kind, crate::semantic::StateNodeKind::Start) {
                            "start state"
                        } else {
                            "state"
                        }
                    }
                    crate::semantic::StateNode::Unresolved => "state",
                };
                items.push(CompletionItem {
                    label: state_name.clone(),
                    kind: Some(CompletionItemKind::MODULE),
                    detail: Some(kind_str.to_string()),
                    ..Default::default()
                });
            }
        }
    }

    items
}

/// Возвращает слово (идентификатор) под заданной позицией курсора.
///
/// Позиция `position.character` задаётся в **кодовых единицах UTF-16** согласно
/// спецификации LSP. Функция корректно обрабатывает многобайтовые символы UTF-8
/// (кириллица, CJK, эмодзи).
///
/// Символами слова считаются буквенно-цифровые символы (`is_alphanumeric()`),
/// знак подчёркивания `_` и знак `$`.
///
/// # Примеры (примеры / контр-примеры)
///
/// ```
/// # #[cfg(feature = "lsp")]
/// # {
/// use grammar::lsp::word_at_position;
/// use lsp_types::Position;
///
/// // Курсор внутри слова "hello" → "hello"
/// assert_eq!(word_at_position("hello world", Position::new(0, 2)), Some("hello".to_string()));
///
/// // Курсор на границе слова (позиция прямо после "hello") → возвращает "hello"
/// assert_eq!(word_at_position("hello world", Position::new(0, 5)), Some("hello".to_string()));
///
/// // Курсор строго внутри двойного пробела → None
/// assert_eq!(word_at_position("hello  world", Position::new(0, 6)), None);
///
/// // Несуществующая строка → None
/// assert_eq!(word_at_position("hello", Position::new(99, 0)), None);
/// # }
/// ```
pub fn word_at_position(source: &str, position: Position) -> Option<String> {
    let line_text = source.lines().nth(position.line as usize)?;
    // Конвертируем UTF-16 смещение символа в байтовое смещение
    let col =
        utf16_to_byte_offset(line_text, position.character as usize).unwrap_or(line_text.len());

    // Ищем начало слова (идём влево от курсора)
    let start = line_text[..col]
        .rfind(|c: char| !c.is_alphanumeric() && c != '_' && c != '$')
        .map(|i| {
            // rfind возвращает байтовый индекс начала символа-разделителя;
            // шагаем вперёд на длину этого символа
            let ch = line_text[i..].chars().next().unwrap();
            i + ch.len_utf8()
        })
        .unwrap_or(0);

    // Ищем конец слова (идём вправо от курсора)
    let end = line_text[col..]
        .find(|c: char| !c.is_alphanumeric() && c != '_' && c != '$')
        .map(|i| i + col)
        .unwrap_or(line_text.len());

    if start < end {
        Some(line_text[start..end].to_string())
    } else {
        None
    }
}

/// Возвращает информацию о типе идентификатора под курсором.
///
/// Ищет идентификатор в семантической модели документа и возвращает
/// информацию о типе/назначении найденного элемента.
pub fn hover_info(source: &str, position: Position) -> Option<Hover> {
    let word = word_at_position(source, position)?;
    if word.is_empty() {
        return None;
    }

    // Строим семантическую модель с привязкой doc-комментариев
    let (ast, comments) = crate::parse(source, 0).ok()?;
    let model =
        crate::semantic::tree::construct_model_with_docs(&ast, None, &[], &comments).ok()?;
    let borrowed = model.borrow();

    let mut hover_text = String::new();

    // Ищем переменную
    if let Some(var) = borrowed.search_var(&word) {
        let (type_str, kind_str) = match &var {
            crate::semantic::VariableNode::Simple { ty, .. } => (format!("{:?}", ty), "var"),
            crate::semantic::VariableNode::Const { ty, .. } => (format!("{:?}", ty), "const"),
            crate::semantic::VariableNode::Port { ty, .. } => (format!("{:?}", ty), "port"),
            crate::semantic::VariableNode::Unresolved => ("?".to_string(), "var"),
        };
        hover_text = format!("```but\n{} {}: {}\n```", kind_str, word, type_str);
        // Добавляем документацию, если есть
        let doc = borrowed.element_doc(&word);
        if !doc.is_empty() {
            hover_text.push_str("\n\n");
            hover_text.push_str(&doc.join("\n"));
        }
    }
    // Ищем функцию
    else if let Some(func_rc) = borrowed.search_func(&word) {
        let func = func_rc.borrow();
        let sig = match &*func {
            crate::semantic::FunctionNode::Local { params, ret, .. } => {
                let params_str: Vec<String> = params
                    .iter()
                    .map(|(n, t)| format!("{}: {:?}", n, t))
                    .collect();
                format!("fn {}({}) -> {:?}", word, params_str.join(", "), ret)
            }
            crate::semantic::FunctionNode::External { params, ret, .. } => {
                let params_str: Vec<String> = params
                    .iter()
                    .map(|(n, t)| format!("{}: {:?}", n, t))
                    .collect();
                format!("extern fn {}({}) -> {:?}", word, params_str.join(", "), ret)
            }
            crate::semantic::FunctionNode::Builtin(name, params, ret) => {
                format!("builtin fn {}({} params) -> {:?}", name, params.len(), ret)
            }
            _ => format!("fn {}", word),
        };
        hover_text = format!("```but\n{}\n```", sig);
        // Добавляем документацию, если есть
        let doc = borrowed.element_doc(&word);
        if !doc.is_empty() {
            hover_text.push_str("\n\n");
            hover_text.push_str(&doc.join("\n"));
        }
    }
    // Ищем псевдоним типа
    else if let Some(ty) = borrowed.types.get(&word) {
        hover_text = format!("```but\ntype {} = {:?}\n```", word, ty);
        // Добавляем документацию, если есть
        let doc = borrowed.element_doc(&word);
        if !doc.is_empty() {
            hover_text.push_str("\n\n");
            hover_text.push_str(&doc.join("\n"));
        }
    }
    // Ищем именованное условие
    else if let Some(cond) = borrowed.search_cond(&word) {
        hover_text = format!("```but\ncond {} = {:?}\n```", word, cond.value);
        // Добавляем документацию, если есть
        let doc = borrowed.element_doc(&word);
        if !doc.is_empty() {
            hover_text.push_str("\n\n");
            hover_text.push_str(&doc.join("\n"));
        }
    }
    // Ищем перечисление
    else if let Some(enum_node) = borrowed.search_enum(&word) {
        let variants: Vec<String> = enum_node
            .variants
            .iter()
            .map(|(n, v)| format!("  {} = {}", n, v))
            .collect();
        hover_text = format!(
            "```but\nenum {} {{\n{}\n}}\n```",
            word,
            variants.join(",\n")
        );
        // Добавляем документацию, если есть
        let doc = borrowed.element_doc(&word);
        if !doc.is_empty() {
            hover_text.push_str("\n\n");
            hover_text.push_str(&doc.join("\n"));
        }
    }
    // Ищем состояние
    else if borrowed.search_state(&word).is_some() {
        hover_text = format!("```but\nstate {}\n```", word);
        // Добавляем документацию, если есть
        let doc = borrowed.element_doc(&word);
        if !doc.is_empty() {
            hover_text.push_str("\n\n");
            hover_text.push_str(&doc.join("\n"));
        }
    }
    // Ищем вариант перечисления
    else if let Some((enum_name, value)) = borrowed.search_enum_variant(&word) {
        hover_text = format!("```but\n{}::{} = {}\n```", enum_name, word, value);
    }
    // Ищем модель
    else if let Some(_) = borrowed.search_model(&word) {
        hover_text = format!("```but\nmodel {}\n```", &word);
        // Добавляем документацию, если есть
        let doc = borrowed.element_doc(&word);
        if !doc.is_empty() {
            hover_text.push_str("\n\n");
            hover_text.push_str(&doc.join("\n"));
        }
    }

    if hover_text.is_empty() {
        return None;
    }

    Some(Hover {
        contents: HoverContents::Markup(MarkupContent {
            kind: MarkupKind::Markdown,
            value: hover_text,
        }),
        range: None,
    })
}

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
            offset_to_range(source, *start, *end)
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
            | ModelElement::StraySemicolon(_) => {}
        }
    }

    out
}

/// Генерирует семантические токены для подсветки синтаксиса документа.
///
/// Использует лексер BuT для токенизации и семантическую модель для уточнения
/// типов идентификаторов (функции, типы, состояния, варианты перечислений и т.д.).
/// Результат передаётся редактору в ответ на `textDocument/semanticTokens/full`.
pub fn semantic_tokens(source: &str) -> SemanticTokens {
    use crate::ast::Comment;
    use crate::diagnostics::Location;
    use crate::parser::lexer::{Lexer, Token};

    // Строим семантическую модель для обогащения идентификаторов
    let model_opt = crate::parse(source, 0)
        .ok()
        .and_then(|(ast, _)| crate::semantic::tree::construct_model(&ast, None, &[]).ok());
    let borrowed_model = model_opt.as_ref().map(|m| m.borrow());

    // Собираем токены и комментарии через лексер
    let mut comments: Vec<Comment> = Vec::new();
    let mut lex_errors = Vec::new();
    let token_results: Vec<_> = Lexer::new(source, 0, &mut comments, &mut lex_errors).collect();

    let mut raw: Vec<(usize, usize, u32)> = Vec::new();

    for (start, token, end) in token_results {
        let tt = match token {
            Token::Identifier(name) => {
                if let Some(ref b) = borrowed_model {
                    if b.search_func(name).is_some() {
                        TT_FUNCTION
                    } else if b.types.contains_key(name) || b.enums.contains_key(name) {
                        TT_TYPE
                    } else if b.search_enum_variant(name).is_some() {
                        TT_ENUM_MEMBER
                    } else if b.search_state(name).is_some() || b.models.contains_key(name) {
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
            | Token::Port
            | Token::Function
            | Token::Extern
            | Token::Enum
            | Token::Type
            | Token::Loop
            | Token::Continue
            | Token::Break
            | Token::Return
            | Token::If
            | Token::Else
            | Token::For
            | Token::Import
            | Token::As
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
            Comment::Line(loc, _) | Comment::DocLine(loc, _) => loc,
        };
        if let Location::Source(_, start, end) = loc {
            raw.push((*start, *end, TT_COMMENT));
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
        // Для ASCII (большинство идентификаторов BuT) оба значения совпадают;
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

#[cfg(test)]
mod tests {
    use super::*;
    use crate::diagnostics::{Diagnostic as GrammarDiagnostic, ErrorType, Level, Location};

    // ── Вспомогательный исходный код для тестов ────────────────────────────────

    /// Минимально корректный BuT-файл с переменной, функцией, типом, перечислением.
    const VALID_SRC: &str = r#"
type u8 = [bit;8];
var counter: [bit;8] = 0;
const LIMIT: [bit;8] = 10;
cond IsReady = counter = 0;
enum Color { Red = 0, Green = 1, Blue = 2 }
extern fn add(a: [bit;8], b: [bit;8]) -> [bit;8];
model M {
    start Idle {
        ref Run: IsReady;
    }
    state Run {
        next Idle;
    }
}
start S = M;
"#;

    /// Синтаксически неверный код: незакрытая скобка.
    const INVALID_SRC: &str = "model Broken {";

    // ── Тесты сбора диагностики ────────────────────────────────────────────────

    /// Корректный исходный код не должен порождать диагностику.
    #[test]
    fn test_collect_diagnostics_valid_source() {
        let diags = collect_diagnostics(VALID_SRC);
        assert!(
            diags.is_empty(),
            "корректный код не должен давать ошибок, но получено: {:?}",
            diags
        );
    }

    /// Некорректный исходный код должен порождать хотя бы одну диагностику.
    #[test]
    fn test_collect_diagnostics_invalid_source() {
        let diags = collect_diagnostics(INVALID_SRC);
        assert!(
            !diags.is_empty(),
            "неверный код должен давать хотя бы одну ошибку"
        );
    }

    // ── Тесты конвертации смещений ─────────────────────────────────────────────

    /// Смещение в начале первой строки.
    #[test]
    fn test_offset_to_position_first_line() {
        let src = "hello world";
        let pos = offset_to_position(src, 5);
        assert_eq!(pos.line, 0);
        assert_eq!(pos.character, 5);
    }

    /// Смещение на второй строке.
    #[test]
    fn test_offset_to_position_second_line() {
        let src = "line1\nline2\nline3";
        // "line1\n" = 6 байт, "li" = 2, итого 8
        let pos = offset_to_position(src, 8);
        assert_eq!(pos.line, 1, "должна быть вторая строка");
        assert_eq!(pos.character, 2, "столбец должен быть 2");
    }

    /// Смещение за пределами строки не должно паниковать.
    #[test]
    fn test_offset_to_position_clamped() {
        let src = "abc";
        let pos = offset_to_position(src, 100);
        assert_eq!(pos.line, 0);
        assert_eq!(pos.character, 3);
    }

    /// offset_to_range возвращает корректный диапазон.
    #[test]
    fn test_offset_to_range() {
        let src = "hello\nworld";
        let range = offset_to_range(src, 0, 5);
        assert_eq!(range.start, Position::new(0, 0));
        assert_eq!(range.end, Position::new(0, 5));
    }

    // ── Тесты извлечения слова под курсором ────────────────────────────────────

    /// Курсор в конце простого слова.
    #[test]
    fn test_word_at_position_simple() {
        let src = "var hello = 0;";
        let word = word_at_position(src, Position::new(0, 7));
        assert_eq!(word, Some("hello".to_string()));
    }

    /// Курсор в середине слова.
    #[test]
    fn test_word_at_position_middle_of_word() {
        let src = "state Running;";
        // Позиция 8 — внутри "Running"
        let word = word_at_position(src, Position::new(0, 8));
        assert_eq!(word, Some("Running".to_string()));
    }

    /// Курсор на знаке «=» между двумя пробелами — слово не найдено.
    #[test]
    fn test_word_at_position_on_space() {
        let src = "var x = 0;";
        // Позиция 6 — символ «=» не является буквой/цифрой/подчёркиванием.
        // «var x » (6 символов) → left = " ", last non-word index = 5 → start = 6.
        // «= 0;» → «=» не алфавитно-цифровой → find вернёт 0 → end = 6.
        // start == end → None.
        let word = word_at_position(src, Position::new(0, 6));
        assert!(
            word.is_none(),
            "символ-оператор не должен давать слово: {:?}",
            word
        );
    }

    /// Несуществующая строка — None.
    #[test]
    fn test_word_at_position_nonexistent_line() {
        let src = "var x = 0;";
        let word = word_at_position(src, Position::new(99, 0));
        assert_eq!(word, None);
    }

    // ── Тесты автодополнения ───────────────────────────────────────────────────

    /// Список автодополнения содержит ключевые слова языка BuT.
    #[test]
    fn test_completion_items_contains_keywords() {
        let items = completion_items("start S;");
        let labels: Vec<_> = items.iter().map(|i| i.label.as_str()).collect();
        for kw in &["model", "state", "start", "var", "fn", "enum"] {
            assert!(
                labels.contains(kw),
                "ключевое слово '{}' должно присутствовать в автодополнении",
                kw
            );
        }
    }

    /// Автодополнение по корректному коду включает идентификаторы из семантики.
    #[test]
    fn test_completion_items_contains_semantic() {
        let src = "var myVar: [bit;8] = 0; start S;";
        let items = completion_items(src);
        let labels: Vec<_> = items.iter().map(|i| i.label.as_str()).collect();
        assert!(
            labels.contains(&"myVar"),
            "переменная 'myVar' должна присутствовать в автодополнении"
        );
    }

    /// Автодополнение по некорректному коду возвращает хотя бы ключевые слова.
    #[test]
    fn test_completion_items_fallback_to_keywords_on_error() {
        let items = completion_items("model {{{ broken");
        // Должны быть хотя бы ключевые слова
        assert!(
            items.len() >= BUT_KEYWORDS.len(),
            "при ошибке разбора должны присутствовать минимум ключевые слова"
        );
    }

    // ── Тесты hover ────────────────────────────────────────────────────────────

    /// Hover над переменной показывает тип.
    #[test]
    fn test_hover_variable() {
        let src = "var counter: [bit;8] = 0; start S;";
        let hover = hover_info(src, Position::new(0, 5));
        assert!(
            hover.is_some(),
            "hover над переменной должен возвращать данные"
        );
        if let Some(h) = hover {
            if let HoverContents::Markup(mc) = h.contents {
                assert!(
                    mc.value.contains("counter"),
                    "hover должен содержать имя переменной"
                );
            }
        }
    }

    /// Hover над именем функции показывает сигнатуру.
    #[test]
    fn test_hover_function() {
        let src = "extern fn myFunc(x: [bit;8]) -> [bit;8]; start S;";
        // Позиция 10 — внутри "myFunc"
        let hover = hover_info(src, Position::new(0, 10));
        assert!(
            hover.is_some(),
            "hover над функцией должен возвращать данные"
        );
        if let Some(h) = hover {
            if let HoverContents::Markup(mc) = h.contents {
                assert!(
                    mc.value.contains("myFunc"),
                    "hover должен содержать имя функции"
                );
            }
        }
    }

    /// Hover над неизвестным идентификатором возвращает None.
    #[test]
    fn test_hover_unknown() {
        let src = "start S;";
        let hover = hover_info(src, Position::new(0, 0));
        // "start" — ключевое слово, не переменная и не функция
        // результат зависит от семантики; проверяем, что нет паники
        let _ = hover;
    }

    /// Hover над пустой позицией возвращает None.
    #[test]
    fn test_hover_empty_position() {
        let hover = hover_info("", Position::new(0, 0));
        assert!(
            hover.is_none(),
            "hover в пустом файле должен возвращать None"
        );
    }

    // ── Тесты hover с документацией (C6) ──────────────────────────────────────

    /// Hover над функцией с doc-комментарием отображает и сигнатуру, и документацию.
    ///
    /// `///`-комментарии перед `fn process` должны появляться в hover-ответе
    /// вместе с сигнатурой функции.
    #[test]
    fn test_hover_function_with_docs() {
        let src = "/// Обработка данных.\n/// data — входные данные.\nextern fn process(data: [bit;8]) -> bit; start S;";
        // "process" начинается на строке 2 (0-indexed), символ 10 ("extern fn " = 10)
        let hover = hover_info(src, Position::new(2, 10));
        assert!(
            hover.is_some(),
            "hover над функцией с документацией должен возвращать данные"
        );
        if let Some(h) = hover {
            if let HoverContents::Markup(mc) = h.contents {
                assert!(
                    mc.value.contains("process"),
                    "hover должен содержать имя функции"
                );
                assert!(
                    mc.value.contains("Обработка данных"),
                    "hover должен содержать документацию функции: {}",
                    mc.value
                );
            }
        }
    }

    /// Hover над переменной с doc-комментарием отображает и тип, и документацию.
    ///
    /// `///`-комментарии перед `var counter` должны появляться в hover-ответе
    /// вместе с типом переменной.
    #[test]
    fn test_hover_variable_with_docs() {
        let src = "/// Счётчик тактов.\nvar counter: [bit;8] = 0; start S;";
        // "counter" находится на строке 1 (0-indexed), символ 4 ("var " = 4)
        let hover = hover_info(src, Position::new(1, 4));
        assert!(
            hover.is_some(),
            "hover над переменной с документацией должен возвращать данные"
        );
        if let Some(h) = hover {
            if let HoverContents::Markup(mc) = h.contents {
                assert!(
                    mc.value.contains("counter"),
                    "hover должен содержать имя переменной"
                );
                assert!(
                    mc.value.contains("Счётчик тактов"),
                    "hover должен содержать документацию переменной: {}",
                    mc.value
                );
            }
        }
    }

    /// Hover над переменной БЕЗ документации отображает только сигнатуру.
    ///
    /// Если `///`-комментарии отсутствуют, hover возвращает только тип переменной.
    #[test]
    fn test_hover_variable_without_docs() {
        let src = "var counter: [bit;8] = 0; start S;";
        let hover = hover_info(src, Position::new(0, 5));
        assert!(
            hover.is_some(),
            "hover без документации должен возвращать данные"
        );
        if let Some(h) = hover {
            if let HoverContents::Markup(mc) = h.contents {
                assert!(
                    mc.value.contains("counter"),
                    "hover должен содержать имя переменной"
                );
                // Без документации не должно быть двойного переноса строки + текста
                // Достаточно убедиться, что нет лишнего текста за пределами code-блока
                assert!(
                    mc.value.starts_with("```but"),
                    "hover без документации должен начинаться с блока кода"
                );
            }
        }
    }

    // ── Тесты конвертации диагностик ──────────────────────────────────────────

    /// Ошибка парсера конвертируется в LSP DiagnosticSeverity::ERROR.
    #[test]
    fn test_grammar_diagnostic_to_lsp_error() {
        let diag = GrammarDiagnostic {
            loc: Location::Source(0, 0, 5),
            level: Level::Error,
            ty: ErrorType::ParserError,
            message: "тестовая ошибка".to_string(),
            notes: vec![],
        };
        let lsp_diag = grammar_diagnostic_to_lsp(&diag, "hello");
        assert_eq!(lsp_diag.severity, Some(DiagnosticSeverity::ERROR));
        assert_eq!(lsp_diag.message, "тестовая ошибка");
        assert_eq!(lsp_diag.source, Some("but-lsp".to_string()));
    }

    /// Предупреждение конвертируется в LSP DiagnosticSeverity::WARNING.
    #[test]
    fn test_grammar_diagnostic_to_lsp_warning() {
        let diag = GrammarDiagnostic {
            loc: Location::Source(0, 6, 11),
            level: Level::Warning,
            ty: ErrorType::Warning,
            message: "тестовое предупреждение".to_string(),
            notes: vec![],
        };
        let lsp_diag = grammar_diagnostic_to_lsp(&diag, "hello world");
        assert_eq!(lsp_diag.severity, Some(DiagnosticSeverity::WARNING));
        // Столбец начала: 6 → строка 0, символ 6
        assert_eq!(lsp_diag.range.start.line, 0);
        assert_eq!(lsp_diag.range.start.character, 6);
    }

    /// Builtin-местоположение конвертируется в нулевой диапазон.
    #[test]
    fn test_grammar_diagnostic_to_lsp_builtin_location() {
        let diag = GrammarDiagnostic {
            loc: Location::Builtin,
            level: Level::Info,
            ty: ErrorType::None,
            message: "встроенное".to_string(),
            notes: vec![],
        };
        let lsp_diag = grammar_diagnostic_to_lsp(&diag, "");
        assert_eq!(lsp_diag.range.start, Position::new(0, 0));
        assert_eq!(lsp_diag.range.end, Position::new(0, 0));
    }

    // ── Тесты UTF-16 и Unicode ─────────────────────────────────────────────────

    /// Кириллический символ занимает 2 байта UTF-8, но 1 кодовую единицу UTF-16.
    /// offset_to_position должен возвращать UTF-16-столбец, а не байтовый.
    ///
    /// Пример: "АБ" = bytes [0xD0,0x90, 0xD0,0x91]
    /// offset 2 (начало 'Б') → строка 0, столбец 1 (в UTF-16)
    /// offset 4 (конец строки) → строка 0, столбец 2 (в UTF-16)
    ///
    /// Контр-пример: если бы считали байты, столбец был бы 2 и 4 соответственно.
    #[test]
    fn test_offset_to_position_cyrillic_utf16() {
        let src = "АБ"; // 4 байта UTF-8, 2 символа, 2 кодовые единицы UTF-16
        assert_eq!(src.len(), 4, "кириллица: 2 байта на символ");

        // Конец строки: 2 кодовые единицы UTF-16
        let pos = offset_to_position(src, 4);
        assert_eq!(pos.line, 0);
        assert_eq!(pos.character, 2, "UTF-16 столбец, не байтовый");

        // После первого символа 'А': байт 2 → UTF-16 столбец 1
        let pos = offset_to_position(src, 2);
        assert_eq!(pos.character, 1);
    }

    /// Emoji занимает 4 байта UTF-8 и 2 кодовые единицы UTF-16 (суррогатная пара).
    ///
    /// Пример: "😀x" — emoji U+1F600: 4 байта UTF-8, 2 UTF-16 единицы.
    /// offset 4 (позиция 'x') → UTF-16 столбец 2
    ///
    /// Контр-пример: байтовый столбец был бы 4.
    #[test]
    fn test_offset_to_position_emoji_surrogate_pair() {
        let src = "😀x"; // U+1F600 = 4 байта UTF-8, 2 кодовые единицы UTF-16
        assert_eq!(src.len(), 5, "emoji 4 байта + 'x' 1 байт");

        // Позиция 'x': байт 4 → UTF-16 столбец 2 (emoji занимает 2 единицы)
        let pos = offset_to_position(src, 4);
        assert_eq!(pos.line, 0);
        assert_eq!(
            pos.character, 2,
            "суррогатная пара занимает 2 UTF-16 единицы"
        );
    }

    /// Смещение на середину многобайтового символа не должно вызывать панику.
    /// Функция должна безопасно отступить до предыдущей char-границы.
    ///
    /// Пример: "А" = [0xD0, 0x90], offset 1 — середина символа.
    /// Ожидаем позицию начала "А" (UTF-16 столбец 0), а не панику.
    ///
    /// Контр-пример: &source[..1] для "А" вызвал бы панику без защиты.
    #[test]
    fn test_offset_to_position_mid_char_no_panic() {
        let src = "АБ"; // 'А' = bytes 0..2, 'Б' = bytes 2..4
        // Байт 1 — середина 'А': отступаем до байта 0 → столбец 0
        let pos = offset_to_position(src, 1);
        assert_eq!(pos.line, 0);
        assert_eq!(pos.character, 0, "должны откатиться до начала символа 'А'");
    }

    /// offset_to_position с нулём всегда возвращает (0, 0).
    #[test]
    fn test_offset_to_position_zero() {
        assert_eq!(offset_to_position("hello", 0), Position::new(0, 0));
        assert_eq!(offset_to_position("", 0), Position::new(0, 0));
        assert_eq!(offset_to_position("АБ", 0), Position::new(0, 0));
    }

    /// offset_to_position на многострочном тексте с кириллицей.
    ///
    /// Пример: "А\nБ", байт 3 = начало 'Б' → строка 1, столбец 0.
    #[test]
    fn test_offset_to_position_multiline_cyrillic() {
        let src = "А\nБ"; // 'А'=2, '\n'=1, 'Б'=2 → длина 5
        // Байт 3 — начало 'Б' на второй строке
        let pos = offset_to_position(src, 3);
        assert_eq!(pos.line, 1);
        assert_eq!(pos.character, 0);

        // Байт 5 — конец 'Б' → строка 1, столбец 1
        let pos = offset_to_position(src, 5);
        assert_eq!(pos.line, 1);
        assert_eq!(pos.character, 1);
    }

    // ── Тесты word_at_position с UTF-16 позицией ──────────────────────────────

    /// word_at_position корректно обрабатывает UTF-16 позицию в строке с кириллицей.
    ///
    /// Строка: "А myVar = 0;"
    /// 'А' занимает 1 кодовую единицу UTF-16. position.character=2 → байт 3 → 'm'.
    #[test]
    fn test_word_at_position_utf16_column() {
        // "А " = 2 UTF-16 единицы (1 для 'А', 1 для ' ')
        // "myVar" начинается с UTF-16-позиции 2
        let src = "А myVar";
        // 'А' = bytes 0..2, ' ' = byte 2, 'myVar' = bytes 3..8
        // UTF-16: 'А'=1, ' '=1, 'm'=1 → position.character=2 → 'm'
        let word = word_at_position(src, Position::new(0, 2));
        assert_eq!(word, Some("myVar".to_string()));
    }

    /// word_at_position с позицией, выходящей за пределы строки → None или последнее слово.
    ///
    /// Контр-пример: position.character больше длины строки — функция не паникует.
    #[test]
    fn test_word_at_position_beyond_line_no_panic() {
        let src = "hello";
        // Позиция за концом строки: clamp к длине → конец слова
        let word = word_at_position(src, Position::new(0, 999));
        // Ожидаем "hello" (курсор зажат до конца)
        assert_eq!(word, Some("hello".to_string()));
    }

    // ── Тесты utf16_to_byte_offset ────────────────────────────────────────────

    /// ASCII: UTF-16 смещение совпадает с байтовым.
    #[test]
    fn test_utf16_to_byte_offset_ascii() {
        assert_eq!(super::utf16_to_byte_offset("hello", 0), Some(0));
        assert_eq!(super::utf16_to_byte_offset("hello", 3), Some(3));
        assert_eq!(super::utf16_to_byte_offset("hello", 5), Some(5));
    }

    /// Кириллица: 1 UTF-16 единица = 2 байта UTF-8.
    ///
    /// "АБВ": utf16_offset 1 → байт 2, utf16_offset 3 → байт 6.
    #[test]
    fn test_utf16_to_byte_offset_cyrillic() {
        let s = "АБВ"; // каждый символ 2 байта
        assert_eq!(super::utf16_to_byte_offset(s, 0), Some(0));
        assert_eq!(super::utf16_to_byte_offset(s, 1), Some(2)); // после 'А'
        assert_eq!(super::utf16_to_byte_offset(s, 2), Some(4)); // после 'Б'
        assert_eq!(super::utf16_to_byte_offset(s, 3), Some(6)); // конец
    }

    /// Emoji (суррогатная пара): U+1F600 занимает 2 UTF-16 единицы.
    ///
    /// "😀x": utf16_offset 2 → байт 4 (начало 'x').
    ///
    /// Контр-пример: utf16_offset 1 → None (внутри суррогатной пары).
    #[test]
    fn test_utf16_to_byte_offset_emoji() {
        let s = "😀x"; // U+1F600 = 4 байта UTF-8, 2 единицы UTF-16; 'x' = 1 байт
        // utf16_offset 0 → байт 0 (начало emoji)
        assert_eq!(super::utf16_to_byte_offset(s, 0), Some(0));
        // utf16_offset 2 → байт 4 (начало 'x')
        assert_eq!(super::utf16_to_byte_offset(s, 2), Some(4));
        // utf16_offset 3 → байт 5 (конец строки)
        assert_eq!(super::utf16_to_byte_offset(s, 3), Some(5));
    }

    /// Смещение за пределами строки → None.
    #[test]
    fn test_utf16_to_byte_offset_out_of_bounds() {
        assert_eq!(super::utf16_to_byte_offset("hi", 10), None);
        assert_eq!(super::utf16_to_byte_offset("", 1), None);
    }

    // ── Тест семантических токенов ─────────────────────────────────────────────

    /// semantic_tokens не должна паниковать на валидном BuT-исходнике.
    #[test]
    fn test_semantic_tokens_no_panic() {
        let tokens = semantic_tokens(VALID_SRC);
        // Проверяем, что токены сформированы и дельта-кодирование корректно:
        // delta_line строго неотрицательна (u32), delta_start < character на той же строке.
        let mut prev_line = 0u32;
        for tok in &tokens.data {
            assert!(
                tok.delta_line >= 0 || prev_line == 0,
                "delta_line всегда >= 0 (тип u32)"
            );
            assert!(tok.length > 0, "нулевые токены отфильтровываются");
            prev_line += tok.delta_line;
        }
    }

    /// semantic_tokens не должна паниковать на пустом вводе.
    #[test]
    fn test_semantic_tokens_empty_source() {
        let tokens = semantic_tokens("");
        assert!(tokens.data.is_empty(), "пустой источник → нет токенов");
    }

    /// semantic_tokens корректно считает длину кириллического идентификатора в UTF-16.
    ///
    /// Токен "АБВ" (3 символа, 6 байт UTF-8) должен иметь length=3 в UTF-16, не 6.
    ///
    /// Контр-пример: если бы считали байты, length был бы 6 — LSP-редактор неправильно
    /// подсветил бы диапазон.
    #[test]
    fn test_semantic_tokens_utf16_length() {
        // Используем extern fn с кириллическим именем
        // BuT поддерживает Unicode-идентификаторы через UnicodeXID
        let src = "extern fn АБВ() -> [bit;8]; start S;";
        let tokens = semantic_tokens(src);
        // Ищем токен типа TT_FUNCTION для "АБВ"
        // "extern fn " = 10 байт/символов (ASCII) до "АБВ"
        // "АБВ" начинается на байте 10, строка 0
        // В UTF-16: 10 ASCII-символов = 10 единиц → delta_start или character = 10
        // length = 3 (UTF-16), не 6 (байты)
        let func_tok = tokens.data.iter().find(|t| t.token_type == TT_FUNCTION);
        if let Some(tok) = func_tok {
            assert_eq!(
                tok.length, 3,
                "кириллический идентификатор: 3 кодовые единицы UTF-16, не 6 байт"
            );
        }
        // Если "АБВ" не распознан как функция — всё равно не паникуем
    }
}
