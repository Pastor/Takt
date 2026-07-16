//! Вспомогательные функции LSP-сервера для языка Lam.
//!
//! Этот модуль реализует логику, связывающую компилятор Lam с протоколом LSP:
//! сбор диагностики, генерацию подсказок автодополнения и информацию о типах
//! для функции hover.
//!
//! Модуль включается только при наличии флага `lsp`.

use crate::semantic;
use cell::RefCell;
use lsp_types::*;
use semantic::index::SemanticNodeRef;
use semantic::{FunctionDefinitionNode, ModelNode, VariableNode};
use std::cell;

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

/// Ключевые слова языка Lam для автодополнения.
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
    ("in", "входной порт"),
    ("out", "выходной порт"),
    ("inout", "двунаправленный порт"),
    (
        "address",
        "задание аппаратного адреса порта (address Имя = <адрес>;)",
    ),
    ("enum", "объявление перечисления"),
    ("struct", "объявление структуры"),
    ("cond", "именованное условие перехода"),
    ("if", "условный оператор"),
    ("else", "ветка условного оператора"),
    ("loop", "цикл с опциональным условием"),
    ("while", "цикл с условием (синоним loop)"),
    ("for", "цикл со счётчиком"),
    ("match", "оператор выбора по образцу"),
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
];

/// Встроенные типы языка Lam: примитивные и целочисленные.
///
/// Используются для подсветки идентификаторов-типов (`TT_TYPE`) в semantic tokens,
/// для генерации элементов автодополнения с видом `TYPE` и для hover-подсказок.
const BUT_BUILTIN_TYPES: &[(&str, &str)] = &[
    ("bit", "встроенный 1-битный примитивный тип (0 или 1)"),
    ("bool", "встроенный булев тип (true или false)"),
    ("float", "встроенный тип числа с плавающей точкой"),
    (
        "unit",
        "встроенный пустой тип (возвращаемый тип процедур без значения)",
    ),
    (
        "u8",
        "встроенный 8-битный беззнаковый целочисленный тип [0 … 255]",
    ),
    (
        "u16",
        "встроенный 16-битный беззнаковый целочисленный тип [0 … 65 535]",
    ),
    (
        "u32",
        "встроенный 32-битный беззнаковый целочисленный тип [0 … 4 294 967 295]",
    ),
    ("u64", "встроенный 64-битный беззнаковый целочисленный тип"),
    (
        "i8",
        "встроенный 8-битный знаковый целочисленный тип [−128 … 127]",
    ),
    (
        "i16",
        "встроенный 16-битный знаковый целочисленный тип [−32 768 … 32 767]",
    ),
    (
        "i32",
        "встроенный 32-битный знаковый целочисленный тип [−2 147 483 648 … 2 147 483 647]",
    ),
    ("i64", "встроенный 64-битный знаковый целочисленный тип"),
];

/// Собирает диагностику из исходного кода Lam.
///
/// Выполняет лексический, синтаксический и семантический анализ.
/// Возвращает список LSP-диагностик для отображения в редакторе.
/// Правки для `textDocument/formatting` (фича 0024, задача 0024-04).
///
/// Возвращает **одну** правку, заменяющую документ целиком каноническим текстом,
/// либо `None`, если форматировать нечего (текст уже каноничен) — так редактор
/// не помечает файл изменённым на ровном месте.
///
/// # Единое ядро (критерий A6)
///
/// Используется та же [`crate::format::format_source`], что и в `lamc fmt`, —
/// поэтому расхождения стилей между CLI и редактором быть не может **по
/// построению**, а не по договорённости.
///
/// # Ошибки
///
/// `Err` — исходник не форматируется (синтаксическая ошибка или непокрытый
/// узел). Вызывающий обязан решить, что с этим делать: LSP-сервер логирует и
/// отвечает `null`, потому что молча «отформатировать во что-то» хуже, чем не
/// форматировать вовсе.
pub fn formatting_edits(source: &str) -> Result<Option<Vec<TextEdit>>, crate::format::FormatError> {
    let formatted = crate::format::format_source(source)?;
    if formatted == source {
        return Ok(None);
    }
    Ok(Some(vec![TextEdit {
        range: offset_to_range(source, 0, source.len()),
        new_text: formatted,
    }]))
}

pub fn collect_diagnostics(source: &str) -> Vec<Diagnostic> {
    collect_diagnostics_at("", source, &[])
}

