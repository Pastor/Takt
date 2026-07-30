//! Переход к декларации, в том числе в импортированный файл (фича 0056).
//!
//! Часть модуля `lsp` (фича 0027: деление по логике).

use super::*;

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
    use crate::diagnostics::ROOT_FILE_NO;

    let (ast, _) = crate::parse(source, 0).ok()?;
    let model = semantic::tree::construct_model(&ast, None, &[]).ok()?;
    let node = node_at_position(source, position, &model)?;
    let DiagLoc::Source(file_no, start, end) = declaration_location_of(&node)? else {
        return None;
    };
    // Однофайловый вариант отдаёт диапазон в СВОЁМ тексте: сказать, где искать
    // чужой файл, ему нечем (путей поиска нет, реестра нет).
    (file_no as u64 == ROOT_FILE_NO).then(|| offset_to_range(source, start as usize, end as usize))
}

/// Разрешает позицию декларации с поддержкой кросс-файловых переходов.
///
/// Расширенная версия [`goto_declaration`], принимающая пути поиска импортов.
/// Строит семантическую модель (с разрешением импортов) и возвращает
/// [`Location`] объявления идентификатора под курсором.
///
/// ## Алгоритм
///
/// 1. Ищет узел по позиции — среди узлов **своего** файла (индекс различает
///    файлы по паре `(file_no, offset)`, фича 0056).
/// 2. Находит позицию объявления ([`declaration_location_of`]). Она несёт номер
///    файла.
/// 3. Свой файл — возвращает диапазон по своему тексту, URI пустой (его
///    подставляет вызывающий). Чужой — берёт **путь из реестра**
///    ([`FileTable::path`]) и считает диапазон по тексту целевого файла.
///
/// ## Почему путь берётся из реестра, а не угадывается
///
/// Прежняя редакция строила кандидатов из **имени модели**
/// (`to_snake_case(name) + ".takt"`) и перебирала `search_paths`. Это неверно
/// в принципе: `import "engine.takt" as Motor;` связывает имя `Motor` с файлом
/// `engine.takt`, и кандидат `motor.takt` не совпадёт **никогда**. Реестр 0053
/// хранит точный путь по `file_no` — гадать не о чем.
///
/// ## Ограничения
///
/// - Переход находит лишь то, что разрешает неявный путь импорта (каталог
///   документа) и `search_paths`: LSP не читает `initializationOptions`
///   (кандидат из 0055).
///
/// Возвращает `None` если:
/// - исходный текст не компилируется,
/// - курсор вне идентификатора,
/// - объявление не имеет позиции в тексте (встроенное/порождённое),
/// - путь целевого файла неизвестен реестру либо файл не читается.
pub fn goto_declaration_with_paths(
    source: &str,
    position: Position,
    search_paths: &[String],
) -> Option<Location> {
    // Путь документа неизвестен → неявный путь импорта (каталог документа,
    // фича 0055) не работает: разрешится только то, что дают `search_paths`.
    goto_declaration_at("", source, position, search_paths)
}

/// Переход к декларации **с путём открытого документа** — вход для сервера.
///
/// Зачем путь: каталог документа — **неявный путь поиска импортов** (фича 0055).
/// Без него `import "helper.takt";` не разрешится, даже когда файл лежит рядом, и
/// переходить будет некуда. Тот же довод и та же форма, что у
/// [`collect_diagnostics_at`].
///
/// `search_paths` — дополнительные каталоги (как `-I` у `taktc`).
pub fn goto_declaration_at(
    path: &str,
    source: &str,
    position: Position,
    search_paths: &[String],
) -> Option<Location> {
    use crate::diagnostics::{FileTable, ROOT_FILE_NO};

    let (ast, _) = crate::parse(source, 0).ok()?;
    let mut files = FileTable::new(path);
    let model =
        semantic::tree::construct_model_with_files(&ast, None, search_paths, &mut files, false)
            .ok()?;
    let node = node_at_position(source, position, &model)?;

    let DiagLoc::Source(file_no, start, end) = declaration_location_of(&node)? else {
        return None;
    };

    // Свой файл: диапазон считается по уже имеющемуся тексту, URI подставит
    // вызывающий — контракт сохранён (ADR 0056, A4).
    if file_no as u64 == ROOT_FILE_NO {
        return Some(Location {
            uri: String::new(),
            range: offset_to_range(source, start as usize, end as usize),
        });
    }

    // Чужой файл: точный путь — из реестра; диапазон — по ЕГО тексту (смещения
    // чужого файла к своему не относятся).
    let path = files.path(file_no as u64)?;
    let target_source = std::fs::read_to_string(path).ok()?;
    let uri = std::path::Path::new(path)
        .canonicalize()
        .map(|p| format!("file://{}", p.display()))
        .unwrap_or_else(|_| format!("file://{}", path));
    Some(Location {
        uri,
        range: offset_to_range(&target_source, start as usize, end as usize),
    })
}

/// Возвращает **позицию объявления** для семантического узла.
///
/// Для декларационных видов узла `loc` уже указывает на объявление. Для
/// использований (`Reference`, `ReferenceCondition`, `ReferenceModel`) ищет
/// целевой элемент в модели по имени.
///
/// # Почему `Location`, а не `Range`
///
/// [`Location`] несёт **номер файла**, а [`Range`] — только строку и колонку.
/// Прежняя редакция возвращала `Range`, вычисляя его по тексту **текущего**
/// документа, — даже когда объявление найдено в импортированном файле. Смещения
/// чужого файла накладывались на свой текст, давая мусорный диапазон, и
/// кросс-файловая ветка ниже по коду до работы не доходила: ранний возврат уже
/// отдал «успех» (фича 0056).
///
/// Решать, чей это файл, обязан вызывающий: у него есть реестр путей.
fn declaration_location_of(node: &SemanticNodeRef) -> Option<DiagLoc> {
    use crate::semantic::index::SemanticNodeKind::*;

    let loc = match node.kind {
        // Декларационные виды: loc уже указывает на объявление
        Variable | Const | Port | Function | ExternFunction | State | StartState | EndState
        | TypeAlias | Condition | Enum | Model | LocalVar => node.loc,
        // Ссылка-переход (`ref Имя`) ИЛИ имя состояния в условии (`S(Ping) = End`,
        // фича 0071): ищем декларацию целевого состояния тем же поиском, каким
        // состояние разрешилось (`search_state` в области условия).
        Reference | ReferenceState => {
            let model_rc = node.model.as_ref()?.clone();
            let state_rc = model_rc.borrow().search_state(&node.name)?;

            state_rc.borrow().loc()
        }
        // Ссылка на модель (`= Helper`, `S(Helper)`): единственный вид, способный
        // указать в другой файл — имя из `import` связано с корнем чужого файла.
        ReferenceModel => {
            let model_rc = node.model.as_ref()?.clone();
            let target = model_rc.borrow().search_model(&node.name)?;

            target.borrow().loc
        }
        // Использование в условии перехода: ищем переменную или функцию
        ReferenceCondition => {
            let model_rc = node.model.as_ref()?.clone();
            let model = model_rc.borrow();
            if let Some(var) = model.search_var(&node.name) {
                var.loc()
            } else {
                let func_rc = model.search_func(&node.name)?;

                func_rc.borrow().loc()
            }
        }
    };
    // Позиции без файла (`Codegen`/`Implicit`/`Builtin`) переходу не годятся:
    // объявления в тексте нет.
    matches!(loc, DiagLoc::Source(_, _, _)).then_some(loc)
}
