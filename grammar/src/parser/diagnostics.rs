use std::fmt;

use crate::ast::Location;

/// Уровень серьёзности диагностического сообщения.
#[derive(Clone, Debug, Hash, PartialOrd, Ord, PartialEq, Eq)]
pub enum Level {
    /// Отладочное сообщение — не отображается конечному пользователю.
    Debug,
    /// Информационное сообщение.
    Info,
    /// Предупреждение — код валиден, но может содержать потенциальную проблему.
    Warning,
    /// Ошибка — код содержит нарушение.
    Error,
}

impl fmt::Display for Level {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(self.as_str())
    }
}

impl Level {
    /// Возвращает строковое представление уровня.
    pub fn as_str(&self) -> &'static str {
        match self {
            Level::Debug => "debug",
            Level::Info => "info",
            Level::Warning => "warning",
            Level::Error => "error",
        }
    }
}

/// Категория диагностического сообщения.
#[derive(Clone, Debug, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub enum ErrorType {
    /// Категория не задана.
    None,
    /// Ошибка лексического или синтаксического анализатора.
    ParserError,
    /// Синтаксическая ошибка на уровне языка.
    SyntaxError,
    /// Ошибка объявления (например, неизвестный идентификатор).
    DeclarationError,
    /// Ошибка приведения типов.
    CastError,
    /// Ошибка типизации.
    TypeError,
    /// Предупреждение (не ошибка).
    Warning,
}

/// Вспомогательная заметка, прикреплённая к диагностическому сообщению.
///
/// Используется для указания дополнительного контекста об ошибке
/// (например, место первого объявления при дублировании имени).
#[derive(Clone, Debug, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct Note {
    /// Местоположение в исходном тексте.
    pub loc: Location,
    /// Текст заметки.
    pub message: String,
}

/// Диагностическое сообщение, возникающее в процессе компиляции BuT-программы.
///
/// Каждое сообщение содержит местоположение в исходном тексте, уровень серьёзности,
/// категорию ошибки, основной текст и список вспомогательных заметок.
#[derive(Clone, Debug, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct Diagnostic {
    /// Местоположение в исходном тексте, к которому относится диагностика.
    pub loc: Location,
    /// Уровень серьёзности сообщения.
    pub level: Level,
    /// Категория ошибки.
    pub ty: ErrorType,
    /// Текст диагностического сообщения.
    pub message: String,
    /// Вспомогательные заметки.
    pub notes: Vec<Note>,
}

impl Diagnostic {
    /// Создаёт отладочное сообщение.
    pub fn debug(loc: Location, message: String) -> Self {
        Diagnostic {
            level: Level::Debug,
            ty: ErrorType::None,
            loc,
            message,
            notes: Vec::new(),
        }
    }

    /// Создаёт информационное сообщение.
    pub fn info(loc: Location, message: String) -> Self {
        Diagnostic {
            level: Level::Info,
            ty: ErrorType::None,
            loc,
            message,
            notes: Vec::new(),
        }
    }

    /// Создаёт ошибку синтаксического/лексического анализатора.
    pub fn parser_error(loc: Location, message: String) -> Self {
        Diagnostic {
            level: Level::Error,
            ty: ErrorType::ParserError,
            loc,
            message,
            notes: Vec::new(),
        }
    }

    /// Создаёт синтаксическую ошибку.
    pub fn error(loc: Location, message: String) -> Self {
        Diagnostic {
            level: Level::Error,
            ty: ErrorType::SyntaxError,
            loc,
            message,
            notes: Vec::new(),
        }
    }

    /// Создаёт ошибку объявления (неизвестный идентификатор и т.д.).
    pub fn declaration_error(loc: Location, message: String) -> Self {
        Diagnostic {
            level: Level::Error,
            ty: ErrorType::DeclarationError,
            loc,
            message,
            notes: Vec::new(),
        }
    }

    /// Создаёт ошибку приведения типов.
    pub fn cast_error(loc: Location, message: String) -> Self {
        Diagnostic {
            level: Level::Error,
            ty: ErrorType::CastError,
            loc,
            message,
            notes: Vec::new(),
        }
    }

    /// Создаёт ошибку приведения типов с дополнительной заметкой.
    pub fn cast_error_with_note(
        loc: Location,
        message: String,
        note_loc: Location,
        note: String,
    ) -> Self {
        Diagnostic {
            level: Level::Error,
            ty: ErrorType::CastError,
            loc,
            message,
            notes: vec![Note {
                loc: note_loc,
                message: note,
            }],
        }
    }

    /// Создаёт ошибку типизации.
    pub fn type_error(loc: Location, message: String) -> Self {
        Diagnostic {
            level: Level::Error,
            ty: ErrorType::TypeError,
            loc,
            message,
            notes: Vec::new(),
        }
    }

    /// Создаёт предупреждение о небезопасном приведении типов.
    pub fn cast_warning(loc: Location, message: String) -> Self {
        Diagnostic {
            level: Level::Warning,
            ty: ErrorType::CastError,
            loc,
            message,
            notes: Vec::new(),
        }
    }

    /// Создаёт предупреждение.
    pub fn warning(loc: Location, message: String) -> Self {
        Diagnostic {
            level: Level::Warning,
            ty: ErrorType::Warning,
            loc,
            message,
            notes: Vec::new(),
        }
    }

    /// Создаёт предупреждение с дополнительной заметкой.
    pub fn warning_with_note(
        loc: Location,
        message: String,
        note_loc: Location,
        note: String,
    ) -> Self {
        Diagnostic {
            level: Level::Warning,
            ty: ErrorType::Warning,
            loc,
            message,
            notes: vec![Note {
                loc: note_loc,
                message: note,
            }],
        }
    }

    /// Создаёт предупреждение с набором вспомогательных заметок.
    pub fn warning_with_notes(loc: Location, message: String, notes: Vec<Note>) -> Self {
        Diagnostic {
            level: Level::Warning,
            ty: ErrorType::Warning,
            loc,
            message,
            notes,
        }
    }

    /// Создаёт ошибку с дополнительной заметкой.
    pub fn error_with_note(
        loc: Location,
        message: String,
        note_loc: Location,
        note: String,
    ) -> Self {
        Diagnostic {
            level: Level::Error,
            ty: ErrorType::None,
            loc,
            message,
            notes: vec![Note {
                loc: note_loc,
                message: note,
            }],
        }
    }

    /// Создаёт ошибку с набором вспомогательных заметок.
    pub fn error_with_notes(loc: Location, message: String, notes: Vec<Note>) -> Self {
        Diagnostic {
            level: Level::Error,
            ty: ErrorType::None,
            loc,
            message,
            notes,
        }
    }
}