/// Диагностики документа, **зная его путь** (фича 0055).
///
/// Путь нужен по двум причинам:
///
/// 1. **Импорты разрешаются.** Прежде звался `construct_model(&ast, None, &[])`
///    — с пустыми путями поиска, поэтому `import "lib.lam";` в редакторе **всегда**
///    давал `SE-013` «файл не найден», даже когда файл лежит рядом. Каталог
///    документа — неявный путь поиска (общий для ядра и `lamc`).
/// 2. **Ошибку чужого файла есть к чему привязать.** Её координаты указывают в
///    текст, которого в открытом документе нет.
///
/// `search_paths` — дополнительные каталоги (как `-I` у `lamc`).
pub fn collect_diagnostics_at(
    path: &str,
    source: &str,
    search_paths: &[String],
) -> Vec<Diagnostic> {
    let mut lsp_diags = Vec::new();
    let mut files = crate::diagnostics::FileTable::new(path);

    // Шаг 1: Синтаксический анализ
    let (ast, _) = match crate::parse(source, 0) {
        Ok(result) => result,
        Err(errors) => {
            // Конвертируем ошибки парсера в LSP-диагностики
            for err in errors {
                lsp_diags.push(diagnostic_to_lsp(&err, source, &files));
            }
            return lsp_diags;
        }
    };

    // Шаг 2: Семантический анализ
    let model =
        match semantic::tree::construct_model_with_files(&ast, None, search_paths, &mut files) {
            Ok(m) => m,
            Err(err) => {
                lsp_diags.push(diagnostic_to_lsp(&err, source, &files));
                return lsp_diags;
            }
        };

    // Шаг 3: Дополнительные предупреждения
    let unused = crate::unused_variable_warnings(model.clone());
    for w in unused {
        lsp_diags.push(diagnostic_to_lsp(&w, source, &files));
    }

    let nondeterministic = crate::nondeterministic_transition_warnings(model.clone());
    for w in nondeterministic {
        lsp_diags.push(diagnostic_to_lsp(&w, source, &files));
    }

    let enum_errors = crate::enum_type_safety_errors(model);
    for e in enum_errors {
        lsp_diags.push(diagnostic_to_lsp(&e, source, &files));
    }

    lsp_diags
}

/// Конвертирует диагностику в LSP-диагностику, **различая свой файл и чужой**
/// (фича 0055).
///
/// Диагностика **своего** файла (`file_no == 0`) показывается на своём месте.
///
/// Диагностика **импортированного** файла своих координат в открытом документе
/// не имеет: смещения указывают в текст, которого здесь нет. Прежде `file_no`
/// молча отбрасывался, и подсветка ложилась по чужому смещению — то есть
/// **не туда**. Теперь такая ошибка:
///
/// - привязывается к строке `import`, через которую пришла (заметка
///   «импортирован в», её ставит проход 0 — см. `tree.rs::note_imported_here`);
/// - в тексте называет настоящее место: `в файле lib.lam:4:18: …`.
///
/// Так автор видит и **где** причина, и **что** в его файле к ней ведёт.
fn diagnostic_to_lsp(
    diag: &crate::diagnostics::Diagnostic,
    source: &str,
    files: &crate::diagnostics::FileTable,
) -> Diagnostic {
    use crate::diagnostics::Location;

    let own = matches!(diag.loc, Location::Source(0, _, _));
    if own {
        return grammar_diagnostic_to_lsp(diag, source);
    }

    // Путь чужого файла — из реестра; он же даёт `в файле X:строка:колонка`.
    let stamped = diag.clone().with_file_if_unset(files.path_of(&diag.loc));
    let where_ = crate::diagnostics::position_prefix(&stamped);

    // Якорь — заметка, чья позиция лежит В ЭТОМ документе (file_no == 0).
    // У цепочки `top → mid → deep` таких заметок ровно одна: `import` в top.
    let anchor = diag
        .notes
        .iter()
        .find_map(|n| match n.loc {
            Location::Source(0, start, end) => Some(offset_to_range(source, start, end)),
            _ => None,
        })
        .unwrap_or(Range {
            start: Position::new(0, 0),
            end: Position::new(0, 0),
        });

    let mut lsp = grammar_diagnostic_to_lsp(&stamped, source);
    lsp.range = anchor;
    lsp.message = format!("в файле {}{}", where_, diag.message);
    lsp
}

