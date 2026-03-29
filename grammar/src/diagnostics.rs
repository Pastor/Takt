//! Диагностические сообщения компилятора BuT.
//!
//! Этот модуль содержит типы для описания ошибок, предупреждений и
//! информационных сообщений, возникающих в ходе лексического, синтаксического
//! и семантического анализа.
//!
//! ## Основные типы
//!
//! - [`Level`] — уровень серьёзности сообщения (`Debug`, `Info`, `Warning`, `Error`).
//! - [`ErrorType`] — категория ошибки (синтаксис, типизация, семантика и т.п.).
//! - [`Location`] — позиция в исходном тексте (файл, смещение начала и конца).
//! - [`Diagnostic`] — единица диагностики, объединяющая уровень, тип, сообщение
//!   и список дополнительных замечаний.
//!
//! ## Конвертации
//!
//! `Diagnostic` реализует `From<&str>` и `From<String>` для удобного создания
//! ошибок из строк. По умолчанию создаётся сообщение уровня `Error` с категорией
//! `SematicError`.

#[cfg(feature = "ast-serde")]
use serde::{Deserialize, Serialize};
use std::fmt;

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
    /// Семантическая ошрибка
    SematicError,
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

impl Into<Diagnostic> for &str {
    fn into(self) -> Diagnostic {
        Diagnostic {
            loc: Default::default(),
            level: Level::Error,
            ty: ErrorType::SematicError,
            message: self.to_string(),
            notes: vec![],
        }
    }
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

/// Местоположение узла АСД в исходном тексте.
///
/// Вариант [`Source`](Location::Source) хранит номер файла, байтовое смещение
/// начала и байтовое смещение конца (не включительно).
#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, Hash)]
#[cfg_attr(feature = "ast-serde", derive(Serialize, Deserialize))]
pub enum Location {
    /// Встроенный элемент (не относится к пользовательскому коду).
    Builtin,
    /// Элемент, заданный через командную строку компилятора.
    CommandLine,
    /// Неявно сгенерированный элемент.
    Implicit,
    /// Элемент, сгенерированный кодогенератором.
    Codegen,
    /// Элемент в исходном файле: `(файл, начало, конец)`.
    Source(u64, usize, usize),
}

impl Default for Location {
    fn default() -> Self {
        Self::Source(0, 0, 0)
    }
}

/// Вызывается при попытке получить позицию из не-файлового варианта [`Location`].
#[inline(never)]
#[cold]
#[track_caller]
fn not_a_file() -> ! {
    panic!("местоположение не является файловой позицией")
}

impl Location {
    /// Возвращает [`Location`] нулевой длины, указывающий на начало данного диапазона.
    #[inline]
    pub fn begin_range(&self) -> Self {
        match self {
            Location::Source(filename, start, _) => {
                Location::Source(filename.clone(), *start, *start)
            }
            loc => loc.clone(),
        }
    }

    /// Возвращает [`Location`] нулевой длины, указывающий на конец данного диапазона.
    #[inline]
    pub fn end_range(&self) -> Self {
        match self {
            Location::Source(filename, _, end) => Location::Source(filename.clone(), *end, *end),
            loc => loc.clone(),
        }
    }

    /// Возвращает строковое представление номера файла.
    ///
    /// # Паника
    ///
    /// Паникует, если `self` не является вариантом [`Location::Source`].
    #[track_caller]
    #[inline]
    pub fn filename(&self) -> String {
        match self {
            Location::Source(file_no, _, _) => format!("{}", file_no),
            _ => not_a_file(),
        }
    }

    /// Возвращает `Some(номер_файла)` для варианта [`Source`](Location::Source),
    /// или `None` для остальных вариантов.
    #[inline]
    pub fn try_file_no(&self) -> Option<String> {
        match self {
            Location::Source(file_no, _, _) => Some(format!("{}", file_no)),
            _ => None,
        }
    }

    /// Возвращает байтовое смещение начала диапазона.
    ///
    /// # Паника
    ///
    /// Паникует, если `self` не является вариантом [`Location::Source`].
    #[track_caller]
    #[inline]
    pub fn start(&self) -> usize {
        match self {
            Location::Source(_, start, _) => *start,
            _ => not_a_file(),
        }
    }

