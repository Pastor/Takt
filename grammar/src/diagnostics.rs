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
//! ## Коды ошибок
//!
//! Каждая диагностика может содержать код вида `XX-YYY`, где:
//! - `LE` — лексические ошибки (лексер)
//! - `SY` — синтаксические ошибки (парсер)
//! - `SE` — семантические ошибки (анализ)
//! - `CC` — ошибки кодогенерации Си
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
    /// Семантическая ошибка.
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
/// категорию ошибки, основной текст, опциональный код ошибки и список замечаний.
///
/// # Коды ошибок
///
/// Поле [`code`](Diagnostic::code) содержит строку вида `XX-YYY` (например, `LE-001`),
/// однозначно идентифицирующую тип ошибки. Используйте [`Diagnostic::with_code`]
/// для назначения кода существующей диагностике.
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
    /// Код ошибки в формате `XX-YYY` (например, `LE-001`, `SE-005`).
    /// `None` если код не назначен.
    pub code: Option<String>,
    /// Вспомогательные заметки.
    pub notes: Vec<Note>,
}

impl From<&str> for Diagnostic {
    fn from(s: &str) -> Diagnostic {
        Diagnostic {
            loc: Default::default(),
            level: Level::Error,
            ty: ErrorType::SematicError,
            message: s.to_string(),
            code: None,
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
            code: None,
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
            code: None,
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
            code: None,
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
            code: None,
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
            code: None,
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
            code: None,
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
            code: None,
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
            code: None,
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
            code: None,
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
            code: None,
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
            code: None,
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
            code: None,
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
            code: None,
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
            code: None,
            notes,
        }
    }

    /// Назначает код ошибки и возвращает изменённую диагностику (builder-метод).
    ///
    /// Код должен иметь формат `XX-YYY`, например `LE-001`, `SE-015`, `CC-003`.
    ///
    /// # Пример
    ///
    /// ```
    /// use grammar::diagnostics::{Diagnostic, Location};
    ///
    /// let d = Diagnostic::error(Location::Builtin, "ошибка".to_string())
    ///     .with_code("SE-001");
    /// assert_eq!(d.code.as_deref(), Some("SE-001"));
    /// ```
    #[must_use]
    pub fn with_code(mut self, code: &str) -> Self {
        self.code = Some(code.to_string());
        self
    }