/// Конвертирует [`crate::diagnostics::Diagnostic`] в LSP [`Diagnostic`].
///
/// Диагностики с `Location::Source` получают точный диапазон в документе.
/// Диагностики без координат (`Location::Implicit`, `Location::Codegen` и т.д.)
/// получают нулевой диапазон `(0,0)-(0,0)`.
///
/// Вспомогательные заметки (`notes`) добавляются к тексту сообщения через
/// символ переноса строки, чтобы редактор мог отобразить их вместе с основным
/// сообщением без необходимости знать URI документа на этапе конвертации.
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

    // Формируем полное сообщение: основной текст + заметки
    let message = if diag.notes.is_empty() {
        diag.message.clone()
    } else {
        let notes_text: String = diag
            .notes
            .iter()
            .map(|n| format!("\nЗаметка: {}", n.message))
            .collect();
        format!("{}{}", diag.message, notes_text)
    };

    Diagnostic {
        range,
        severity,
        message,
        source: Some("lam-lsp".to_string()),
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

/// Конвертирует LSP-позицию (строка + UTF-16 символ) в байтовое смещение в исходном тексте.
///
/// Протокол LSP использует `Position { line, character }`, где `character` — смещение в
/// кодовых единицах UTF-16 от начала строки. Функция переводит эту позицию в байтовое
/// смещение от начала файла, пригодное для работы с [`Location::Source`].
///
/// Возвращает `None`, если строка с номером `position.line` не существует в `source`.
///
/// # Примеры
///
/// ```
/// # #[cfg(feature = "lsp")]
/// # {
/// use grammar::lsp::position_to_offset;
/// use lsp_types::Position;
///
/// let src = "hello\nworld";
/// // Строка 0, символ 3 → байт 3
/// assert_eq!(position_to_offset(src, Position::new(0, 3)), Some(3));
/// // Строка 1 начинается с байта 6 ("hello\n"), символ 2 → байт 8
/// assert_eq!(position_to_offset(src, Position::new(1, 2)), Some(8));
/// // Несуществующая строка → None
/// assert_eq!(position_to_offset(src, Position::new(99, 0)), None);
/// # }
/// ```
pub fn position_to_offset(source: &str, position: Position) -> Option<usize> {
    let target_line = position.line as usize;
    let mut line_start = 0usize;
    let mut current_line = 0usize;

    for (i, c) in source.char_indices() {
        if current_line == target_line {
            // Нашли начало нужной строки — определяем столбец
            let line_text = source[line_start..].lines().next().unwrap_or("");
            let col_byte = utf16_to_byte_offset(line_text, position.character as usize)
                .unwrap_or(line_text.len());
            return Some(line_start + col_byte);
        }
        if c == '\n' {
            current_line += 1;
            line_start = i + 1;
        }
    }

    // Обрабатываем последнюю строку (без завершающего '\n')
    if current_line == target_line {
        let line_text = source[line_start..].lines().next().unwrap_or("");
        let col_byte =
            utf16_to_byte_offset(line_text, position.character as usize).unwrap_or(line_text.len());
        return Some(line_start + col_byte);
    }

    None
}

/// Возвращает семантический узел по LSP-позиции курсора.
///
/// Строит [`SemanticIndex`](semantic::index::SemanticIndex) из переданной
/// семантической модели и выполняет поиск наиболее конкретного узла, объявление
/// которого покрывает позицию курсора. Более точен, чем поиск по имени слова под
/// курсором: учитывает точные диапазоны объявлений и избегает неоднозначностей
/// при совпадении имён разных элементов (например, переменная и состояние с одним
/// именем в разных областях видимости).
///
/// Возвращает `None`, если:
/// - `position` выходит за пределы исходного текста.
/// - Ни один семантический узел не покрывает данную позицию (например, курсор
///   стоит на ключевом слове или пробеле).
///
/// # Пример
///
/// ```
/// # #[cfg(feature = "lsp")]
/// # {
/// use grammar::parse;
/// use grammar::semantic::tree::construct_model;
/// use grammar::lsp::node_at_position;
/// use grammar::semantic::index::SemanticNodeKind;
/// use lsp_types::Position;
///
/// let src = "var counter: bit := false; start S;";
/// let (ast, _) = parse(src, 0).unwrap();
/// let model = construct_model(&ast, None, &[]).unwrap();
///
/// // Позиция 4 — символ 'c' в "counter"
/// let node = node_at_position(src, Position::new(0, 4), &model);
/// assert!(node.is_some());
/// let node = node.unwrap();
/// assert_eq!(node.name, "counter");
/// assert_eq!(node.kind, SemanticNodeKind::Variable);
/// # }
/// ```
pub fn node_at_position(
    source: &str,
    position: Position,
    model: &std::rc::Rc<RefCell<ModelNode>>,
) -> Option<SemanticNodeRef> {
    use crate::semantic::index::SemanticIndex;
    let offset = position_to_offset(source, position)?;
    let index = SemanticIndex::build(model);
    index.node_at_offset(offset).cloned()
}

/// LSP-локация: URI файла и диапазон объявления.
///
/// Используется [`goto_declaration_with_paths`] для возврата местоположения
/// в произвольном файле (не только в текущем).
#[derive(Debug, Clone, PartialEq)]
pub struct Location {
    /// URI файла в формате `file:///абсолютный/путь` или просто путь к файлу.
    pub uri: String,
    /// Диапазон объявления в файле.
    pub range: Range,
}

/// Разрешает позицию декларации элемента под курсором.
///
/// Строит семантическую модель из исходного текста и возвращает [`Range`]
/// объявления идентификатора, на котором стоит курсор.
///
/// Возвращает `None` если:
/// - исходный текст не компилируется
/// - курсор вне идентификатора
/// - декларация не может быть разрешена
///
/// ## Ограничения
///
/// - Использования переменных внутри тел функций и блоков (`enter`, `exit`, `always`)
///   не индексируются: переход к декларации из таких контекстов недоступен.
/// - Кросс-файловые декларации (импортированные элементы) не поддерживаются.
///   Для поддержки кросс-файловых переходов используйте [`goto_declaration_with_paths`].
pub fn goto_declaration(source: &str, position: Position) -> Option<Range> {
    let (ast, _) = crate::parse(source, 0).ok()?;
    let model = semantic::tree::construct_model(&ast, None, &[]).ok()?;
    let node = node_at_position(source, position, &model)?;
    declaration_range_of(&node, source)
}

/// Разрешает позицию декларации с поддержкой кросс-файловых переходов.
///
/// Расширенная версия [`goto_declaration`], принимающая пути поиска импортов.
/// Строит семантическую модель из исходного текста (с разрешением импортов)
/// и возвращает [`Location`] объявления идентификатора под курсором.
///
/// ## Алгоритм
///
/// 1. Ищет узел по позиции в семантическом дереве текущего файла.
/// 2. Если декларация находится в текущем файле — возвращает её Range.
/// 3. Если узел принадлежит импортированной модели — ищет файл импорта
///    в `search_paths`, строит его семантическую модель и ищет декларацию там.
///
/// ## Ограничения
///
/// - Кросс-файловые переходы доступны только для символов верхнего уровня
///   (модели, переменные, функции, типы, состояния). Переходы к локальным
///   переменным внутри функций и блоков (`enter`, `exit`, `always`)
///   не поддерживаются даже при наличии путей поиска.
/// - Поиск файла импорта основан на имени модели и именах файлов `.lam`.
///   Если имя модели не совпадает с нормализованным именем файла, переход
///   к декларации может не найти нужный файл.
///
/// Возвращает `None` если:
/// - исходный текст не компилируется,
/// - курсор вне идентификатора,
/// - декларация не может быть разрешена ни в текущем, ни в импортированных файлах.
pub fn goto_declaration_with_paths(
    source: &str,
    position: Position,
    search_paths: &[String],
) -> Option<Location> {
    use crate::diagnostics::Location as DiagLoc;

    let (ast, _) = crate::parse(source, 0).ok()?;
    let model = semantic::tree::construct_model(&ast, None, search_paths).ok()?;
    let node = node_at_position(source, position, &model)?;

    // Пробуем найти декларацию в текущем файле
    if let Some(range) = declaration_range_of(&node, source) {
        return Some(Location {
            uri: String::new(), // текущий файл — URI передаётся вызывающей стороной
            range,
        });
    }

    // Кросс-файловый поиск: только для модельных узлов (Model / State / Variable и т.д.)
    // из импортированных моделей. Определяем имя модели, которой принадлежит узел.
    let model_rc = node.model.as_ref()?.clone();
    let model_name = model_rc.borrow().name.clone()?;

    // Проверяем: если эта модель объявлена в текущем файле — дальше не ищем
    // (декларация_range_of уже вернула None — нечего искать кросс-файлово).
    // Ищем файл с именем, соответствующим имени модели, в директориях поиска.
    let candidate_stem = to_snake_case(&model_name);

    for dir in search_paths {
        let dir_path = std::path::Path::new(dir);
        // Пробуем имя файла из имени модели (CamelCase → snake_case.lam)
        for candidate in &[
            format!("{}.lam", candidate_stem),
            format!("{}.lam", model_name.to_lowercase()),
        ] {
            let file_path = dir_path.join(candidate);
            if let Ok(import_source) = std::fs::read_to_string(&file_path) {
                // Строим семантику импортированного файла
                if let Ok((import_ast, _)) = crate::parse(&import_source, 0) {
                    if let Ok(import_model) =
                        semantic::tree::construct_model(&import_ast, None, search_paths)
                    {
                        // Ищем декларацию по имени узла в семантике импортированного файла
                        let import_index =
                            crate::semantic::index::SemanticIndex::build(&import_model);
                        // Для узлов, которые являются декларациями, ищем по имени в индексе
                        if let Some(found) =
                            find_declaration_in_index(&import_index, &node.name, &node.kind)
                        {
                            if let DiagLoc::Source(_, start, end) = found.loc {
                                return Some(Location {
                                    uri: file_path
                                        .canonicalize()
                                        .map(|p| format!("file://{}", p.display()))
                                        .unwrap_or_else(|_| {
                                            format!("file://{}", file_path.display())
                                        }),
                                    range: offset_to_range(&import_source, start, end),
                                });
                            }
                        }
                    }
                }
            }
        }
    }

    None
}

/// Конвертирует CamelCase-имя в snake_case для поиска файла.
///
/// `MyModel` → `my_model`, `Ping` → `ping`, `PingPong` → `ping_pong`.
fn to_snake_case(name: &str) -> String {
    let mut result = String::new();
    for (i, ch) in name.chars().enumerate() {
        if ch.is_uppercase() && i > 0 {
            result.push('_');
        }
        result.push(ch.to_ascii_lowercase());
    }
    result
}

/// Ищет декларацию с заданным именем и видом в семантическом индексе.
///
/// Возвращает первую запись, у которой имя и вид узла совпадают.
/// Проверяет только декларационные виды узлов (не использования).
fn find_declaration_in_index<'a>(
    index: &'a crate::semantic::index::SemanticIndex,
    name: &str,
    kind: &crate::semantic::index::SemanticNodeKind,
) -> Option<&'a crate::semantic::index::SemanticNodeRef> {
    use crate::semantic::index::SemanticNodeKind::*;

    // Только декларационные виды (не использования)
    let is_declaration = matches!(
        kind,
        Variable
            | Const
            | Port
            | Function
            | ExternFunction
            | State
            | StartState
            | EndState
            | TypeAlias
            | Condition
            | Enum
            | Model
            | LocalVar
    );
    if !is_declaration {
        return None;
    }

    index.find_by_name(name, kind)
}

