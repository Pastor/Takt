use std::fmt;

use crate::ast::Location;

#[derive(Clone, Debug, Hash, PartialOrd, Ord, PartialEq, Eq)]
pub enum Level {
    Debug,
    Info,
    Warning,
    Error,
}

impl fmt::Display for Level {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(self.as_str())
    }
}

impl Level {
    pub fn as_str(&self) -> &'static str {
        match self {
            Level::Debug => "debug",
            Level::Info => "info",
            Level::Warning => "warning",
            Level::Error => "error",
        }
    }
}

#[derive(Clone, Debug, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub enum ErrorType {
    None,
    ParserError,
    SyntaxError,
    DeclarationError,
    CastError,
    TypeError,
    Warning,
}

#[derive(Clone, Debug, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct Note {
    pub loc: Location,
    pub message: String,
}

#[derive(Clone, Debug, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct Diagnostic {
    pub loc: Location,
    pub level: Level,
    pub ty: ErrorType,
    pub message: String,
    pub notes: Vec<Note>,
}

impl Diagnostic {
    pub fn debug(loc: Location, message: String) -> Self {
        Diagnostic {
            level: Level::Debug,
            ty: ErrorType::None,
            loc,
            message,
            notes: Vec::new(),
        }
    }

    pub fn info(loc: Location, message: String) -> Self {
        Diagnostic {
            level: Level::Info,
            ty: ErrorType::None,
            loc,
            message,
            notes: Vec::new(),
        }
    }

    pub fn parser_error(loc: Location, message: String) -> Self {
        Diagnostic {
            level: Level::Error,
            ty: ErrorType::ParserError,
            loc,
            message,
            notes: Vec::new(),
        }
    }

    pub fn error(loc: Location, message: String) -> Self {
        Diagnostic {
            level: Level::Error,
            ty: ErrorType::SyntaxError,
            loc,
            message,
            notes: Vec::new(),
        }
    }

    pub fn declaration_error(loc: Location, message: String) -> Self {
        Diagnostic {
            level: Level::Error,
            ty: ErrorType::DeclarationError,
            loc,
            message,
            notes: Vec::new(),
        }
    }

    pub fn cast_error(loc: Location, message: String) -> Self {
        Diagnostic {
            level: Level::Error,
            ty: ErrorType::CastError,
            loc,
            message,
            notes: Vec::new(),
        }
    }

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

    pub fn type_error(loc: Location, message: String) -> Self {
        Diagnostic {
            level: Level::Error,
            ty: ErrorType::TypeError,
            loc,
            message,
            notes: Vec::new(),
        }
    }

    pub fn cast_warning(loc: Location, message: String) -> Self {
        Diagnostic {
            level: Level::Warning,
            ty: ErrorType::CastError,
            loc,
            message,
            notes: Vec::new(),
        }
    }

    pub fn warning(loc: Location, message: String) -> Self {
        Diagnostic {
            level: Level::Warning,
            ty: ErrorType::Warning,
            loc,
            message,
            notes: Vec::new(),
        }
    }

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

    pub fn warning_with_notes(loc: Location, message: String, notes: Vec<Note>) -> Self {
        Diagnostic {
            level: Level::Warning,
            ty: ErrorType::Warning,
            loc,
            message,
            notes,
        }
    }

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