    /// Возвращает байтовое смещение конца диапазона (не включительно, exclusive).
    ///
    /// Значение `end` указывает на первый байт **после** последнего символа диапазона,
    /// то есть является exclusive-концом в стиле `start..end` (Rust-срезы, `Range`).
    ///
    /// # Примеры
    ///
    /// ```
    /// use grammar::diagnostics::Location;
    ///
    /// // Исходный текст: "hello world"
    /// // Токен "hello": байты 0..5
    /// let loc = Location::Source(0, 0, 5);
    /// assert_eq!(loc.end(), 5);
    /// // source[0..5] == "hello" — правильный срез
    /// ```
    ///
    /// # Паника
    ///
    /// Паникует, если `self` не является вариантом [`Location::Source`].
    #[track_caller]
    #[inline]
    pub fn end(&self) -> usize {
        match self {
            Location::Source(_, _, end) => *end,
            _ => not_a_file(),
        }
    }

    /// Возвращает байтовое смещение `end + 1`.
    ///
    /// **Важно:** поле `end` в [`Location::Source`] уже является exclusive-концом
    /// (один байт после последнего символа). Этот метод возвращает `end + 1`, то есть
    /// позицию, следующую за exclusive-концом. Используется в редких случаях, когда
    /// нужен инклюзивный диапазон или адресация за пределами диапазона.
    ///
    /// Для большинства задач используйте [`range()`](Location::range) или [`end()`](Location::end).
    ///
    /// # Примеры
    ///
    /// ```
    /// use grammar::diagnostics::Location;
    ///
    /// let loc = Location::Source(0, 3, 8); // байты 3..8 (exclusive)
    /// assert_eq!(loc.end(), 8);
    /// assert_eq!(loc.exclusive_end(), 9); // end + 1
    /// ```
    ///
    /// # Паника
    ///
    /// Паникует, если `self` не является вариантом [`Location::Source`].
    #[track_caller]
    #[inline]
    pub fn exclusive_end(&self) -> usize {
        self.end() + 1
    }

    /// Устанавливает начало текущего диапазона равным началу `other`.
    ///
    /// # Паника
    ///
    /// Паникует, если любой из операндов не является [`Location::Source`].
    #[track_caller]
    #[inline]
    pub fn use_start_from(&mut self, other: &Location) {
        match (self, other) {
            (Location::Source(_, start, _), Location::Source(_, other_start, _)) => {
                *start = *other_start;
            }
            _ => not_a_file(),
        }
    }

    /// Устанавливает конец текущего диапазона равным концу `other`.
    ///
    /// # Паника
    ///
    /// Паникует, если любой из операндов не является [`Location::Source`].
    #[track_caller]
    #[inline]
    pub fn use_end_from(&mut self, other: &Location) {
        match (self, other) {
            (Location::Source(_, _, end), Location::Source(_, _, other_end)) => {
                *end = *other_end;
            }
            _ => not_a_file(),
        }
    }

    /// Возвращает копию с началом, взятым из `other`.
    ///
    /// # Паника
    ///
    /// Паникует, если любой из операндов не является [`Location::Source`].
    #[track_caller]
    #[inline]
    pub fn with_start_from(mut self, other: &Self) -> Self {
        self.use_start_from(other);
        self
    }

    /// Возвращает копию с концом, взятым из `other`.
    ///
    /// # Паника
    ///
    /// Паникует, если любой из операндов не является [`Location::Source`].
    #[track_caller]
    #[inline]
    pub fn with_end_from(mut self, other: &Self) -> Self {
        self.use_end_from(other);
        self
    }

    /// Возвращает копию с заменённым началом.
    ///
    /// # Паника
    ///
    /// Паникует, если `self` не является [`Location::Source`].
    #[track_caller]
    #[inline]
    pub fn with_start(self, start: usize) -> Self {
        match self {
            Self::Source(no, _, end) => Self::Source(no, start, end),
            _ => not_a_file(),
        }
    }

    /// Возвращает копию с заменённым концом.
    ///
    /// # Паника
    ///
    /// Паникует, если `self` не является [`Location::Source`].
    #[track_caller]
    #[inline]
    pub fn with_end(self, end: usize) -> Self {
        match self {
            Self::Source(no, start, _) => Self::Source(no, start, end),
            _ => not_a_file(),
        }
    }

    /// Преобразует [`Location`] в стандартный диапазон `start..end`.
    ///
    /// # Паника
    ///
    /// Паникует, если `self` не является [`Location::Source`].
    #[track_caller]
    #[inline]
    pub fn range(self) -> std::ops::Range<usize> {
        match self {
            Self::Source(_, start, end) => start..end,
            _ => not_a_file(),
        }
    }
}