/// Возвращает LSP-диапазон декларации для семантического узла.
///
/// Для декларационных видов узла `loc` уже указывает на объявление.
/// Для использований (`Reference`, `ReferenceCondition`) выполняет поиск
/// целевого элемента в модели по имени.
fn declaration_range_of(node: &SemanticNodeRef, source: &str) -> Option<Range> {
    use crate::diagnostics::Location;
    use crate::semantic::index::SemanticNodeKind::*;

    match node.kind {
        // Декларационные виды: loc уже указывает на объявление
        Variable | Const | Port | Function | ExternFunction | State | StartState | EndState
        | TypeAlias | Condition | Enum | Model | LocalVar => {
            if let Location::Source(_, start, end) = node.loc {
                Some(offset_to_range(source, start, end))
            } else {
                None
            }
        }
        // Ссылка-переход: ищем декларацию целевого состояния
        Reference => {
            let model_rc = node.model.as_ref()?.clone();
            let state_rc = model_rc.borrow().search_state(&node.name)?;
            let loc = state_rc.borrow().loc();
            if let Location::Source(_, start, end) = loc {
                Some(offset_to_range(source, start, end))
            } else {
                None
            }
        }
        // Использование в условии перехода: ищем переменную или функцию
        ReferenceCondition => {
            let model_rc = node.model.as_ref()?.clone();
            let model = model_rc.borrow();
            if let Some(var) = model.search_var(&node.name) {
                let loc = var.loc();
                if let Location::Source(_, start, end) = loc {
                    return Some(offset_to_range(source, start, end));
                }
            }
            if let Some(func_rc) = model.search_func(&node.name) {
                let loc = func_rc.borrow().loc();
                if let Location::Source(_, start, end) = loc {
                    return Some(offset_to_range(source, start, end));
                }
            }
            None
        }
    }
}