    /// Возвращает форматированный префикс кода для вывода, например `[SE-001] `.
    /// Если код не задан, возвращает пустую строку.
    pub fn code_prefix(&self) -> String {
        self.code
            .as_deref()
            .map(|c| format!("[{}] ", c))
            .unwrap_or_default()
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
                Location::Source(*filename, *start, *start)
            }
            loc => *loc,
        }
    }

    /// Возвращает [`Location`] нулевой длины, указывающий на конец данного диапазона.
    #[inline]
    pub fn end_range(&self) -> Self {
        match self {
            Location::Source(filename, _, end) => Location::Source(*filename, *end, *end),
            loc => *loc,
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

    // ── V5: Безопасный API для не-файловых вариантов Location ─────────────────

    /// Возвращает `Some(start)` для [`Location::Source`], `None` для остальных.
    ///
    /// Используйте вместо [`start()`](Location::start) там, где Location может
    /// быть [`Builtin`](Location::Builtin), [`CommandLine`](Location::CommandLine),
    /// [`Implicit`](Location::Implicit) или [`Codegen`](Location::Codegen).
    ///
    /// # Примеры
    ///
    /// ```
    /// use grammar::diagnostics::Location;
    ///
    /// let src = Location::Source(0, 3, 8);
    /// assert_eq!(src.try_start(), Some(3));
    ///
    /// let imp = Location::Implicit;
    /// assert_eq!(imp.try_start(), None);
    /// ```
    #[inline]
    pub fn try_start(&self) -> Option<usize> {
        match self {
            Location::Source(_, start, _) => Some(*start),
            _ => None,
        }
    }

    /// Возвращает `Some(end)` для [`Location::Source`], `None` для остальных.
    ///
    /// Используйте вместо [`end()`](Location::end) там, где Location может
    /// быть не-файловым вариантом.
    ///
    /// # Примеры
    ///
    /// ```
    /// use grammar::diagnostics::Location;
    ///
    /// let src = Location::Source(0, 3, 8);
    /// assert_eq!(src.try_end(), Some(8));
    ///
    /// let cmd = Location::CommandLine;
    /// assert_eq!(cmd.try_end(), None);
    /// ```
    #[inline]
    pub fn try_end(&self) -> Option<usize> {
        match self {
            Location::Source(_, _, end) => Some(*end),
            _ => None,
        }
    }

    /// Возвращает `Some(start..end)` для [`Location::Source`], `None` для остальных.
    ///
    /// Используйте вместо [`range()`](Location::range) там, где Location может
    /// быть [`Builtin`](Location::Builtin), [`Codegen`](Location::Codegen) и т.д.
    ///
    /// # Примеры
    ///
    /// ```
    /// use grammar::diagnostics::Location;
    ///
    /// let src = Location::Source(0, 3, 8);
    /// assert_eq!(src.try_range(), Some(3..8));
    ///
    /// let builtin = Location::Builtin;
    /// assert_eq!(builtin.try_range(), None);
    /// ```
    #[inline]
    pub fn try_range(&self) -> Option<std::ops::Range<usize>> {
        match self {
            Location::Source(_, start, end) => Some(*start..*end),
            _ => None,
        }
    }
}

// ─── V5: Тесты безопасного API ────────────────────────────────────────────────

#[cfg(test)]
mod tests_v5_location {
    use super::Location;

    /// V5: try_start() для Source возвращает начало диапазона.
    #[test]
    fn try_start_source_returns_start() {
        assert_eq!(Location::Source(0, 10, 20).try_start(), Some(10));
    }

    /// V5: try_start() для Implicit возвращает None (не паникует).
    #[test]
    fn try_start_implicit_returns_none() {
        assert_eq!(Location::Implicit.try_start(), None);
    }

    /// V5: try_start() для Builtin возвращает None (не паникует).
    #[test]
    fn try_start_builtin_returns_none() {
        assert_eq!(Location::Builtin.try_start(), None);
    }

    /// V5: try_start() для CommandLine возвращает None (не паникует).
    #[test]
    fn try_start_commandline_returns_none() {
        assert_eq!(Location::CommandLine.try_start(), None);
    }

    /// V5: try_start() для Codegen возвращает None (не паникует).
    #[test]
    fn try_start_codegen_returns_none() {
        assert_eq!(Location::Codegen.try_start(), None);
    }

    /// V5: try_end() для Source возвращает конец диапазона.
    #[test]
    fn try_end_source_returns_end() {
        assert_eq!(Location::Source(0, 3, 8).try_end(), Some(8));
    }

    /// V5: try_end() для Implicit возвращает None (не паникует).
    #[test]
    fn try_end_implicit_returns_none() {
        assert_eq!(Location::Implicit.try_end(), None);
    }

    /// V5: try_range() для Source возвращает диапазон start..end.
    #[test]
    fn try_range_source_returns_range() {
        assert_eq!(Location::Source(0, 3, 8).try_range(), Some(3..8));
    }

    /// V5: try_range() для Builtin возвращает None (не паникует).
    #[test]
    fn try_range_builtin_returns_none() {
        assert_eq!(Location::Builtin.try_range(), None);
    }

    /// V5: try_range() для Codegen возвращает None (не паникует).
    #[test]
    fn try_range_codegen_returns_none() {
        assert_eq!(Location::Codegen.try_range(), None);
    }

    /// V5: Инвариант — start() паникует для Implicit (документированное поведение).
    #[test]
    #[should_panic(expected = "местоположение не является файловой позицией")]
    fn start_implicit_panics_as_documented() {
        let _ = Location::Implicit.start();
    }

    /// V5: Инвариант — end() паникует для Builtin (документированное поведение).
    #[test]
    #[should_panic(expected = "местоположение не является файловой позицией")]
    fn end_builtin_panics_as_documented() {
        let _ = Location::Builtin.end();
    }

    /// V5: Инвариант — range() паникует для Codegen (документированное поведение).
    #[test]
    #[should_panic(expected = "местоположение не является файловой позицией")]
    fn range_codegen_panics_as_documented() {
        let _ = Location::Codegen.range();
    }
}
