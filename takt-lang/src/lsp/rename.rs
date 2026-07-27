//! `textDocument/rename` и `prepareRename` — фича 0131.
//!
//! ## Полнота или отказ
//!
//! Частичное переименование — не «неполный результат», а **испорченный
//! исходник**: половина вхождений сменит имя, половина останется, и в лучшем
//! случае файл перестанет компилироваться, в худшем — начнёт ссылаться на другой
//! символ (затенение оставляет текст компилируемым, меняя смысл). Поэтому здесь
//! нет «сделаем сколько сможем»: каждая причина, по которой полнота не
//! гарантирована, даёт **отказ**, и по возможности — на `prepareRename`, то есть
//! до того, как редактор покажет поле ввода.
//!
//! Часть модуля `lsp` (фича 0027: деление по логике).

use super::*;
use crate::semantic::usages::{self, SymbolKind, UsageTable};

/// Причина отказа переименовать.
///
/// Отдельный тип, а не `None`: редактору есть что показать пользователю, и
/// «почему нельзя» — половина пользы отказа.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum RenameRefusal {
    /// Текст не разбирается — областей видимости нет, полнота недостижима.
    Unparsable,
    /// Под курсором нет имени.
    NoSymbol,
    /// Имя объявлено вне открытого документа (импортированный символ).
    ForeignDeclaration,
    /// Имя модели: оно экспортируемо через `import`, и файлы-потребители
    /// серверу не видны.
    ModelName,
    /// Обход встретил узел, которого не знает, либо имя, которое не смог
    /// связать: часть вхождений могла остаться незамеченной.
    Incomplete,
    /// Новое имя не является идентификатором языка.
    NotAnIdentifier,
    /// Новое имя — ключевое слово.
    Keyword,
}

impl RenameRefusal {
    /// Сообщение для пользователя редактора.
    pub fn message(&self) -> &'static str {
        match self {
            Self::Unparsable => "файл не компилируется: переименование недоступно",
            Self::NoSymbol => "под курсором нет имени",
            Self::ForeignDeclaration => {
                "имя объявлено в другом файле: переименование затронуло бы файлы вне рабочего набора"
            }
            Self::ModelName => {
                "имя модели видно за пределами файла: переименование затронуло бы импортирующие файлы"
            }
            Self::Incomplete => {
                "в файле есть места, которые сервер не разобрал: полнота переименования не гарантируется"
            }
            Self::NotAnIdentifier => "новое имя не является идентификатором языка",
            Self::Keyword => "новое имя — ключевое слово языка",
        }
    }
}

/// Диапазон имени, которое будет переименовано, либо причина отказа.
///
/// Соответствует `textDocument/prepareRename`: редактор спрашивает **до** ввода
/// нового имени, можно ли вообще переименовывать здесь.
pub fn prepare_rename_at(source: &str, position: Position) -> Result<Range, RenameRefusal> {
    let (table, offset) = prepared(source, position)?;
    let usage = table.usage_at(offset).ok_or(RenameRefusal::NoSymbol)?;
    Ok(offset_to_range(
        source,
        usage.start as usize,
        usage.end as usize,
    ))
}

/// Правки переименования: объявление и **все** вхождения символа.
///
/// Возвращает список правок в координатах редактора; применять их следует от
/// конца к началу либо все разом (диапазоны не пересекаются).
pub fn rename_at(
    source: &str,
    position: Position,
    new_name: &str,
) -> Result<Vec<TextEdit>, RenameRefusal> {
    validate_new_name(new_name)?;
    let (table, offset) = prepared(source, position)?;
    let usage = table.usage_at(offset).ok_or(RenameRefusal::NoSymbol)?;
    let symbol = usage.symbol;

    // Имя символа могло встретиться и там, где связать его не удалось. Что это
    // «наверное, другое имя» — догадка, а цена ошибки — испорченный файл.
    if table.has_unresolved_named(&usage.name) {
        return Err(RenameRefusal::Incomplete);
    }

    let edits = table
        .occurrences_of(symbol)
        .into_iter()
        .map(|u| TextEdit {
            range: offset_to_range(source, u.start as usize, u.end as usize),
            new_text: new_name.to_string(),
        })
        .collect();
    Ok(edits)
}

/// Общая часть обоих входов: таблица вхождений + проверки, не зависящие от
/// нового имени.
///
/// ⚠️ Проверки живут здесь, а не в каждом входе: разъехавшись, `prepareRename`
/// разрешил бы то, на чём `rename` затем откажет, — худший из возможных
/// сценариев (пользователь ввёл имя и получил ошибку).
fn prepared(source: &str, position: Position) -> Result<(UsageTable, usize), RenameRefusal> {
    let (ast, _) = crate::parse(source, 0).map_err(|_| RenameRefusal::Unparsable)?;
    let table = usages::collect_usages(&ast);
    if !table.is_complete() {
        return Err(RenameRefusal::Incomplete);
    }
    let offset = position_to_offset(source, position).ok_or(RenameRefusal::NoSymbol)?;

    match table.usage_at(offset) {
        Some(usage) if usage.symbol_kind == SymbolKind::Model => Err(RenameRefusal::ModelName),
        Some(usage) if table.declaration_of(usage.symbol).is_none() => {
            Err(RenameRefusal::ForeignDeclaration)
        }
        Some(_) => Ok((table, offset)),
        // Имя есть, но связать его не удалось — оно объявлено вне файла.
        None if table.unresolved_at(offset).is_some() => Err(RenameRefusal::ForeignDeclaration),
        None => Err(RenameRefusal::NoSymbol),
    }
}

/// Проверяет, что новое имя — идентификатор языка и не ключевое слово.
///
/// Правила берутся из лексера (`XID_Start`/`XID_Continue` плюс `_` и `$`) и его
/// же таблицы ключевых слов — второй список разъехался бы с языком.
fn validate_new_name(new_name: &str) -> Result<(), RenameRefusal> {
    use unicode_xid::UnicodeXID;

    let mut chars = new_name.chars();
    let first = chars.next().ok_or(RenameRefusal::NotAnIdentifier)?;
    if !(first == '_' || first == '$' || UnicodeXID::is_xid_start(first)) {
        return Err(RenameRefusal::NotAnIdentifier);
    }
    if !chars.all(|c| c == '$' || UnicodeXID::is_xid_continue(c)) {
        return Err(RenameRefusal::NotAnIdentifier);
    }
    if crate::parser::lexer::is_keyword(new_name) {
        return Err(RenameRefusal::Keyword);
    }
    Ok(())
}