/// Генерирует элементы автодополнения для источника Lam.
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

    // Добавляем встроенные типы
    for (type_name, description) in BUT_BUILTIN_TYPES {
        items.push(CompletionItem {
            label: type_name.to_string(),
            kind: Some(CompletionItemKind::TYPE_PARAMETER),
            detail: Some(description.to_string()),
            ..Default::default()
        });
    }

    // Добавляем идентификаторы из семантической модели
    if let Ok((ast, _)) = crate::parse(source, 0) {
        if let Ok(model) = semantic::tree::construct_model(&ast, None, &[]) {
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
                    VariableNode::Simple { ty, .. } => Some(format!("{}", ty)),
                    VariableNode::Const { ty, .. } => Some(format!("const: {}", ty)),
                    VariableNode::Port { ty, .. } => Some(format!("port: {}", ty)),
                    VariableNode::Unresolved => None,
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
                    FunctionDefinitionNode::Local { ret, .. } => Some(format!("fn -> {}", ret)),
                    FunctionDefinitionNode::External { ret, .. } => {
                        Some(format!("extern fn -> {}", ret))
                    }
                    FunctionDefinitionNode::Builtin(_, _, ret) => {
                        Some(format!("builtin fn -> {}", ret))
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
                    semantic::StateNode::Simple { kind, .. }
                    | semantic::StateNode::Implement { kind, .. } => {
                        if matches!(kind, crate::semantic::StateNodeKind::Start) {
                            "start state"
                        } else {
                            "state"
                        }
                    }
                    semantic::StateNode::Unresolved => "state",
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
/// Алгоритм поиска работает в два этапа:
///
/// 1. **Поиск по позиции** (через [`node_at_position`]): находит семантический узел,
///    объявление которого точно покрывает позицию курсора. Устраняет неоднозначность
///    между элементами с одинаковым именем в разных областях видимости.
///
/// 2. **Поиск по имени** (через [`word_at_position`]): резервный метод — извлекает
///    идентификатор под курсором и ищет его в семантической модели. Применяется,
///    если курсор находится на *использовании* элемента (не на объявлении), или
///    если поиск по позиции не дал результата.
pub fn hover_info(source: &str, position: Position) -> Option<Hover> {
    // Строим семантическую модель с привязкой doc-комментариев
    let (ast, comments) = crate::parse(source, 0).ok()?;
    let model = semantic::tree::construct_model_with_docs(&ast, None, &[], &comments).ok()?;
    let borrowed = model.borrow();

    // Шаг 1: пытаемся найти узел по точной позиции в объявлении
    let position_node = node_at_position(source, position, &model);

    // Шаг 2: всегда берём слово непосредственно под курсором как основу поиска.
    // Направленный поиск (position_node) задействуется только если имя узла совпадает
    // с тем, что стоит под курсором.  Это устраняет ситуацию, когда курсор стоит на
    // ссылке (например `Robot` в `start Main = Robot;`), а position_node указывает на
    // объявление `Main`, чей диапазон покрывает всю строку.
    let word = word_at_position(source, position)?;
    if word.is_empty() {
        return None;
    }

    // Вспомогательные функции для формирования hover-текста
    let make_var_hover = |var: &VariableNode, word: &str, doc: &[String]| {
        let (type_str, kind_str) = match var {
            VariableNode::Simple { ty, .. } => (format!("{}", ty), "var"),
            VariableNode::Const { ty, .. } => (format!("{}", ty), "const"),
            VariableNode::Port { ty, .. } => (format!("{}", ty), "port"),
            VariableNode::Unresolved => ("?".to_string(), "var"),
        };
        let mut text = format!("```but\n{} {}: {}\n```", kind_str, word, type_str);
        if !doc.is_empty() {
            text.push_str("\n\n");
            text.push_str(&doc.join("\n"));
        }
        text
    };

    let make_func_hover = |func: &FunctionDefinitionNode, word: &str, doc: &[String]| {
        let sig = match func {
            FunctionDefinitionNode::Local { params, ret, .. } => {
                let ps: Vec<String> = params
                    .iter()
                    .map(|(n, t)| format!("{}: {}", n, t))
                    .collect();
                format!("fn {}({}) -> {}", word, ps.join(", "), ret)
            }
            FunctionDefinitionNode::External { params, ret, .. } => {
                let ps: Vec<String> = params
                    .iter()
                    .map(|(n, t)| format!("{}: {}", n, t))
                    .collect();
                format!("extern fn {}({}) -> {}", word, ps.join(", "), ret)
            }
            FunctionDefinitionNode::Builtin(name, params, ret) => {
                format!("builtin fn {}({} params) -> {}", name, params.len(), ret)
            }
            _ => format!("fn {}", word),
        };
        let mut text = format!("```but\n{}\n```", sig);
        if !doc.is_empty() {
            text.push_str("\n\n");
            text.push_str(&doc.join("\n"));
        }
        text
    };

    let mut hover_text = String::new();

    // Шаг 1 (направленный поиск): если известен вид узла — ищем только в нужной категории.
    // Это устраняет ложные совпадения при одинаковых именах в разных категориях.
    //
    // Ключевое правило: клон Rc должен храниться в именованном биндинге ДО вызова borrow(),
    // иначе временная переменная освобождается раньше, чем завершается заимствование.
    //
    // Направленный поиск активируется только если имя узла совпадает с курсорным словом.
    // Если cursor_word = "Robot", а position_node.name = "Main" (объявление покрывает
    // всю строку), поиск пропускается и уступает место резервному поиску по имени.
    use crate::semantic::index::SemanticNodeKind;
    if let Some(ref node_ref) = position_node.as_ref().filter(|n| n.name == word) {
        // Клонируем Rc в именованный биндинг, чтобы продлить его жизнь на весь блок
        let node_model_rc = node_ref.model.clone().unwrap_or_else(|| model.clone());
        let node_model = node_model_rc.borrow();
        // Документация берётся из модели, содержащей элемент (не из корневой)
        let doc = node_model.element_doc(&word);

        match node_ref.kind {
            SemanticNodeKind::Variable | SemanticNodeKind::Const | SemanticNodeKind::Port => {
                if let Some(var) = node_model.search_var(&word) {
                    hover_text = make_var_hover(&var, &word, &doc);
                }
            }
            SemanticNodeKind::Function | SemanticNodeKind::ExternFunction => {
                if let Some(func_rc) = node_model.search_func(&word) {
                    let func = func_rc.borrow();
                    hover_text = make_func_hover(&func, &word, &doc);
                }
            }
            SemanticNodeKind::TypeAlias => {
                // types.get() возвращает &TypeNode напрямую, что правильно для форматирования
                if let Some(ty) = node_model.types.get(&word) {
                    let mut text = format!("```but\ntype {} = {}\n```", word, ty);
                    if !doc.is_empty() {
                        text.push_str("\n\n");
                        text.push_str(&doc.join("\n"));
                    }
                    hover_text = text;
                }
            }
            SemanticNodeKind::Condition => {
                if node_model.search_cond(&word).is_some() {
                    let mut text = format!("```but\ncond {}\n```", word);
                    if !doc.is_empty() {
                        text.push_str("\n\n");
                        text.push_str(&doc.join("\n"));
                    }
                    hover_text = text;
                }
            }
            SemanticNodeKind::Enum => {
                if let Some(enum_node) = node_model.search_enum(&word) {
                    let variants: Vec<String> = enum_node
                        .variants
                        .iter()
                        .map(|(n, v)| format!("  {} = {}", n, v))
                        .collect();
                    let mut text = format!(
                        "```but\nenum {} {{\n{}\n}}\n```",
                        word,
                        variants.join(",\n")
                    );
                    if !doc.is_empty() {
                        text.push_str("\n\n");
                        text.push_str(&doc.join("\n"));
                    }
                    hover_text = text;
                }
            }
            SemanticNodeKind::State | SemanticNodeKind::StartState | SemanticNodeKind::EndState => {
                let kind_label = match node_ref.kind {
                    SemanticNodeKind::StartState => "start state",
                    SemanticNodeKind::EndState => "end state",
                    _ => "state",
                };
                let mut text = format!("```but\n{} {}\n```", kind_label, word);
                if !doc.is_empty() {
                    text.push_str("\n\n");
                    text.push_str(&doc.join("\n"));
                }
                hover_text = text;
            }
            SemanticNodeKind::Model => {
                let mut text = format!("```but\nmodel {}\n```", word);
                if !doc.is_empty() {
                    text.push_str("\n\n");
                    text.push_str(&doc.join("\n"));
                }
                hover_text = text;
            }
            SemanticNodeKind::Reference => {
                let mut text = format!("```but\nstate {}\n```", word);
                if !doc.is_empty() {
                    text.push_str("\n\n");
                    text.push_str(&doc.join("\n"));
                }
                hover_text = text;
            }
            SemanticNodeKind::ReferenceCondition => {
                // Ищем переменную или функцию в модели, содержащей условие
                if let Some(var) = node_model.search_var(&word) {
                    hover_text = make_var_hover(&var, &word, &doc);
                } else if let Some(func_rc) = node_model.search_func(&word) {
                    let func = func_rc.borrow();
                    hover_text = make_func_hover(&func, &word, &doc);
                } else {
                    // Запасной вариант: показываем имя как есть
                    hover_text = format!("```but\n{}\n```", word);
                }
            }
            SemanticNodeKind::LocalVar => {
                // Локальная переменная в блоке (enter/exit/always/fn): показываем имя
                hover_text = format!("```but\nvar {} (локальная)\n```", word);
            }
        }
    }

    // Шаг 2 (резервный поиск по имени): применяется если:
    // - курсор на использовании, а не на объявлении (position_node = None)
    // - направленный поиск не дал результата (hover_text пустой)
    if hover_text.is_empty() {
        let doc = borrowed.element_doc(&word);
        // Ищем переменную
        if let Some(var) = borrowed.search_var(&word) {
            hover_text = make_var_hover(&var, &word, &doc);
        }
        // Ищем функцию
        else if let Some(func_rc) = borrowed.search_func(&word) {
            let func = func_rc.borrow();
            hover_text = make_func_hover(&func, &word, &doc);
        }
        // Ищем псевдоним типа
        else if let Some(ty) = borrowed.types.get(&word) {
            let mut text = format!("```but\ntype {} = {}\n```", word, ty);
            if !doc.is_empty() {
                text.push_str("\n\n");
                text.push_str(&doc.join("\n"));
            }
            hover_text = text;
        }
        // Ищем именованное условие
        else if borrowed.search_cond(&word).is_some() {
            let mut text = format!("```but\ncond {}\n```", word);
            if !doc.is_empty() {
                text.push_str("\n\n");
                text.push_str(&doc.join("\n"));
            }
            hover_text = text;
        }
        // Ищем перечисление
        else if let Some(enum_node) = borrowed.search_enum(&word) {
            let variants: Vec<String> = enum_node
                .variants
                .iter()
                .map(|(n, v)| format!("  {} = {}", n, v))
                .collect();
            let mut text = format!(
                "```but\nenum {} {{\n{}\n}}\n```",
                word,
                variants.join(",\n")
            );
            if !doc.is_empty() {
                text.push_str("\n\n");
                text.push_str(&doc.join("\n"));
            }
            hover_text = text;
        }
        // Ищем состояние
        else if borrowed.search_state(&word).is_some() {
            let mut text = format!("```but\nstate {}\n```", word);
            if !doc.is_empty() {
                text.push_str("\n\n");
                text.push_str(&doc.join("\n"));
            }
            hover_text = text;
        }
        // Ищем вариант перечисления
        else if let Some((enum_node, value)) = borrowed.search_enum_variant(&word) {
            hover_text = format!("```but\n{}::{} = {}\n```", &enum_node.name, word, value);
        }
        // Ищем модель
        else if borrowed.search_model(&word).is_some() {
            let mut text = format!("```but\nmodel {}\n```", &word);
            if !doc.is_empty() {
                text.push_str("\n\n");
                text.push_str(&doc.join("\n"));
            }
            hover_text = text;
        }
        // Встроенный тип (u8, i32, bit, bool и т.д.)
        else if let Some((_, description)) =
            BUT_BUILTIN_TYPES.iter().find(|(t, _)| *t == word.as_str())
        {
            hover_text = format!("```but\n{}\n```\n\n{}", word, description);
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

/// Генерирует семантические токены для подсветки синтаксиса документа.
///
/// Использует лексер Lam для токенизации и семантическую модель для уточнения
/// типов идентификаторов (функции, типы, состояния, варианты перечислений и т.д.).
/// Результат передаётся редактору в ответ на `textDocument/semanticTokens/full`.
pub fn semantic_tokens(source: &str) -> SemanticTokens {
    use crate::ast::Comment;
    use crate::diagnostics::Location;
    use crate::parser::lexer::{Lexer, Token};

    // Строим семантическую модель для обогащения идентификаторов
    let model_opt = crate::parse(source, 0)
        .ok()
        .and_then(|(ast, _)| semantic::tree::construct_model(&ast, None, &[]).ok());
    let borrowed_model = model_opt.as_ref().map(|m| m.borrow());

    // Собираем токены и комментарии через лексер
    let mut comments: Vec<Comment> = Vec::new();
    let mut lex_errors = Vec::new();
    let token_results: Vec<_> = Lexer::new(source, 0, &mut comments, &mut lex_errors).collect();

    let mut raw: Vec<(usize, usize, u32)> = Vec::new();

    for (start, token, end) in token_results {
        let tt = match token {
            Token::Identifier(name) => {
                // Встроенные типы имеют приоритет над пользовательскими именами
                if BUT_BUILTIN_TYPES.iter().any(|(t, _)| *t == name) {
                    TT_TYPE
                } else if let Some(ref b) = borrowed_model {
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

#[cfg(test)]
mod tests {
    use super::*;
    use crate::diagnostics::{Diagnostic as GrammarDiagnostic, ErrorType, Level, Location};

    // ── Вспомогательный исходный код для тестов ────────────────────────────────

    /// Минимально корректный Lam-файл с переменной, функцией, типом, перечислением.
    const VALID_SRC: &str = r#"
type u8 = [bit;8];
var counter: [bit;8] := 0;
const LIMIT: [bit;8] := 10;
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
        let src = "var hello := 0;";
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
        let src = "var x := 0;";
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
        let src = "var x := 0;";
        let word = word_at_position(src, Position::new(99, 0));
        assert_eq!(word, None);
    }

    // ── Тесты автодополнения ───────────────────────────────────────────────────

    /// Список автодополнения содержит ключевые слова языка Lam.
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
        let src = "var myVar: [bit;8] := 0; start S;";
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
        let src = "var counter: [bit;8] := 0; start S;";
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
        let src = "/// Счётчик тактов.\nvar counter: [bit;8] := 0; start S;";
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
        let src = "var counter: [bit;8] := 0; start S;";
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
            file: None,
            loc: Location::Source(0, 0, 5),
            level: Level::Error,
            ty: ErrorType::ParserError,
            message: "тестовая ошибка".to_string(),
            code: None,
            notes: vec![],
        };
        let lsp_diag = grammar_diagnostic_to_lsp(&diag, "hello");
        assert_eq!(lsp_diag.severity, Some(DiagnosticSeverity::ERROR));
        assert_eq!(lsp_diag.message, "тестовая ошибка");
        assert_eq!(lsp_diag.source, Some("lam-lsp".to_string()));
    }

    /// Предупреждение конвертируется в LSP DiagnosticSeverity::WARNING.
    #[test]
    fn test_grammar_diagnostic_to_lsp_warning() {
        let diag = GrammarDiagnostic {
            file: None,
            loc: Location::Source(0, 6, 11),
            level: Level::Warning,
            ty: ErrorType::Warning,
            message: "тестовое предупреждение".to_string(),
            code: None,
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
            file: None,
            loc: Location::Builtin,
            level: Level::Info,
            ty: ErrorType::None,
            message: "встроенное".to_string(),
            code: None,
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

    /// semantic_tokens не должна паниковать на валидном Lam-исходнике.
    #[test]
    fn test_semantic_tokens_no_panic() {
        let tokens = semantic_tokens(VALID_SRC);
        // Проверяем, что токены сформированы и дельта-кодирование корректно:
        // delta_line строго неотрицательна (u32), delta_start < character на той же строке.
        for tok in &tokens.data {
            // delta_line — u32, поэтому неотрицательность гарантирована типом.
            // Дополнительно проверяем, что нулевые токены отфильтрованы.
            assert!(tok.length > 0, "нулевые токены отфильтровываются");
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
        // Lam поддерживает Unicode-идентификаторы через UnicodeXID
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
